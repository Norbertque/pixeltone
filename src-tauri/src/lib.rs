//! PixelPlayer PC desktop app: wires the Rust core to the Svelte UI through Tauri.

mod ai;
mod artist_images;
mod commands;
mod history;
mod library;
#[cfg(windows)]
mod media;
#[cfg(windows)]
mod power;
mod queue;
mod secret;
mod settings;
mod tags;
mod tray;
mod userdata;
#[cfg(windows)]
mod window_theme;

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use pp_core::{DeviceWatch, OutputBackend};
use pp_engine::{Engine, EngineEvent};
use pp_library::Library;
use pp_lyrics::LyricsService;
use serde::Serialize;
use tauri::http::Response;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_window_state::StateFlags;
use tracing_subscriber::EnvFilter;

use crate::artist_images::ArtistImages;
use crate::history::History;
use crate::queue::QueueStore;
use crate::settings::AppSettings;

pub struct AppState {
    pub engine: Engine,
    pub backend: Arc<dyn OutputBackend>,
    pub library: Arc<Library>,
    pub settings: Mutex<AppSettings>,
    pub settings_path: PathBuf,
    pub queue_store: QueueStore,
    pub history: History,
    pub lyrics: Arc<LyricsService>,
    pub artist_images: ArtistImages,
    /// The notification-area icon; `None` where the system has no tray.
    pub tray: Option<tray::Tray>,
    /// Device notifications stay on while this is held.
    pub device_watch: Mutex<Option<DeviceWatch>>,
    #[cfg(windows)]
    pub media: Mutex<Option<media::Media>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionEvent {
    position_ms: f64,
}

#[derive(Clone, Serialize)]
struct ErrorEvent {
    code: String,
    message: String,
}

fn output_backend() -> Arc<dyn OutputBackend> {
    #[cfg(windows)]
    {
        Arc::new(pp_output_wasapi::WasapiBackend::new())
    }
    #[cfg(not(windows))]
    {
        // No audio output off Windows: a capture device keeps the app usable for development.
        Arc::new(pp_engine::testing::CaptureBackend::new(
            pp_engine::testing::CaptureBackend::typical_dac(),
        ))
    }
}

/// `%2F` → `/` etc.: protocol URLs arrive percent-encoded.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |b: u8| char::from(b).to_digit(16);
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn image_response(mime: &str, data: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .header("Content-Type", mime)
        .header("Cache-Control", "max-age=31536000, immutable")
        // The UI reads cover pixels on a canvas to pick the theme colour.
        .header("Access-Control-Allow-Origin", "*")
        .body(data)
        .unwrap_or_default()
}

/// `cover://localhost/lib/<hash>-<size>` (library thumbnails) or `cover://localhost/<key>`
/// (embedded covers the engine holds for files played outside the library).
fn cover(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let key = percent_decode(path.trim_start_matches('/'));
    let Some(state) = app.try_state::<AppState>() else {
        return Response::builder()
            .status(503)
            .body(Vec::new())
            .unwrap_or_default();
    };
    if let Some(rest) = key.strip_prefix("lib/") {
        let (hash, size) = rest.rsplit_once('-').unwrap_or((rest, "300"));
        let size = size.parse().unwrap_or(300);
        if let Some(data) = state
            .library
            .art()
            .existing(hash, size)
            .and_then(|p| std::fs::read(p).ok())
        {
            return image_response("image/jpeg", data);
        }
    } else if let Some(art) = state.engine.covers().get(&key) {
        return image_response(art.mime, art.data);
    }
    Response::builder()
        .status(404)
        .body(Vec::new())
        .unwrap_or_default()
}

pub(crate) fn emit_settings(app: &AppHandle, settings: &AppSettings) {
    let _ = app.emit("settings://changed", settings);
}

fn forward(app: &AppHandle, event: EngineEvent) {
    let state = app.try_state::<AppState>();
    match event {
        EngineEvent::State(snapshot) => {
            #[cfg(windows)]
            power::keep_awake(snapshot.status == pp_engine::PlaybackStatus::Playing);
            if let Some(state) = &state {
                state.queue_store.on_state(&snapshot);
                state.history.on_state(&snapshot);
                if let Some(tray) = &state.tray {
                    tray.update(&snapshot);
                }
                #[cfg(windows)]
                if let Some(m) = &*state.media.lock() {
                    m.send(media::Update::State(snapshot.clone()));
                }
            }
            let _ = app.emit("player://state", snapshot);
        }
        EngineEvent::Position { position_ms } => {
            if let Some(state) = &state {
                state.queue_store.on_position();
                state.history.on_position(position_ms);
                #[cfg(windows)]
                if let Some(m) = &*state.media.lock() {
                    m.send(media::Update::Position(position_ms));
                }
            }
            let _ = app.emit("player://position", PositionEvent { position_ms });
        }
        EngineEvent::Error { code, message } => {
            let _ = app.emit("player://error", ErrorEvent { code, message });
        }
    }
}

/// Tells the engine and the UI when audio devices come, go or change roles.
fn watch_devices(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let handle = app.clone();
    let watch = state.backend.watch_devices(Box::new(move |change| {
        if let Some(state) = handle.try_state::<AppState>() {
            state.engine.device_changed(change.clone());
        }
        let _ = handle.emit("output://devices", &change);
    }));
    match watch {
        Ok(watch) => *state.device_watch.lock() = watch,
        Err(e) => tracing::warn!("can't watch audio devices: {e}"),
    }
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("PP_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info,pixelplayer_pc_lib=debug")),
        )
        .init();

    tauri::Builder::default()
        // A second launch focuses the running window instead of opening another player.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::reveal_main(app);
        }))
        // Positions and sizes only: a window hidden at exit (mini player, tray) must still
        // open visibly next time, and the mini player keeps its own frameless look.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() - StateFlags::VISIBLE - StateFlags::DECORATIONS)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("cover", |ctx, request| {
            cover(ctx.app_handle(), request.uri().path())
        })
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = AppSettings::load(&settings_path);
            let library = Library::open(
                app.path().app_local_data_dir()?.join("library.db"),
                app.path().app_cache_dir()?.join("art"),
            )?;
            let backend = output_backend();
            let handle = app.handle().clone();
            let engine = Engine::new(backend.clone(), settings.effective_output(), move |event| {
                forward(&handle, event)
            });
            let queue_path = app.path().app_local_data_dir()?.join("queue.json");
            if let Some(saved) = queue::load(&queue_path) {
                engine.restore(saved);
            }
            let queue_store = QueueStore::start(app.handle(), queue_path);
            library::start(app.handle(), &library);
            let lyrics = Arc::new(LyricsService::new(
                app.path().app_local_data_dir()?.join("lyrics"),
                app.path().app_cache_dir()?.join("lyrics"),
            ));
            lyrics.set_online(settings.preferences.lyrics_online);
            let handle = app.handle().clone();
            let artist_images = ArtistImages::start(
                library.clone(),
                settings.preferences.artist_images_online,
                move || {
                    let _ = handle.emit("library://changed", ());
                },
            );
            let tray = tray::Tray::create(app.handle())
                .inspect_err(|e| tracing::warn!("no tray icon: {e}"))
                .ok();
            app.manage(AppState {
                engine,
                backend,
                history: History::new(library.clone()),
                library,
                lyrics,
                artist_images,
                tray,
                settings: Mutex::new(settings),
                settings_path,
                queue_store,
                device_watch: Mutex::new(None),
                #[cfg(windows)]
                media: Mutex::new(None),
            });
            watch_devices(app.handle());
            #[cfg(windows)]
            {
                let state = app.state::<AppState>();
                if let Some(m) = media::start(app.handle()) {
                    m.send(media::Update::State(Box::new(state.engine.snapshot())));
                    *state.media.lock() = Some(m);
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // With "keep playing in the tray", closing the main window only hides it.
            WindowEvent::CloseRequested { api, .. }
                if window.label() == "main"
                    && window
                        .app_handle()
                        .try_state::<AppState>()
                        .is_some_and(|s| s.settings.lock().preferences.close_to_tray) =>
            {
                api.prevent_close();
                let _ = window.hide();
            }
            // The mini player closed some other way (Alt+F4): the main window comes back.
            WindowEvent::Destroyed if window.label() == tray::MINI => {
                tray::reveal_main(window.app_handle());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::set_window_theme,
            commands::player_state,
            commands::player_play_files,
            commands::player_action,
            commands::player_seek,
            commands::player_set_repeat,
            commands::output_devices,
            commands::output_capabilities,
            commands::get_settings,
            commands::set_output_settings,
            commands::hardware_volume,
            commands::set_hardware_volume,
            commands::set_hardware_mute,
            tags::tags_read,
            tags::tags_write,
            tray::open_mini_player,
            tray::show_main_window,
            ai::ai_status,
            ai::ai_set_key,
            ai::ai_remove_key,
            ai::ai_playlist,
            library::library_overview,
            library::library_folders,
            library::library_add_folder,
            library::library_remove_folder,
            library::library_rescan,
            library::library_tracks,
            library::library_albums,
            library::library_album,
            library::library_artists,
            library::library_artist,
            library::library_genres,
            library::library_genre,
            library::library_folder,
            library::library_search,
            library::library_delimiters,
            library::library_set_delimiters,
            library::play_tracks,
            library::play_library,
            queue::queue_items,
            queue::queue_remove,
            queue::queue_move,
            queue::queue_clear,
            queue::queue_jump,
            queue::queue_add_tracks,
            queue::queue_add_files,
            queue::player_set_shuffle,
            commands::set_preferences,
            userdata::library_playlists,
            userdata::library_playlist,
            userdata::playlist_create,
            userdata::playlist_rename,
            userdata::playlist_delete,
            userdata::playlist_add_tracks,
            userdata::playlist_remove,
            userdata::playlist_move,
            userdata::playlist_import,
            userdata::playlist_export,
            userdata::play_playlist,
            userdata::library_set_favorite,
            userdata::library_is_favorite,
            userdata::library_favorites,
            userdata::library_stats,
            userdata::library_home,
            userdata::play_daily_mix,
            userdata::lyrics_find,
            userdata::lyrics_save,
        ])
        .build(tauri::generate_context!())
        .expect("error while building PixelPlayer PC")
        .run(|app, event| {
            if let RunEvent::Exit = event
                && let Some(state) = app.try_state::<AppState>()
            {
                state.queue_store.save_now(&state.engine);
                state.history.flush();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::percent_decode;

    #[test]
    fn decodes_protocol_paths() {
        assert_eq!(percent_decode("lib%2Fabc-300"), "lib/abc-300");
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("%C3%A9"), "é");
    }
}
