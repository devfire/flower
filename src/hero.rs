//! Page hero: title and round-level actions.

use crate::confirm::PendingConfirm;
use crate::model::RoundData;
use crate::openrouter::{self, ApiKey};
use crate::state::{RoundState, print_page, save};
use leptos::prelude::*;
use leptos::task::spawn_local;

pub(crate) struct HeroProps {
    pub(crate) state: RoundState,
    pub(crate) pending_confirm: RwSignal<Option<PendingConfirm>>,
    pub(crate) reset_round_walk: Callback<()>,
    pub(crate) openrouter_key: RwSignal<Option<ApiKey>>,
    pub(crate) openrouter_status: RwSignal<Option<String>>,
}

pub(crate) fn hero_header(props: HeroProps) -> impl IntoView {
    let HeroProps {
        state,
        pending_confirm,
        reset_round_walk,
        openrouter_key,
        openrouter_status,
    } = props;
    let st_sample = state.clone();
    view! {
            <header class="hero">
                <div class="hero-mascot" aria-hidden="true">""</div>
                <div class="hero-text">
                    <h1>"Flow-er" <span class="hero-sub">"easy flowing for PF judges"</span></h1>
                    <p class="hero-tag">
                        "New to flowing? Perfect — you're exactly who this is for. "
                        "Follow the speech, follow the arrows ➜, and let Momo-chan's bubbles tell you what to write."
                    </p>
                </div>
                <div class="hero-actions no-print">
                    {move || if openrouter_key.get().is_some() {
                        view! {
                            <button class="btn btn-soft" title="Forget the OpenRouter key stored for this tab"
                                on:click=move |_| {
                                    openrouter::disconnect();
                                    openrouter_key.set(None);
                                    openrouter_status.set(Some("Disconnected from OpenRouter.".to_owned()));
                                }>
                                "✓ OpenRouter connected · Disconnect"
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <button class="btn btn-soft" title="Sign in with your own OpenRouter account to enable AI summaries (your credits, your key — stored only for this tab)"
                                on:click=move |_| {
                                    spawn_local(async move {
                                        if let Err(e) = openrouter::begin_connect().await {
                                            openrouter_status.set(Some(format!("⚠️ {e}")));
                                        }
                                    });
                                }>
                                "Connect to OpenRouter"
                            </button>
                        }.into_any()
                    }}
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
                {move || openrouter_status.get().map(|status| view! { <p class="or-status no-print" role="status" aria-live="polite">{status}</p> })}
            </header>
    }
}
