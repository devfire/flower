use crate::model::*;
use crate::progress::RoundProgress;
use crate::shorthand::{InsertCtx, TextTarget, insert_shorthand};
use crate::state::*;
use crate::timer::{PREP_SECS, TimerBank, TimerKind, fmt_time};
use crate::topic::{self, TopicAction};
use gloo_timers::callback::Interval;
use leptos::prelude::*;
use leptos::task::spawn_local;

fn placeholder_for(sheet: Side, col: usize) -> &'static str {
    let speech_side = SPEECH_COLS[col].side;
    match col {
        0 | 1 => "Claim + why it matters…\ne.g. “Plastic → oceans”",
        4 | 5 => "➜ Extend: repeat the argument\n+ answer their best reply…",
        6 | 7 => "⚖️ “Vote for us because…”\ntheir #1 reason + weighing…",
        _ if speech_side == sheet => "Defend: answer their reply…",
        _ => "↩️ Their answer to this argument…\n(leave blank = dropped 💤)",
    }
}

/// Destructive actions that ask in the shared in-UI dialog before
/// running, instead of the native `window.confirm` (which appears
/// wherever the browser puts it and matches nothing else on the page).
#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingConfirm {
    NewRound,
    ExampleRound,
    ResetPrep(Side),
}

impl PendingConfirm {
    fn title(self) -> &'static str {
        match self {
            Self::NewRound => "Start a brand-new round?",
            Self::ExampleRound => "Load the example round?",
            Self::ResetPrep(Side::Pro) => "Reset Pro prep?",
            Self::ResetPrep(Side::Con) => "Reset Con prep?",
        }
    }

    fn body(self) -> &'static str {
        match self {
            Self::NewRound => {
                "This clears the current flow and resets both prep timers. It can't be undone."
            }
            Self::ExampleRound => "This replaces the current flow. It can't be undone.",
            Self::ResetPrep(Side::Pro) => {
                "This resets only Pro prep back to 3:00. Con prep is untouched."
            }
            Self::ResetPrep(Side::Con) => {
                "This resets only Con prep back to 3:00. Pro prep is untouched."
            }
        }
    }

    fn keep_label(self) -> &'static str {
        match self {
            Self::NewRound => "Keep current round",
            Self::ExampleRound => "Keep current flow",
            Self::ResetPrep(_) => "Keep prep time",
        }
    }

    fn confirm_label(self) -> &'static str {
        match self {
            Self::NewRound => "Start new round",
            Self::ExampleRound => "Load example round",
            Self::ResetPrep(Side::Pro) => "Reset Pro prep",
            Self::ResetPrep(Side::Con) => "Reset Con prep",
        }
    }
}

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
    let step = RwSignal::new(progress.step);
    let tab = RwSignal::new(progress.tab);
    let focus_mode = RwSignal::new(progress.focus_mode);
    let insert_ctx = InsertCtx::new();
    let timer_secs = RwSignal::new(progress.displayed_secs());
    // Remainders of the timers not on screen: the speech timer plus each
    // team's whole-round prep balance (issue #6). Banking the display
    // into this on every freeze/tick is what lets timers survive switches.
    let timer_bank = RwSignal::new(progress.timer_bank());
    let timer_label = RwSignal::new(match progress.active_prep {
        Some(side) => format!("{} prep time ⏱️", side.label()),
        None => initial_order.guide()[progress.step].title.to_string(),
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
            timer_secs.set(((end - now) / 1_000.0).ceil().max(0.0) as i32);
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
        let full = (guide_now[next].minutes * 60) as i32;
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
    let toggle_timer = move |_| {
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
                } else {
                    let secs = (remaining_ms / 1_000.0).ceil() as i32;
                    timer_secs.set(secs);
                    timer_bank.update(|b| b.bank_displayed(secs));
                }
                persist_progress.run(());
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
        if timer_bank.get_untracked().active() == TimerKind::Prep(side) {
            stop_timer();
            timer_secs.set(PREP_SECS);
        }
        timer_bank.update(|b| b.reset_prep(side));
        persist_progress.run(());
    };
    // Shared Reset: in prep it only returns to the (preserved) speech
    // timer — it must never silently refill prep. During a speech it
    // restarts that speech's full time, as before.
    let reset_or_back = move |_| {
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
    let st_saved = state.clone();
    let st_sample = state.clone();
    let st_confirm = state.clone();
    let st_clear = state.clone();
    let st_export = state.clone();
    let st_topic = state.clone();
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
            <header class="hero">
                <div class="hero-mascot" aria-hidden="true">""</div>
                <div class="hero-text">
                    <h1>"Flower" <span class="hero-sub">"easy flowing for PF judges"</span></h1>
                    <p class="hero-tag">
                        "New to flowing? Perfect — you're exactly who this is for. "
                        "Follow the speech, follow the arrows ➜, and let Momo-chan's bubbles tell you what to write."
                    </p>
                </div>
                <div class="hero-actions no-print">
                    <button class="btn btn-soft" aria-haspopup="dialog"
                        aria-expanded=move || if pending_confirm.get() == Some(PendingConfirm::ExampleRound) { "true" } else { "false" }
                        on:click=move |_| {
                            // An empty flow has nothing to lose, so the
                            // example loads straight away; only a flow
                            // with content asks first.
                            if st_sample.snapshot().is_empty() {
                                st_sample.replace(RoundData::sample());
                                save(&st_sample);
                                reset_round_walk.run(());
                            } else {
                                pending_confirm.set(Some(PendingConfirm::ExampleRound));
                            }
                        }>
                        "See an example round"
                    </button>
                    <button class="btn btn-soft" aria-haspopup="dialog"
                        aria-expanded=move || if pending_confirm.get() == Some(PendingConfirm::NewRound) { "true" } else { "false" }
                        on:click=move |_| pending_confirm.set(Some(PendingConfirm::NewRound))>"New round"</button>
                    <button class="btn btn-soft" on:click=move |_| print_page()>"🖨️ Print / save PDF"</button>
                </div>
            </header>

            {move || pending_confirm.get().map(|pending| { let app_owner = app_owner.clone(); view! {
                <div class="confirm-overlay no-print" on:click=move |_| pending_confirm.set(None)>
                    <div class="confirm-dialog" role="alertdialog" aria-modal="true"
                        aria-labelledby="confirm-title" aria-describedby="confirm-desc"
                        on:click=move |ev| ev.stop_propagation()
                        on:keydown=move |ev| {
                            if ev.key() == "Escape" { pending_confirm.set(None); }
                        }>
                        <h2 id="confirm-title">{pending.title()}</h2>
                        <p id="confirm-desc">{pending.body()}</p>
                        <div class="confirm-actions">
                            <button class="btn btn-soft" node_ref=keep_confirm_button
                                on:click=move |_| pending_confirm.set(None)>{pending.keep_label()}</button>
                            <button class="btn btn-danger" on:click=move |_| {
                                pending_confirm.set(None);
                                match &app_owner {
                                    Some(owner) => owner.with(|| confirm_action.run(pending)),
                                    None => confirm_action.run(pending),
                                }
                            }>{pending.confirm_label()}</button>
                        </div>
                    </div>
                </div>
            } })}

            {move || st_notice.recovery_notice.get().map(|notice| {
                let s = st_notice.clone();
                view! {
                    <div class="save-warning" role="alert">
                        <p>{notice}</p>
                        <button class="btn btn-main" on:click=move |_| { s.recovery_notice.set(None); save(&s); }>
                            "Start fresh & replace the old save"
                        </button>
                    </div>
                }
            })}

            <main>
            // ---------- Round setup ----------
            <section class="card setup-card">
                <h2><span class="card-ico">""</span> "1 · Set up the round" <span class="saved-note" class:saved-note-failed=move || st_saved.save_failed.get()>{move || if st_saved.save_failed.get() { "⚠️ Not saved — this browser is blocking or out of storage; copy anything important before closing" } else { "✓ auto-saves on this device" }}</span></h2>
                <div class="setup-layout">
                <div class="setup-grid">
                    <label class="field field-wide">
                        <span>"Resolution / topic"</span>
                        <input type="text" id="field-topic" placeholder="Resolved: …"
                            on:focus=move |_| insert_ctx.focus(TextTarget::Topic)
                            prop:value=move || state.topic.get()
                            on:input={let s = state.clone(); move |ev| { s.topic.set(event_target_value(&ev)); save(&s); }} />
                    </label>
                    <div class="field-wide topic-row">
                        {move || {
                            let nsda = nsda_topic.get();
                            let current = st_topic.topic.get();
                            let offered = !current.trim().is_empty()
                                && current.trim() != nsda.resolution.trim();
                            let note = format!("\u{1f338} {}", nsda.provenance());
                            if offered {
                                let s = st_topic.clone();
                                let resolution = nsda.resolution.clone();
                                view! {
                                    <>
                                        <p class="topic-note">{note}</p>
                                        <button class="btn btn-soft btn-chip" on:click=move |_| {
                                            s.topic.set(resolution.clone());
                                            save(&s);
                                        }>"Use current NSDA resolution"</button>
                                    </>
                                }.into_any()
                            } else {
                                view! { <p class="topic-note">{note}</p> }.into_any()
                            }
                        }}
                    </div>
                    <label class="field">
                        <span>"Pro team"</span>
                        <input type="text" id="field-pro-team" placeholder="School / speaker names"
                            on:focus=move |_| insert_ctx.focus(TextTarget::ProTeam)
                            prop:value=move || state.pro_team.get()
                            on:input={let s = state.clone(); move |ev| { s.pro_team.set(event_target_value(&ev)); save(&s); }} />
                    </label>
                    <label class="field">
                        <span>"Con team"</span>
                        <input type="text" id="field-con-team" placeholder="School / speaker names"
                            on:focus=move |_| insert_ctx.focus(TextTarget::ConTeam)
                            prop:value=move || state.con_team.get()
                            on:input={let s = state.clone(); move |ev| { s.con_team.set(event_target_value(&ev)); save(&s); }} />
                    </label>
                </div>

                    <div class="bubble-wrap no-print">
                        <div class="bubble">
                            <p class="bubble-title">
                                {move || guide.get()[step.get()].title}
                                <span class="bubble-time">{move || format!(" · {} min", guide.get()[step.get()].minutes)}</span>
                            </p>
                            <p class="bubble-main">{move || guide.get()[step.get()].bubble}</p>
                            <ul class="bubble-list">
                                <li><b>"Listen for: "</b>{move || guide.get()[step.get()].listen_for}</li>
                                <li><b>"Write: "</b>{move || guide.get()[step.get()].do_it}</li>
                                <li><b>"Watch out: "</b>{move || guide.get()[step.get()].watch_out}</li>
                            </ul>
                        </div>
                    </div>

                    <div class="timer-box no-print">
                        <div class="timer-head">
                            <div class="timer-readout">
                                <div class="timer-label">{move || timer_label.get()}</div>
                                <div class="timer-digits" role="timer" aria-label=move || format!("{}: {} remaining", timer_label.get(), fmt_time(timer_secs.get())) class:timer-done=move || timer_secs.get() == 0>
                                    {move || fmt_time(timer_secs.get())}
                                </div>
                            </div>
                            <div class="sr-only" role="status" aria-live="polite">
                                {move || timer_announcement.get()}
                            </div>
                            <div class="timer-btns">
                                <button class="btn btn-main" on:click=toggle_timer>
                                    {move || if running.get() { "⏸ Pause" } else { "▶ Start" }}
                                </button>
                                <button class="btn btn-soft" on:click=reset_or_back>
                                    {move || if matches!(timer_bank.get().active(), TimerKind::Prep(_)) { "↺ Back to speech" } else { "↺ Reset" }}
                                </button>
                            </div>
                        </div>
                        <div class="prep-row">
                            <span>"Prep left (3m/team):"</span>
                            <button class="btn btn-chip btn-pro" on:click=move |_| start_prep(Side::Pro)>
                                {move || format!("Pro · {}", fmt_time(timer_bank.get().secs_for(TimerKind::Prep(Side::Pro), timer_secs.get())))}
                            </button>
                            <button class="btn btn-chip" title="Reset only Pro prep back to 3:00" aria-label="Reset only Pro prep back to 3:00" aria-haspopup="dialog"
                                aria-expanded=move || if pending_confirm.get() == Some(PendingConfirm::ResetPrep(Side::Pro)) { "true" } else { "false" }
                                on:click=move |_| pending_confirm.set(Some(PendingConfirm::ResetPrep(Side::Pro)))>"↺ "</button>
                            <button class="btn btn-chip btn-con" on:click=move |_| start_prep(Side::Con)>
                                {move || format!("Con · {}", fmt_time(timer_bank.get().secs_for(TimerKind::Prep(Side::Con), timer_secs.get())))}
                            </button>
                            <button class="btn btn-chip" title="Reset only Con prep back to 3:00" aria-label="Reset only Con prep back to 3:00" aria-haspopup="dialog"
                                aria-expanded=move || if pending_confirm.get() == Some(PendingConfirm::ResetPrep(Side::Con)) { "true" } else { "false" }
                                on:click=move |_| pending_confirm.set(Some(PendingConfirm::ResetPrep(Side::Con)))>"↺"</button>
                        </div>
                        <div class="step-nav">
                            <button class="btn btn-soft" disabled={move || step.get() == 0}
                                on:click=move |_| set_step.run(step.get_untracked().saturating_sub(1))>"⬅ Prev speech"</button>
                            <button class="btn btn-main" disabled={move || step.get() >= guide.get().len() - 1}
                                on:click=move |_| set_step.run(step.get_untracked() + 1)>"Next speech ➜"</button>
                        </div>
                    </div>
                </div>
            </section>

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

            // ---------- Crossfire notes ----------
            <section class="card notes-card">
                <h2>"Crossfire notes" <span class="card-sub">"optional — only concessions & admissions"</span></h2>
                <textarea class="big-notes" id="notes-crossfire" rows="3" aria-label="Crossfire notes" placeholder="e.g. “Pro agreed recycling is only 9% — that helps Con C1…”"
                    on:focus=move |_| insert_ctx.focus(TextTarget::CrossfireNotes)
                    prop:value=move || state.crossfire_notes.get()
                    on:input={let s = state.clone(); move |ev| { s.crossfire_notes.set(event_target_value(&ev)); save(&s); }}></textarea>
                <div class="notes-print" aria-hidden="true">{move || state.crossfire_notes.get()}</div>
            </section>

            // ---------- Decision ----------
            <section class="card ballot-card">
                <h2>"3 · Your decision" </h2>
                <p class="ballot-intro">"No perfect flow required. Ask three questions:"</p>
                <ol class="ballot-qs">
                    <li>"Which arguments were " <b>"answered the least"</b> " (💤 dropped or weakly answered)?"</li>
                    <li>"Which side " <b>"extended"</b> " ➜ their arguments into Summary & Final Focus?"</li>
                    <li>"In Final Focus, who gave the better " <b>"weighing"</b> " ⚖️ — bigger, sooner, or more likely impact?"</li>
                </ol>
                <DroppedHints state=state.clone() step=step />
                <div class="winner-row no-print">
                    <span>"I vote for:"</span>
                    <button class="btn winner-btn btn-pro" class:winner-on=move || state.winner.get() == "Pro" aria-pressed=move || if state.winner.get() == "Pro" { "true" } else { "false" }
                        on:click={let s = state.clone(); move |_| { s.winner.set("Pro".to_string()); save(&s); }}>"Pro"</button>
                    <button class="btn winner-btn btn-con" class:winner-on=move || state.winner.get() == "Con" aria-pressed=move || if state.winner.get() == "Con" { "true" } else { "false" }
                        on:click={let s = state.clone(); move |_| { s.winner.set("Con".to_string()); save(&s); }}>"Con"</button>
                </div>
                <p class="winner-print">{move || {
                    let w = state.winner.get();
                    if w.is_empty() { "Winner: __________".to_string() } else { format!("Winner: {w}") }
                }}</p>
                <label class="field">
                    <span>"Reason for decision (say it in plain words — debaters learn from this!) 🗣️"</span>
                    <textarea class="big-notes" id="notes-rfd" rows="4" aria-label="Reason for decision"
                        on:focus=move |_| insert_ctx.focus(TextTarget::Rfd)
                        placeholder="“I voted ___ because the ___ argument was dropped / extended / weighed better when…”"
                        prop:value=move || state.rfd.get()
                        on:input={let s = state.clone(); move |ev| { s.rfd.set(event_target_value(&ev)); save(&s); }}></textarea>
                    <div class="notes-print" aria-hidden="true">{move || state.rfd.get()}</div>
                </label>
                <div class="ballot-actions no-print">
                    <button class="btn btn-main" on:click=move |_| {
                        let text = decision_text(&st_export);
                        spawn_local(async move {
                            copy_feedback.set(Some(if copy_text(&text).await {
                                "Copied ✓"
                            } else {
                                "⚠️ Copy failed — clipboard blocked by the browser"
                            }));
                        });
                    }>"📋 Copy decision summary"</button>
                    <span class="copy-feedback" role="status" aria-live="polite">{move || copy_feedback.get().unwrap_or("")}</span>
                    <button class="btn btn-soft" on:click=move |_| print_page()>"🖨️ Print flow + ballot"</button>
                </div>
            </section>

            </main>

            <footer class="footer">
                <p class="footer-small">"Your flow auto-saves only in this browser (localStorage) — nothing leaves your device."</p>
            </footer>
        </div>
    }
}

