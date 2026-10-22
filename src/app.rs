use crate::ballot::{BallotProps, ballot_card};
use crate::confirm::{PendingConfirm, confirm_dialog};
use crate::flow_sheet::FlowSheet;
use crate::hero::{HeroProps, hero_header};
use crate::model::{RoundData, SHORTHAND, Side, SpeakingOrder, sheet_for};
use crate::openrouter;
use crate::progress::RoundProgress;
use crate::setup_card::{SetupProps, setup_card};
use crate::shorthand::{InsertCtx, TextTarget, insert_shorthand};
use crate::state::{
    RoundState, load_initial, now_ms, save, save_progress, timer_expired_feedback, unlock_audio,
};
use crate::timer::{TimerBank, TimerKind, ceil_secs, tick_secs};
use crate::topic::{self, TopicAction};
use gloo_timers::callback::Interval;
use leptos::prelude::*;
use leptos::task::spawn_local;

// Pedantic `too_many_lines`: the Leptos root component owns every
// round/timer signal and callback the view closes over; splitting it
// would thread all of them through a child component for no gain.
#[allow(clippy::too_many_lines)]
#[component]
pub fn App() -> impl IntoView {
    // Issue #13: pre-fill an empty topic field with the bundled NSDA
    // resolution. A saved topic is never touched here (non-empty text
    // passes through), and with an unreadable save the in-memory fill
    // cannot overwrite anything: `save` stays blocked until the judge
    // dismisses the recovery notice below.
    let bundled_topic = topic::bundled();
    let mut loaded = load_initial();
    if loaded.data.topic.trim().is_empty() {
        loaded.data.topic.clone_from(&bundled_topic.resolution);
    }
    // Issue #25: snapshot the field as spawned (after the auto-fill
    // above). The late `topics.json` fetch may only replace this exact
    // untouched value; a deliberate clear/edit since spawn must stick.
    let topic_at_spawn = loaded.data.topic.clone();
    let initial_order = loaded.data.speaking_order;
    let state = RoundState::from_data(loaded.data);
    state.recovery_notice.set(loaded.recovery_notice);
    // Issue #21: every guide/timer/sheet view derives from the round's
    // speaking order, so the coin-flip toggle re-sequences them all.
    let guide = Signal::derive({
        let s = state.clone();
        move || s.speaking_order.get().guide()
    });
    let speaking_order = Signal::derive({
        let s = state.clone();
        move || s.speaking_order.get()
    });
    // The NSDA topic currently on offer: starts as the bundled fallback
    // (so provenance works offline), replaced by the same-origin
    // `topics.json` the deploy build extracted when that fetch succeeds.
    // OpenRouter (PKCE): a key minted in a previous authorize this tab,
    // plus any `?code=` callback landing on this load. The key stays in
    // sessionStorage — never in the round save.
    let openrouter_key = RwSignal::new(openrouter::load_key());
    let openrouter_status = RwSignal::new(None::<String>);
    spawn_local(async move {
        match openrouter::handle_callback().await {
            Ok(Some(key)) => {
                openrouter_key.set(Some(key));
                openrouter_status.set(Some(
                    "✓ Connected to OpenRouter — the AI summary button is ready in section 3."
                        .to_owned(),
                ));
            }
            Ok(None) => {}
            Err(e) => openrouter_status.set(Some(format!("⚠️ {e}"))),
        }
    });
    let nsda_topic = RwSignal::new(bundled_topic.clone());
    {
        let state_for_fetch = state.clone();
        let auto_filled = bundled_topic.resolution.clone();
        spawn_local(async move {
            if let Some(fetched) = topic::fetch_current().await {
                if topic::decide(
                    &state_for_fetch.topic.get_untracked(),
                    &auto_filled,
                    &topic_at_spawn,
                    &fetched,
                ) == TopicAction::Prefill
                {
                    state_for_fetch.topic.set(fetched.resolution.clone());
                    save(&state_for_fetch);
                }
                // Offer / UpToDate: leave the field exactly as it is;
                // the provenance row below offers the new wording.
                nsda_topic.set(fetched);
            }
        });
    }
    // Issue #17: restore the walk, timers and sheet from the progress
    // save. The timer itself never auto-resumes after a reload — only
    // its remainder does; the judge presses Start again.
    let progress = loaded.progress;
    let step = RwSignal::new(progress.step());
    let tab = RwSignal::new(progress.tab());
    let focus_mode = RwSignal::new(progress.focus_mode());
    let insert_ctx = InsertCtx::new();
    let timer_secs = RwSignal::new(progress.displayed_secs());
    // Remainders of the timers not on screen: the speech timer plus each
    // team's whole-round prep balance (issue #6). Banking the display
    // into this on every freeze/tick is what lets timers survive switches.
    let timer_bank = RwSignal::new(progress.timer_bank());
    let timer_label = RwSignal::new(match progress.active_prep() {
        Some(side) => format!("{} prep time ⏱️", side.label()),
        None => initial_order.guide()[progress.step()].title.to_string(),
    });
    let timer_announcement = RwSignal::new(String::new());
    let running = RwSignal::new(false);
    let copy_feedback = RwSignal::new(None::<&'static str>);
    let interval = StoredValue::new_local(None::<Interval>);
    // Wall-clock deadline (performance.now ms) the running timer counts to.
    // Remaining time is derived from it on every tick, so background-tab
    // throttling cannot make the timer run slow.
    let deadline = StoredValue::new_local(None::<f64>);
    let audio_ctx = StoredValue::new_local(None::<web_sys::AudioContext>);

    // Issue #17: snapshot the walk/timers/sheet for the next load. Reads
    // are untracked and the write is debounced in `state`, so calling
    // this on every timer tick costs no storage write per tick.
    let st_progress = state.clone();
    let persist_progress = Callback::new(move |()| {
        save_progress(
            &st_progress,
            RoundProgress::capture(
                step.get_untracked(),
                tab.get_untracked(),
                focus_mode.get_untracked(),
                timer_secs.get_untracked(),
                timer_bank.get_untracked(),
            ),
        );
    });

    let stop_timer = move || {
        // Freeze the displayed remainder before dropping the deadline, so
        // pausing between ticks does not lose (or gain) a second.
        if let (Some(end), Some(now)) = (deadline.get_value(), now_ms())
            && running.get_untracked()
        {
            timer_secs.set(ceil_secs(end - now));
        }
        timer_bank.update(|b| b.bank_displayed(timer_secs.get_untracked()));
        persist_progress.run(());
        running.set(false);
        deadline.set_value(None);
        // Dropping the Interval cancels it: no callbacks fire afterwards.
        interval.set_value(None);
    };
    // Advancing the walk updates speech, timer and sheet in place:
    // it must never scroll/jump the page under the judge.
    let set_step = Callback::new(move |next: usize| {
        let guide_now = guide.get_untracked();
        let next = next.min(guide_now.len() - 1);
        step.set(next);
        stop_timer();
        let full = (guide_now[next].minutes * 60).cast_signed();
        timer_bank.update(|b| {
            b.start_speech(full);
        });
        timer_secs.set(full);
        timer_label.set(guide_now[next].title.to_string());
        timer_announcement.set(String::new());
        if let Some(side) = sheet_for(&guide_now[next]) {
            tab.set(side);
        }
        persist_progress.run(());
    });
    // Flow-header click: jump to the guided step that writes in that
    // column. Crossfire steps have no column, so they stay reachable
    // only via Prev/Next, exactly as before.
    let on_speech_click = Callback::new(move |col: usize| {
        if let Some(i) = guide
            .get_untracked()
            .iter()
            .position(|g| g.flow_col == Some(col))
        {
            set_step.run(i);
        }
    });
    let toggle_timer = move |_: web_sys::MouseEvent| {
        if running.get_untracked() {
            stop_timer();
        } else if timer_secs.get_untracked() > 0 {
            let Some(now) = now_ms() else { return };
            deadline.set_value(Some(now + f64::from(timer_secs.get_untracked()) * 1_000.0));
            timer_announcement.set(String::new());
            if audio_ctx.get_value().is_none() {
                audio_ctx.set_value(unlock_audio());
            }
            running.set(true);
            interval.set_value(Some(Interval::new(250, move || {
                let (Some(end), Some(now)) = (deadline.get_value(), now_ms()) else {
                    return;
                };
                let remaining_ms = end - now;
                if remaining_ms <= 0.0 {
                    timer_secs.set(0);
                    timer_bank.update(|b| b.bank_displayed(0));
                    running.set(false);
                    deadline.set_value(None);
                    interval.set_value(None);
                    timer_announcement.set(format!(
                        "Time! {} has finished.",
                        timer_label.get_untracked()
                    ));
                    timer_expired_feedback(audio_ctx.get_value().as_ref());
                    persist_progress.run(());
                } else if let Some(secs) = tick_secs(remaining_ms, timer_secs.get_untracked()) {
                    // Most 250 ms ticks land in the displayed second:
                    // skip the writes and persist entirely for those,
                    // so subscribers are not notified with the same value.
                    timer_secs.set(secs);
                    timer_bank.update(|b| b.bank_displayed(secs));
                    persist_progress.run(());
                }
            })));
        }
    };
    // Entering prep resumes that team's remaining balance — never a
    // fresh 3:00. `stop_timer` has already banked the outgoing timer.
    let start_prep = move |side: Side| {
        stop_timer();
        timer_bank.update(|b| {
            b.switch_to(TimerKind::Prep(side));
        });
        timer_secs.set(timer_bank.get_untracked().prep_secs(side));
        timer_label.set(format!("{} prep time ⏱️", side.label()));
        timer_announcement.set(String::new());
        persist_progress.run(());
    };
    // The only way a prep balance returns to 3:00: this explicit,
    // per-team action. Resetting the on-screen prep stops its countdown;
    // resetting the other team's prep disturbs nothing else.
    let reset_prep = move |side: Side| {
        let on_screen = timer_bank.get_untracked().active() == TimerKind::Prep(side);
        if on_screen {
            stop_timer();
        }
        let mut shown = timer_secs.get_untracked();
        timer_bank.update(|b| {
            shown = b.reset_prep(side, shown);
        });
        if on_screen {
            timer_secs.set(shown);
        }
        persist_progress.run(());
    };
    // Shared Reset: in prep it only returns to the (preserved) speech
    // timer — it must never silently refill prep. During a speech it
    // restarts that speech's full time, as before.
    let reset_or_back = move |_: web_sys::MouseEvent| {
        stop_timer();
        if matches!(timer_bank.get_untracked().active(), TimerKind::Prep(_)) {
            // Read the banked speech remainder before Speech becomes the
            // active kind (`secs_for` reports the display for the active one).
            let speech_secs = timer_bank.get_untracked().secs_for(TimerKind::Speech, 0);
            timer_bank.update(|b| {
                b.switch_to(TimerKind::Speech);
            });
            timer_secs.set(speech_secs);
        } else {
            let full = (guide.get_untracked()[step.get_untracked()].minutes * 60).cast_signed();
            timer_bank.update(|b| {
                b.start_speech(full);
            });
            timer_secs.set(full);
        }
        let current = guide.get_untracked()[step.get_untracked()];
        timer_label.set(current.title.to_string());
        // (Re-)entering a speech must show its write-sheet: prep entry
        // and manual tab switches can leave the other sheet visible.
        if let Some(side) = sheet_for(&current) {
            tab.set(side);
        }
        timer_announcement.set(String::new());
        persist_progress.run(());
    };

    let st_flow = state.clone();
    let st_notice = state.clone();
    let st_confirm = state.clone();
    let st_clear = state.clone();
    let st_order = state.clone();
    // Speaking-order switch: the step is a speaking position, so it
    // stays put; re-point the sheet and speech label at that step in
    // the new order.
    let apply_order = Callback::new(move |order: SpeakingOrder| {
        st_order.speaking_order.set(order);
        save(&st_order);
        if !matches!(timer_bank.get_untracked().active(), TimerKind::Prep(_)) {
            let g = order.guide()[step.get_untracked()];
            timer_label.set(g.title.to_string());
            if let Some(side) = sheet_for(&g) {
                tab.set(side);
            }
        }
        persist_progress.run(());
    });
    // Issue #20: a derived signal, not a plain value snapshot, so step
    // changes update only the `cell-now`/`fh-now` class bindings in the
    // already-mounted sheets instead of rebuilding every row/textarea.
    let current_col = Signal::derive(move || guide.get()[step.get()].flow_col);
    // One shared in-UI confirmation dialog for every destructive
    // action (`PendingConfirm`). The safe action takes focus, like
    // row delete.
    let pending_confirm = RwSignal::new(None::<PendingConfirm>);
    let keep_confirm_button = NodeRef::<leptos::html::Button>::new();
    Effect::new(move |_| {
        let keep = pending_confirm
            .get()
            .is_some()
            .then(|| keep_confirm_button.get())
            .flatten();
        if let Some(button) = keep {
            let _ = button.focus();
        }
    });
    // `replace` mints the fresh rows' signals when it runs. They must
    // belong to the App owner, not the confirmation dialog's: closing
    // the dialog disposes its subtree, which would kill the new rows'
    // signals and panic every later save/read of them.
    let app_owner = Owner::current();
    // Every round replacement (New round, example round) restarts the
    // walk identically: `set_step(0)` stops the running timer (dropping
    // its Interval and deadline, so a pending expiry cannot fire into
    // the new round) and re-points label/sheet at the first speech, then
    // both prep balances return to 3:00. Issue #17: the fresh
    // walk/balances replace the saved progress, so a reload cannot
    // resurrect the previous round's step or prep.
    let reset_round_walk = Callback::new(move |()| {
        set_step.run(0);
        timer_bank.update(TimerBank::reset_all_prep);
        persist_progress.run(());
    });
    let start_new_round = Callback::new(move |()| {
        st_clear.replace(RoundData::default());
        // A brand-new round starts empty, so it gets the current NSDA
        // pre-fill like a fresh visit.
        st_clear.topic.set(nsda_topic.get_untracked().resolution);
        save(&st_clear);
        reset_round_walk.run(());
    });
    // Runs whichever destructive action the dialog confirmed. Called
    // under the App owner (see `app_owner` above), never the dialog's.
    let confirm_action = Callback::new(move |pending: PendingConfirm| match pending {
        PendingConfirm::NewRound => start_new_round.run(()),
        PendingConfirm::ExampleRound => {
            st_confirm.replace(RoundData::sample());
            save(&st_confirm);
            reset_round_walk.run(());
        }
        PendingConfirm::ResetPrep(side) => reset_prep(side),
    });

    view! {
        <div class="page">
            {hero_header(HeroProps {
                state: state.clone(),
                pending_confirm,
                reset_round_walk,
                openrouter_key,
                openrouter_status,
            })}
            {confirm_dialog(pending_confirm, keep_confirm_button, app_owner, confirm_action)}
            {move || st_notice.recovery_notice.get().map(|notice| {
                let s = st_notice.clone();
                view! {
                    <div class="save-warning" role="alert">
                        <p>{notice}</p>
                        <button class="btn btn-main" on:click=move |_| { s.recovery_notice.set(None); save(&s); }>
                            "Replace the saved round anyway"
                        </button>
                    </div>
                }
            })}

            <main>
            // ---------- Round setup ----------
            {setup_card(SetupProps {
                state: state.clone(),
                nsda_topic,
                insert_ctx,
                guide,
                step,
                timer_label,
                timer_secs,
                timer_announcement,
                running,
                timer_bank,
                pending_confirm,
                set_step,
                toggle_timer,
                reset_or_back,
                start_prep,
            })}
            // ---------- Flow sheets ----------
            <section class="card flow-card">
                <h2><span class="card-ico">""</span> "2 · Your flow" </h2>
                <div class="flow-controls no-print">
                    <label class="focus-toggle">
                        <input type="checkbox" prop:checked=move || focus_mode.get()
                            on:change=move |ev| { focus_mode.set(event_target_checked(&ev)); persist_progress.run(()); } />
                        <span><b>"Focus mode"</b></span>
                    </label>
                    <div class="order-toggle" role="group" aria-label="Speaking order">
                        <button class="order-btn" class:order-on=move || speaking_order.get() == SpeakingOrder::ProFirst
                            aria-pressed=move || if speaking_order.get() == SpeakingOrder::ProFirst { "true" } else { "false" }
                            title="Pro speaks first"
                            on:click=move |_| apply_order.run(SpeakingOrder::ProFirst)>"Pro"</button>
                        <button class="order-btn" class:order-on=move || speaking_order.get() == SpeakingOrder::ConFirst
                            aria-pressed=move || if speaking_order.get() == SpeakingOrder::ConFirst { "true" } else { "false" }
                            title="Con speaks first"
                            on:click=move |_| apply_order.run(SpeakingOrder::ConFirst)>"Con"</button>
                    </div>
                </div>
                <div class="tabs no-print">
                    <button class="tab tab-pro" class:tab-on=move || tab.get() == Side::Pro
                        aria-pressed=move || if tab.get() == Side::Pro { "true" } else { "false" }
                        on:click=move |_| { tab.set(Side::Pro); persist_progress.run(()); }>
                        "Pro sheet" <span class="tab-hint">"their arguments ↓, round →"</span>
                    </button>
                    <span class="tab-arrow" aria-hidden="true">"⇄"</span>
                    <button class="tab tab-con" class:tab-on=move || tab.get() == Side::Con
                        aria-pressed=move || if tab.get() == Side::Con { "true" } else { "false" }
                        on:click=move |_| { tab.set(Side::Con); persist_progress.run(()); }>
                        "Con sheet" <span class="tab-hint">"their arguments ↓, round →"</span>
                    </button>
                </div>
                <p class="flow-legend no-print">
                    "Read each row left ➜ right: that's one argument's whole story. "
                    <span class="legend-chip">"💤 dropped = nobody answered it"</span>
                    <span class="legend-chip">"➜ extended = they carried it forward"</span>
                    <span class="legend-chip">"⭐ star the arguments the round will be decided on"</span>
                </p>

                // Issue #20: both sheets mount once; the tab only toggles
                // visibility. Rebuilding on tab/step used to destroy every
                // textarea (focus, caret, undo history, scroll position).
                <div class:sheet-hidden=move || tab.get() != Side::Pro>
                    <FlowSheet state=st_flow.clone() side=Side::Pro speaking_order=speaking_order current_col=Some(current_col) focus_mode=focus_mode insert_ctx=Some(insert_ctx) on_speech_click=Some(on_speech_click) />
                </div>
                <div class:sheet-hidden=move || tab.get() != Side::Con>
                    <FlowSheet state=st_flow.clone() side=Side::Con speaking_order=speaking_order current_col=Some(current_col) focus_mode=focus_mode insert_ctx=Some(insert_ctx) on_speech_click=Some(on_speech_click) />
                </div>

                <details class="shorthand no-print">
                    <summary>"✂️ Shorthand cheat sheet — tap a symbol to drop it into the box you're writing in"</summary>
                    <div class="shorthand-grid">
                        {SHORTHAND.iter().map(|(sym, meaning)| {
                            let sym = *sym;
                            let meaning = *meaning;
                            let st = state.clone();
                            view! {
                                <button class="short-chip" on:click=move |_| insert_shorthand(&st, insert_ctx, sym)>
                                    <b>{sym}</b><span>{meaning}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                    {move || insert_ctx.hint.get().map(|hint| view! { <p class="shorthand-hint" role="alert">{hint}</p> })}
                    <p class="shorthand-note">"Also handy: write only author + year for evidence (“Smith '24”), and use debaters' own signpost words — “first”, “next”, “finally” — as row breaks."</p>
                </details>
            </section>

            // Print-only: both sheets, like the two paper pages.
            <section class="card print-only">
                <h2>"Pro flow"</h2>
                <FlowSheet state=st_flow.clone() side=Side::Pro speaking_order=speaking_order current_col=None focus_mode=focus_mode insert_ctx=None on_speech_click=None />
                <h2 style="margin-top:20px">"Con flow"</h2>
                <FlowSheet state=st_flow.clone() side=Side::Con speaking_order=speaking_order current_col=None focus_mode=focus_mode insert_ctx=None on_speech_click=None />
            </section>

            // ---------- Notes ----------
            <section class="card notes-card">
                <h2>"Notes" <span class="card-sub">"optional — only concessions & admissions"</span></h2>
                <textarea class="big-notes" id="field-notes" rows="3" aria-label="Notes" placeholder="e.g. “Pro agreed recycling is only 9% — that helps Con C1…”"
                    on:focus=move |_| insert_ctx.focus(TextTarget::Notes)
                    prop:value=move || state.notes.get()
                    on:input={let s = state.clone(); move |ev| { s.notes.set(event_target_value(&ev)); save(&s); }}></textarea>
                <div class="notes-print" aria-hidden="true">{move || state.notes.get()}</div>
            </section>

            // ---------- Decision ----------
            {ballot_card(BallotProps {
                state: state.clone(),
                step,
                insert_ctx,
                copy_feedback,
                openrouter_key,
            })}
            </main>

            <footer class="footer">
                <p class="footer-small">"Your flow auto-saves only in this browser (localStorage) — nothing leaves your device unless you tap the AI summary button, which sends the flow to OpenRouter under your own account."</p>
            </footer>
        </div>
    }
}
