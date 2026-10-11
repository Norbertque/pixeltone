# PixelPlayer PC

A Material 3 Expressive music player for Windows, inspired by PixelPlayer for Android, with
**bit-perfect WASAPI exclusive output**: your files reach the DAC untouched, at their own
sample rate and bit depth.

- **Audio engine (Rust):** FFmpeg decoding (FLAC, ALAC, WAV, AIFF, MP3, AAC, Vorbis, Opus,
  APE, WavPack, DSD → PCM, …), an exclusive-mode format negotiator, gapless playback, and
  WASAPI shared/exclusive output with hardware or safe software volume.
- **Library (Rust + SQLite):** scans your music folders incrementally and keeps watching
  them; albums, artists (with configurable "feat." splitting), genres and folders; instant
  search that ignores accents ("Bjork" finds "Björk"); cover art from the files or from
  `cover.jpg`/`folder.jpg` next to them.
- **UI (Svelte 5 in WebView2 via Tauri 2):** Material You colours from the album art, M3
  Expressive shapes and spring motion, Google Sans Flex.

See [`docs/exclusive-mode.md`](docs/exclusive-mode.md) for exactly what "bit-perfect" means
here, and [`docs/hardware-checklist.md`](docs/hardware-checklist.md) for testing with a DAC.

## Getting builds

Every push to `main` builds a Windows installer and a portable zip on GitHub Actions and
publishes them as the rolling **Development build** release; tags `v*` publish versioned
releases. Builds are unsigned, so Windows SmartScreen asks first: *More info → Run anyway*.

## Development

The project is developed on Linux (no Windows needed for most work):

```bash
scripts/fetch-ffmpeg.sh linux64   # FFmpeg 8.1 (LGPL, shared) into third_party/
npm ci                            # UI + Tauri CLI
cargo test --workspace            # Rust tests, including end-to-end bit-perfect checks
npm run check && npm test         # UI type-check and unit tests
npm run dev                       # UI in a browser with a mock backend (http://localhost:5173)
npm -w ui run dev:remote          # same, reachable from other machines on the network
cargo test -p pp-library --release -- --ignored   # 10k-track library timing test
```

Building the Rust code on Linux needs a C toolchain and libclang (`build-essential`,
`clang`, `pkg-config`); building the Tauri app crate itself also needs the WebKitGTK dev
packages listed in `.github/workflows/ci.yml`.

Windows-only code (`crates/pp-output-wasapi`, title bar, power) compiles on CI; check it
locally with `cargo check --target x86_64-pc-windows-msvc -p pp-output-wasapi`.

| Path | What |
|---|---|
| `crates/pp-core` | Formats, negotiator, PCM packing, gain, badge, output traits |
| `crates/pp-decode` | FFmpeg decoder (via `rsmpeg`), tags, cover art |
| `crates/pp-engine` | Queue, gapless pipeline, engine thread, capture backend for tests |
| `crates/pp-library` | Library: scanner, tag reading, SQLite + FTS5 search, thumbnails, folder watcher |
| `crates/pp-output-wasapi` | WASAPI backend (Windows) |
| `src-tauri` | The app: commands, events, cover protocol, settings, library glue |
| `ui` | Svelte 5 UI |

## Licence and credits

PixelPlayer PC is an independent project; it shares no code or assets with PixelPlayer for
Android. It bundles FFmpeg (LGPL-2.1-or-later, dynamically linked), Google Sans Flex
(SIL OFL 1.1) and Material Symbols (Apache-2.0).
