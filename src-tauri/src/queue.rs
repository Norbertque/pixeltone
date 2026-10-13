//! Queue commands, and the saved queue (`queue.json`) that lets a restart pick up where
//! playback left off: same queue, same track, same position, paused.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use pp_engine::{
    Engine, PlaybackStatus, PlayerSnapshot, QueueEntry, QueueItem, QueueSnapshot, RepeatMode,
};
use pp_library::TrackRow;
use tauri::{AppHandle, Manager, State};

use crate::AppState;
use crate::library::{blocking, cover_key};

/// A library track as a queue item.
pub fn item_from_row(row: TrackRow) -> QueueItem {
    QueueItem {
        path: PathBuf::from(row.path),
        cover_key: row.cover.as_deref().map(cover_key),
        title: Some(row.title),
        artist: (!row.artist.is_empty()).then_some(row.artist),
        album: (!row.album.is_empty()).then_some(row.album),
        duration_ms: Some(f64::from(row.duration_ms)),
        track_id: (row.id != 0).then_some(row.id),
    }
}

/// A file outside the library as a queue item, with what its tags say.
pub fn item_from_file(path: PathBuf) -> QueueItem {
    match pp_library::tags::read_track(&path, &[]) {
        Ok(t) => QueueItem {
            title: Some(t.title),
            artist: (!t.artist.is_empty()).then_some(t.artist),
            album: t.album,
            duration_ms: Some(t.duration_ms as f64),
            ..QueueItem::from(path)
        },
        Err(_) => QueueItem::from(path),
    }
}

/// Reads tags off the async runtime's threads.
pub async fn items_from_files(paths: Vec<String>) -> Result<Vec<QueueItem>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|p| item_from_file(PathBuf::from(p)))
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn queue_items(state: State<'_, AppState>, offset: usize, limit: usize) -> Vec<QueueEntry> {
    state.engine.queue_entries(offset, limit)
}

#[tauri::command]
pub fn queue_remove(state: State<'_, AppState>, ids: Vec<u32>) {
    state.engine.remove_from_queue(ids);
}

#[tauri::command]
pub fn queue_move(state: State<'_, AppState>, id: u32, to: usize) {
    state.engine.move_in_queue(id, to);
}

#[tauri::command]
pub fn queue_clear(state: State<'_, AppState>) {
    state.engine.clear_queue();
}

#[tauri::command]
pub fn queue_jump(state: State<'_, AppState>, id: u32) {
    state.engine.jump_to(id);
}

#[tauri::command]
pub fn player_set_shuffle(state: State<'_, AppState>, on: bool) {
    state.engine.set_shuffle(on);
}

/// "Play next" (`next`) or "Add to queue" for library tracks.
#[tauri::command]
pub async fn queue_add_tracks(
    state: State<'_, AppState>,
    ids: Vec<u32>,
    next: bool,
) -> Result<(), String> {
    let rows = blocking(state.library.clone(), move |l| l.tracks_by_id(&ids)).await?;
    state
        .engine
        .enqueue(rows.into_iter().map(item_from_row).collect(), next);
    Ok(())
}

/// "Play next" (`next`) or "Add to queue" for files.
#[tauri::command]
pub async fn queue_add_files(
    state: State<'_, AppState>,
    paths: Vec<String>,
    next: bool,
) -> Result<(), String> {
    let items = items_from_files(paths).await?;
    state.engine.enqueue(items, next);
    Ok(())
}

// ── Saving ──────────────────────────────────────────────────────────────────────────

/// While playing, the position is saved this often (besides every queue change).
const POSITION_SAVE_INTERVAL: Duration = Duration::from_secs(10);
/// Quiet time before a burst of changes is written.
const DEBOUNCE: Duration = Duration::from_millis(1500);

/// What makes the saved queue out of date (besides the position).
#[derive(Clone, Copy, PartialEq)]
struct SavedKey {
    version: u32,
    current: Option<u32>,
    repeat: RepeatMode,
    shuffle: bool,
    status: PlaybackStatus,
}

pub struct QueueStore {
    path: PathBuf,
    tx: Mutex<mpsc::Sender<()>>,
    last: Mutex<(Option<SavedKey>, Instant)>,
}

impl QueueStore {
    /// Starts the background writer; nothing is written until something changes.
    pub fn start(app: &AppHandle, path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel::<()>();
        let handle = app.clone();
        let file = path.clone();
        thread::Builder::new()
            .name("pp-queue-save".into())
            .spawn(move || {
                while rx.recv().is_ok() {
                    thread::sleep(DEBOUNCE);
                    while rx.try_recv().is_ok() {}
                    if let Some(state) = handle.try_state::<AppState>() {
                        save(&file, &state.engine);
                    }
                }
            })
            .expect("spawn queue writer");
        Self {
            path,
            tx: Mutex::new(tx),
            last: Mutex::new((None, Instant::now())),
        }
    }

