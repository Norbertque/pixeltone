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
- `cargo test -p pp-library --release -- --ignored` — 10k-track scan/query timing.

## This container (no sudo, apt packages missing)

- Run `. ~/.local/opt/pp-toolchain/env.sh` before any cargo command: Zig as `cc`/`c++`/`ar`,
  libclang for bindgen, FFmpeg on `LD_LIBRARY_PATH`.
- WebKitGTK is missing, so the app crate (`pixelplayer-pc`) can't link here. Check it with
  `PKG_CONFIG=~/.local/opt/pp-toolchain/bin/pkg-config-stub cargo clippy -p pixelplayer-pc
  --all-targets -- -D warnings`; its tests run on CI. Test everything else with
  `cargo test --workspace --exclude pixelplayer-pc`.
- No browser, so no screenshots; UI changes are verified with check/test/build only.
- Windows type-check of everything, including the app crate's `cfg(windows)` code:
  `. ~/.local/opt/pp-toolchain/env.sh && . ~/.local/opt/pp-toolchain/win-check.sh &&
  cargo clippy --target x86_64-pc-windows-msvc -p pixelplayer-pc -p pp-media-controls
  -p pp-output-wasapi --all-targets -- -D warnings` (needs `scripts/fetch-ffmpeg.sh win64`;
  type-checks only, nothing links). CI doesn't run clippy on Windows, so run this after
  touching Windows-only code.

## Library

- `crates/pp-library` has no Tauri dependency; `src-tauri/src/library.rs` is thin glue.
- Album identity: (title, album artist), else (title, folder). Artist splitting: tag
  delimiters in `artists.rs` (changeable in Settings → Library; `AC/DC` is protected).
- Library covers are `cover://localhost/lib/<hash>-<300|1000>`; queue items carry the cover
  key (`QueueItem`) so Now Playing uses the same art, including folder images.
- Bump `PRAGMA user_version` in `db.rs` with a migration for any schema change.

## Playback (M3)

- Queue items have ids (`pp-engine` queue.rs) that stay stable across edits; the worker
  follows ids, not positions. Edits bump `queue_version`; the UI refetches pages on change.
- Transport fades live in `pp-core` render.rs (`RenderFeed`), shared by every backend; the
  engine waits for `StreamControls::faded_out` (set once the device has played the ramp).
- `src-tauri/src/queue.rs` saves the queue (`queue.json`, debounced, and on exit) and restores
  it paused at startup. `media.rs` drives the flyout from a thread of its own.
- Device notifications: `OutputBackend::watch_devices` → `Engine::device_changed` and the
  `output://devices` event.

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
