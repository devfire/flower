# Tech stack

- Rust edition 2024, single binary crate `flower` 0.1.0; `#![forbid(unsafe_code)]`; toolchain pinned to 1.97.1 in `rust-toolchain.toml` (rustfmt+clippy, wasm32 target) — bump deliberately with the `dtolnay/rust-toolchain` tag in both workflows.
- Leptos 0.8 CSR (`features = ["csr"]`) — client-side WASM only, no SSR/hydration/server functions.
- Build: Trunk 0.21.14 in CI (`index.html` with `data-trunk` css+rust+copy-file `topics.json` links); target `wasm32-unknown-unknown`.
- Deps (Cargo.toml): `serde`/`serde_json` (localStorage snapshot), `wasm-bindgen` 0.2, `wasm-bindgen-futures`, `js-sys`, `gloo-timers` 0.3 (timers), `console_error_panic_hook` (readable panics under `panic="abort"`), `web-sys` 0.3 with many features (Storage, Clipboard, Crypto/SubtleCrypto for PKCE, AudioContext for timer chime, fetch types, etc.) — add features there when using new web APIs.
- Release profile: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"` (size-optimized for Pages).
- Clippy lints in Cargo.toml `[lints.clippy]`: `pedantic` warn (priority -1), `unwrap_used`/`expect_used` warn; `indexing_slicing` intentionally off (justification comment there). AGENTS.md gate: `cargo clippy --workspace --all-targets -- -Dclippy::pedantic`; CI additionally runs the wasm target with `-D warnings`.
- Tests exist: Rust unit tests in-module (`model`, `state`, `topic`, `timer`, `progress`, `shorthand`, `openrouter`) run via `cargo test --locked`; Python tests for the topic script via `python3 -m unittest discover -s scripts`.
- Hosting: GitHub Pages via Actions. `gh-pages.yml`: reusable `ci.yml` quality gate first, build-time NSDA topic fetch into `topics.json` (weekly cron Mondays 06:17 UTC, `--strict` on scheduled runs), `trunk build --release --public-url ./`, `404.html` SPA fallback rewritten to root-absolute URLs, `CNAME` = flower.devfire.io. `ci.yml` runs on PRs/pushes to main (fmt, wasm clippy pedantic, cargo test, Python tests, release trunk build, all `--locked` where cargo runs).
- External network: Google Fonts (Fredoka + Nunito in `index.html`) and, only on user action, OpenRouter OAuth/chat endpoints + same-origin `topics.json`. No other dependencies at runtime.
- Serena: project `flower`, Rust LSP only, utf-8 (`.serena/project.yml`).
