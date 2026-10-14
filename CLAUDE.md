# PixelPlayer PC — notes for Claude

Windows music player: Tauri 2 (Rust core + Svelte 5 UI in WebView2). Developed in a
headless Debian container; Windows builds come from GitHub Actions (`.github/workflows/ci.yml`).

## Commands

- `scripts/fetch-ffmpeg.sh linux64` once per checkout (FFmpeg 8.1 shared → `third_party/`;
  `.cargo/config.toml` points the build and tests at it).
- Windows builds ship an audio-only FFmpeg (~6 MB instead of ~160 MB) cross-compiled by
  `scripts/build-ffmpeg-slim.sh` in CI's `ffmpeg` job (cached until the script changes). A
  format the player should read needs its decoder, demuxer and parser listed there.
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
- Screenshots of the UI (mock backend): Playwright's headless shell is installed in
  `~/.cache/ms-playwright` with its libraries unpacked from Debian packages into
  `~/.local/opt/chromium-libs` (no root). Run it with
  `LD_LIBRARY_PATH=~/.local/opt/chromium-libs/usr/lib/x86_64-linux-gnu:~/.local/opt/chromium-libs/lib/x86_64-linux-gnu`,
  `executablePath` pointing at `chrome-headless-shell` and `--no-sandbox`, against `npm run dev`.
  Look at layout changes this way before shipping them.
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

## User data and lyrics (M4)

- Playlists, favourites and play history are schema v2 of the library DB (`db/user.rs`),
  keyed by file path, never touched by scans; playlist entries outside the library come back
  as `TrackRow` with id 0 (the UI hides album/folder actions for them).
- A play counts after half the track or 4 min, and at least 30 s (`src-tauri/src/history.rs`).
- Lyrics order (`pp-lyrics`): `.lrc` next to the file → copy saved in app data → embedded tag
  → LRCLIB (cached in app cache; misses retried after 7 days). Saving writes the `.lrc` next
  to the file, or into app data if the folder is read-only.
- Chart marks use `--pp-chart-mark` (primary palette tone 40 light / 58 dark); keep
  `theme/chart.test.ts` passing when touching the theme.

## Sound (M5)

- DoP (`pp-core` dop.rs) is packed in the decode worker from raw DSD (`Decoder::next_dsd`);
  DoP sessions never take fades, gain or EQ, so DoP needs a device volume or line level.
- EQ runs on the render thread (`RenderFeed`, handed over with `eq_version`) and changes
  live; ReplayGain is applied in the worker per track; crossfade is a hold-back buffer in the
  worker (shared mode, F32 only). EQ presets live in the UI (`ui/src/lib/eq.ts`).
- Volume: shared mode always has the app's own gain (`shared_gain_db`); exclusive mode uses
  the device volume unless it has none or `software_volume` is set (`Control::plan`).
  `ui/src/lib/volume.ts` mirrors those rules for the player bar and Settings.
- The UI's output settings live in one store (`output.svelte.ts`) that saves changes one
  after another; send `$state.snapshot` copies, never the reactive objects themselves.

## Desktop and tags (M6)

- Artist pictures: `pp-deezer` (search by name, exact matches only) + `src-tauri/src/artist_images.rs`
  (background worker, ~4 lookups/s, misses retried after 30 days). Stored in the art store and
  `artist_images` (schema v3, keyed by `artists.norm`); `ARTIST_SELECT` prefers them to album
  covers. Turning the setting off deletes the rows.
- Tag editor: `pp-library` edit.rs writes only the file's primary tag through lofty (covers and
  other fields survive; tests compare every untouched item). `tags_write` reloads the engine
  if the playing file was edited (`Engine::reload`) and rescans.
- AI playlists: `pp-ai` (raw HTTP to the Messages API; the library goes in a cached system
  block, structured JSON output, `fallbacks: "default"`; returned ids are checked against
  the catalog sent). The key lives in Windows Credential Manager (`src-tauri/src/secret.rs`)
  and never reaches the UI.
- Never create a webview window in a synchronous command or an event handler: on Windows
  it deadlocks (Tauri documents it). `tray::open_mini` builds on its own thread.
- Tray and mini player: `src-tauri/src/tray.rs`. The mini player is the same UI at `#mini`
  (`ui/src/MiniWindow.svelte`); its window label `mini` is in the capability file.

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
