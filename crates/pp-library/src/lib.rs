//! The music library: folders, incremental scanning, tags, artwork thumbnails and an SQLite
//! database with full-text search. No UI or Tauri dependencies.
//!
//! Queries run on their own connection while a background scan writes through another
//! (SQLite WAL), so browsing stays responsive during a scan.

pub mod artists;
pub mod artwork;
mod db;
pub mod edit;
pub mod m3u;
pub mod model;
pub mod scan;
pub mod tags;
mod watch;

use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use parking_lot::Mutex;

pub use artwork::ArtStore;
pub use model::*;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("library database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, LibraryError>;

pub type ProgressFn = Arc<dyn Fn(&ScanProgress) + Send + Sync>;

const DELIMITERS_KEY: &str = "artist_delimiters";

pub struct Library {
    db: Mutex<db::Db>,
    db_path: PathBuf,
    art: ArtStore,
    scanning: AtomicBool,
    /// A scan was requested while one was running: run again when it finishes.
    rescan: AtomicBool,
    cancel: AtomicBool,
    watcher: Mutex<Option<watch::FolderWatcher>>,
}

impl Library {
    pub fn open(db_path: impl Into<PathBuf>, art_dir: impl Into<PathBuf>) -> Result<Arc<Self>> {
        let db_path = db_path.into();
        if let Some(dir) = db_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Ok(Arc::new(Self {
            db: Mutex::new(db::Db::open(&db_path)?),
            db_path,
            art: ArtStore::new(art_dir)?,
            scanning: AtomicBool::new(false),
            rescan: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            watcher: Mutex::new(None),
        }))
    }

    pub fn art(&self) -> &ArtStore {
        &self.art
    }

    // ── Settings ────────────────────────────────────────────────────────────────

    pub fn delimiters(&self) -> Vec<String> {
        self.db
            .lock()
            .setting(DELIMITERS_KEY)
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_else(artists::default_delimiters)
    }

    /// Changes the artist delimiters. Existing tracks are re-read on the next scan.
    pub fn set_delimiters(&self, delimiters: Vec<String>) -> Result<()> {
        let db = self.db.lock();
        let json =
            serde_json::to_string(&delimiters).map_err(|e| LibraryError::Other(e.to_string()))?;
        db.set_setting(DELIMITERS_KEY, &json)?;
        db.mark_all_stale()?;
        Ok(())
    }

    // ── Folders ─────────────────────────────────────────────────────────────────

    pub fn folders(&self) -> Result<Vec<LibraryFolder>> {
        Ok(self.db.lock().folders()?)
    }

    pub fn add_folder(&self, path: &Path) -> Result<()> {
        let path = path.to_string_lossy();
        let trimmed = path.trim_end_matches(MAIN_SEPARATOR);
        // Keep a drive root like "D:\" intact.
        let path = if trimmed.ends_with(':') {
            path.as_ref()
        } else {
            trimmed
        };
        if !Path::new(path).is_dir() {
            return Err(LibraryError::Other(format!("not a folder: {path}")));
        }
        self.db.lock().add_folder(path, scan::now_secs())?;
        Ok(())
    }

    pub fn remove_folder(&self, id: u32) -> Result<()> {
        Ok(self.db.lock().remove_folder(id)?)
    }

    // ── Scanning ────────────────────────────────────────────────────────────────

    pub fn is_scanning(&self) -> bool {
        self.scanning.load(Ordering::Acquire)
    }

    /// Scans in the background. While a scan runs, another request schedules one more pass
    /// after it instead of starting a second scan.
    pub fn scan(self: &Arc<Self>, on_progress: ProgressFn) {
        if self.scanning.swap(true, Ordering::AcqRel) {
            self.rescan.store(true, Ordering::Release);
            return;
        }
        self.cancel.store(false, Ordering::Release);
        let lib = self.clone();
        thread::Builder::new()
            .name("pp-library-scan".into())
            .spawn(move || {
                loop {
                    lib.rescan.store(false, Ordering::Release);
                    if let Err(e) = lib.scan_once(&on_progress) {
                        tracing::warn!("library scan failed: {e}");
                    }
                    if !lib.rescan.load(Ordering::Acquire) || lib.cancel.load(Ordering::Acquire) {
                        break;
                    }
                }
                lib.scanning.store(false, Ordering::Release);
            })
            .expect("spawn library scan thread");
    }

    /// Runs one scan on the calling thread.
    pub fn scan_once(&self, on_progress: &ProgressFn) -> Result<ScanProgress> {
        let delimiters = self.delimiters();
        let mut db = db::Db::open(&self.db_path)?;
        let mut report = |p: &ScanProgress| on_progress(p);
        Ok(scan::scan(
            &mut db,
            &self.art,
            &delimiters,
            &self.cancel,
            &mut report,
        )?)
    }

    pub fn cancel_scan(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    /// (Re)starts watching the library folders; `on_change` runs after changes settle.
    pub fn watch(&self, on_change: impl Fn() + Send + 'static) -> Result<()> {
        let roots: Vec<PathBuf> = self
            .folders()?
            .into_iter()
            .map(|f| PathBuf::from(f.path))
            .collect();
        let watcher = watch::FolderWatcher::new(&roots, on_change).map_err(LibraryError::Other)?;
        *self.watcher.lock() = Some(watcher);
        Ok(())
    }

    // ── Queries ─────────────────────────────────────────────────────────────────

    pub fn overview(&self) -> Result<LibraryOverview> {
        Ok(self.db.lock().overview(self.is_scanning())?)
    }

    pub fn tracks(&self, sort: TrackSort, offset: u32, limit: u32) -> Result<Vec<TrackRow>> {
        Ok(self.db.lock().tracks(sort, offset, limit)?)
    }

    pub fn track_ids(&self, sort: TrackSort) -> Result<Vec<u32>> {
        Ok(self.db.lock().track_ids(sort)?)
    }

    pub fn albums(&self, sort: AlbumSort, offset: u32, limit: u32) -> Result<Vec<AlbumRow>> {
        Ok(self.db.lock().albums(sort, offset, limit)?)
    }

    pub fn album(&self, id: u32) -> Result<Option<AlbumDetail>> {
        Ok(self.db.lock().album(id)?)
    }

    pub fn artists(&self, offset: u32, limit: u32) -> Result<Vec<ArtistRow>> {
        Ok(self.db.lock().artists(offset, limit)?)
    }

    pub fn artist(&self, id: u32) -> Result<Option<ArtistDetail>> {
        Ok(self.db.lock().artist(id)?)
    }

    pub fn genres(&self) -> Result<Vec<GenreRow>> {
        Ok(self.db.lock().genres()?)
    }

    pub fn genre(&self, id: u32) -> Result<Option<GenreDetail>> {
        Ok(self.db.lock().genre(id)?)
    }

    pub fn folder(&self, path: Option<&str>) -> Result<FolderListing> {
        Ok(self.db.lock().folder(path)?)
    }

    pub fn search(&self, query: &str, limit: u32) -> Result<SearchResults> {
        Ok(self.db.lock().search(query, limit)?)
    }

    /// (id, path, cover hash) for playback, in the order given.
    /// Tracks by id, in the order asked for; unknown ids are skipped.
    pub fn tracks_by_id(&self, ids: &[u32]) -> Result<Vec<TrackRow>> {
        Ok(self.db.lock().tracks_by_id(ids)?)
    }

    // ── Playlists ───────────────────────────────────────────────────────────────

    pub fn playlists(&self) -> Result<Vec<PlaylistRow>> {
        Ok(self.db.lock().playlists()?)
    }

    pub fn playlist(&self, id: u32) -> Result<Option<PlaylistDetail>> {
        Ok(self.db.lock().playlist(id)?)
    }

    pub fn playlist_items(&self, id: u32) -> Result<Vec<PlaylistInput>> {
        Ok(self.db.lock().playlist_items(id)?)
    }

    /// Creates a playlist holding `items`; returns its id.
    pub fn create_playlist(&self, name: &str, items: &[PlaylistInput]) -> Result<u32> {
        let name = playlist_name(name)?;
        let mut db = self.db.lock();
        let now = scan::now_secs();
        let id = db.create_playlist(&name, now)?;
        if !items.is_empty() {
            db.playlist_add(id, items, now)?;
        }
        Ok(id)
    }

    pub fn rename_playlist(&self, id: u32, name: &str) -> Result<()> {
        let name = playlist_name(name)?;
        Ok(self
            .db
            .lock()
            .rename_playlist(id, &name, scan::now_secs())?)
    }

    pub fn delete_playlist(&self, id: u32) -> Result<()> {
        Ok(self.db.lock().delete_playlist(id)?)
    }

    pub fn playlist_add(&self, id: u32, items: &[PlaylistInput]) -> Result<()> {
        Ok(self.db.lock().playlist_add(id, items, scan::now_secs())?)
    }

    /// Appends library tracks.
    pub fn playlist_add_tracks(&self, id: u32, track_ids: &[u32]) -> Result<()> {
        let rows = self.tracks_by_id(track_ids)?;
        self.playlist_add(id, &rows.iter().map(playlist_input).collect::<Vec<_>>())
    }

    pub fn playlist_remove(&self, id: u32, positions: &[u32]) -> Result<()> {
        Ok(self
            .db
            .lock()
            .playlist_remove(id, positions, scan::now_secs())?)
    }

    pub fn playlist_move(&self, id: u32, from: u32, to: u32) -> Result<()> {
        Ok(self
            .db
            .lock()
            .playlist_move(id, from, to, scan::now_secs())?)
    }

    /// Imports an M3U/M3U8 file as a new playlist named after it.
    pub fn import_m3u(&self, file: &Path) -> Result<u32> {
        let (name, items) = m3u::read(file)?;
        if items.is_empty() {
            return Err(LibraryError::Other(format!(
                "{} lists no files that could be imported",
                file.display()
            )));
        }
        self.create_playlist(&name, &items)
    }

    pub fn export_m3u(&self, id: u32, file: &Path) -> Result<()> {
        let detail = self
            .playlist(id)?
            .ok_or_else(|| LibraryError::Other("the playlist no longer exists".into()))?;
        Ok(m3u::write(file, &detail.tracks)?)
    }

    // ── Favourites ──────────────────────────────────────────────────────────────

    pub fn set_favorite(&self, path: &str, on: bool) -> Result<()> {
        Ok(self.db.lock().set_favorite(path, on, scan::now_secs())?)
    }

    pub fn is_favorite(&self, path: &str) -> Result<bool> {
        Ok(self.db.lock().is_favorite(path)?)
    }

    pub fn favorites(&self) -> Result<Vec<TrackRow>> {
        Ok(self.db.lock().favorites()?)
    }

    // ── Play history ────────────────────────────────────────────────────────────

    pub fn record_play(&self, play: &PlayRecord) -> Result<()> {
        Ok(self.db.lock().record_play(play)?)
    }

    /// Listening over the last `days` days (or everything).
    pub fn stats(&self, days: Option<u32>) -> Result<ListeningStats> {
        let since = days.map_or(0, |d| scan::now_secs() - i64::from(d) * 86_400);
        Ok(self.db.lock().stats(since, days, 20)?)
    }

    /// What Home shows besides the library counts.
    pub fn home(&self) -> Result<HomeFeed> {
        let db = self.db.lock();
        let now = scan::now_secs();
        Ok(HomeFeed {
            recent_tracks: db.recent_tracks(12)?,
            recent_albums: db.recent_albums(12)?,
            top_artists: db.stats(now - 30 * 86_400, Some(30), 8)?.top_artists,
            daily_mix: db.daily_mix(db.local_day()?, now, 40)?,
        })
    }

    /// Today's mix, in full.
    pub fn daily_mix(&self) -> Result<Vec<TrackRow>> {
        let db = self.db.lock();
        Ok(db.daily_mix(db.local_day()?, scan::now_secs(), 40)?)
    }
}

fn playlist_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(LibraryError::Other("a playlist needs a name".into()));
    }
    Ok(name.chars().take(200).collect())
}

