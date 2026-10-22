# Flower — core

- Purpose: kawaii pastel PF (Public Forum) flow sheet for lay judges; static Rust+WASM, no backend/DB. Based on NSDA Intro to Flowing. Deployed at flower.devfire.io (GitHub Pages).
- Audience invariant: beginner lay judges — guided, one speech at a time; never assume debate jargon knowledge in UI copy.
- Privacy invariant: round data stays in localStorage; nothing leaves device except the opt-in OpenRouter AI summary (user's own key, see `openrouter.rs`). App has no API key of its own.

## Source map (src/, all modules ≤500 LOC — AGENTS.md cap; `app.rs` is at the cap, split before growing it)
- `main.rs` — entry only: `#![forbid(unsafe_code)]`, `console_error_panic_hook::set_once()`, `mount_to_body(app::App)`; declares all modules.
- `model.rs` — pure serde data: `Side`, `Mark`, `CellData`/`RowData`/`RoundData`, `SHORTHAND`; custom `cells_serde`/`winner_serde` keep old saves loadable. Non-test code is small; the file is mostly its test module.
- `guide.rs` — speech catalogue: `SpeechCol`/`SPEECH_COLS` (8 columns), `GuideStep`/`GUIDE` (11 speeches/crossfires, Momo-chan tips), `SpeakingOrder` (`ProFirst` default / `ConFirst`), `sheet_for`/`edit_speech`.
- `analysis.rs` — `RoundData::dropped_arguments`/`marked_arguments` (`DroppedArgument`/`MarkedArgument`) feeding ballot hints.
- `round_save.rs` — storage keys + save repair: `STORAGE_KEY="flower-round-v1"`, `STORAGE_BACKUP_KEY`, `STORAGE_REVISION_KEY` (monotonic revision, issue #18), `stored_revision`/`next_revision`.
- `state.rs` — reactive mirror: `CellState`/`RowState`/`RoundState` (per-cell Leptos signals) + `RoundData` conversions. No browser/storage APIs here.
- `persistence.rs` — localStorage load/save: debounced autosave (`SAVE_DEBOUNCE_MS`), unreadable-save backup + save blocking, revision guard vs other tabs, progress save, flush hooks on hide/unload.
- `platform.rs` — browser helpers: `print_page`, `copy_text`, `now_ms`, timer audio feedback (`unlock_audio`, `timer_expired_feedback`).
- `progress.rs` — `RoundProgress`/`SavedProgress` (guide step, timers, prep) persisted separately under `PROGRESS_STORAGE_KEY="flower-progress-v1"`.
- `app.rs` — `App` component only: wires state/persistence/timers/guide, composes hero, setup, sheets, notes, ballot; owns confirm state. Orchestration, not leaf UI.
- `hero.rs` — `hero_header`; `setup_card.rs` — `setup_card` (topic/team/order/focus); `flow_sheet.rs` — `FlowSheet`/`FlowRow` + `placeholder_for`; `ballot.rs` — `ballot_card`, `DroppedHints`, `decision_text`, AI summary button/output; `confirm.rs` — in-UI `confirm_dialog`/`PendingConfirm` (never `window.confirm`).
- `timer.rs` — `TimerBank`, `TimerKind`, `PREP_SECS`, `fmt_time`/`ceil_secs`/`tick_secs`. `shorthand.rs` — tap-to-insert shorthand (`InsertCtx`, `TextTarget`, UTF-16↔byte caret mapping). `topic.rs` — NSDA topic: bundled `topics.json` fallback + same-origin fetch, `decide` never overwrites a saved/typed topic (only offers newer wording). `openrouter.rs` — OAuth PKCE connect/callback, sessionStorage-only key (`ApiKey`), `summarize` on `openrouter/free`; prompts externalized to `prompts/summary_*.md` via `include_str!` (`{round}` placeholder).
- `style.css` — single stylesheet, responsive + print. `index.html` — Trunk entry, also copies `topics.json`. `scripts/fetch_nsda_topic.py` — build-time NSDA topic extraction (+ Python tests). Workflows: `.github/workflows/ci.yml`, `gh-pages.yml`.

## Invariants
- Two sheets: Pro (blue) / Con (green/mint); one column per speech; answers written right of argument answered; blank box = dropped argument (ballot `DroppedHints` surfaces these, derived via memoized dropped/marked analysis — do not clone the round per keystroke).
- `RowData.cells` invariant: exactly `SPEECH_COLS.len()` cells; indices are clamped/enumerated — clippy `indexing_slicing` intentionally off per Cargo.toml.
- Persistence: auto-save whole `RoundData` as JSON under `STORAGE_KEY`; progress under its own key. Changing stored shape risks breaking saved rounds — keep serde backward-compatible (see `winner_serde`/`speaking_order` default) or bump key version. Unreadable saves are backed up, not overwritten.
- Guide/timer/sheet views all derive from the round's `speaking_order` (issue #21); persisted fields must be added to `RoundData` (model), `RoundState` (state), and conversion code.
- Data flow: `model::RoundData` (serializable) <-> `state::RoundState` (signals) -> components. Keep model free of Leptos/browser types; browser APIs only in `platform.rs`/`persistence.rs`/`openrouter.rs`/`topic.rs`.

## Further memories
- Stack, versions, build/deploy details: `mem:tech_stack`
- Dev/build/deploy commands: `mem:suggested_commands`
- Code style, naming, UI/design patterns: `mem:conventions`
- Done criteria (checks to run before finishing a task): `mem:task_completion`
