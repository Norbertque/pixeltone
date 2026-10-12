# Hardware checklist

Automated tests cover everything above the WASAPI calls (negotiation, packing, gain, gapless
transitions, fallbacks, fades, queue edits, device changes — see `crates/pp-engine/tests/`).
These need a real PC with a DAC; a DAC with a sample-rate display is the ground truth.

1. **Settings → Audio output** lists your devices and the exclusive-format table.
2. Play 16/44.1, 24/96 and 24/192 FLAC, plus ALAC, WavPack, APE and MP3 with exclusive mode
   on: the DAC shows the file's rate and the badge says *Bit-perfect*.
3. A queue of 44.1 → 96 → 44.1 kHz tracks: no pops at the switches; tracks of one album at
   the same rate play gapless.
4. While exclusive, other apps can't play through the DAC; after switching to shared they can.
5. Seek, pause and skip: no clicks (a short fade). With *Smooth pause and seek* off they cut
   hard, and the DAC still shows the file's rate throughout.
6. Unplug the DAC mid-song: playback pauses with a message, no crash, the queue is kept; plug
   it back and press play: it continues where it stopped.
7. DAC with hardware volume: the slider changes the DAC's volume. Without: it starts at
   −30 dB, can't go above −10 dB until unlocked, and the badge shows the dB value.
8. Media keys and the Windows media flyout (next to the volume indicator) control playback
   and show the title, artist and artwork; its seek bar moves and can seek.
   *Device: System default:* switch the default device in Windows mid-song: playback moves
   to it. Unplug headphones that are the default: playback pauses instead of moving to the
   speakers.
9. Turn off "Allow applications to take exclusive control" in the Sound control panel: a
   clear message, and playback continues in shared mode.
10. One hour of playback: *Dropouts* stays at 0 in Audio output diagnostics.
11. DSD64/128: DSD indicator on a DoP-capable DAC; PCM conversion elsewhere. *(M5)*
12. Queue: right-click a song → *Play next* / *Add to queue*; drag rows in the queue panel;
    quit mid-song and start the app again: same queue and track, paused at the same spot.
