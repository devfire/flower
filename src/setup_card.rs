//! Round setup card: topic/teams, guide bubble, speech + prep timers.

use crate::confirm::PendingConfirm;
use crate::guide::{GUIDE, GuideStep};
use crate::model::Side;
use crate::shorthand::{InsertCtx, TextTarget};
use crate::state::{RoundState, save};
use crate::timer::{TimerBank, fmt_time};
use crate::topic::NsdaTopic;
use leptos::prelude::*;

pub(crate) struct SetupProps<F, G, H> {
    pub(crate) state: RoundState,
    pub(crate) nsda_topic: RwSignal<NsdaTopic>,
    pub(crate) insert_ctx: InsertCtx,
    pub(crate) guide: Signal<[GuideStep; GUIDE.len()]>,
    pub(crate) step: RwSignal<usize>,
    pub(crate) timer_label: RwSignal<String>,
    pub(crate) timer_secs: RwSignal<i32>,
    pub(crate) timer_announcement: RwSignal<String>,
    pub(crate) running: RwSignal<bool>,
    pub(crate) timer_bank: RwSignal<TimerBank>,
    pub(crate) prep_running: RwSignal<Option<Side>>,
    pub(crate) pending_confirm: RwSignal<Option<PendingConfirm>>,
    pub(crate) set_step: Callback<usize>,
    pub(crate) toggle_timer: F,
    pub(crate) reset_speech: G,
    pub(crate) toggle_prep: H,
}

// Pedantic `too_many_lines`: one cohesive card view; splitting it would
// thread every timer signal through a child view for no gain.
#[allow(clippy::too_many_lines)]
pub(crate) fn setup_card<F, G, H>(props: SetupProps<F, G, H>) -> impl IntoView
where
    F: Fn(web_sys::MouseEvent) + 'static,
    G: Fn(web_sys::MouseEvent) + 'static,
    H: Fn(Side) + Copy + 'static,
{
    let SetupProps {
        state,
        nsda_topic,
        insert_ctx,
        guide,
        step,
        timer_label,
        timer_secs,
        timer_announcement,
        running,
        timer_bank,
        prep_running,
        pending_confirm,
        set_step,
        toggle_timer,
        reset_speech,
        toggle_prep,
    } = props;
    let st_saved = state.clone();
    let st_topic = state.clone();
    view! {
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
                                <button class="btn btn-soft" on:click=reset_speech>"↺ Reset"</button>
                            </div>
                        </div>
                        <div class="prep-row">
                            <span class="prep-title">"Prep left " <span class="prep-sub">"(3m/team)"</span></span>
                            <div class="prep-group">
                                <button class="btn btn-chip btn-pro prep-main" class:chip-on=move || prep_running.get() == Some(Side::Pro)
                                    aria-pressed=move || if prep_running.get() == Some(Side::Pro) { "true" } else { "false" }
                                    on:click=move |_| toggle_prep(Side::Pro)>
                                    {move || {
                                        let action = if prep_running.get() == Some(Side::Pro) { "❚❚" } else { "▶" };
                                        format!("Pro · {} · {action}", fmt_time(timer_bank.get().prep_secs(Side::Pro)))
                                    }}
                                </button>
                                <button class="btn btn-chip prep-reset" title="Reset only Pro prep back to 3:00" aria-label="Reset only Pro prep back to 3:00" aria-haspopup="dialog"
                                    aria-expanded=move || if pending_confirm.get() == Some(PendingConfirm::ResetPrep(Side::Pro)) { "true" } else { "false" }
                                    on:click=move |_| pending_confirm.set(Some(PendingConfirm::ResetPrep(Side::Pro)))>"↺"</button>
                            </div>
                            <div class="prep-group">
                                <button class="btn btn-chip btn-con prep-main" class:chip-on=move || prep_running.get() == Some(Side::Con)
                                    aria-pressed=move || if prep_running.get() == Some(Side::Con) { "true" } else { "false" }
                                    on:click=move |_| toggle_prep(Side::Con)>
                                    {move || {
                                        let action = if prep_running.get() == Some(Side::Con) { "❚❚" } else { "▶" };
                                        format!("Con · {} · {action}", fmt_time(timer_bank.get().prep_secs(Side::Con)))
                                    }}
                                </button>
                                <button class="btn btn-chip prep-reset" title="Reset only Con prep back to 3:00" aria-label="Reset only Con prep back to 3:00" aria-haspopup="dialog"
                                    aria-expanded=move || if pending_confirm.get() == Some(PendingConfirm::ResetPrep(Side::Con)) { "true" } else { "false" }
                                    on:click=move |_| pending_confirm.set(Some(PendingConfirm::ResetPrep(Side::Con)))>"↺"</button>
                            </div>
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
    }
}