/// A track in brief, for describing the library to an AI.
#[derive(Clone, Debug, PartialEq)]
pub struct SongSummary {
    pub id: u32,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i32>,
    /// "Breakcore; Electronic"
    pub genres: String,
}

impl Library {
    /// Every track in brief, by artist and album.
    pub fn song_summaries(&self) -> Result<Vec<SongSummary>> {
        Ok(self.db.lock().song_summaries()?)
    }
}

// ── Artist pictures (found online by the app) ────────────────────────────────────

impl Library {
    /// (name, key) of artists still without a picture: never looked up, or last found
    /// nothing before `retry_before` (Unix seconds). Most tracks first.
    pub fn artists_without_images(
        &self,
        retry_before: i64,
        limit: u32,
    ) -> Result<Vec<(String, String)>> {
        Ok(self.db.lock().artists_without_images(retry_before, limit)?)
    }

    /// Records a lookup for the artist `key`: the picture's bytes, or `None` for "nothing
    /// found". The picture becomes the artist's cover everywhere.
    pub fn set_artist_image(&self, key: &str, image: Option<&[u8]>) -> Result<()> {
        let hash = match image {
            Some(bytes) => Some(self.art.store(bytes).map_err(LibraryError::Other)?),
            None => None,
        };
        Ok(self
            .db
            .lock()
            .set_artist_image(key, hash.as_deref(), scan::now_secs())?)
    }

    /// Album and song titles by the artist `key` (up to `n` of each), to check an online
    /// match against: see `Db::artist_titles`.
    pub fn artist_titles(&self, key: &str, n: u32) -> Result<(Vec<String>, Vec<String>)> {
        Ok(self.db.lock().artist_titles(key, n)?)
    }

    /// Drops every picture found online; artists show album covers again.
    pub fn forget_artist_images(&self) -> Result<()> {
        Ok(self.db.lock().forget_artist_images()?)
    }
}

/// A library track as a playlist entry.
pub fn playlist_input(t: &TrackRow) -> PlaylistInput {
    PlaylistInput {
        path: t.path.clone(),
        title: Some(t.title.clone()),
        artist: (!t.artist.is_empty()).then(|| t.artist.clone()),
        duration_ms: Some(t.duration_ms),
    }
}
