# Task completion

No test suite exists (no `#[test]`/`tests/`); verification is build + manual smoke run.

Run before considering a coding task done:
1. `cargo fmt --check` (fix with `cargo fmt`)
2. `cargo clippy --target wasm32-unknown-unknown -- -D warnings` — note `unwrap_used`/`expect_used` are warn-level in Cargo.toml; treat new warnings as failures.
3. `trunk build --release` — full WASM build must succeed (plain `cargo check` on host target can pass while wasm-only code paths break).
4. Smoke test: `trunk serve` -> http://127.0.0.1:8080; exercise the changed path (flow editing, timers, guide steps, ballot, print preview if touched) and confirm localStorage auto-save survives reload.
5. If stored data shape changed: verify old saved round still loads, or bump `STORAGE_KEY` in `model.rs`.
6. If UI changed: check print layout (both sheets + ballot) and mobile-width responsive layout.
