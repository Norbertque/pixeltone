//! What phones and the PC say to each other (JSON, camelCase). Documented for client
//! authors in `docs/remote-protocol.md`; bump `PROTOCOL` for incompatible changes.

use pp_engine::{PlayerSnapshot, RepeatMode};
use serde::{Deserialize, Serialize};

use crate::jam::{GuestPermissions, JamInfo};

pub const PROTOCOL: u32 = 1;
/// Most tracks one `load` may queue.
pub const MAX_LOAD: usize = 10_000;

/// `GET /v1/info`: enough to show the PC in a list and check its certificate.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub app: &'static str,
    pub id: String,
    pub name: String,
    pub version: String,
    pub protocol: u32,
    /// SHA-256 of the TLS certificate, hex.
    pub fingerprint: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairRequest {
    pub code: String,
    pub device_id: String,
    pub device_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairResponse {
    pub token: String,
    pub server: ServerInfo,
}

/// The volume control in charge on the PC (see the app's volume rules).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteVolume {
    /// "app", "device", "software" or "fixed".
    pub kind: String,
    pub db: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub muted: bool,
}

/// A phone's track: a file it serves itself, with what to show before it opens.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteItem {
    pub url: String,
    pub art_url: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadRequest {
    pub items: Vec<RemoteItem>,
    #[serde(default)]
    pub start: usize,
    #[serde(default)]
    pub position_ms: f64,
    #[serde(default = "yes")]
    pub play: bool,
    #[serde(default)]
    pub repeat: Option<RepeatMode>,
}

fn yes() -> bool {
    true
}

/// `POST /v1/jam/join`: a guest with the jam's QR code (or link).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRequest {
    pub secret: String,
    pub device_id: String,
    pub name: String,
}

/// A guest in: `token` goes with every request from now on, like a paired phone's.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinResponse<'a> {
    pub token: &'a str,
    pub guest_id: &'a str,
    pub server: &'a ServerInfo,
}

/// A removed guest's rejoin request, waiting for the owner (`202`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinWaiting<'a> {
    pub request_id: &'a str,
}

/// The jam as owners see it: who's in, who asks to come back, the guest permissions, and the
/// link its QR code shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JamView {
    pub link: String,
    #[serde(flatten)]
    pub info: JamInfo,
}

/// The jam as a guest sees it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestView {
    pub guest_id: String,
    pub name: String,
    pub permissions: GuestPermissions,
}

/// A library track as phones list it (search results, album pages).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTrack {
    pub id: u32,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: f64,
    pub cover_key: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAlbum {
    pub id: u32,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<i32>,
    pub track_count: u32,
    pub cover_key: Option<String>,
}

/// `GET /v1/library/search`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryResults {
    pub tracks: Vec<LibraryTrack>,
    pub albums: Vec<LibraryAlbum>,
}

/// `GET /v1/library/albums/{id}`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAlbumDetail {
    pub album: LibraryAlbum,
    pub tracks: Vec<LibraryTrack>,
}

/// Commands from a phone, over the WebSocket.
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Command {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    Seek {
        ms: f64,
    },
    SetRepeat {
        mode: RepeatMode,
    },
    SetShuffle {
        on: bool,
    },
    /// Play this queue item (an id from the state or `/v1/queue`).
    Jump {
        id: u32,
    },
    SetVolume {
        db: f32,
    },
    SetMuted {
        muted: bool,
    },
    /// Hand the phone's queue over to the PC (playback moves to the PC).
    Load(LoadRequest),
    /// Owner: start a jam (or keep the one that runs).
    JamStart,
    /// Owner: end the jam; guests lose access.
    JamEnd,
    /// Owner: what guests may do.
    JamPermissions {
        permissions: GuestPermissions,
    },
    /// Owner: take a guest out of the jam (their upcoming tracks go too).
    #[serde(rename_all = "camelCase")]
    JamRemoveGuest {
        guest_id: String,
    },
    /// Owner: answer a removed guest's rejoin request.
    #[serde(rename_all = "camelCase")]
    JamAnswer {
        request_id: String,
        allow: bool,
    },
    /// Library tracks to the end of the queue (in a jam: the guest's name goes with them).
    #[serde(rename_all = "camelCase")]
    QueueAdd {
        track_ids: Vec<u32>,
    },
    /// Take a queue item out (guests: only their own, if allowed).
    QueueRemove {
        id: u32,
    },
    /// Move a queue item (guests: only their own, if allowed).
    QueueMove {
        id: u32,
        to: usize,
    },
    /// Guest: leave the jam (their tracks stay).
    JamLeave,
}

/// Messages to phones, over the WebSocket.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message<'a> {
    Hello {
        server: &'a ServerInfo,
    },
    State {
        state: &'a PlayerSnapshot,
    },
    #[serde(rename_all = "camelCase")]
    Position {
        position_ms: f64,
    },
    Volume {
        volume: Option<&'a RemoteVolume>,
    },
    /// The library changed: lists a phone shows of it may be stale.
    LibraryChanged,
    /// To owners: the jam now (`null`: none runs).
    Jam {
        jam: Option<&'a JamView>,
    },
    /// To a guest: their place in the jam now.
    Guest {
        guest: &'a GuestView,
    },
    /// To a guest: the jam is over for them (ended, or they were removed).
    #[serde(rename_all = "camelCase")]
    JamClosed {
        reason: &'a str,
    },
    Error {
        code: &'a str,
        message: &'a str,
    },
}

impl Message<'_> {
    pub fn json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_read_as_documented() {
        let c: Command = serde_json::from_str(r#"{"cmd":"seek","ms":61000}"#).unwrap();
        assert!(matches!(c, Command::Seek { ms } if ms == 61000.0));
        let c: Command = serde_json::from_str(r#"{"cmd":"setRepeat","mode":"one"}"#).unwrap();
        assert!(matches!(
            c,
            Command::SetRepeat {
                mode: RepeatMode::One
            }
        ));
        let c: Command = serde_json::from_str(
            r#"{"cmd":"load","items":[{"url":"http://192.168.1.2:9000/file/1","title":"A"}],"start":0,"positionMs":1500}"#,
        )
        .unwrap();
        let Command::Load(load) = c else { panic!() };
        assert!(load.play, "plays unless asked not to");
        assert_eq!(load.items[0].title.as_deref(), Some("A"));
        assert!(serde_json::from_str::<Command>(r#"{"cmd":"format","disk":"C:"}"#).is_err());
        let c: Command = serde_json::from_str(
            r#"{"cmd":"jamPermissions","permissions":{"addTracks":true,"changeOwnTracks":false,"controlPlayback":true}}"#,
        )
        .unwrap();
        assert!(
            matches!(c, Command::JamPermissions { permissions } if permissions.control_playback && !permissions.change_own_tracks)
        );
        let c: Command = serde_json::from_str(r#"{"cmd":"queueAdd","trackIds":[3,4]}"#).unwrap();
        assert!(matches!(c, Command::QueueAdd { track_ids } if track_ids == [3, 4]));
        let c: Command =
            serde_json::from_str(r#"{"cmd":"jamAnswer","requestId":"r1","allow":false}"#).unwrap();
        assert!(matches!(c, Command::JamAnswer { request_id, allow: false } if request_id == "r1"));
    }

    #[test]
    fn messages_are_tagged_by_type() {
        assert_eq!(
            Message::Position { position_ms: 12.5 }.json(),
            r#"{"type":"position","positionMs":12.5}"#
        );
        assert_eq!(
            Message::LibraryChanged.json(),
            r#"{"type":"libraryChanged"}"#
        );
    }
}
