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
11. DSD64/128 on a DoP-capable DAC with exclusive mode: the badge says *DSD64 over DoP •
    Bit-perfect* and the DAC shows DSD; pause/resume keeps it in DSD mode without pops. With
    *DSD over DoP* off, or on a DAC without DoP: *DSD→… • Converted* and the DSD plays as PCM.
12. Queue: right-click a song → *Play next* / *Add to queue*; drag rows in the queue panel;
    quit mid-song and start the app again: same queue and track, paused at the same spot.
13. Lyrics: a song with synced lyrics (LRCLIB, or an `.lrc` next to it) highlights the line
    being sung; *Edit* → *Stamp* through a few lines → *Save* writes an `.lrc` next to it.
14. Playlists: import an M3U8 exported by another player (relative paths too), reorder by
    dragging, export it again; after a few songs, Home shows *Recently played* and the stats
    count them.
15. Volume with sensitive earphones: the player bar's volume button turns the level down in
    shared mode (on top of the Windows volume). In exclusive mode it moves the device's own
    volume; if the quietest setting is still too loud, *Use the app's volume instead* in
    Settings → Audio output takes over (starting at −30 dB; the badge then shows the level).
16. Settings → Sound: the equaliser and ReplayGain change the sound at once and the badge
    says *Converted* and names them; a 5 s crossfade in shared mode blends songs of different
    albums but keeps an album gapless.
