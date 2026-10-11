//! Library commands and background scanning.

use std::path::PathBuf;
use std::sync::Arc;

use pp_engine::QueueItem;
use pp_library::{
    AlbumDetail, AlbumRow, AlbumSort, ArtistDetail, ArtistRow, FolderListing, GenreDetail,
    GenreRow, Library, LibraryFolder, LibraryOverview, ProgressFn, ScanPhase, SearchResults,
    TrackRow, TrackSort,
};

use tauri::{AppHandle, Emitter, State};

use crate::AppState;

/// Cover keys of library thumbnails, as the engine and the UI see them.
pub fn cover_key(hash: &str) -> String {
    format!("lib/{hash}")
}

async fn blocking<T: Send + 'static>(
    library: Arc<Library>,
    f: impl FnOnce(&Library) -> pp_library::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || f(&library))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Starts an incremental scan; progress goes to `library://scan`, completion to
/// `library://changed`.
pub fn scan(app: &AppHandle, library: &Arc<Library>) {
    let handle = app.clone();
    let progress: ProgressFn = Arc::new(move |p| {
        let _ = handle.emit("library://scan", p);
        if p.phase == ScanPhase::Done {
            let _ = handle.emit("library://changed", ());
        }
    });
    library.scan(progress);
}

/// Watches the library folders, rescanning when files change.
pub fn watch(app: &AppHandle, library: &Arc<Library>) {
    let handle = app.clone();
    let lib = library.clone();
    if let Err(e) = library.watch(move || scan(&handle, &lib)) {
        tracing::warn!("can't watch library folders: {e}");
    }
}

/// Opens the library at startup and catches up with changes made while the app was closed.
pub fn start(app: &AppHandle, library: &Arc<Library>) {
    if library.folders().is_ok_and(|f| !f.is_empty()) {
        scan(app, library);
        watch(app, library);
    }
}

#[tauri::command]
pub async fn library_overview(state: State<'_, AppState>) -> Result<LibraryOverview, String> {
    blocking(state.library.clone(), |l| l.overview()).await
}

#[tauri::command]
pub async fn library_folders(state: State<'_, AppState>) -> Result<Vec<LibraryFolder>, String> {
    blocking(state.library.clone(), |l| l.folders()).await
}

#[tauri::command]
pub async fn library_add_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let library = state.library.clone();
    blocking(library.clone(), move |l| l.add_folder(&PathBuf::from(path))).await?;
    scan(&app, &library);
    watch(&app, &library);
    Ok(())
}

#[tauri::command]
pub async fn library_remove_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u32,
) -> Result<(), String> {
    let library = state.library.clone();
    blocking(library.clone(), move |l| l.remove_folder(id)).await?;
    watch(&app, &library);
    let _ = app.emit("library://changed", ());
    Ok(())
}

#[tauri::command]
pub fn library_rescan(app: AppHandle, state: State<'_, AppState>) {
    scan(&app, &state.library);
}

#[tauri::command]
pub async fn library_tracks(
    state: State<'_, AppState>,
    sort: TrackSort,
    offset: u32,
    limit: u32,
) -> Result<Vec<TrackRow>, String> {
    blocking(state.library.clone(), move |l| {
        l.tracks(sort, offset, limit)
    })
    .await
}

#[tauri::command]
pub async fn library_albums(
    state: State<'_, AppState>,
    sort: AlbumSort,
    offset: u32,
    limit: u32,
) -> Result<Vec<AlbumRow>, String> {
    blocking(state.library.clone(), move |l| {
        l.albums(sort, offset, limit)
    })
    .await
}

#[tauri::command]
pub async fn library_album(
    state: State<'_, AppState>,
    id: u32,
) -> Result<Option<AlbumDetail>, String> {
    blocking(state.library.clone(), move |l| l.album(id)).await
}

#[tauri::command]
pub async fn library_artists(
    state: State<'_, AppState>,
    offset: u32,
    limit: u32,
) -> Result<Vec<ArtistRow>, String> {
    blocking(state.library.clone(), move |l| l.artists(offset, limit)).await
}

#[tauri::command]
pub async fn library_artist(
    state: State<'_, AppState>,
    id: u32,
) -> Result<Option<ArtistDetail>, String> {
    blocking(state.library.clone(), move |l| l.artist(id)).await
}

#[tauri::command]
pub async fn library_genres(state: State<'_, AppState>) -> Result<Vec<GenreRow>, String> {
    blocking(state.library.clone(), |l| l.genres()).await
}

#[tauri::command]
pub async fn library_genre(
    state: State<'_, AppState>,
    id: u32,
) -> Result<Option<GenreDetail>, String> {
    blocking(state.library.clone(), move |l| l.genre(id)).await
}

#[tauri::command]
pub async fn library_folder(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<FolderListing, String> {
    blocking(state.library.clone(), move |l| l.folder(path.as_deref())).await
}

#[tauri::command]
pub async fn library_search(
    state: State<'_, AppState>,
    query: String,
    limit: u32,
) -> Result<SearchResults, String> {
    blocking(state.library.clone(), move |l| l.search(&query, limit)).await
}

#[tauri::command]
pub fn library_delimiters(state: State<'_, AppState>) -> Vec<String> {
    state.library.delimiters()
}

#[tauri::command]
pub async fn library_set_delimiters(
    app: AppHandle,
    state: State<'_, AppState>,
    delimiters: Vec<String>,
) -> Result<(), String> {
    let library = state.library.clone();
    blocking(library.clone(), move |l| l.set_delimiters(delimiters)).await?;
    scan(&app, &library);
    Ok(())
}

/// Fisher–Yates with a time-seeded xorshift (no RNG dependency needed for a shuffle).
fn shuffle(ids: &mut [u32]) {
    let mut x = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        | 1;
    for i in (1..ids.len()).rev() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        ids.swap(i, (x % (i as u64 + 1)) as usize);
    }
}

/// Queues the whole library in `sort` order (or shuffled) and plays from `start`.
#[tauri::command]
pub async fn play_library(
    state: State<'_, AppState>,
    sort: TrackSort,
    start: usize,
    shuffled: bool,
) -> Result<(), String> {
    let mut ids = blocking(state.library.clone(), move |l| l.track_ids(sort)).await?;
    let start = if shuffled {
        shuffle(&mut ids);
        0
    } else {
        start
    };
    play_tracks(state, ids, start).await
}

/// Plays library tracks, starting at `start` within `ids`.
#[tauri::command]
pub async fn play_tracks(
    state: State<'_, AppState>,
    ids: Vec<u32>,
    start: usize,
) -> Result<(), String> {
    let requested = ids.get(start).copied();
    let rows = blocking(state.library.clone(), move |l| l.track_paths(&ids)).await?;
    // Unknown ids are skipped, so find the start track again in what came back.
    let start = requested
        .and_then(|id| rows.iter().position(|(row_id, _, _)| *row_id == id))
        .unwrap_or(0);
    let items = rows
        .into_iter()
        .map(|(_, path, cover)| QueueItem {
            path: PathBuf::from(path),
            cover_key: cover.as_deref().map(cover_key),
        })
        .collect();
    state.engine.play_items(items, start);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn shuffle_keeps_every_id() {
        let mut ids: Vec<u32> = (0..100).collect();
        super::shuffle(&mut ids);
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..100).collect::<Vec<_>>());
        assert_ne!(ids, sorted, "100 items virtually never stay in order");
    }
}
