//! What phones and the PC say to each other (JSON, camelCase). Documented for client
//! authors in `docs/remote-protocol.md`; bump `PROTOCOL` for incompatible changes.

use pp_engine::{PlayerSnapshot, RepeatMode};
use serde::{Deserialize, Serialize};

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
