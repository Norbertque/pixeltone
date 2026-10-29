# USB output on Android (bit-perfect to a USB DAC)

Opt-in: with **Settings → USB DAC → Exclusive USB output** on and a USB DAC attached, Pixeltone
for Android claims the DAC's USB Audio Class interface itself (libusb on the file descriptor
of a `UsbDeviceConnection`, no root) and streams decoded PCM at the track's own sample rate and
bit depth, past AudioTrack/AudioFlinger and their resampling and mixing. Off, playback goes
through Android's output exactly as before.

The engine came over from our own work in the private PixelPlayer fork (ADR 0001); the player
integration, settings and screens were written for this app.

## Architecture

```
MediaCodec decoders (ExoPlayer renderers)
        │ PCM 16/24/32/float — the sink takes float directly, so 24-bit sources stay whole
        ▼
UsbAudioSink (Media3 AudioSink, app/usb)         ← PlaybackService.buildPlayer() builds the
        │ FormatNegotiator picks alt setting + rate   phone's player around it while a DAC is
        │ PcmRepacker left-justifies into the DAC's   ready, and around Android's own sink
        ▼ subslot (lossless on the bit-perfect path)  otherwise
UsbAudioSession (:usbaudio Kotlin)
        │ JNI (direct ByteBuffers, non-blocking ring backpressure)
        ▼
Native driver (:usbaudio C++ / libusb 1.0.30, shared .so, LGPL-2.1)
  UacDevice   — wrap fd, detach kernel driver, claim AC+AS, alt settings,
                UAC1 endpoint / UAC2 clock-source sample rate, feature-unit volume
  IsoStream   — isochronous OUT pipeline (8 transfers × 8 packets), Q16.16
                fractional packet sizing, explicit-feedback correction on async
                endpoints, silence-fill on underrun, alt 0 on stop
```

Control plane (all unit-tested, no hardware needed): `UsbDescriptorParser` builds a
`UacTopology` from `getRawDescriptors()`; `UacCapabilityProber` resolves UAC2 rates through a
clock-source RANGE request on EP0; `FormatNegotiator` picks the alt setting and rate. The
`UsbExclusiveModeController` state machine (`Disabled → NoDevice → PermissionPending → Ready →
Active`, plus `PermissionDenied`/`Error`) owns the session; `PlaybackService` follows it and
rebuilds the phone's player, carrying the queue, position, repeat and shuffle over.

## Bit-perfect policy (what we do and don't touch)

The same promise as on Windows (`docs/exclusive-mode.md`):

- **Never** on any path: dithering, volume scaling (`UsbAudioSink.setVolume` is a no-op),
  platform audio effects (there is no AudioTrack session to apply them to).
- **Bit-perfect** in the badge: the only transformation is left-justified subslot packing —
  16→24/32-bit zero-padding, and float32→int (exact for anything that was an integer of up to
  24 bits, which is everything a decoder makes of FLAC/ALAC ≤ 24-bit). Mono duplicated into a
  stereo DAC also counts (samples unchanged).
- **Converted** in the badge: the DAC lacks the source rate (resampled with Sonic, in 16-bit),
  has less depth (truncated, no dither) or fewer channels (5.1/7.1 folded to stereo,
  `SurroundToStereo.kt`).
- Playback speed and pitch stay at 1×. Future volume-scaling features (ReplayGain, crossfade,
  EQ) must stay off while a DAC is in use.

The badge in Now Playing reads like the Windows one: `USB • 24-bit/96 kHz • Bit-perfect`, the
format being what goes to the DAC.

## Audio focus, calls and system sounds

ExoPlayer's audio focus handling stays: calls and alarms pause playback, which resumes after a
transient loss. There is no ducking (that would be volume scaling; the sink ignores it). While
the DAC is claimed, other apps and system sounds can't open it: ringtones and notifications
play on the phone's own output.

## Volume

A DAC with a UAC feature unit (master volume) gets a **DAC volume** slider in Settings: SET CUR
on the feature unit, in the DAC's own 1/256 dB steps; the samples are untouched.

