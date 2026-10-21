//! In-UI confirmation for destructive actions.

use crate::model::Side;
use leptos::prelude::*;
/// Destructive actions that ask in the shared in-UI dialog before
/// running, instead of the native `window.confirm` (which appears
/// wherever the browser puts it and matches nothing else on the page).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingConfirm {
    NewRound,
    ExampleRound,
    ResetPrep(Side),
}

impl PendingConfirm {
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::NewRound => "Start a brand-new round?",
            Self::ExampleRound => "Load the example round?",
            Self::ResetPrep(Side::Pro) => "Reset Pro prep?",
            Self::ResetPrep(Side::Con) => "Reset Con prep?",
        }
    }

    pub(crate) fn body(self) -> &'static str {
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

    pub(crate) fn keep_label(self) -> &'static str {
        match self {
            Self::NewRound => "Keep current round",
            Self::ExampleRound => "Keep current flow",
            Self::ResetPrep(_) => "Keep prep time",
        }
    }

    pub(crate) fn confirm_label(self) -> &'static str {
        match self {
            Self::NewRound => "Start new round",
            Self::ExampleRound => "Load example round",
            Self::ResetPrep(Side::Pro) => "Reset Pro prep",
            Self::ResetPrep(Side::Con) => "Reset Con prep",
        }
    }
}

pub(crate) fn confirm_dialog(
    pending_confirm: RwSignal<Option<PendingConfirm>>,
    keep_button: NodeRef<leptos::html::Button>,
    app_owner: Option<Owner>,
    on_confirm: Callback<PendingConfirm>,
) -> impl IntoView {
    let keep_confirm_button = keep_button;
    let confirm_action = on_confirm;
    {
        move || {
            pending_confirm.get().map(|pending| { let app_owner = app_owner.clone(); view! {
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
            } })
        }
    }
}
