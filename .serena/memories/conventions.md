# Conventions

## Structure
- Strict 3-layer split, do not blur: `model.rs` = pure serde data + constants (no Leptos, no web-sys); `state.rs` = signal-backed mirror types + localStorage/browser helpers; `app.rs` = all Leptos components/UI logic. New persisted fields must be added to both `RoundData` (model) and `RoundState` (state) plus conversion code.
- Components are plain `#[component]` fns in `app.rs` (PascalCase: `App`, `FlowSheet`, `FlowRow`, `DroppedHints`); helpers snake_case (`fmt_time`, `placeholder_for`, `decision_text`, `insert_shorthand`). No separate component files — keep UI in `app.rs` unless it outgrows it.
- Constants SCREAMING_SNAKE in `model.rs`: `SPEECH_COLS`, `GUIDE`, `SHORTHAND`, `STORAGE_KEY`. Content/guide copy lives as data in `model.rs`, not hardcoded in components.

## Rust style
- Avoid `unwrap`/`expect` (clippy warn): use `unwrap_or_default`, `let _ =` for fire-and-forget web-sys/storage results, `.ok().flatten()` chains (see `load_initial` in state.rs).
- Indexing is allowed by design (clamped steps, enumerated columns, exactly-sized cell vecs); don't add guards that fight this without changing the invariant.
- Derive `Clone, Serialize, Deserialize` on stored model types; stored JSON must stay backward-loadable or bump `STORAGE_KEY` version (`flower-round-v1`).

## UI / design
- Kawaii pastel Japanese aesthetic: Pro = pink sheet, Con = green sheet; Fredoka headings / Nunito body; emoji as iconography (🌸🍃🐰⭐⚖️) — part of the design language, keep consistent.
- Beginner-first UX: Momo-chan suggestion bubbles guide through the 11 `GUIDE` steps one at a time; Focus mode dims non-current speeches; tap-to-insert shorthand; timers for speeches + prep.
- `style.css` is the single stylesheet (no CSS modules/inline style system); responsive + `@media print` must keep printing both sheets + ballot like the two paper pages.
- Copy tone: plain-language for lay judges, explain jargon inline; suggestion/guidance text goes in `GUIDE` data.
