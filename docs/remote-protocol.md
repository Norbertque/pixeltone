# Remote control protocol (v1)

How a phone app (Pixeltone for Android) finds Pixeltone on the local network, pairs with it,
follows and drives the player, and hands its own queue over to the PC (playback moves to the
PC, like Spotify Connect); and how guests join a jam. Server: `crates/pp-remote`. Off until
the user turns on **Settings → Remote control**, except while a jam runs.

## Finding the PC

- mDNS / DNS-SD service type `_pixeltone._tcp`, instance name = the PC's name. TXT records:
  `id` (stable per installation), `v` (protocol version), `name`.
- Or by address: Settings shows `ip:port` (port 47821 unless it was taken).

## Security

- **Local network only**: requests from addresses outside private/link-local ranges get 403.
- **TLS** with a self-signed certificate made once per installation. The phone pins its
  **SHA-256 fingerprint** (of the DER certificate, lowercase hex) when it pairs and refuses
  any other certificate afterwards. Hostnames aren't checked (the pin is the identity).
- **Pairing**: the user presses *Pair a phone* on the PC and types the six-digit code on
  the phone. A code works once, for 10 minutes, and stops working after 5 wrong tries. The
  phone gets a token (keep it secret; the PC stores only its SHA-256) and sends it as
  `Authorization: Bearer <token>` on every other request.

## HTTPS endpoints