fn decision_text(state: &RoundState) -> String {
    state.snapshot().decision_text()
}

#[component]
fn DroppedHints(state: RoundState, step: RwSignal<usize>) -> impl IntoView {
    // Tracked snapshot + the guide step: hints re-derive live as the
    // judge types or taps a mark, and inferred drops are not flagged
    // before their rebuttal is over (explicit 💤 marks are immediate).
    move || {
        let hints: Vec<String> = state
            .snapshot_tracked()
            .dropped_arguments(step.get())
            .into_iter()
            .map(|d| {
                let mut hint = if d.explicit {
                    format!(
                        "💤 {}'s “{}” — you marked this dropped; no {} answer counts against it.",
                        d.owner.label(),
                        d.name,
                        d.answerer.label()
                    )
                } else {
                    format!(
                        "💤 {}'s “{}” got no {} answer in any speech so far",
                        d.owner.label(),
                        d.name,
                        d.answerer.label()
                    )
                };
                if d.extended {
                    hint.push_str(" ➜ It was extended — that can decide the round.");
                } else if !d.explicit {
                    hint.push_str(" — if it was extended, it may decide the round.");
                }
                if d.key {
                    hint.push_str(" ⭐ You starred it as key.");
                }
                hint
            })
            .collect();
        if hints.is_empty() {
            view! { <p class="dropped-none">"✨ No clearly dropped arguments spotted yet — check for blank answer boxes on each row."</p> }.into_any()
        } else {
            view! {
                <div class="dropped-box">
                    <p class="dropped-title">"🔎 Arguments that look dropped (blank answer box or 💤 mark):"</p>
                    <ul>{hints.into_iter().map(|h| view! { <li>{h}</li> }).collect_view()}</ul>
                </div>
            }.into_any()
        }
    }
}

