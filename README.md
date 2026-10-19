# 🌸 Flower — easy PF flowing for lay judges

A kawaii pastel, Rust + WebAssembly flow sheet for **Public Forum** debate, built for
lay judges who want to flow but don't really know how yet. Based on the NSDA
[*Intro to Flowing* judge training](https://www.speechanddebate.org/wp-content/uploads/Big-Questions-Judge-Training-Intro-to-Flowing.pdf):

- 📄 **Two sheets** — 🌸 Pro arguments on the pink sheet, 🍃 Con arguments on the green sheet
- ➜ **One column per speech**, answers written directly right of the argument they answer
- 💤 **Blank box = dropped argument** (the ballot section spots these for you)
- ⭐ **Extended / key-argument tags**, ⚖️ weighing prompts, speech + prep timers
- 🐰 **Momo-chan suggestion bubbles** walk you through all 11 speeches/crossfires, one at a time
- 🎯 **Focus mode** dims speeches that aren't happening right now
- ✂️ Tap-to-insert **shorthand cheat sheet**, crossfire notes, guided decision ballot
- 💾 Auto-saves in your browser (localStorage) — nothing leaves your device
- 🖨️ Print / save-PDF prints both sheets + the ballot, like the two paper pages

## Run it locally

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk          # or: cargo install --locked trunk
trunk serve                  # → http://127.0.0.1:8080
```

Release build: `trunk build --release` → static files in `dist/`.

## Host it on GitHub Pages (free)

1. Push this repo to GitHub (branch `main`).
2. In the repo: **Settings → Pages → Build and deployment → Source: GitHub Actions**.
3. Push to `main` (or run **Actions → “Deploy Flower to GitHub Pages” → Run workflow**).
   The workflow in [`.github/workflows/gh-pages.yml`](.github/workflows/gh-pages.yml)
   installs Rust + Trunk, runs `trunk build --release --public-url ./`, and deploys `dist/`.
4. Your site appears at `https://<user>.github.io/<repo>/`. The relative `--public-url ./`
   also works for user pages and custom domains — no path edits needed.

No server, database, or API keys: it's a fully static WASM site.

## Project layout

| File | What |
| --- | --- |
| `src/main.rs` | Entry point (`mount_to_body`) |
| `src/model.rs` | Speeches, the 11-step guide + tips, marks, shorthand, stored data |
| `src/state.rs` | Per-cell reactive signals, localStorage snapshot save/load |
| `src/app.rs` | All UI: hero, guide + timers, flow sheets, ballot |
| `style.css` | Kawaii pastel theme, responsive + print styles |
| `index.html` | Trunk entry page |
