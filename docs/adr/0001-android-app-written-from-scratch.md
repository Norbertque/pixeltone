# Pixeltone for Android is written from scratch, not grown from our PixelPlayer fork

Jams need an app that friends can install. The Android player we already use, our fork of
PixelPlayer with bit-perfect USB output and Pixeltone remote control, can't be shared:
PixelPlayer moved from MIT to a proprietary licence on 2026-05-12 that forbids redistributing
derivative works, and the fork is built on the proprietary version. So Pixeltone for Android
is a new app written from scratch, the same clean-room way as Pixeltone for Windows: nothing
from PixelPlayer, neither code nor UI nor assets. Only code we wrote ourselves that doesn't
build on PixelPlayer may move over (the USB output engine and the Pixeltone remote client),
each file checked first; screens and player integration are written anew.

## Considered options

- **Start from PixelPlayer's last MIT release** (commit `39030156`, 2026-05-12). Allowed, but
  PixelPlayer's code and design would stay at the heart of the app; we want an app that is
  wholly ours, like the Windows one.
- **Ask PixelPlayer's author for permission** to distribute the fork. Possible at any time,
  but nothing to build a plan on.
- **Keep the fork private and give guests a small jam-only app.** Guests would get a remote
  rather than a player, and two Android apps would need jam support.
- **Tauri 2, sharing the Rust core with Windows.** Background playback, the media
  notification, USB access and audio output need native Android code anyway, and a WebView UI
  is less smooth. We use Kotlin, Jetpack Compose and Media3; Windows and Android share the
  remote protocol (`docs/remote-protocol.md`), not code.

## Consequences

- The fork stays for personal use and gets no new features.
- The app lives in `android/` of this repository (application id
  `io.github.norbertque.pixeltone`, Android 8.0+) and ships as an APK on GitHub with in-app
  updates. F-Droid isn't an option without an open-source licence.
