//! A track opened for playback, and the metadata the UI shows for it.

use std::path::{Path, PathBuf};

use pp_core::{Negotiated, SourceFormat};
use pp_decode::{CoverArt, Decoder};
use serde::Serialize;
use ts_rs::TS;

use crate::pipeline::OutputPlan;

/// What the Now Playing screen shows.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TrackMeta {
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<f64>,
    /// Decoder name, e.g. "flac".
    pub codec: String,
    pub source: SourceFormat,
    pub lossless: bool,
    pub dsd: bool,
    /// Key of the embedded cover in the artwork store, when the file has one.
    pub cover_key: Option<String>,
}

pub struct LoadedTrack {
    pub pos: usize,
    pub decoder: Decoder,
    pub negotiated: Negotiated,
    pub start_ms: u64,
    pub meta: TrackMeta,
    pub cover: Option<CoverArt>,
}

/// What the engine needs to know when a track becomes the one being decoded.
#[derive(Clone, Debug)]
pub struct StartedTrack {
    pub pos: usize,
    pub negotiated: Negotiated,
    pub start_ms: u64,
    pub meta: TrackMeta,
    pub cover: Option<CoverArt>,
}

impl LoadedTrack {
    /// Track details for the engine; the cover moves along with them.
    pub fn started(&mut self) -> StartedTrack {
        StartedTrack {
            pos: self.pos,
            negotiated: self.negotiated,
            start_ms: self.start_ms,
            meta: self.meta.clone(),
            cover: self.cover.take(),
        }
    }
}

/// Opens `path` at `start_ms` and negotiates its output format under `plan`.
pub fn load(
    pos: usize,
    path: &Path,
    start_ms: u64,
    plan: &OutputPlan,
) -> Result<LoadedTrack, String> {
    let mut decoder = Decoder::open(path).map_err(|e| e.to_string())?;
    let info = decoder.info().clone();
    let negotiated = plan
        .negotiate(&info.source, info.dsd)
        .ok_or_else(|| format!("no output format for {}", path.display()))?;
    if start_ms > 0 {
        let frame = start_ms * u64::from(info.source.sample_rate) / 1000;
        decoder.seek(frame).map_err(|e| e.to_string())?;
    }
    let tags = decoder.tags();
    let cover = decoder.cover_art();
    let meta = TrackMeta {
        path: path.display().to_string(),
        title: tags.title.unwrap_or_else(|| file_stem(path)),
        artist: tags.artist.or(tags.album_artist),
        album: tags.album,
        duration_ms: info.duration_ms.map(|d| d as f64),
        codec: info.codec.clone(),
        source: info.source,
        lossless: info.lossless,
        dsd: info.dsd,
        cover_key: cover.as_ref().map(|c| cover_key(&c.data)),
    };
    Ok(LoadedTrack {
        pos,
        decoder,
        negotiated,
        start_ms,
        meta,
        cover,
    })
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| PathBuf::from(path).display().to_string())
}

/// Stable key for cover bytes (FNV-1a, hex).
pub fn cover_key(data: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}
