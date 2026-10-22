//! Ballot helpers: dropped-argument hints for the decision section.

use crate::model::{DroppedArgument, Side};
use crate::openrouter::{self, ApiKey};
use crate::shorthand::{InsertCtx, TextTarget};
use crate::state::{RoundState, copy_text, print_page, save};
use leptos::prelude::*;
use leptos::task::spawn_local;
pub(crate) fn decision_text(state: &RoundState) -> String {
    state.snapshot().decision_text()
}

fn dropped_hint(d: &DroppedArgument) -> String {
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
}

#[component]
pub(crate) fn DroppedHints(state: RoundState, step: RwSignal<usize>) -> impl IntoView {
    // Memo over only the real inputs (speaking order, row titles/cells,
    // guide step): typing in the topic, notes, or RFD boxes never
    // re-derives hints, and the `Eq` hint list suppresses the downstream
    // DOM rebuild whenever the computed hints are unchanged.
    let hints = Memo::new(move |_| {
        state
            .dropped_arguments_tracked(step.get())
            .iter()
            .map(dropped_hint)
            .collect::<Vec<String>>()
    });
    move || {
        let hints = hints.get();
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

pub(crate) struct BallotProps {
    pub(crate) state: RoundState,
    pub(crate) step: RwSignal<usize>,
    pub(crate) insert_ctx: InsertCtx,
    pub(crate) copy_feedback: RwSignal<Option<&'static str>>,
    pub(crate) openrouter_key: RwSignal<Option<ApiKey>>,
}

fn ai_summary_button(
    state: RoundState,
    openrouter_key: RwSignal<Option<ApiKey>>,
    ai_summary: RwSignal<Option<String>>,
    ai_busy: RwSignal<bool>,
    ai_error: RwSignal<Option<String>>,
) -> impl IntoView {
    let st_click = state;
    view! {
            <button class="btn btn-soft" aria-describedby="ai-summary-note" disabled=move || ai_busy.get() || openrouter_key.get().is_none()
                on:click=move |_| {
                    let Some(key) = openrouter_key.get_untracked() else { return };
                    let round = st_click.snapshot();
                    ai_busy.set(true);
                    ai_error.set(None);
                    spawn_local(async move {
                        match openrouter::summarize(&key, &round).await {
                            Ok(text) => ai_summary.set(Some(text)),
                            Err(e) => ai_error.set(Some(e)),
                        }
                        ai_busy.set(false);
                    });
                }>
                {move || if ai_busy.get() { "✨ Asking OpenRouter…" } else { "✨ AI summary of this round" }}
            </button>
    }
}

fn ai_summary_output(
    state: RoundState,
    openrouter_key: RwSignal<Option<ApiKey>>,
    copy_feedback: RwSignal<Option<&'static str>>,
    ai_summary: RwSignal<Option<String>>,
    ai_error: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="ai-summary no-print">
            {move || if openrouter_key.get().is_none() {
                view! { <span class="ai-hint" id="ai-summary-note">"Connect to OpenRouter (top of page) first — it uses your own account."</span> }.into_any()
            } else {
                view! { <span class="ai-hint" id="ai-summary-note">"Sends this flow to OpenRouter — nothing is sent until you tap."</span> }.into_any()
            }}
            {move || ai_error.get().map(|e| view! { <p class="ai-error" role="alert">{format!("⚠️ {e}")}</p> })}
            {move || ai_summary.get().map(|text| {
                let for_copy = text.clone();
                let for_rfd = text;
                let s = state.clone();
                view! {
                    <div class="ai-summary-box">
                        <p class="ai-summary-body">{for_rfd.clone()}</p>
                        <div class="ballot-actions">
                            <button class="btn btn-soft" on:click=move |_| {
                                let text = for_copy.clone();
                                spawn_local(async move {
                                    copy_feedback.set(Some(if copy_text(&text).await { "Copied ✓" } else { "⚠️ Copy failed — clipboard blocked by the browser" }));
                                });
                            }>"📋 Copy AI summary"</button>
                            <button class="btn btn-soft" on:click=move |_| { s.rfd.set(for_rfd.clone()); save(&s); }>"Use as my reason for decision"</button>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}

pub(crate) fn ballot_card(props: BallotProps) -> impl IntoView {
    let BallotProps {
        state,
        step,
        insert_ctx,
        copy_feedback,
        openrouter_key,
    } = props;
    let st_export = state.clone();
    let ai_summary = RwSignal::new(None::<String>);
    let ai_busy = RwSignal::new(false);
    let ai_error = RwSignal::new(None::<String>);
    view! {
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
                    <button class="btn winner-btn btn-pro" class:winner-on=move || state.winner.get() == Some(Side::Pro) aria-pressed=move || if state.winner.get() == Some(Side::Pro) { "true" } else { "false" }
                        on:click={let s = state.clone(); move |_| { s.winner.set(Some(Side::Pro)); save(&s); }}>"Pro"</button>
                    <button class="btn winner-btn btn-con" class:winner-on=move || state.winner.get() == Some(Side::Con) aria-pressed=move || if state.winner.get() == Some(Side::Con) { "true" } else { "false" }
                        on:click={let s = state.clone(); move |_| { s.winner.set(Some(Side::Con)); save(&s); }}>"Con"</button>
                </div>
                <p class="winner-print">{move || match state.winner.get() {
                    None => "Winner: __________".to_string(),
                    Some(w) => format!("Winner: {}", w.label()),
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
                    <button class="btn btn-soft" on:click=move |_| {
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
                    {ai_summary_button(state.clone(), openrouter_key, ai_summary, ai_busy, ai_error)}
                </div>
                {ai_summary_output(state.clone(), openrouter_key, copy_feedback, ai_summary, ai_error)}
            </section>
    }
}