A DAC **without** one (dongles straight into sensitive earphones) gets a **software volume**
instead of raw full scale:

- Attenuation is one Q16 fixed-point multiply in `PcmRepacker`, in the 32-bit domain before
  subslot packing, and the negotiator then prefers the deepest subslot so attenuated 16-bit
  material keeps its low bits. **At 0 dB the multiply is skipped**: only then does the badge
  say bit-perfect; otherwise it shows the level (`USB • 16-bit/44.1 kHz • −30 dB`).
- A new DAC starts at a safe **−30 dB**, and the slider stops at **−10 dB** until the user
  allows louder. The level is remembered per DAC.
- **Line level** turns the software volume off: full scale, for an amplifier after the DAC.

The phone's **volume keys** turn the DAC's volume while it is in use: the session's player is
wrapped in `UsbDeviceVolumePlayer`, a `ForwardingPlayer` with remote device volume (30 steps),
on the feature unit when there is one, else on the software range (−60…0 dB, the floor mutes).

## Lifecycle and edge cases

- **Unplugged while playing**: the driver reports the dead stream and the DETACHED broadcast
  arrives; playback pauses, the session closes once, and the player is rebuilt onto Android's
  output with the queue and position. No crash, nothing lost.
- **Plugged back in**: granted DACs are remembered (`vendorId:productId:serial`, in the app's
  preferences, `UsbSettings`); permission is asked again if Android forgot it, and playback goes
  on if it stopped because of the unplugging (per-DAC `autoResume`, on by default; no switch for
  it in Settings yet).
- **Sample-rate changes between tracks** (44.1 → 96 kHz): the sink reprograms the alt setting
  and clock in `configure()`; the stream starts in silence so the DAC's PLL locks first.
- **Screen off**: the playback service's foreground notification and `WAKE_MODE_LOCAL` keep the
  stream alive.
- **Media session**: the session's player is swapped with the rebuilt one, so notification,
  lock screen and headset buttons follow.

## Known limitations

- Async endpoints with *implicit* feedback (no feedback endpoint) are paced at the nominal
  rate; such DACs may drift over very long playback.
- No DSD (DoP) yet; `UacCapabilities.dsdSupport` is the hook.
- Only the first attached DAC is used.

## Trying it on hardware

Everything above the JNI line is covered by unit tests (`./gradlew :usbaudio:testDebugUnitTest
:app:testDebugUnitTest`: descriptor blobs, negotiation table, repack golden vectors, controller
transitions, sink position/drain/backpressure). With a real DAC (ideally a UAC2 async one with a
rate display, and a UAC1 dongle):

1. Turn the setting on, attach the DAC → Android's permission dialog → allow → Settings shows
   the DAC ready with its rates.
2. Play FLAC 16/44.1, 24/96, 24/192, WAV 24-bit, MP3, Opus: Now Playing shows `USB • …` and the
   DAC's own display shows the *source* rate.
3. While playing, other apps can't use the DAC; system sounds stay on the phone.
4. A queue going 44.1 → 96 → 44.1 kHz: no pops at the changes.
5. Seek, notification controls and headset buttons work; the seek bar keeps up.
6. Unplug mid-track: playback pauses, no crash, Android's output works.
7. Plug back in: permission (or remembered) → playback goes on.
8. A call and an alarm: playback pauses and resumes; nothing mixes into the DAC.
9. An hour with the screen off: the stream survives doze.
10. A DAC with volume: the slider changes loudness. Without: software volume at −30 dB, the
    card holds levels above −10 dB, the badge shows the attenuation.
11. Volume keys change the DAC's loudness (both kinds); with Line level on they do nothing and
    output is full scale.

## Licensing

libusb is vendored unmodified (`android/usbaudio/src/main/cpp/external/libusb/VENDORED.md`) and
built as its own shared library, `libusb-1.0.so`, as LGPL-2.1 asks; its licence text is next to
it (`COPYING`), and Settings names it.
