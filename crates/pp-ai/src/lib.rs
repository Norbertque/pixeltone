//! AI playlists: Claude picks songs for a request ("upbeat songs for a run") from the user's
//! own library, sent as a compact song list. Uses the user's own Anthropic API key over the
//! Messages API (raw HTTP: there is no official Rust SDK).

use std::collections::HashSet;
use std::fmt::Write as _;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use ureq::Agent;

pub const API: &str = "https://api.anthropic.com";
pub const MODEL: &str = "claude-opus-5-5";
/// Input price of `MODEL`, US dollars per million tokens (for the cost estimate).
pub const INPUT_USD_PER_MTOK: f64 = 4.0;
/// Most songs sent; bigger libraries are sampled evenly across artists.
pub const MAX_CATALOG: usize = 6000;
pub const MAX_SONGS: u32 = 100;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("The API key was not accepted. Check it in Settings → AI playlists.")]
    BadKey,
    #[error("The API key has no credit left or isn't allowed to use this model ({0}).")]
    NotAllowed(String),
    #[error("Anthropic is busy or rate-limiting this key. Try again in a minute.")]
    Busy,
    #[error("Claude declined this request. Try wording it differently.")]
    Refused,
    #[error("The answer was cut off. Ask for fewer songs.")]
    Truncated,
    #[error("Anthropic: {0}")]
    Api(String),
    #[error("Can't reach Anthropic: {0}")]
    Network(String),
}

/// One song as Claude sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct Song {
    pub id: u32,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i32>,
    pub genres: String,
}

/// The song list sent with every request, and the ids in it.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    /// One tab-separated line per song, after a header line.
    pub text: String,
    pub ids: HashSet<u32>,
}

impl Catalog {
    /// Libraries above `MAX_CATALOG` keep every n-th song (in the order given), so every
    /// artist stays represented.
    pub fn new(songs: &[Song]) -> Self {
        let stride = songs.len().div_ceil(MAX_CATALOG).max(1);
        let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        let mut text = String::with_capacity(songs.len().min(MAX_CATALOG) * 80);
        let mut ids = HashSet::new();
        text.push_str("id\ttitle\tartist\talbum\tyear\tgenres\n");
        for song in songs.iter().step_by(stride) {
            ids.insert(song.id);
            let _ = writeln!(
                text,
                "{}\t{}\t{}\t{}\t{}\t{}",
                song.id,
                clean(&song.title),
                clean(&song.artist),
                clean(&song.album),
                song.year.map(|y| y.to_string()).unwrap_or_default(),
                clean(&song.genres),
            );
        }
        Self { text, ids }
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// A rough price of one request in US dollars: the song list dominates it.
    pub fn estimate_usd(&self) -> f64 {
        let tokens = self.text.len() as f64 / 3.5 + 1500.0;
        tokens * INPUT_USD_PER_MTOK / 1_000_000.0
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AiPlaylist {
    pub name: String,
    pub description: String,
    pub track_ids: Vec<u32>,
}

const SYSTEM: &str = "You build playlists from the user's own music library. The library is \
listed below, one song per line: id, title, artist, album, year and genres, separated by tabs. \
Pick songs only from this list and refer to them by id. Choose songs that fit the request, and \
order them so the playlist flows well (mood, energy and tempo). Unless asked otherwise, avoid \
more than two songs in a row by the same artist. When the library has fewer songs that fit \
than asked for, return fewer rather than padding with poor fits. Give the playlist a short \
name without quotation marks, and describe it in one sentence.";

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "description": {"type": "string"},
            "track_ids": {"type": "array", "items": {"type": "integer"}}
        },
        "required": ["name", "description", "track_ids"],
        "additionalProperties": false
    })
}

/// The Messages API request body.
pub fn request_body(catalog: &str, prompt: &str, songs: u32) -> Value {
    json!({
        "model": MODEL,
        "max_tokens": 16000,
        // Thinking is always on for this model; effort sets its depth.
        "output_config": {
            "effort": "medium",
            "format": {"type": "json_schema", "schema": schema()}
        },
        // A policy decline is retried on the model Anthropic recommends for it.
        "fallbacks": "default",
        "system": [
            {"type": "text", "text": SYSTEM},
            // The library rarely changes between requests: cache it.
            {"type": "text", "text": catalog, "cache_control": {"type": "ephemeral"}}
        ],
        "messages": [{
            "role": "user",
            "content": format!("Request: {}\nNumber of songs: {}", prompt.trim(), songs.clamp(1, MAX_SONGS))
        }]
    })
}

