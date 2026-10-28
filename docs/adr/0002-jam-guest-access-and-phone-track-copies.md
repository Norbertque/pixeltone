# Jam guests get access to one jam only, and their phone tracks are copied to the PC

A jam lets friends on the same Wi-Fi share the PC's queue. Guests are never paired: the jam's
QR code carries the PC's address, its certificate fingerprint and a secret for this jam, which
gives access to this jam only, and that access ends with the jam. The owner (the person at
the PC and every paired phone) starts and ends jams, sets guest permissions and removes
guests; a removed guest gets back in only through a rejoin request the owner allows.

Phone tracks work differently from a hand-over. When a guest, or the owner during a jam, adds
a track from their phone, the phone sends a temporary copy to the PC instead of serving it
while it plays, so the track still plays after the phone locks or leaves the Wi-Fi. The copy
holds the original bytes, so playback stays bit-perfect, and it is deleted when the jam ends.

## Consequences

- While a jam runs, a hand-over can't replace the queue: phone tracks join the end of it like
  everyone else's.
- Tracks played during a jam don't count in history or statistics.
