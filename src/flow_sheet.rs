//! The flow sheet: one row per argument, one column per speech.

use crate::model::{Mark, SPEECH_COLS, Side, SpeakingOrder};
use crate::shorthand::{InsertCtx, TextTarget};
use crate::state::{RoundState, RowState, save};
use leptos::prelude::*;
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
// Leptos component props are owned by design; `state` is cloned into
// the row closures rather than consumed once.
#[allow(clippy::needless_pass_by_value)]
#[component]
pub(crate) fn FlowSheet(
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
                <div class="flow-grid" style=format!("grid-template-columns: minmax(8rem, 11rem) repeat({}, minmax(6.5rem, 1fr)) auto;", SPEECH_COLS.len())>
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
    let target = (side, row_id);
    let deleting_row = state.deleting_row;
    let confirming_delete = Memo::new(move |_| deleting_row.get() == Some(target));
    let close_confirm = move || deleting_row.update(|open| if *open == Some(target) { *open = None; });
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
        </div>
        <div class="row-del-wrap no-print" style=format!("grid-column: {}; grid-row: {};", SPEECH_COLS.len() + 2, row_index + 1)
                on:keydown=move |ev| {
                    if ev.key() == "Escape" { close_confirm(); }
                }>
                <button class="row-del" class:row-del-armed=move || confirming_delete.get()
                    title="Delete this argument row" aria-label=format!("Delete {} argument {} row", side.label(), row_index)
                    aria-expanded=move || if confirming_delete.get() { "true" } else { "false" }
                    on:click=move |_| {
                        deleting_row.update(|open| *open = if *open == Some(target) { None } else { Some(target) });
                    }>"−"</button>
                {{let state = state.clone(); move || confirming_delete.get().then(|| {
                    let s = state.clone();
                    view! {
                        <div class="row-del-pop" role="alertdialog"
                            aria-label=format!("Confirm deleting {} argument {}", side.label(), row_index)>
                            <span class="row-del-q">"Delete this row?"</span>
                            <span class="row-del-actions">
                                <button class="row-del-confirm" on:click={let s = s.clone(); move |_| {
                                    close_confirm();
                                    s.remove_row(side, row_id);
                                }}>"Delete"</button>
                                <button class="row-del-keep" node_ref=keep_button
                                    on:click=move |_| close_confirm()>"Keep"</button>
                            </span>
                        </div>
                    }
                })}}
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
