//! Lyrics for a track, from (in order) an `.lrc` file next to it, a copy the user saved in
//! the app, the file's embedded lyrics tag, or LRCLIB online — cached on disk, with misses
//! retried after a week.

mod lrc;
mod lrclib;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use lrc::{parse, render, stamp};

/// The public LRCLIB service.
pub const LRCLIB: &str = "https://lrclib.net";
/// How long "not found" online is believed.
const MISS_TTL_SECS: u64 = 7 * 86_400;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LyricLine {
    /// Start time; `None` for plain (unsynced) lyrics.
    pub time_ms: Option<u32>,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LyricsSource {
    /// An `.lrc` file next to the track.
    File,
    /// Saved by the user inside the app (the music folder wasn't writable).
    Saved,
    /// The file's lyrics tag.
    Embedded,
    /// LRCLIB.
    Online,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Lyrics {
    pub source: LyricsSource,
    pub synced: bool,
    /// Known to have no words.
    pub instrumental: bool,
    pub lines: Vec<LyricLine>,
    /// As LRC text, for editing.
    pub text: String,
}

impl Lyrics {
    fn from_text(source: LyricsSource, text: &str) -> Option<Self> {
        let (lines, synced) = parse(text);
        (!lines.is_empty()).then(|| Self {
            source,
            synced,
            instrumental: false,
            text: render(&lines),
            lines,
        })
    }
}

/// The track lyrics are wanted for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LyricsQuery {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum LyricsError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("there are no lyrics to save")]
    Empty,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    fetched_at: u64,
    synced: Option<String>,
    plain: Option<String>,
    instrumental: bool,
    found: bool,
}

pub struct LyricsService {
    saved_dir: PathBuf,
    cache_dir: PathBuf,
    online: AtomicBool,
    lrclib: lrclib::Lrclib,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// UTF-8 (with or without BOM), else Latin-1.
fn read_text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    Some(match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    })
}

fn sidecar(path: &Path) -> PathBuf {
    path.with_extension("lrc")
}

fn embedded(path: &Path) -> Option<String> {
    let file = lofty::read_from_path(path).ok()?;
    file.tags()
        .iter()
        .filter_map(|tag| tag.get_string(ItemKey::Lyrics))
        .find(|text| !text.trim().is_empty())
        .map(str::to_string)
}

impl LyricsService {
    /// `saved_dir`: lyrics the user saved when the music folder isn't writable;
    /// `cache_dir`: what came from LRCLIB.
    pub fn new(saved_dir: impl Into<PathBuf>, cache_dir: impl Into<PathBuf>) -> Self {
        Self::with_endpoint(saved_dir, cache_dir, LRCLIB)
    }

    pub fn with_endpoint(
        saved_dir: impl Into<PathBuf>,
        cache_dir: impl Into<PathBuf>,
        endpoint: &str,
    ) -> Self {
        Self {
            saved_dir: saved_dir.into(),
            cache_dir: cache_dir.into(),
            online: AtomicBool::new(true),
            lrclib: lrclib::Lrclib::new(endpoint),
        }
    }

    /// Whether lyrics may be looked up online.
    pub fn set_online(&self, on: bool) {
        self.online.store(on, Ordering::Relaxed);
    }

    fn saved_path(&self, track: &Path) -> PathBuf {
        let key = xxhash_rust::xxh3::xxh3_64(track.to_string_lossy().as_bytes());
        self.saved_dir.join(format!("{key:016x}.lrc"))
    }

    fn cache_path(&self, q: &LyricsQuery) -> PathBuf {
        let sig = format!(
            "{}\0{}\0{}\0{}",
            q.artist.to_lowercase(),
            q.title.to_lowercase(),
            q.album.to_lowercase(),
            q.duration_ms.unwrap_or(0) / 1000
        );
        let key = xxhash_rust::xxh3::xxh3_64(sig.as_bytes());
        self.cache_dir.join(format!("{key:016x}.json"))
    }

    /// Lyrics found on this computer: next to the file, saved in the app, or embedded.
    pub fn local(&self, q: &LyricsQuery) -> Option<Lyrics> {
        read_text(&sidecar(&q.path))
            .and_then(|t| Lyrics::from_text(LyricsSource::File, &t))
            .or_else(|| {
                read_text(&self.saved_path(&q.path))
                    .and_then(|t| Lyrics::from_text(LyricsSource::Saved, &t))
            })
            .or_else(|| {
                embedded(&q.path).and_then(|t| Lyrics::from_text(LyricsSource::Embedded, &t))
            })
    }

    /// Local lyrics, else LRCLIB's (when online lookups are on). May take a few seconds.
    pub fn find(&self, q: &LyricsQuery) -> Option<Lyrics> {
        if let Some(l) = self.local(q) {
            return Some(l);
        }
        if q.title.trim().is_empty() || !self.online.load(Ordering::Relaxed) {
            return None;
        }
        let path = self.cache_path(q);
        let cached: Option<CacheEntry> = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .filter(|e: &CacheEntry| e.found || now().saturating_sub(e.fetched_at) < MISS_TTL_SECS);
        let entry = match cached {
            Some(e) => e,
            None => {
                let query = lrclib::Query {
                    title: &q.title,
                    artist: &q.artist,
                    album: &q.album,
                    duration_s: q.duration_ms.map(|d| (d + 500) / 1000),
                };
                let found = match self.lrclib.find(&query) {
                    Ok(found) => found,
                    Err(e) => {
                        // Offline or the service is down: try again next time.
                        tracing::debug!("LRCLIB: {e}");
                        return None;
                    }
                };
                let entry = CacheEntry {
                    fetched_at: now(),
                    found: found.is_some(),
                    instrumental: found.as_ref().is_some_and(|t| t.instrumental),
                    synced: found.as_ref().and_then(|t| t.synced_lyrics.clone()),
                    plain: found.and_then(|t| t.plain_lyrics),
                };
                if let Ok(json) = serde_json::to_vec(&entry) {
                    let _ = std::fs::create_dir_all(&self.cache_dir);
                    let _ = std::fs::write(&path, json);
                }
                entry
            }
        };
        if entry.instrumental {
            return Some(Lyrics {
                source: LyricsSource::Online,
                synced: false,
                instrumental: true,
                lines: Vec::new(),
                text: String::new(),
            });
        }
        entry
            .synced
            .as_deref()
            .and_then(|t| Lyrics::from_text(LyricsSource::Online, t))
            .or_else(|| {
                entry
                    .plain
                    .as_deref()
                    .and_then(|t| Lyrics::from_text(LyricsSource::Online, t))
            })
    }

    /// Saves the user's lyrics as an `.lrc` next to the track, or inside the app when the
    /// folder isn't writable.
    pub fn save(&self, q: &LyricsQuery, text: &str) -> Result<Lyrics, LyricsError> {
        if text.trim().is_empty() {
            return Err(LyricsError::Empty);
        }
        let text = text.replace("\r\n", "\n");
        match std::fs::write(sidecar(&q.path), &text) {
            Ok(()) => {
                // A copy saved earlier would only hide behind the new file; drop it.
                let _ = std::fs::remove_file(self.saved_path(&q.path));
                Lyrics::from_text(LyricsSource::File, &text).ok_or(LyricsError::Empty)
            }
            Err(e) => {
                tracing::debug!("can't write next to {}: {e}", q.path.display());
                std::fs::create_dir_all(&self.saved_dir)?;
                std::fs::write(self.saved_path(&q.path), &text)?;
                Lyrics::from_text(LyricsSource::Saved, &text).ok_or(LyricsError::Empty)
            }
        }
    }
}