#[component]
fn FlowSheet(
    state: RoundState,
    side: Side,
    speaking_order: Signal<SpeakingOrder>,
    current_col: Option<Signal<Option<usize>>>,
    focus_mode: RwSignal<bool>,
    insert_ctx: Option<InsertCtx>,
    on_speech_click: Option<Callback<usize>>,
) -> impl IntoView {
    let rows = state.rows(side);
    let sheet_class = match side {
        Side::Pro => "sheet sheet-pro",
        Side::Con => "sheet sheet-con",
    };
    view! {
        <div class=sheet_class>
            <div class="flow-scroll">
                <div class="flow-grid" style=format!("grid-template-columns: minmax(8rem, 11rem) repeat({}, minmax(6.5rem, 1fr));", SPEECH_COLS.len())>
                    <div class="fh fh-corner" style="grid-column: 1; grid-row: 1;">
                        {format!("{} arguments ↓", side.label())}
                        <span class="fh-sub">"one row = one argument"</span>
                    </div>
                    {(0..SPEECH_COLS.len()).map(|col| {
                        let sc = SPEECH_COLS[col];
                        // Canonical render + CSS grid placement (issue #26):
                        // a speaking-order flip only restyles these nodes,
                        // never remounts the textareas below them.
                        let grid_style = move || format!("grid-column: {}; grid-row: 1;", speaking_order.get().grid_column(col));
                        let has_arrow = move || speaking_order.get().position_of(col).unwrap_or(col) > 0;
                        let is_now = move || current_col.is_some_and(|signal| signal.get() == Some(col));
                        let dim = move || {
                            focus_mode.get()
                                && current_col
                                    .is_some_and(|signal| matches!(signal.get(), Some(cur) if cur != col))
                        };
                        match on_speech_click {
                            Some(on_click) => view! {
                                <button type="button" class="fh fh-click" class:fh-now=is_now class:fh-dim=dim style=grid_style
                                    aria-current=move || is_now().then_some("step")
                                    title=format!("Go to {}", sc.full)
                                    on:click=move |_| on_click.run(col)>
                                    {move || has_arrow().then(|| view! { <span class="fh-arrow" aria-hidden="true">"➜"</span> })}
                                    {move || is_now().then(|| view! { <span class="fh-badge">"✏️ you are here"</span> })}
                                    <span class="fh-name">{sc.full}</span>
                                    <span class="fh-sub">{format!("{} min", sc.minutes)}</span>
                                </button>
                            }.into_any(),
                            None => view! {
                                <div class="fh" class:fh-now=is_now class:fh-dim=dim style=grid_style>
                                    {move || is_now().then(|| view! { <span class="fh-badge">"✏️ you are here"</span> })}
                                    <span class="fh-name">{sc.full}</span>
                                    <span class="fh-sub">{format!("{} min", sc.minutes)}</span>
                                </div>
                            }.into_any(),
                        }
                    }).collect_view()}

                    {let s = state.clone(); move || rows.get().into_iter().enumerate().map(|(idx, row)| {
                        view! { <FlowRow state=s.clone() side=side row=row row_index=idx + 1 speaking_order=speaking_order current_col=current_col focus_mode=focus_mode insert_ctx=insert_ctx /> }
                    }).collect_view()}
                </div>
            </div>
            <button class="btn btn-main add-row no-print" on:click={let s = state.clone(); move |_| s.add_row(side)}>
                "➕ Add an argument row"
            </button>
            <p class="sheet-tip no-print">
                {match side {
                    Side::Pro => "Tip: start a new row each time Pro says “our next contention is…”. ",
                    Side::Con => "Tip: start a new row each time Con says “our next contention is…”. ",
                }}
            </p>
        </div>
    }
}

