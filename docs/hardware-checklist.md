# Hardware checklist

Automated tests cover everything above the WASAPI calls (negotiation, packing, gain, gapless
transitions, fallbacks — see `crates/pp-engine/tests/end_to_end.rs`). These need a real PC
with a DAC; a DAC with a sample-rate display is the ground truth.

1. **Settings → Audio output** lists your devices and the exclusive-format table.
2. Play 16/44.1, 24/96 and 24/192 FLAC, plus ALAC, WavPack, APE and MP3 with exclusive mode
   on: the DAC shows the file's rate and the badge says *Bit-perfect*.
3. A queue of 44.1 → 96 → 44.1 kHz tracks: no pops at the switches; tracks of one album at
   the same rate play gapless.
4. While exclusive, other apps can't play through the DAC; after switching to shared they can.
5. Seek, pause and skip: no clicks.
6. Unplug the DAC mid-song: playback pauses, no crash, the queue is kept; plug it back and
   press play.
7. DAC with hardware volume: the slider changes the DAC's volume. Without: it starts at
   −30 dB, can't go above −10 dB until unlocked, and the badge shows the dB value.
8. Media keys and the Windows media flyout control playback and show the artwork. *(M3)*
9. Turn off "Allow applications to take exclusive control" in the Sound control panel: a
   clear message, and playback continues in shared mode.
10. One hour of playback: *Dropouts* stays at 0 in Audio output diagnostics.
11. DSD64/128: DSD indicator on a DoP-capable DAC; PCM conversion elsewhere. *(M5)*
