# Conventions

## Structure
- Layered split, do not blur: `model.rs`/`guide.rs`/`analysis.rs`/`round_save.rs`/`progress.rs`/`timer.rs` = pure data/logic (no Leptos DOM, no web-sys except where noted); `state.rs` = signal-backed mirror types; `persistence.rs`/`platform.rs`/`topic.rs`/`openrouter.rs` = browser/network boundary; leaf UI in `hero.rs`/`setup_card.rs`/`flow_sheet.rs`/`ballot.rs`/`confirm.rs`; `app.rs` = `App` orchestration only.
- Hard cap: ≤500 LOC per module (AGENTS.md). `app.rs` is at the cap — extract a new module instead of growing it; `model.rs` exceeds it only via its `#[cfg(test)]` module.
- New persisted fields: add to `RoundData` (model) + `RoundState` (state) + conversion code, with serde defaults/custom deserializers so old saves load (precedents: `speaking_order`, `winner_serde`, `cells_serde`).
- Components: `#[component]` PascalCase for signal-owning components (`App`, `FlowSheet`, `FlowRow`, `DroppedHints`); plain snake_case fns taking a `*Props` struct for presentational pieces (`hero_header`, `setup_card`, `ballot_card`, `confirm_dialog`). Helpers snake_case. Constants SCREAMING_SNAKE, mostly in their owning module (`SPEECH_COLS`/`GUIDE` in guide.rs, storage keys in round_save.rs/progress.rs). Guide/speech content lives as data in `guide.rs`, not hardcoded in components.

## Rust style
- AGENTS.md: activate Serena first, use Serena for targeted reads/writes (not in worktrees — write conflicts), read `rust-advanced-patterns` + `rust-defensive-programming` skills before writing new Rust.
- Avoid `unwrap`/`expect` (clippy warn + pedantic gate): use `unwrap_or_default`, `let _ =` for fire-and-forget web-sys/storage results, `.ok().flatten()` chains.
- Indexing is allowed by design (clamped steps, enumerated columns, exactly-sized cell vecs); don't add guards that fight this without changing the invariant.
- Derive `Clone, Serialize, Deserialize` on stored model types; stored JSON must stay backward-loadable or bump the storage key version.
- Secrets: `ApiKey` in openrouter.rs is opaque with redacted `Debug`; keys live only in sessionStorage, never localStorage/logs.

## UI / design
- Kawaii pastel Japanese aesthetic: Pro = blue sheet, Con = green/mint sheet; Fredoka headings / Nunito body; emoji as iconography — part of the design language, keep consistent (but no `Side::emoji()`; it was removed as needless API).
- Beginner-first UX: Momo-chan suggestion bubbles guide through the 11 `GUIDE` steps one at a time; Focus mode dims non-current speeches; tap-to-insert shorthand; speech + prep timers; Notes section is titled just "Notes" (shipped to OpenRouter as `Notes:`, not "Crossfire notes").
- Confirmations are in-UI modals (`confirm.rs`, safe action focused, Esc/backdrop cancels), never native `window.confirm`.
- Ballot action buttons (Print flow + ballot, Copy decision summary, AI summary) share the lighter `btn-soft` shade.
- `style.css` is the single stylesheet (no CSS modules/inline style system); responsive + `@media print` must keep printing both sheets + ballot like the two paper pages.
- Copy tone: plain language for lay judges, explain jargon inline; guidance text goes in `GUIDE` data.