    /// Called for every player state: schedules a save when the queue or track changed.
    pub fn on_state(&self, s: &PlayerSnapshot) {
        let key = SavedKey {
            version: s.queue_version,
            current: s.current_id,
            repeat: s.repeat,
            shuffle: s.shuffle,
            status: s.status,
        };
        let mut last = self.last.lock();
        if last.0 != Some(key) {
            *last = (Some(key), Instant::now());
            let _ = self.tx.lock().send(());
        }
    }

    /// Called with position updates: saves the position now and then.
    pub fn on_position(&self) {
        let mut last = self.last.lock();
        if last.1.elapsed() >= POSITION_SAVE_INTERVAL {
            last.1 = Instant::now();
            let _ = self.tx.lock().send(());
        }
    }

    /// Writes right away (on exit).
    pub fn save_now(&self, engine: &Engine) {
        save(&self.path, engine);
    }
}

/// The saved queue, if there is a usable one.
pub fn load(path: &Path) -> Option<QueueSnapshot> {
    let bytes = std::fs::read(path).ok()?;
    let snapshot: QueueSnapshot = serde_json::from_slice(&bytes)
        .inspect_err(|e| tracing::warn!("ignoring {}: {e}", path.display()))
        .ok()?;
    (!snapshot.items.is_empty()).then_some(snapshot)
}

fn save(path: &Path, engine: &Engine) {
    let snapshot = sanitize(engine.queue_snapshot());
    let result = serde_json::to_vec(&snapshot)
        .map_err(std::io::Error::other)
        .and_then(|bytes| {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, bytes)?;
            std::fs::rename(tmp, path)
        });
    if let Err(e) = result {
        tracing::warn!("can't save the queue: {e}");
    }
}

/// Drops items whose paths JSON can't hold (not valid Unicode), keeping indices right.
fn sanitize(mut s: QueueSnapshot) -> QueueSnapshot {
    if s.items.iter().all(|i| i.path.to_str().is_some()) {
        return s;
    }
    let mut new_index = Vec::with_capacity(s.items.len());
    let mut kept = 0u32;
    for item in &s.items {
        if item.path.to_str().is_some() {
            new_index.push(Some(kept));
            kept += 1;
        } else {
            new_index.push(None);
        }
    }
    let remap = |i: u32| new_index.get(i as usize).copied().flatten();
    s.items.retain(|i| i.path.to_str().is_some());
    s.current = s.current.and_then(remap);
    s.original = s
        .original
        .map(|order| order.into_iter().filter_map(remap).collect());
    if s.current.is_none() {
        s.position_ms = 0.0;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> QueueItem {
        QueueItem::from(PathBuf::from(name))
    }

    #[test]
    fn sanitize_keeps_valid_snapshots_as_they_are() {
        let s = QueueSnapshot {
            items: vec![item("a.flac"), item("b.flac")],
            original: Some(vec![1, 0]),
            current: Some(1),
            position_ms: 5.0,
            repeat: RepeatMode::All,
        };
        assert_eq!(sanitize(s.clone()), s);
    }

    #[cfg(unix)]
    #[test]
    fn sanitize_drops_paths_json_cannot_hold() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let bad = QueueItem::from(PathBuf::from(OsStr::from_bytes(b"\xff.flac")));
        let s = QueueSnapshot {
            items: vec![item("a.flac"), bad, item("c.flac")],
            original: Some(vec![2, 1, 0]),
            current: Some(2),
            position_ms: 5.0,
            repeat: RepeatMode::Off,
        };
        let clean = sanitize(s);
        assert_eq!(clean.items.len(), 2);
        assert_eq!(clean.current, Some(1));
        assert_eq!(clean.original, Some(vec![1, 0]));
        assert!(serde_json::to_vec(&clean).is_ok());
    }

    #[test]
    fn load_ignores_missing_or_broken_files() {
        let dir = std::env::temp_dir().join(format!("pp-queue-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("queue.json");
        assert!(load(&path).is_none());
        std::fs::write(&path, b"{ not json").unwrap();
        assert!(load(&path).is_none());
        let s = QueueSnapshot {
            items: vec![item("a.flac")],
            current: Some(0),
            position_ms: 12.5,
            ..QueueSnapshot::default()
        };
        std::fs::write(&path, serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(load(&path), Some(s));
        let _ = std::fs::remove_dir_all(dir);
    }
}