#[component]
fn FlowRow(
    state: RoundState,
    side: Side,
    row: RowState,
    row_index: usize,
    speaking_order: Signal<SpeakingOrder>,
    current_col: Option<Signal<Option<usize>>>,
    focus_mode: RwSignal<bool>,
    insert_ctx: Option<InsertCtx>,
) -> impl IntoView {
    let row_id = row.id;
    let title_target = TextTarget::RowTitle { side, row_id };
    let title_id = insert_ctx.map(|_| title_target.id());
    // Inline delete confirmation, anchored to the ✕ below: the native
    // `window.confirm` dialog appears wherever the browser puts it,
    // far from the row the judge is looking at.
    let confirming_delete = RwSignal::new(false);
    let keep_button = NodeRef::<leptos::html::Button>::new();
    Effect::new(move |_| {
        let keep = confirming_delete.get().then(|| keep_button.get()).flatten();
        if let Some(button) = keep {
            let _ = button.focus();
        }
    });
    view! {
        <div class="fr-title" style=format!("grid-column: 1; grid-row: {};", row_index + 1)>
            <input type="text" id=title_id placeholder="Contention name…" aria-label=format!("{} argument {} title", side.label(), row_index)
                on:focus=move |_| if let Some(ctx) = insert_ctx { ctx.focus(title_target) }
                prop:value=move || row.title.get()
                on:input={let s = state.clone(); let r = row.clone(); move |ev| { r.title.set(event_target_value(&ev)); save(&s); }} />
            <div class="title-print" aria-hidden="true">{move || row.title.get()}</div>
            <div class="row-del-wrap no-print"
                on:keydown=move |ev| {
                    if ev.key() == "Escape" { confirming_delete.set(false); }
                }>
                <button class="row-del" class:row-del-armed=move || confirming_delete.get()
                    title="Delete this argument row" aria-label=format!("Delete {} argument {} row", side.label(), row_index)
                    aria-expanded=move || if confirming_delete.get() { "true" } else { "false" }
                    on:click=move |_| {
                        confirming_delete.update(|armed| *armed = !*armed);
                    }>"✕"</button>
                {{let state = state.clone(); move || confirming_delete.get().then(|| {
                    let s = state.clone();
                    view! {
                        <div class="row-del-pop" role="alertdialog"
                            aria-label=format!("Confirm deleting {} argument {}", side.label(), row_index)>
                            <span class="row-del-q">"Delete this row?"</span>
                            <span class="row-del-actions">
                                <button class="row-del-confirm" on:click={let s = s.clone(); move |_| {
                                    confirming_delete.set(false);
                                    s.remove_row(side, row_id);
                                }}>"Delete"</button>
                                <button class="row-del-keep" node_ref=keep_button
                                    on:click=move |_| confirming_delete.set(false)>"Keep"</button>
                            </span>
                        </div>
                    }
                })}}
            </div>
        </div>
        {let r = row.clone(); (0..SPEECH_COLS.len()).map(|col| {
            let cell = r.cells[col];
            let row = r.clone();
            let grid_style = move || format!("grid-column: {}; grid-row: {};", speaking_order.get().grid_column(col), row_index + 1);
            let has_arrow = move || speaking_order.get().position_of(col).unwrap_or(col) > 0;
            let is_now = move || current_col.is_some_and(|signal| signal.get() == Some(col));
            let dim = move || {
                focus_mode.get()
                    && current_col
                        .is_some_and(|signal| matches!(signal.get(), Some(cur) if cur != col))
            };
            let placeholder = placeholder_for(side, col);
            let title_signal = row.title;
            let cell_target = TextTarget::FlowCell { side, row_id, col };
            let cell_id = insert_ctx.map(|_| cell_target.id());
            view! {
                <div class="cell" class:cell-now=is_now class:cell-dim=dim style=grid_style
                    class:cell-dropped=move || cell.mark.get() == Mark::Dropped
                    class:cell-extended=move || cell.mark.get() == Mark::Extended
                    class:cell-key=move || cell.mark.get() == Mark::Key>
                    {move || has_arrow().then(|| view! { <span class="cell-arrow" aria-hidden="true">"➜"</span> })}
                    <textarea rows="3" id=cell_id placeholder=placeholder aria-label=move || { let title = title_signal.get(); let title = title.trim(); if title.is_empty() { format!("{}, {} argument {}", SPEECH_COLS[col].full, side.label(), row_index) } else { format!("{}, {} argument {}: {}", SPEECH_COLS[col].full, side.label(), row_index, title) } }
                        prop:value=move || cell.text.get()
                        on:input={let s = state.clone(); move |ev| { cell.text.set(event_target_value(&ev)); save(&s); }}
                        on:focus=move |_| if let Some(ctx) = insert_ctx { ctx.focus(cell_target) }></textarea>
                    <div class="cell-print" aria-hidden="true">{move || cell.text.get()}</div>
                    <div class="cell-marks no-print">
                        {[(Mark::Dropped, "💤 dropped"), (Mark::Extended, "➜ extended"), (Mark::Key, "⭐ key")].into_iter().map(|(mark, label)| {
                            view! {
                                <button class="mark-btn"
                                    class:mark-on=move || cell.mark.get() == mark
                                    aria-pressed=move || if cell.mark.get() == mark { "true" } else { "false" }
                                    on:click={let s = state.clone(); move |_| {
                                        cell.mark.update(|m| *m = if *m == mark { Mark::None } else { mark });
                                        save(&s);
                                    }}>{label}</button>
                            }
                        }).collect_view()}
                    </div>
                    {move || {
                        let label = cell.mark.get().label();
                        if label.is_empty() { None } else { Some(view! { <span class="mark-print">{label}</span> }) }
                    }}
                </div>
            }
        }).collect_view()}
    }
}
