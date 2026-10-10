# PixelPlayer PC — notes for Claude

Windows music player: Tauri 2 (Rust core + Svelte 5 UI in WebView2). Developed in a
headless Debian container; Windows builds come from GitHub Actions (`.github/workflows/ci.yml`).

## Commands

- `scripts/fetch-ffmpeg.sh linux64` once per checkout (FFmpeg 8.1 shared → `third_party/`;
  `.cargo/config.toml` points the build and tests at it).
- `cargo test --workspace` — also regenerates `ui/src/lib/api/bindings/` (ts-rs); commit them.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all`.
- `npm run check`, `npm test`, `npm run build` (UI), `npm run dev` (UI with mock backend).
- Windows-only crate: `cargo check --target x86_64-pc-windows-msvc -p pp-output-wasapi`.

## Rules

- **Bit-perfect policy** (`docs/exclusive-mode.md`) is a product promise: never add
  dithering, volume scaling, DSP or resampling to the exclusive path unless the badge says
  "Converted". 0 dB software gain must stay a no-op.
- **Clean room:** never copy code, icons or other assets from PixelPlayer for Android
  (proprietary since 2026-05-12). Matching the look is fine.
- **Browser support:** the UI only runs in WebView2 (evergreen Chromium) or a current
  Chromium for development. Chromium-supported features may be used without fallbacks.
- Secrets never go into git (updater key → GitHub Actions secrets; `.env` is ignored).
- Keep `ui/src/lib/api/types.ts` in sync with the Rust types (serde camelCase).
