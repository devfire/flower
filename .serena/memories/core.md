# Flower — core

- Purpose: kawaii pastel PF (Public Forum) flow sheet for lay judges; static Rust+WASM, no backend/DB/API keys. Based on NSDA Intro to Flowing.
- Audience invariant: beginner lay judges — guided, one speech at a time; never assume debate jargon knowledge in UI copy.

## Source map
- `src/main.rs` — entry only: `#![forbid(unsafe_code)]`, `mount_to_body(app::App)`; modules `app`, `model`, `state`.
- `src/model.rs` — pure data/constants: `Side`, `Mark`, `SpeechCol`/`SPEECH_COLS` (8 speech columns), `GuideStep`/`GUIDE` (11 speeches/crossfires, Momo-chan tips), `SHORTHAND`, `CellData`/`RowData`/`RoundData` (serde), `STORAGE_KEY = "flower-round-v1"`.
- `src/state.rs` — reactive mirror of model: `CellState`/`RowState`/`RoundState` (per-cell Leptos signals), `load_initial`/`save` (localStorage snapshot), browser helpers `confirm`/`print_page`/`copy_text`.
- `src/app.rs` — all UI in one file: `App` (hero, guide+timers, sheets, ballot), `FlowSheet`, `FlowRow`, `DroppedHints`, `decision_text`, `insert_shorthand`, `fmt_time`, `placeholder_for`.
- `style.css` — kawaii pastel theme, responsive + print (prints both sheets + ballot). `index.html` — Trunk entry. `.github/workflows/gh-pages.yml` — Pages deploy.

## Invariants
- Two sheets: Pro (pink) / Con (green); one column per speech; answers written right of argument answered; blank box = dropped argument (ballot `DroppedHints` surfaces these).
- `RowData.cells` invariant: exactly `SPEECH_COLS.len()` cells (see model.rs comment); indices are clamped/enumerated — clippy `indexing_slicing` intentionally off per Cargo.toml.
- Persistence: auto-save whole `RoundData` as JSON to localStorage under `STORAGE_KEY`; nothing leaves device. Changing stored shape risks breaking saved rounds — bump key version.
- Data flow: `model::RoundData` (serializable) <-> `state::RoundState` (signals) -> `app` components. Keep model free of Leptos/browser types; keep browser APIs in `state.rs`/`app.rs`.

## Further memories
- Stack, versions, build/deploy details: `mem:tech_stack`
- Dev/build/deploy commands: `mem:suggested_commands`
- Code style, naming, UI/design patterns: `mem:conventions`
- Done criteria (checks to run before finishing a task): `mem:task_completion`