| Request | Auth | Answer |
|---|---|---|
| `GET /v1/info` | — | `{app, id, name, version, protocol, fingerprint}` |
| `POST /v1/pair` `{code, deviceId, deviceName}` | — | 200 `{token, server: <info>}`; 403 wrong code; 410 pairing closed, expired or too many tries |
| `DELETE /v1/pair` | ✓ | 204: forgets this phone |
| `GET /v1/queue?offset=0&limit=200` | ✓ | queue rows `[{id, path, title, artist, album, durationMs, coverKey, trackId, addedBy, guestId}]` |
| `GET /v1/cover/<coverKey>` | ✓ | the image (JPEG/PNG/WebP) of a state's or a queue row's `coverKey` |
| `GET /v1/ws` | ✓ | WebSocket (below) |
| `GET /v1/library/search?q=…&limit=50` | ✓ | `{tracks: [<track>], albums: [<album>]}` |
| `GET /v1/library/albums?offset=0&limit=200` | ✓ | `[<album>]` by title |
| `GET /v1/library/albums/<id>` | ✓ | `{album, tracks: [<track>]}`; 404 unknown |
| `POST /v1/jam/join` `{secret, deviceId, name}` | — | see [Jam](#jam) |
| `GET /v1/jam/requests/<id>` | — | see [Jam](#jam) |
| `POST /v1/jam/tracks?name=…` + the file | ✓ | see [Jam](#jam) |

A library `<track>` is `{id, title, artist, album, durationMs, coverKey}`, an `<album>`
`{id, title, artist, year, trackCount, coverKey}`; ids go into `queueAdd`.

✓ takes a paired phone's token, or a jam guest's while the jam runs. 401 means the token is
unknown (unpaired on the PC, or the guest is out of the jam): pair (or join) again.

## WebSocket `/v1/ws`

JSON text messages. On connect the PC sends `hello`, `state`, `volume` and `jam` (a guest:
`hello`, `state` and `guest`), then events as they happen.

PC → phone (`type`):

- `hello` — `{server: <info>}`
- `state` — `{state: PlayerSnapshot}`: `status` (`playing`/`paused`/`stopped`), `track`
  (`title`, `artist`, `album`, `durationMs`, `coverKey`, `source`…), `currentId` (queue id),
  `queueIndex`, `queueLength`, `queueVersion` (changes whenever the queue does: refetch
  `/v1/queue`), `positionMs`, `repeat` (`off`/`all`/`one`), `shuffle`, `output` (`badge.label`,
  e.g. "Exclusive • 24-bit/96 kHz • Bit-perfect").
- `position` — `{positionMs}` while playing (interpolate in between).
- `volume` — `{volume: {kind, db, min, max, step, muted} | null}`; `kind` is `app` (the app's
  own level in shared mode), `device` (the DAC's own volume), `software` (exclusive mode,
  attenuated in the app) or `fixed` (line level: no volume).
- `libraryChanged`, `error` — `{code, message}` (a refused command says why).
- `jam` (owners) — `{jam: {link, permissions, guests: [{id, name}], requests: [{id, name}]} |
  null}`, again whenever it changes.
- `guest` (a guest) — `{guest: {guestId, name, permissions}}`, again when the permissions
  change.
- `jamClosed` (a guest) — `{reason: "ended" | "removed"}`; the connection closes after it.

Phone → PC (`cmd`):

```json
{"cmd": "play"} {"cmd": "pause"} {"cmd": "toggle"} {"cmd": "stop"}
{"cmd": "next"} {"cmd": "previous"}
{"cmd": "seek", "ms": 61000}
{"cmd": "setRepeat", "mode": "off"}      {"cmd": "setShuffle", "on": true}
{"cmd": "jump", "id": 12}                 (a queue id)
{"cmd": "setVolume", "db": -20.5}         {"cmd": "setMuted", "muted": true}
{"cmd": "load", "items": [...], "start": 3, "positionMs": 61000, "play": true, "repeat": "all"}
{"cmd": "queueAdd", "trackIds": [101, 102]}   (library tracks, to the end of the queue)
{"cmd": "queueRemove", "id": 12}          {"cmd": "queueMove", "id": 12, "to": 0}
{"cmd": "jamStart"} {"cmd": "jamEnd"}
{"cmd": "jamPermissions", "permissions": {"addTracks": true, "changeOwnTracks": true, "controlPlayback": false}}
{"cmd": "jamRemoveGuest", "guestId": "…"}  {"cmd": "jamAnswer", "requestId": "…", "allow": true}
{"cmd": "jamLeave"}                       (a guest)
```

### Handing a queue over (`load`)

Each item: `{url, artUrl?, title?, artist?, album?, durationMs?}`. `url` must be plain
`http://` **on the phone's own address** (the one the WebSocket comes from) and serve the
**original file bytes** with `Range` support; the PC decodes it like a local file, so
exclusive mode stays bit-perfect. `artUrl` (same host) is the cover; title/artist/album/
duration show in the queue before each file is opened. Up to 10 000 items; playback starts
at `start`, `positionMs` into it. Phone tracks are kept out of the PC's history and of the
queue it restores after a restart.

To take playback back: read `positionMs` and the current item from the last `state`, send
`pause`, and continue on the phone.

## Jam

A temporary shared queue on the PC (`GLOSSARY.md`, ADR 0002). The owner (whoever is at the PC,
or a paired phone) starts it with `jamStart`; it runs until `jamEnd`, or until Pixeltone quits
— then guests get `jamClosed`, their tokens die and the old QR code with them. The server runs
for a jam even with remote control off.

**The link** (`jam.link`, what the QR code shows):
`https://norbertque.github.io/pixeltone/jam/#v=1&h=<ip>&p=<port>&f=<fingerprint>&s=<secret>&n=<PC name>`.
Everything after `#` stays on the phone (browsers don't send it). The app opens such links;
the page tells people without it where to get it. The phone talks to `h:p`, pinning `f` like
a pairing would.

**Joining**: `POST /v1/jam/join {secret, deviceId, name}` →

- 200 `{token, guestId, server}`: in; `token` works like a paired phone's, within the guest
  permissions, until the jam ends or the owner removes the guest.
- 202 `{requestId}`: this phone was removed from the jam, and the owner has to let it back in.
  Ask `GET /v1/jam/requests/<requestId>` every few seconds: 202 while waiting, 200 (as above)
  when allowed, 403 declined (final for this jam), 410 when it ran out after 2 minutes (join
  again to ask again) or the jam ended.
- 409 the jam is full (32 guests); 410 wrong secret: the jam has ended.

**Guest permissions** (the owner changes them live): `addTracks` (default on),
`changeOwnTracks` (remove and move the tracks they added; on), `controlPlayback` (play, pause,
stop, skip, seek, jump; off). The volume, repeat, shuffle, hand-overs and the jam commands are
the owner's alone. Tracks a guest adds carry `addedBy` (their name) and `guestId` in the queue.
Removing a guest also removes their tracks still to come; a guest who leaves (`jamLeave`)
keeps theirs in the queue. Nothing played during a jam counts in the PC's history.

**A phone's own tracks** join the jam as copies on the PC, so they play on when the phone goes
(a hand-over with `load` is refused during a jam):
`POST /v1/jam/tracks?name=<file name>&title=…&artist=…&album=…&durationMs=…` with the
original file bytes as the body and its `Content-Length` (the copy is bit for bit). Answers:
204 added to the end of the queue; 403 guests may not add tracks; 409 no jam; 411 no length;
413 over 2 GB; 507 the PC would have less than 5 GB free; 410 the jam ended (or the guest was
removed) meanwhile. `name` gives the extension, and the title when `title` is missing. The
copies leave the queue and the disk when the jam ends.

## Trying a client without Windows

`cargo run -p pp-remote --example dev_server [port]` runs the same server on any OS, playing
into a simulated device in real time. It prints its port, the certificate fingerprint, a
pairing code (a fresh one after each pairing) and the player's state as it changes. It has a
small library of test tones, and reads commands from its input: `jam` starts a jam and prints
its link, `end` ends it, `allow <request id>` / `decline <request id>` answer a rejoin
request, `remove <guest id>` removes a guest.
