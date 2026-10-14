//! A track opened for playback, and the metadata the UI shows for it.

use std::path::{Path, PathBuf};

use pp_core::{Negotiated, SourceFormat};
use pp_decode::{CoverArt, Decoder};
use serde::Serialize;
use ts_rs::TS;

use crate::effects::replay_gain_db;
use crate::pipeline::OutputPlan;
use crate::queue::QueueItem;

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
    /// Queue item id.
    pub id: u32,
    pub decoder: Decoder,
    pub negotiated: Negotiated,
    pub start_ms: u64,
    pub meta: TrackMeta,
    pub cover: Option<CoverArt>,
    /// ReplayGain for this track, in dB (applied while decoding).
    pub replay_gain_db: Option<f32>,
}

/// What the engine needs to know when a track becomes the one being decoded.
#[derive(Clone, Debug)]
pub struct StartedTrack {
    pub id: u32,
    pub negotiated: Negotiated,
    pub start_ms: u64,
    pub meta: TrackMeta,
    pub cover: Option<CoverArt>,
    pub replay_gain_db: Option<f32>,
}

impl LoadedTrack {
    /// Track details for the engine; the cover moves along with them.
    pub fn started(&mut self) -> StartedTrack {
        StartedTrack {
            id: self.id,
            negotiated: self.negotiated,
            start_ms: self.start_ms,
            meta: self.meta.clone(),
            cover: self.cover.take(),
            replay_gain_db: self.replay_gain_db,
        }
    }

    /// The ReplayGain factor to multiply decoded samples by.
    pub fn gain(&self) -> Option<f32> {
        self.replay_gain_db.map(|db| 10f32.powf(db / 20.0))
    }
}

/// Opens a queue item at `start_ms` and negotiates its output format under `plan`.
pub fn load(
    id: u32,
    item: &QueueItem,
    start_ms: u64,
    plan: &OutputPlan,
) -> Result<LoadedTrack, String> {
    let path = item.path.as_path();
    let mut decoder = open(item)?;
    let info = decoder.info().clone();
    let negotiated = plan
        .negotiate(&info.source, info.dsd)
        .ok_or_else(|| format!("no output format for {}", path.display()))?;
    if start_ms > 0 {
        let frame = start_ms * u64::from(info.source.sample_rate) / 1000;
        if negotiated.conversion.dop {
            // Raw DSD: FFmpeg's DSD rate counts bytes per channel, so the frame is the byte.
            decoder.seek_dsd(frame).map_err(|e| e.to_string())?;
        } else {
            decoder.seek(frame).map_err(|e| e.to_string())?;
        }
    }
    // DoP carries DSD untouched: no ReplayGain there.
    let replay_gain_db = if negotiated.conversion.dop {
        None
    } else {
        replay_gain_db(
            plan.replay_gain,
            plan.replay_gain_preamp_db,
            &decoder.tags().replay_gain,
        )
    };
    let (meta, cover) = describe(&mut decoder, item);
    Ok(LoadedTrack {
        id,
        decoder,
        negotiated,
        start_ms,
        meta,
        cover,
        replay_gain_db,
    })
}

/// Reads what Now Playing shows for an item (and its embedded cover) without playing it.
pub fn read_meta(item: &QueueItem) -> Result<(TrackMeta, Option<CoverArt>), String> {
    let mut decoder = open(item)?;
    Ok(describe(&mut decoder, item))
}

/// A local file, or a file a paired phone serves (remote control).
fn open(item: &QueueItem) -> Result<Decoder, String> {
    if item.remote {
        Decoder::open_http(&item.path.to_string_lossy())
    } else {
        Decoder::open(&item.path)
    }
    .map_err(|e| e.to_string())
}

fn describe(decoder: &mut Decoder, item: &QueueItem) -> (TrackMeta, Option<CoverArt>) {
    let path = item.path.as_path();
    let info = decoder.info().clone();
    let tags = decoder.tags();
    // A library cover wins; otherwise the embedded picture is served from the engine.
    let cover = if item.cover_key.is_some() {
        None
    } else {
        decoder.cover_art()
    };
    let meta = TrackMeta {
        path: path.display().to_string(),
        title: tags
            .title
            .or_else(|| item.title.clone())
            .unwrap_or_else(|| file_stem(path)),
        artist: tags
            .artist
            .or(tags.album_artist)
            .or_else(|| item.artist.clone()),
        album: tags.album.or_else(|| item.album.clone()),
        duration_ms: info.duration_ms.map(|d| d as f64),
        codec: info.codec.clone(),
        source: info.source,
        lossless: info.lossless,
        dsd: info.dsd,
        cover_key: item
            .cover_key
            .clone()
            .or_else(|| cover.as_ref().map(|c| cover_key(&c.data))),
    };
    (meta, cover)
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
