//! Playlists, favourites, listening stats, Home, and lyrics.

use std::path::PathBuf;

use pp_engine::QueueItem;
use pp_library::{HomeFeed, ListeningStats, PlaylistDetail, PlaylistRow, TrackRow};
use pp_lyrics::{Lyrics, LyricsQuery};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

use crate::AppState;
use crate::library::blocking;
use crate::queue::item_from_row;

/// Open views refetch on this (track lists carry the favourite flag).
fn changed(app: &AppHandle) {
    let _ = app.emit("library://changed", ());
}

/// A playlist entry as a queue item: library tracks with their cover, other files as they are.
fn item(row: TrackRow) -> QueueItem {
    if row.id != 0 {
        return item_from_row(row);
    }
    QueueItem {
        path: PathBuf::from(&row.path),
        title: Some(row.title),
        artist: (!row.artist.is_empty()).then_some(row.artist),
        duration_ms: (row.duration_ms > 0).then_some(f64::from(row.duration_ms)),
        ..QueueItem::default()
    }
}

// ── Playlists ───────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn library_playlists(state: State<'_, AppState>) -> Result<Vec<PlaylistRow>, String> {
    blocking(state.library.clone(), |l| l.playlists()).await
}

#[tauri::command]
pub async fn library_playlist(
    state: State<'_, AppState>,
    id: u32,
) -> Result<Option<PlaylistDetail>, String> {
    blocking(state.library.clone(), move |l| l.playlist(id)).await
}

#[tauri::command]
pub async fn playlist_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    track_ids: Vec<u32>,
) -> Result<u32, String> {
    let id = blocking(state.library.clone(), move |l| {
        let id = l.create_playlist(&name, &[])?;
        l.playlist_add_tracks(id, &track_ids)?;
        Ok(id)
    })
    .await?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub async fn playlist_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u32,
    name: String,
) -> Result<(), String> {
    blocking(state.library.clone(), move |l| l.rename_playlist(id, &name)).await?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn playlist_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u32,
) -> Result<(), String> {
    blocking(state.library.clone(), move |l| l.delete_playlist(id)).await?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn playlist_add_tracks(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u32,
    track_ids: Vec<u32>,
) -> Result<(), String> {
    blocking(state.library.clone(), move |l| {
        l.playlist_add_tracks(id, &track_ids)
    })
    .await?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn playlist_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u32,
    positions: Vec<u32>,
) -> Result<(), String> {
    blocking(state.library.clone(), move |l| {
        l.playlist_remove(id, &positions)
    })
    .await?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn playlist_move(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u32,
    from: u32,
    to: u32,
) -> Result<(), String> {
    blocking(state.library.clone(), move |l| {
        l.playlist_move(id, from, to)
    })
    .await?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn playlist_import(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<u32, String> {
    let id = blocking(state.library.clone(), move |l| {
        l.import_m3u(&PathBuf::from(path))
    })
    .await?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub async fn playlist_export(
    state: State<'_, AppState>,
    id: u32,
    path: String,
) -> Result<(), String> {
    blocking(state.library.clone(), move |l| {
        l.export_m3u(id, &PathBuf::from(path))
    })
    .await
}

/// Plays a playlist from `start` (or shuffled); `shuffle` as in `play_tracks`.
#[tauri::command]
pub async fn play_playlist(
    state: State<'_, AppState>,
    id: u32,
    start: Option<usize>,
    shuffle: Option<bool>,
) -> Result<(), String> {
    let detail = blocking(state.library.clone(), move |l| l.playlist(id))
        .await?
        .ok_or("the playlist no longer exists")?;
    let items = detail.tracks.into_iter().map(item).collect();
    state.engine.play_items(items, start, shuffle);
    Ok(())
}

// ── Favourites ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn library_set_favorite(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    on: bool,
) -> Result<(), String> {
    blocking(state.library.clone(), move |l| l.set_favorite(&path, on)).await?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn library_is_favorite(state: State<'_, AppState>, path: String) -> Result<bool, String> {
    blocking(state.library.clone(), move |l| l.is_favorite(&path)).await
}

#[tauri::command]
pub async fn library_favorites(state: State<'_, AppState>) -> Result<Vec<TrackRow>, String> {
    blocking(state.library.clone(), |l| l.favorites()).await
}

// ── History ─────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn library_stats(
    state: State<'_, AppState>,
    days: Option<u32>,
) -> Result<ListeningStats, String> {
    blocking(state.library.clone(), move |l| l.stats(days)).await
}

#[tauri::command]
pub async fn library_home(state: State<'_, AppState>) -> Result<HomeFeed, String> {
    blocking(state.library.clone(), |l| l.home()).await
}

/// Plays today's mix from track `start`.
#[tauri::command]
pub async fn play_daily_mix(
    state: State<'_, AppState>,
    start: Option<usize>,
) -> Result<(), String> {
    let rows = blocking(state.library.clone(), |l| l.daily_mix()).await?;
    let items = rows.into_iter().map(item).collect();
    state
        .engine
        .play_items(items, Some(start.unwrap_or(0)), Some(false));
    Ok(())
}

// ── Lyrics ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsRequest {
    path: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: Option<f64>,
}

impl From<LyricsRequest> for LyricsQuery {
    fn from(r: LyricsRequest) -> Self {
        Self {
            path: PathBuf::from(r.path),
            title: r.title,
            artist: r.artist.unwrap_or_default(),
            album: r.album.unwrap_or_default(),
            duration_ms: r.duration_ms.map(|d| d.max(0.0) as u32),
        }
    }
}

/// Lyrics for a track (may look online; takes up to a few seconds).
#[tauri::command]
pub async fn lyrics_find(
    state: State<'_, AppState>,
    track: LyricsRequest,
) -> Result<Option<Lyrics>, String> {
    let lyrics = state.lyrics.clone();
    tauri::async_runtime::spawn_blocking(move || lyrics.find(&track.into()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lyrics_save(
    state: State<'_, AppState>,
    track: LyricsRequest,
    text: String,
) -> Result<Lyrics, String> {
    let lyrics = state.lyrics.clone();
    tauri::async_runtime::spawn_blocking(move || lyrics.save(&track.into(), &text))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
