//! LRCLIB (lrclib.net): a free, open lyrics database with synced LRC for many songs.

use std::time::Duration;

use serde::Deserialize;
use ureq::Agent;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Track {
    pub duration: Option<f64>,
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

pub struct Query<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub duration_s: Option<u32>,
}

pub struct Lrclib {
    agent: Agent,
    base: String,
}

/// "A feat. B" → "A": LRCLIB lists songs under their main artist.
fn main_artist(artist: &str) -> &str {
    let lower = artist.to_lowercase();
    let cut = [" feat.", " feat ", " ft.", " ft ", " featuring ", ";"]
        .iter()
        .filter_map(|d| lower.find(d))
        .min();
    match cut {
        Some(i) => artist[..i].trim(),
        None => artist.trim(),
    }
}

impl Lrclib {
    pub fn new(base: &str) -> Self {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .user_agent(concat!(
                "PixelPlayer PC/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/Norbertque/pixelplayer-pc)"
            ))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
        }
    }

    /// The best match for a track; `None` when LRCLIB doesn't know it.
    pub fn find(&self, q: &Query) -> Result<Option<Track>, ureq::Error> {
        // The exact signature first (LRCLIB allows a couple of seconds on the duration).
        if let (false, Some(duration)) = (q.album.is_empty(), q.duration_s) {
            let mut resp = self
                .agent
                .get(format!("{}/api/get", self.base))
                .query("track_name", q.title)
                .query("artist_name", q.artist)
                .query("album_name", q.album)
                .query("duration", duration.to_string())
                .call()?;
            if resp.status().is_success() {
                return Ok(Some(resp.body_mut().read_json()?));
            }
        }
        // Then a search, keeping the closest duration and preferring synced lyrics.
        let mut resp = self
            .agent
            .get(format!("{}/api/search", self.base))
            .query("track_name", q.title)
            .query("artist_name", main_artist(q.artist))
            .call()?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let results: Vec<Track> = resp.body_mut().read_json()?;
        Ok(best(results, q.duration_s))
    }
}

fn best(results: Vec<Track>, duration_s: Option<u32>) -> Option<Track> {
    let off = |t: &Track| match (t.duration, duration_s) {
        (Some(d), Some(want)) => (d - f64::from(want)).abs(),
        _ => 0.0,
    };
    results
        .into_iter()
        .filter(|t| off(t) <= 3.0)
        .filter(|t| t.instrumental || t.synced_lyrics.is_some() || t.plain_lyrics.is_some())
        .min_by(|a, b| {
            let rank = |t: &Track| (t.synced_lyrics.is_none(), off(t));
            rank(a)
                .partial_cmp(&rank(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_artist_drops_featured_guests() {
        assert_eq!(main_artist("Aurora Lane feat. Neon Child"), "Aurora Lane");
        assert_eq!(main_artist("A ft. B"), "A");
        assert_eq!(main_artist("Simon & Garfunkel"), "Simon & Garfunkel");
    }

    #[test]
    fn picks_synced_lyrics_with_the_closest_duration() {
        let t = |d: f64, synced: bool| Track {
            duration: Some(d),
            synced_lyrics: synced.then(|| "[00:01.00]x".to_string()),
            plain_lyrics: Some("x".into()),
            ..Track::default()
        };
        let pick = best(
            vec![
                t(200.0, false),
                t(203.5, true),
                t(201.0, true),
                t(150.0, true),
            ],
            Some(200),
        );
        assert_eq!(pick.unwrap().duration, Some(201.0));
        assert!(
            best(vec![t(250.0, true)], Some(200)).is_none(),
            "too far off"
        );
    }
}
