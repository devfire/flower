# Tech stack

- Rust edition 2024, single binary crate `flower` 0.1.0; `#![forbid(unsafe_code)]`.
- Leptos 0.7 CSR (`features = ["csr"]`) — client-side WASM only, no SSR/hydration/server functions.
- Build: Trunk (`index.html` with `data-trunk` css+rust links); target `wasm32-unknown-unknown`.
- Deps (Cargo.toml): `serde`/`serde_json` (localStorage snapshot), `wasm-bindgen` 0.2, `gloo-timers` 0.3 (speech/prep timers), `web-sys` 0.3 with only Window/Storage/Navigator/Clipboard features — add features there when using new web APIs.
- Release profile: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"` (size-optimized for Pages).
- Clippy lints in Cargo.toml: `unwrap_used`/`expect_used` = warn; `indexing_slicing` intentionally off (justification comment there).
- Hosting: GitHub Pages via Actions (`.github/workflows/gh-pages.yml`): stable toolchain + wasm32 target, `trunk build --release --public-url ./`, copies `dist/index.html` to `dist/404.html` (SPA fallback), relative public URL works for project/user/custom-domain pages.
- Fonts: Google Fonts Fredoka + Nunito loaded in `index.html` (only external network dependency).
- Serena: project `flower`, Rust LSP only, utf-8 (`.serena/project.yml`).
