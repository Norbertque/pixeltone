//! Rows and views returned to the UI (exported to TypeScript by ts-rs).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TrackRow {
    pub id: u32,
    pub title: String,
    /// Artist as tagged ("" when untagged).
    pub artist: String,
    pub album: String,
    pub album_id: u32,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub duration_ms: u32,
    pub codec: String,
    pub sample_rate: u32,
    /// Bit depth of lossless material, 0 otherwise.
    pub bits: u32,
    pub lossless: bool,
    pub dsd: bool,
    /// Cover hash (track's own, else the album's).
    pub cover: Option<String>,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AlbumRow {
    pub id: u32,
    pub title: String,
    pub artist: String,
    pub year: Option<i32>,
    pub track_count: u32,
    pub duration_ms: f64,
    pub cover: Option<String>,
    /// Unix seconds.
    pub added_at: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtistRow {
    pub id: u32,
    pub name: String,
    pub album_count: u32,
    pub track_count: u32,
    pub cover: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GenreRow {
    pub id: u32,
    pub name: String,
    pub track_count: u32,
    pub cover: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AlbumDetail {
    pub album: AlbumRow,
    pub tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtistDetail {
    pub artist: ArtistRow,
    pub albums: Vec<AlbumRow>,
    pub tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GenreDetail {
    pub genre: GenreRow,
    pub albums: Vec<AlbumRow>,
    pub tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FolderEntry {
    pub path: String,
    pub name: String,
    /// Tracks in this folder and below.
    pub track_count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FolderListing {
    /// `None` at the top level, which lists the library folders.
    pub path: Option<String>,
    pub name: String,
    pub parent: Option<String>,
    pub folders: Vec<FolderEntry>,
    pub tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchResults {
    pub tracks: Vec<TrackRow>,
    pub albums: Vec<AlbumRow>,
    pub artists: Vec<ArtistRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryFolder {
    pub id: u32,
    pub path: String,
    pub track_count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryOverview {
    pub track_count: u32,
    pub album_count: u32,
    pub artist_count: u32,
    pub genre_count: u32,
    pub duration_ms: f64,
    pub folders: Vec<LibraryFolder>,
    pub recently_added: Vec<AlbumRow>,
    pub scanning: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TrackSort {
    #[default]
    Title,
    Artist,
    Album,
    Year,
    Added,
    Duration,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AlbumSort {
    #[default]
    Title,
    Artist,
    Year,
    Added,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ScanPhase {
    Walking,
    Reading,
    Finishing,
    Done,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScanProgress {
    pub phase: ScanPhase,
    /// Files read so far / to read in this scan (only changed files are read).
    pub done: u32,
    pub total: u32,
    pub added: u32,
    pub updated: u32,
    pub removed: u32,
    /// Files that couldn't be read.
    pub errors: u32,
}
