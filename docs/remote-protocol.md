# Remote control protocol (v1)

How a phone app (PixelPlayer) finds Pixeltone on the local network, pairs with it, follows and
drives the player, and hands its own queue over to the PC (playback moves to the PC, like
Spotify Connect). Server: `crates/pp-remote`. Off until the user turns on **Settings → Remote
control**.

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
| `GET /v1/queue?offset=0&limit=200` | ✓ | queue rows `[{id, path, title, artist, album, durationMs, coverKey, trackId}]` |
| `GET /v1/cover/<coverKey>` | ✓ | the image (JPEG/PNG/WebP) of a state's or a queue row's `coverKey` |
| `GET /v1/ws` | ✓ | WebSocket (below) |

401 means the token is unknown (unpaired on the PC): pair again.

## WebSocket `/v1/ws`

JSON text messages. On connect the PC sends `hello`, `state` and `volume`, then events as
they happen.

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

Phone → PC (`cmd`):

```json
{"cmd": "play"} {"cmd": "pause"} {"cmd": "toggle"} {"cmd": "stop"}
{"cmd": "next"} {"cmd": "previous"}
{"cmd": "seek", "ms": 61000}
{"cmd": "setRepeat", "mode": "off"}      {"cmd": "setShuffle", "on": true}
{"cmd": "jump", "id": 12}                 (a queue id)
{"cmd": "setVolume", "db": -20.5}         {"cmd": "setMuted", "muted": true}
{"cmd": "load", "items": [...], "start": 3, "positionMs": 61000, "play": true, "repeat": "all"}
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

## Trying a client without Windows

`cargo run -p pp-remote --example dev_server [port]` runs the same server on any OS, playing
into a simulated device in real time. It prints its port, the certificate fingerprint, a
pairing code (a fresh one after each pairing) and the player's state as it changes.