pub struct Claude {
    agent: Agent,
    base: String,
    key: String,
}

impl Claude {
    pub fn new(base: &str, key: &str) -> Self {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(300)))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
            key: key.trim().to_string(),
        }
    }

    /// Asks for a playlist of up to `songs` songs. Ids that aren't in the catalog are
    /// dropped, and so are repeats.
    pub fn playlist(
        &self,
        catalog: &Catalog,
        prompt: &str,
        songs: u32,
    ) -> Result<AiPlaylist, AiError> {
        let mut resp = self
            .agent
            .post(format!("{}/v1/messages", self.base))
            .header("x-api-key", &self.key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "server-side-fallback-2026-07-01")
            .send_json(request_body(&catalog.text, prompt, songs))
            .map_err(|e| AiError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        let body: Value = resp
            .body_mut()
            .read_json()
            .map_err(|e| AiError::Network(e.to_string()))?;
        if status != 200 {
            let message = body["error"]["message"]
                .as_str()
                .unwrap_or("unknown error")
                .to_string();
            return Err(match status {
                401 => AiError::BadKey,
                402 | 403 => AiError::NotAllowed(message),
                429 | 529 => AiError::Busy,
                _ => AiError::Api(message),
            });
        }
        match body["stop_reason"].as_str() {
            Some("refusal") => return Err(AiError::Refused),
            Some("max_tokens") => return Err(AiError::Truncated),
            _ => {}
        }
        let text = body["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect::<String>();
        let mut playlist: AiPlaylist = serde_json::from_str(&text)
            .map_err(|e| AiError::Api(format!("unexpected answer ({e})")))?;
        let mut seen = HashSet::new();
        playlist
            .track_ids
            .retain(|id| catalog.ids.contains(id) && seen.insert(*id));
        playlist
            .track_ids
            .truncate(songs.clamp(1, MAX_SONGS) as usize);
        playlist.name = playlist.name.trim().trim_matches('"').trim().to_string();
        Ok(playlist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(id: u32, title: &str, artist: &str) -> Song {
        Song {
            id,
            title: title.into(),
            artist: artist.into(),
            album: "Album".into(),
            year: Some(2021),
            genres: "Breakcore; Electronic".into(),
        }
    }

    #[test]
    fn the_catalog_is_one_clean_line_per_song() {
        let c = Catalog::new(&[
            song(7, "Tab\there", "Loser64"),
            song(9, "Two\nlines", "Sewerslvt"),
        ]);
        assert_eq!(c.ids, [7, 9].into_iter().collect());
        let lines: Vec<&str> = c.text.lines().collect();
        assert_eq!(lines[0], "id\ttitle\tartist\talbum\tyear\tgenres");
        assert_eq!(
            lines[1],
            "7\tTab here\tLoser64\tAlbum\t2021\tBreakcore; Electronic"
        );
        assert_eq!(
            lines[2],
            "9\tTwo lines\tSewerslvt\tAlbum\t2021\tBreakcore; Electronic"
        );
    }

    #[test]
    fn big_libraries_are_sampled_evenly() {
        let songs: Vec<Song> = (0..MAX_CATALOG as u32 * 2 + 10)
            .map(|i| song(i, "t", "a"))
            .collect();
        let c = Catalog::new(&songs);
        let rows = c.text.lines().count() - 1;
        assert!(rows <= MAX_CATALOG && rows > MAX_CATALOG / 2, "{rows}");
        assert_eq!(rows, c.len());
        assert!(c.estimate_usd() > 0.05);
    }

    #[test]
    fn the_request_caches_the_library_and_asks_for_json() {
        let body = request_body("id\t…", " chill ", 500);
        assert_eq!(body["model"], MODEL);
        assert_eq!(body["fallbacks"], "default");
        assert_eq!(body["system"][1]["cache_control"]["type"], "ephemeral");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert!(body.get("thinking").is_none());
        assert_eq!(
            body["messages"][0]["content"],
            "Request: chill\nNumber of songs: 100"
        );
    }
}
