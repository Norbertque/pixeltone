# Pixeltone

[![Buy me a coffee](https://img.shields.io/badge/Buy%20me%20a%20coffee-norbertque-FFDD00?logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/norbertque)

A Material 3 Expressive music player for Windows, inspired by PixelPlayer for Android, with
**bit-perfect WASAPI exclusive output**: your files reach the DAC untouched, at their own
sample rate and bit depth.

- **Audio engine (Rust):** FFmpeg decoding (FLAC, ALAC, WAV, AIFF, MP3, AAC, Vorbis, Opus,
  APE, WavPack, DSD, …), an exclusive-mode format negotiator, gapless playback, native DSD
  over DoP, and WASAPI shared/exclusive output with hardware or safe software volume.
- **Sound (opt-in):** a 10-band equaliser with presets, ReplayGain (track or album) and
  crossfading in shared mode; the output badge says when they change the samples.
- **Library (Rust + SQLite):** scans your music folders incrementally and keeps watching
  them; albums, artists (with configurable "feat." splitting), genres and folders; instant
  search that ignores accents ("Bjork" finds "Björk"); cover art from the files or from
  `cover.jpg`/`folder.jpg` next to them.
- **Playback:** a queue with *Play next*, *Add to queue*, drag-to-reorder and a shuffle you
  can undo; queue and position survive a restart; media keys and the Windows media flyout;
  click-free fades on pause and seek; follows device changes (see the keyboard shortcuts
  with `?`).
- **Lyrics:** synced lyrics that follow the song (click a line to jump there), from an
  `.lrc` file next to the song, the file's tags or LRCLIB; edit and sync them yourself.
- **Playlists & favourites:** playlists with drag-to-reorder and M3U/M3U8 import/export, a
  heart for favourites, play history with listening stats and a daily mix on Home.
- **Tags & pictures:** edit tags of a song, an album or a whole artist (written into the
  files); artist pictures from Deezer.
- **Desktop:** a mini player that stays on top, a tray icon with playback controls, and
  optionally keep playing in the tray when the window is closed.
- **UI (Svelte 5 in WebView2 via Tauri 2):** Material You colours from the album art, M3
  Expressive shapes and spring motion, Google Sans Flex.

See [`docs/exclusive-mode.md`](docs/exclusive-mode.md) for exactly what "bit-perfect" means
here, and [`docs/hardware-checklist.md`](docs/hardware-checklist.md) for testing with a DAC.

## Online features

Everything works offline. Two lookups use the internet, and both can be turned off in
**Settings → Online**; AI playlists are a third, opt-in feature:

- **Lyrics (LRCLIB):** for songs without lyrics of their own, the title, artist, album and
  length are sent to [lrclib.net](https://lrclib.net). Answers (and "not found") are cached.
- **Artist pictures (Deezer):** artists in your library are looked up on
  [Deezer](https://www.deezer.com) by name together with a few of their album and song
  titles, so a namesake isn't taken for them; pictures are kept in the app's cache.
  Turning it off removes them.
- **AI playlists (optional, your own Anthropic API key):** describe a playlist and Claude
  picks it from your library. The titles, artists, albums, years and genres of your songs
  are sent to Anthropic with each request, billed to your key; the key is kept in Windows
  Credential Manager. Nothing is sent until you add a key in **Settings → AI playlists**.

## Getting builds

Every push to `main` builds a Windows installer and a portable zip on GitHub Actions and
publishes them as the rolling **Development build** release; tags `v*` publish versioned
releases. Builds are unsigned, so Windows SmartScreen asks first: *More info → Run anyway*.
Once installed, Pixeltone looks for newer builds at start-up and offers to install them
(Settings → Online can turn that off).

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

Windows-only code (`crates/pp-output-wasapi`, `crates/pp-media-controls`, title bar, power)
compiles on CI; check it locally with `cargo check --target x86_64-pc-windows-msvc -p
pp-output-wasapi -p pp-media-controls`. Checking the whole app for Windows also needs
FFmpeg's Windows headers, a C compiler for the target and a resource-compiler stand-in; see
`CLAUDE.md`.

| Path | What |
|---|---|
| `crates/pp-core` | Formats, negotiator, PCM packing, gain, badge, output traits |
| `crates/pp-decode` | FFmpeg decoder (via `rsmpeg`), tags, cover art |
| `crates/pp-engine` | Queue, gapless pipeline, transport fades, engine thread, capture backend for tests |
| `crates/pp-library` | Library: scanner, tags, SQLite + FTS5 search, thumbnails, watcher; playlists, favourites, play history |
| `crates/pp-lyrics` | Lyrics: LRC parsing, `.lrc` files, embedded tags, LRCLIB with a disk cache |
| `crates/pp-media-controls` | Media keys and the Windows media flyout (Windows) |
| `crates/pp-output-wasapi` | WASAPI backend and device notifications (Windows) |
| `src-tauri` | The app: commands, events, cover protocol, settings, library glue |
| `ui` | Svelte 5 UI |

## Support

Pixeltone is free. If it's become your player, you can
[buy me a coffee](https://buymeacoffee.com/norbertque).

## Licence and credits

© 2026 Norbertque. All rights reserved: the code is public to read, not licensed for
reuse. Pixeltone (called PixelPlayer PC until October 2026) is an independent project; it
shares no code or assets with PixelPlayer for Android. It bundles FFmpeg (LGPL-2.1-or-later,
dynamically linked, built audio-only by `scripts/build-ffmpeg-slim.sh`), Google Sans Flex
(SIL OFL 1.1) and Material Symbols (Apache-2.0).
