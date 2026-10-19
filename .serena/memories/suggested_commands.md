# Suggested commands (Linux)

- Setup once: `rustup target add wasm32-unknown-unknown` ; `cargo install trunk` (or `cargo install --locked trunk`).
- Dev server: `trunk serve` -> http://127.0.0.1:8080 (hot reload).
- Release build: `trunk build --release` -> static files in `dist/` (gitignored, as is `/target`).
- Deploy: push to `main` triggers Pages workflow; manual: Actions -> "Deploy Flower to GitHub Pages" -> Run workflow. Requires repo Settings -> Pages -> Source: GitHub Actions.
- Checks: `cargo clippy --target wasm32-unknown-unknown`, `cargo fmt`, `cargo check --target wasm32-unknown-unknown` — see `mem:task_completion` for done criteria.
- Serena memory sanity check: `serena memories check` from project root.
