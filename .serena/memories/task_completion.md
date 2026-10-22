# Task completion

Tests exist: in-module Rust unit tests + `scripts/test_fetch_nsda_topic.py`. Verification is tests + build + manual smoke run (CI in `.github/workflows/ci.yml` is the reference pipeline).

Run before considering a coding task done:
1. `cargo fmt --check` (fix with `cargo fmt`)
2. AGENTS.md gate: `cargo clippy --workspace --all-targets -- -Dclippy::pedantic` (must be clean; `pedantic`/`unwrap_used`/`expect_used` are warn-level in Cargo.toml, treat new warnings as failures). CI also runs `cargo clippy --locked --target wasm32-unknown-unknown --all-targets -- -D warnings -D clippy::pedantic`.
3. `cargo test --locked` — all Rust unit tests pass; add/update tests for pure-logic changes (model/guide/analysis/topic/timer/progress/shorthand precedents). If the topic script changed: `python3 -m unittest discover -s scripts -v`.
4. `trunk build --release` — full WASM build must succeed (plain `cargo check` on host target can pass while wasm-only code paths break).
5. Smoke test: `trunk serve` -> http://127.0.0.1:8080; exercise the changed path (flow editing, timers, guide steps, ballot, print preview if touched) and confirm localStorage auto-save survives reload.
6. If stored data shape changed: verify an old saved round still loads (serde defaults/custom deserializers), or bump the storage key in `round_save.rs`/`progress.rs`.
7. If UI changed: check print layout (both sheets + ballot) and mobile-width responsive layout.
8. Keep every module ≤500 LOC (AGENTS.md); commit when done, do not push unless asked.
