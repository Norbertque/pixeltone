//! The music library: folders, incremental scanning, tags, artwork thumbnails and an SQLite
//! database with full-text search. No UI or Tauri dependencies.
//!
//! Queries run on their own connection while a background scan writes through another
//! (SQLite WAL), so browsing stays responsive during a scan.

pub mod artists;
pub mod artwork;
mod db;
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
}
