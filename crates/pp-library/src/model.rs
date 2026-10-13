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
    /// In the user's favourites.
    pub favorite: bool,
}

impl TrackRow {
    /// A file outside the library (from a playlist or play history), shown with what's known.
    pub fn outside(
        path: &str,
        title: Option<&str>,
        artist: Option<&str>,
        duration_ms: u32,
    ) -> Self {
        let stem = std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string());
        Self {
            id: 0,
            title: title.filter(|t| !t.is_empty()).map_or(stem, str::to_string),
            artist: artist.unwrap_or_default().to_string(),
            album: String::new(),
            album_id: 0,
            track_no: None,
            disc_no: None,
            year: None,
            duration_ms,
            codec: String::new(),
            sample_rate: 0,
            bits: 0,
            lossless: false,
            dsd: false,
            cover: None,
            path: path.to_string(),
            favorite: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaylistRow {
    pub id: u32,
    pub name: String,
    pub track_count: u32,
    pub duration_ms: f64,
    /// Up to four cover hashes for a mosaic.
    pub covers: Vec<String>,
    /// Unix seconds.
    pub updated_at: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaylistDetail {
    pub playlist: PlaylistRow,
    /// In playlist order; files outside the library have id 0.
    pub tracks: Vec<TrackRow>,
}

/// A file to put in a playlist, with what to show if it isn't in the library.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PlaylistInput {
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration_ms: Option<u32>,
}

/// One finished listen, recorded by the app.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayRecord {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Unix seconds.
    pub played_at: i64,
    pub ms_played: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TopTrack {
    pub track: TrackRow,
    pub plays: u32,
    pub ms: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TopArtist {
    pub artist: ArtistRow,
    pub plays: u32,
    pub ms: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TopAlbum {
    pub album: AlbumRow,
    pub plays: u32,
    pub ms: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayListening {
    /// Local date, YYYY-MM-DD.
    pub day: String,
    pub ms: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListeningStats {
    /// The period covered: the last `days` days, or everything when `None`.
    pub days: Option<u32>,
    pub total_ms: f64,
    pub plays: u32,
    pub distinct_tracks: u32,
    pub top_tracks: Vec<TopTrack>,
    pub top_artists: Vec<TopArtist>,
    pub top_albums: Vec<TopAlbum>,
    /// Listening per local day, oldest first (days without listening left out).
    pub per_day: Vec<DayListening>,
    /// Listening per local hour of the day, 24 entries.
    pub per_hour: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HomeFeed {
    pub recent_tracks: Vec<TrackRow>,
    pub recent_albums: Vec<AlbumRow>,
    pub top_artists: Vec<TopArtist>,
    /// Today's mix (changes once a day).
    pub daily_mix: Vec<TrackRow>,
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
