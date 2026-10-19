use crate::model::*;
use crate::shorthand::{InsertCtx, TextTarget, insert_shorthand};
use crate::state::*;
use crate::timer::{PREP_SECS, TimerBank, TimerKind};
use gloo_timers::callback::Interval;
use leptos::prelude::*;

fn fmt_time(secs: i32) -> String {
    let s = secs.max(0);
    format!("{}:{:02}", s / 60, s % 60)
}

fn placeholder_for(sheet: Side, col: usize) -> &'static str {
    let speech_side = SPEECH_COLS[col].side;
    match col {
        0 | 1 => "Claim + why it matters…\ne.g. “Plastic → oceans 🌊”",
        4 | 5 => "➜ Extend: repeat the argument\n+ answer their best reply…",
        6 | 7 => "⚖️ “Vote for us because…”\ntheir #1 reason + weighing…",
        _ if speech_side == sheet => "🛡️ Defend: answer their reply…",
        _ => "↩️ Their answer to this argument…\n(leave blank = dropped 💤)",
    }
}

#[component]
pub fn App() -> impl IntoView {
    let loaded = load_initial();
    let state = RoundState::from_data(loaded.data);
    state.recovery_notice.set(loaded.recovery_notice);
    let step = RwSignal::new(0usize);
    let tab = RwSignal::new(Side::Pro);
    let focus_mode = RwSignal::new(true);
    let insert_ctx = InsertCtx::new();
    let initial_secs = (GUIDE[0].minutes * 60).cast_signed();
    let timer_secs = RwSignal::new(initial_secs);
    // Remainders of the timers not on screen: the speech timer plus each
    // team's whole-round prep balance (issue #6). Banking the display
    // into this on every freeze/tick is what lets timers survive switches.
    let timer_bank = RwSignal::new(TimerBank::new(initial_secs));
    let timer_label = RwSignal::new(GUIDE[0].title.to_string());
    let timer_announcement = RwSignal::new(String::new());
    let running = RwSignal::new(false);
    let interval = StoredValue::new_local(None::<Interval>);
    // Wall-clock deadline (performance.now ms) the running timer counts to.
    // Remaining time is derived from it on every tick, so background-tab
    // throttling cannot make the timer run slow.
    let deadline = StoredValue::new_local(None::<f64>);
    let audio_ctx = StoredValue::new_local(None::<web_sys::AudioContext>);

    let stop_timer = move || {
        // Freeze the displayed remainder before dropping the deadline, so
        // pausing between ticks does not lose (or gain) a second.
        if let (Some(end), Some(now)) = (deadline.get_value(), now_ms())
            && running.get_untracked()
        {
            timer_secs.set(((end - now) / 1_000.0).ceil().max(0.0) as i32);
        }
        timer_bank.update(|b| b.bank_displayed(timer_secs.get_untracked()));
        running.set(false);
        deadline.set_value(None);
        // Dropping the Interval cancels it: no callbacks fire afterwards.
        interval.set_value(None);
    };
    let set_step = move |next: usize| {
        let next = next.min(GUIDE.len() - 1);
        step.set(next);
        stop_timer();
        let full = (GUIDE[next].minutes * 60) as i32;
        timer_bank.update(|b| {
            b.start_speech(full);
        });
        timer_secs.set(full);
        timer_label.set(GUIDE[next].title.to_string());
        timer_announcement.set(String::new());
        // Rebuttals answer the opponent's case, so they are written on the
        // opponent's sheet; every other speech writes on the speaker's sheet.
        let sheet = match GUIDE[next].flow_col {
            Some(2) => Some(Side::Con),
            Some(3) => Some(Side::Pro),
            _ => GUIDE[next].side,
        };
        if let Some(side) = sheet {
            tab.set(side);
        }
    };
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
            let full = (GUIDE[step.get_untracked()].minutes * 60).cast_signed();
            timer_bank.update(|b| {
                b.start_speech(full);
            });
            timer_secs.set(full);
        }
        timer_label.set(GUIDE[step.get_untracked()].title.to_string());
        timer_announcement.set(String::new());
    };

    let st_flow = state.clone();
    let st_notice = state.clone();
    let st_saved = state.clone();
    let st_sample = state.clone();
    let st_clear = state.clone();
    let st_export = state.clone();
    let current_col = move || GUIDE[step.get()].flow_col;

    view! {
        <div class="page">
            <header class="hero">
                <div class="hero-mascot" aria-hidden="true">"🌸"</div>
                <div class="hero-text">
                    <h1>"Flower" <span class="hero-sub">"easy flowing for PF judges"</span></h1>
                    <p class="hero-tag">
                        "New to flowing? Perfect — you're exactly who this is for. "
                        "Follow the speech, follow the arrows ➜, and let Momo-chan's bubbles tell you what to write."
                    </p>
                </div>
                <div class="hero-actions no-print">
                    <button class="btn btn-soft" on:click=move |_| {
                        if st_sample.snapshot().is_empty()
                            || confirm("Load the example round? This replaces the current flow.")
                        {
                            st_sample.replace(RoundData::sample());
                            save(&st_sample);
                        }
                    }>
                        "🧸 See an example round"
                    </button>
                    <button class="btn btn-soft" on:click=move |_| {
                        if confirm("Start a brand-new round? This clears the current flow.") {
                            st_clear.replace(RoundData::default());
                            save(&st_clear);
                            set_step(0);
                            // A brand-new round is the other explicit prep
                            // reset: both teams start again at 3:00.
                            timer_bank.update(TimerBank::reset_all_prep);
                        }
                    }>"🫧 New round"</button>
                    <button class="btn btn-soft" on:click=move |_| print_page()>"🖨️ Print / save PDF"</button>
                </div>
            </header>

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

            // ---------- Round setup ----------
            <section class="card setup-card">
                <h2><span class="card-ico">"🎀"</span> "1 · Set up the round" <span class="saved-note" class:saved-note-failed=move || st_saved.save_failed.get()>{move || if st_saved.save_failed.get() { "⚠️ Not saved — this browser is blocking or out of storage; copy anything important before closing" } else { "✓ auto-saves on this device" }}</span></h2>
                <div class="setup-grid">
                    <label class="field field-wide">
                        <span>"Resolution / topic"</span>
                        <input type="text" id="field-topic" placeholder="Resolved: …"
                            on:focus=move |_| insert_ctx.focus(TextTarget::Topic)
                            prop:value=move || state.topic.get()
                            on:input={let s = state.clone(); move |ev| { s.topic.set(event_target_value(&ev)); save(&s); }} />
                    </label>
                    <label class="field">
                        <span>"🌸 Pro team"</span>
                        <input type="text" id="field-pro-team" placeholder="School / speaker names"
                            on:focus=move |_| insert_ctx.focus(TextTarget::ProTeam)
                            prop:value=move || state.pro_team.get()
                            on:input={let s = state.clone(); move |ev| { s.pro_team.set(event_target_value(&ev)); save(&s); }} />
                    </label>
                    <label class="field">
                        <span>"🍃 Con team"</span>
                        <input type="text" id="field-con-team" placeholder="School / speaker names"
                            on:focus=move |_| insert_ctx.focus(TextTarget::ConTeam)
                            prop:value=move || state.con_team.get()
                            on:input={let s = state.clone(); move |ev| { s.con_team.set(event_target_value(&ev)); save(&s); }} />
                    </label>
                </div>
                <div class="howto">
                    <div class="howto-step"><b>"📄 Two pages"</b><span>"Pro arguments live on the blue sheet, Con arguments on the green sheet — never mixed."</span></div>
                    <div class="howto-arrow" aria-hidden="true">"➜"</div>
                    <div class="howto-step"><b>"⬇️ Then across"</b><span>"Each new speech gets the next column. Answers go directly right of the argument they answer."</span></div>
                    <div class="howto-arrow" aria-hidden="true">"➜"</div>
                    <div class="howto-step"><b>"💤 Blank = dropped"</b><span>"No answer in the box? Leave it blank. If they extend it later, draw the arrow ➜."</span></div>
                </div>
            </section>

            // ---------- Guided speech walk ----------
            <section class="card guide-card no-print">
                <h2><span class="card-ico">"🧭"</span> "2 · Follow the round, one speech at a time"</h2>
                <div class="stepper">
                    {GUIDE.iter().enumerate().map(|(i, g)| {
                        let cls = move || {
                            if step.get() == i { "step-chip is-now" }
                            else if i < step.get() { "step-chip is-done" }
                            else { "step-chip" }
                        };
                        view! {
                            <span class="step-wrap">
                                {if i > 0 { Some(view! { <span class="step-arrow" aria-hidden="true">"➜"</span> }) } else { None }}
                                <button class=cls on:click=move |_| set_step(i)>
                                    {if i < step.get_untracked() { "✓ " } else { "" }}
                                    {g.title}
                                </button>
                            </span>
                        }
                    }).collect_view()}
                </div>

                <div class="guide-body">
                    <div class="bubble-wrap">
                        <div class="mascot" aria-hidden="true">"🐰"</div>
                        <div class="bubble">
                            <p class="bubble-title">
                                {move || GUIDE[step.get()].title}
                                <span class="bubble-time">{move || format!(" · {} min", GUIDE[step.get()].minutes)}</span>
                            </p>
                            <p class="bubble-main">{move || GUIDE[step.get()].bubble}</p>
                            <ul class="bubble-list">
                                <li><b>"👂 Listen for: "</b>{move || GUIDE[step.get()].listen_for}</li>
                                <li><b>"✏️ Write: "</b>{move || GUIDE[step.get()].do_it}</li>
                                <li><b>"🌧️ Watch out: "</b>{move || GUIDE[step.get()].watch_out}</li>
                            </ul>
                        </div>
                    </div>

                    <div class="timer-box">
                        <div class="timer-label">{move || timer_label.get()}</div>
                        <div class="timer-digits" class:timer-done=move || timer_secs.get() == 0>
                            {move || fmt_time(timer_secs.get())}
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
                        <div class="prep-row">
                            <span>"Prep left (3:00 per team, whole round):"</span>
                            <button class="btn btn-chip btn-pro" on:click=move |_| start_prep(Side::Pro)>
                                {move || format!("🌸 Pro · {}", fmt_time(timer_bank.get().secs_for(TimerKind::Prep(Side::Pro), timer_secs.get())))}
                            </button>
                            <button class="btn btn-chip" title="Reset only Pro prep back to 3:00" aria-label="Reset only Pro prep back to 3:00"
                                on:click=move |_| { if confirm("Reset only 🌸 Pro prep back to 3:00?") { reset_prep(Side::Pro); } }>"↺ 🌸"</button>
                            <button class="btn btn-chip btn-con" on:click=move |_| start_prep(Side::Con)>
                                {move || format!("🍃 Con · {}", fmt_time(timer_bank.get().secs_for(TimerKind::Prep(Side::Con), timer_secs.get())))}
                            </button>
                            <button class="btn btn-chip" title="Reset only Con prep back to 3:00" aria-label="Reset only Con prep back to 3:00"
                                on:click=move |_| { if confirm("Reset only 🍃 Con prep back to 3:00?") { reset_prep(Side::Con); } }>"↺ 🍃"</button>
                        </div>
                        <div class="step-nav">
                            <button class="btn btn-soft" disabled={move || step.get() == 0}
                                on:click=move |_| set_step(step.get_untracked().saturating_sub(1))>"⬅ Prev speech"</button>
                            <button class="btn btn-main" disabled={move || step.get() >= GUIDE.len() - 1}
                                on:click=move |_| set_step(step.get_untracked() + 1)>"Next speech ➜"</button>
                        </div>
                    </div>
                </div>

                <label class="focus-toggle">
                    <input type="checkbox" prop:checked=move || focus_mode.get()
                        on:change=move |ev| focus_mode.set(event_target_checked(&ev)) />
                    <span><b>"🎯 Focus mode"</b> " — softly dim speeches that aren't happening right now, so your eyes always know where to write."</span>
                </label>
            </section>

            // ---------- Flow sheets ----------
            <section class="card flow-card">
                <h2><span class="card-ico">"🌷"</span> "3 · Your flow" </h2>
                <div class="tabs no-print" role="tablist">
                    <button class="tab tab-pro" class:tab-on=move || tab.get() == Side::Pro
                        on:click=move |_| tab.set(Side::Pro)>
                        "🌸 Pro sheet" <span class="tab-hint">"their arguments ↓, round →"</span>
                    </button>
                    <span class="tab-arrow" aria-hidden="true">"⇄"</span>
                    <button class="tab tab-con" class:tab-on=move || tab.get() == Side::Con
                        on:click=move |_| tab.set(Side::Con)>
                        "🍃 Con sheet" <span class="tab-hint">"their arguments ↓, round →"</span>
                    </button>
                </div>
                <p class="flow-legend no-print">
                    "Read each row left ➜ right: that's one argument's whole story. "
                    <span class="legend-chip">"💤 dropped = nobody answered it"</span>
                    <span class="legend-chip">"➜ extended = they carried it forward"</span>
                    <span class="legend-chip">"⭐ star the arguments the round will be decided on"</span>
                </p>

                {{let s = st_flow.clone(); move || {
                    let side = tab.get();
                    view! { <FlowSheet state=s.clone() side=side current_col=current_col() focus_mode=focus_mode insert_ctx=Some(insert_ctx) /> }
                }}}

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
                <h2>"🌸 Pro flow"</h2>
                <FlowSheet state=st_flow.clone() side=Side::Pro current_col=None focus_mode=focus_mode insert_ctx=None />
                <h2 style="margin-top:20px">"🍃 Con flow"</h2>
                <FlowSheet state=st_flow.clone() side=Side::Con current_col=None focus_mode=focus_mode insert_ctx=None />
            </section>

            // ---------- Crossfire notes ----------
            <section class="card notes-card">
                <h2><span class="card-ico">"☕"</span> "Crossfire notes" <span class="card-sub">"optional — only concessions & admissions"</span></h2>
                <textarea class="big-notes" id="notes-crossfire" rows="3" placeholder="e.g. “Pro agreed recycling is only 9% — that helps Con C1…”"
                    on:focus=move |_| insert_ctx.focus(TextTarget::CrossfireNotes)
                    prop:value=move || state.crossfire_notes.get()
                    on:input={let s = state.clone(); move |ev| { s.crossfire_notes.set(event_target_value(&ev)); save(&s); }}></textarea>
            </section>

            // ---------- Decision ----------
            <section class="card ballot-card">
                <h2><span class="card-ico">"🏆"</span> "4 · Your decision" </h2>
                <p class="ballot-intro">"No perfect flow required. Ask three questions:"</p>
                <ol class="ballot-qs">
                    <li>"Which ⭐ arguments were " <b>"answered the least"</b> " (💤 dropped or weakly answered)?"</li>
                    <li>"Which side " <b>"extended"</b> " ➜ their arguments into Summary & Final Focus?"</li>
                    <li>"In Final Focus, who gave the better " <b>"weighing"</b> " ⚖️ — bigger, sooner, or more likely impact?"</li>
                </ol>
                <DroppedHints state=state.clone() step=step />
                <div class="winner-row no-print">
                    <span>"I vote for:"</span>
                    <button class="btn winner-btn btn-pro" class:winner-on=move || state.winner.get() == "Pro"
                        on:click={let s = state.clone(); move |_| { s.winner.set("Pro".to_string()); save(&s); }}>"🌸 Pro"</button>
                    <button class="btn winner-btn btn-con" class:winner-on=move || state.winner.get() == "Con"
                        on:click={let s = state.clone(); move |_| { s.winner.set("Con".to_string()); save(&s); }}>"🍃 Con"</button>
                </div>
                <p class="winner-print">{move || {
                    let w = state.winner.get();
                    if w.is_empty() { "Winner: __________".to_string() } else { format!("Winner: {w}") }
                }}</p>
                <label class="field">
                    <span>"Reason for decision (say it in plain words — debaters learn from this!) 🗣️"</span>
                    <textarea class="big-notes" id="notes-rfd" rows="4"
                        on:focus=move |_| insert_ctx.focus(TextTarget::Rfd)
                        placeholder="“I voted ___ because the ___ argument was dropped / extended / weighed better when…”"
                        prop:value=move || state.rfd.get()
                        on:input={let s = state.clone(); move |ev| { s.rfd.set(event_target_value(&ev)); save(&s); }}></textarea>
                </label>
                <div class="ballot-actions no-print">
                    <button class="btn btn-main" on:click=move |_| copy_text(&decision_text(&st_export))>"📋 Copy decision summary"</button>
                    <button class="btn btn-soft" on:click=move |_| print_page()>"🖨️ Print flow + ballot"</button>
                </div>
            </section>

            <footer class="footer">
                <p>"Made with 🌸 for lay judges — based on the NSDA “Intro to Flowing” judge training. "
                "Remember: a blank box is information, an arrow is a story, and your plain-words reason helps debaters most of all."</p>
                <p class="footer-small">"Your flow auto-saves only in this browser (localStorage) — nothing leaves your device."</p>
            </footer>
        </div>
    }
}

