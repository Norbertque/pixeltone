//! AI playlists with the user's own Anthropic API key: Claude picks songs from the library
//! for a request, and the result is saved as an ordinary playlist.

use pp_ai::{Catalog, Claude, Song};
use pp_library::{Library, playlist_input};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::AppState;
use crate::library::blocking;
use crate::secret;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatus {
    pub has_key: bool,
    /// Songs in the library.
    pub songs: u32,
    /// Songs sent with a request (big libraries are sampled).
    pub sent_songs: u32,
    /// Rough price of one request, US dollars.
    pub estimate_usd: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPlaylistResult {
    pub playlist_id: u32,
    pub name: String,
    pub description: String,
    pub songs: u32,
}

fn catalog(library: &Library) -> pp_library::Result<(Catalog, usize)> {
    let songs: Vec<Song> = library
        .song_summaries()?
        .into_iter()
        .map(|s| Song {
            id: s.id,
            title: s.title,
            artist: s.artist,
            album: s.album,
            year: s.year,
            genres: s.genres,
        })
        .collect();
    Ok((Catalog::new(&songs), songs.len()))
}

#[tauri::command]
pub async fn ai_status(state: State<'_, AppState>) -> Result<AiStatus, String> {
    let has_key = secret::api_key()?.is_some();
    blocking(state.library.clone(), move |l| {
        let (catalog, songs) = catalog(l)?;
        Ok(AiStatus {
            has_key,
            songs: songs as u32,
            sent_songs: catalog.len() as u32,
            estimate_usd: catalog.estimate_usd(),
        })
    })
    .await
}

#[tauri::command]
pub fn ai_set_key(key: String) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() || key.chars().any(char::is_whitespace) {
        return Err("That doesn't look like an API key.".into());
    }
    secret::store_api_key(key)
}

#[tauri::command]
pub fn ai_remove_key() -> Result<(), String> {
    secret::delete_api_key()
}

/// Asks Claude for a playlist and saves it; returns what was made.
#[tauri::command]
pub async fn ai_playlist(
    app: AppHandle,
    state: State<'_, AppState>,
    prompt: String,
    songs: u32,
) -> Result<AiPlaylistResult, String> {
    let key =
        secret::api_key()?.ok_or("Add your Anthropic API key in Settings → AI playlists first.")?;
    if prompt.trim().is_empty() {
        return Err("Describe the playlist you'd like.".into());
    }
    let library = state.library.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let (catalog, _) = catalog(&library).map_err(|e| e.to_string())?;
        if catalog.is_empty() {
            return Err("Your library is empty: add a music folder first.".to_string());
        }
        let made = Claude::new(pp_ai::API, &key)
            .playlist(&catalog, &prompt, songs)
            .map_err(|e| e.to_string())?;
        if made.track_ids.is_empty() {
            return Err("No songs in your library fit that request.".to_string());
        }
        let rows = library
            .tracks_by_id(&made.track_ids)
            .map_err(|e| e.to_string())?;
        let items: Vec<_> = rows.iter().map(playlist_input).collect();
        let name = if made.name.is_empty() {
            "AI playlist".to_string()
        } else {
            made.name
        };
        let playlist_id = library
            .create_playlist(&name, &items)
            .map_err(|e| e.to_string())?;
        Ok(AiPlaylistResult {
            playlist_id,
            name,
            description: made.description,
            songs: items.len() as u32,
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit("library://changed", ());
    Ok(result)
}
