# Suggested commands (Linux)

- Setup once: `rustup target add wasm32-unknown-unknown` (or rely on `rust-toolchain.toml`, pinned 1.97.1); `cargo install trunk` (CI pins trunk@0.21.14).
- Dev server: `trunk serve` -> http://127.0.0.1:8080 (hot reload).
- Release build: `trunk build --release` -> static files in `dist/` (gitignored, as is `/target`).
- Tests: `cargo test --locked` (Rust unit tests); `python3 -m unittest discover -s scripts -v` (NSDA topic extraction).
- Gates (see `mem:task_completion`): `cargo fmt --check`; AGENTS.md gate `cargo clippy --workspace --all-targets -- -Dclippy::pedantic`; CI wasm gate `cargo clippy --locked --target wasm32-unknown-unknown --all-targets -- -D warnings -D clippy::pedantic`.
- Deploy: push to `main` triggers Pages workflow (quality gate first); weekly cron rebuild refreshes the NSDA topic; manual: Actions -> "Deploy Flower to GitHub Pages" -> Run workflow. Requires repo Settings -> Pages -> Source: GitHub Actions. Production: https://flower.devfire.io
- Topic refresh locally: `python3 scripts/fetch_nsda_topic.py --output topics.json` (keeps committed file on failure; `--strict` to fail hard).
- Serena memory sanity check: `serena memories check` from project root. Serena memories live in `.serena/memories/` and are committed with the repo.
- Per AGENTS.md: commit work when done (do not push unless asked).