fn decision_text(state: &RoundState) -> String {
    let d = state.snapshot();
    let winner = if d.winner.is_empty() {
        "—".to_string()
    } else {
        d.winner.clone()
    };
    format!(
        "🌸 Flower — PF decision\nTopic: {}\nPro: {} · Con: {}\nWinner: {}\nReason: {}",
        d.topic, d.pro_team, d.con_team, winner, d.rfd
    )
}

#[component]
fn DroppedHints(state: RoundState, step: RwSignal<usize>) -> impl IntoView {
    // Tracked snapshot + the guide step: hints re-derive live as the
    // judge types, and nothing is flagged before its rebuttal is over.
    move || {
        let hints: Vec<String> = state
            .snapshot_tracked()
            .dropped_arguments(step.get())
            .into_iter()
            .map(|d| {
                format!(
                    "💤 {}'s “{}” got no {} answer in any speech so far — if it was extended, it may decide the round.",
                    d.owner.label(),
                    d.name,
                    d.answerer.label()
                )
            })
            .collect();
        if hints.is_empty() {
            view! { <p class="dropped-none">"✨ No clearly dropped arguments spotted yet — check for blank answer boxes on each row."</p> }.into_any()
        } else {
            view! {
                <div class="dropped-box">
                    <p class="dropped-title">"🔎 Arguments that look dropped (blank answer box):"</p>
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
    current_col: Option<usize>,
    focus_mode: RwSignal<bool>,
    insert_ctx: Option<InsertCtx>,
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
                    <div class="fh fh-corner">
                        {format!("{} {} arguments ↓", side.emoji(), side.label())}
                        <span class="fh-sub">"one row = one argument"</span>
                    </div>
                    {SPEECH_COLS.iter().enumerate().map(|(col, sc)| {
                        let is_now = current_col == Some(col);
                        let dim = move || focus_mode.get() && current_col.is_some() && !is_now;
                        view! {
                            <div class="fh" class:fh-now=is_now class:fh-dim=dim>
                                {if is_now { Some(view! { <span class="fh-badge">"✏️ you are here"</span> }) } else { None }}
                                <span class="fh-name">{sc.full}</span>
                                <span class="fh-sub">{format!("{} · {} min", sc.side.emoji(), sc.minutes)}</span>
                            </div>
                        }
                    }).collect_view()}

                    <For
                        each=move || rows.get()
                        key=|row: &RowState| row.id
                        children={let s = state.clone(); move |row: RowState| {
                            view! { <FlowRow state=s.clone() side=side row=row current_col=current_col focus_mode=focus_mode insert_ctx=insert_ctx /> }
                        }}
                    />
                </div>
            </div>
            <button class="btn btn-main add-row no-print" on:click={let s = state.clone(); move |_| s.add_row(side)}>
                "➕ Add an argument row"
            </button>
            <p class="sheet-tip no-print">
                {match side {
                    Side::Pro => "Tip: start a new row each time Pro says “our next contention is…”. 🌸",
                    Side::Con => "Tip: start a new row each time Con says “our next contention is…”. 🍃",
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
    current_col: Option<usize>,
    focus_mode: RwSignal<bool>,
    insert_ctx: Option<InsertCtx>,
) -> impl IntoView {
    let row_id = row.id;
    let title_target = TextTarget::RowTitle { side, row_id };
    let title_id = insert_ctx.map(|_| title_target.id());
    view! {
        <div class="fr-title">
            <input type="text" id=title_id placeholder="Contention name…"
                on:focus=move |_| if let Some(ctx) = insert_ctx { ctx.focus(title_target) }
                prop:value=move || row.title.get()
                on:input={let s = state.clone(); let r = row.clone(); move |ev| { r.title.set(event_target_value(&ev)); save(&s); }} />
            <button class="row-del no-print" title="Delete this argument row"
                on:click={let s = state.clone(); move |_| {
                    if confirm("Delete this argument row?") { s.remove_row(side, row_id); }
                }}>"✕"</button>
        </div>
        {row.cells.iter().enumerate().map(|(col, cell)| {
            let cell = *cell;
            let is_now = current_col == Some(col);
            let dim = move || focus_mode.get() && current_col.is_some() && !is_now;
            let placeholder = placeholder_for(side, col);
            let cell_target = TextTarget::FlowCell { side, row_id, col };
            let cell_id = insert_ctx.map(|_| cell_target.id());
            view! {
                <div class="cell" class:cell-now=is_now class:cell-dim=dim
                    class:cell-dropped=move || cell.mark.get() == Mark::Dropped
                    class:cell-extended=move || cell.mark.get() == Mark::Extended
                    class:cell-key=move || cell.mark.get() == Mark::Key>
                    {if col > 0 {
                        Some(view! { <span class="cell-arrow" aria-hidden="true">"➜"</span> })
                    } else { None }}
                    <textarea rows="3" id=cell_id placeholder=placeholder
                        prop:value=move || cell.text.get()
                        on:input={let s = state.clone(); move |ev| { cell.text.set(event_target_value(&ev)); save(&s); }}
                        on:focus=move |_| if let Some(ctx) = insert_ctx { ctx.focus(cell_target) }></textarea>
                    <div class="cell-marks no-print">
                        {[(Mark::Dropped, "💤 dropped"), (Mark::Extended, "➜ extended"), (Mark::Key, "⭐ key")].into_iter().map(|(mark, label)| {
                            view! {
                                <button class="mark-btn"
                                    class:mark-on=move || cell.mark.get() == mark
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
