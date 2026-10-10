# Exclusive mode (bit-perfect WASAPI output)

Opt-in per device in **Settings → Audio output**. With it off, audio goes through the Windows
mixer (shared mode), resampled by the app to the mixer's rate. The policy matches the
Android USB exclusive mode of the PixelPlayer fork, re-implemented in Rust.

## Pipeline

```
FFmpeg decoder ──► PcmBlock (int left-justified in i32, or f32)
   │  (downmix, resample: only on converted paths)
   ▼
Packer ──► device bytes in the negotiated container ──► ring buffer (≈1.5 s)
                                                            │
                                    render thread (MMCSS "Pro Audio") ──► WASAPI
                                    (software gain applied here, if any)
```

Track changes are frame-accurate markers in the byte stream: position and "now playing"
follow what the device has actually played (silence the render thread inserts is excluded).

## Format negotiation

The driver is probed with `IsFormatSupported(EXCLUSIVE)` for 16-bit, packed 24-bit,
24-in-32, 32-bit and 32-bit float containers at 44.1–768 kHz. For each track the
negotiator (`crates/pp-core/src/negotiate.rs`) prefers, in order:

1. no downmix, 2. no resampling, 3. no depth reduction, 4. a container of the same kind
(int/float), 5. the closest channel count, 6. the tightest adequate depth (deepest when the
software gain stage is active), 7. the closest rate — same 44.1/48 kHz family first,
upsampling before downsampling.

## What "bit-perfect" means

The badge says **Bit-perfect** only when, in exclusive mode, the samples are unchanged
except for lossless steps:

- integer samples widened into a bigger container, left-justified (16 → 24/32-bit);
- float decoder output converted to integer, which is exact for material of up to 24 bits;
- mono duplicated onto both front channels;

and no software gain is active. Anything else (resampling, depth reduction, downmix, DSD
decoded to PCM, attenuation) shows **Converted** or the attenuation in dB.

Never applied in exclusive mode: dithering, EQ, ReplayGain, crossfade, speed changes.

## Volume

- **Hardware volume** (endpoint reports `ENDPOINT_HARDWARE_SUPPORT_VOLUME`): the slider moves
  the device's own volume; samples stay untouched.
- **No hardware volume:** a software gain stage (Q16 multiply in the 32-bit domain, on the
  render thread so it reacts instantly). First use starts at **−30 dB**; levels above
  **−10 dB** stay locked until the loudness warning is acknowledged; the level is remembered
  per device. **Fixed line-level output** bypasses it for DACs feeding an amplifier.

## Track transitions

- Same device format: one continuous stream — gapless.
- Different format (e.g. 44.1 → 96 kHz): the stream drains, the device is reopened in the new
  format and 150 ms of silence lets the DAC's clock lock before the music starts.

## Errors

- Device in use by another app, exclusive mode disabled in the Sound control panel
  ("Allow applications to take exclusive control of this device"), or format rejected:
  the app falls back to shared mode and says why.
- Device unplugged: playback pauses and the queue is kept; pressing play reopens the device.

## Timing

Polling is the default for exclusive mode: the standard Windows USB audio driver can stutter
in event-driven exclusive mode. Event-driven timing is available in settings.
