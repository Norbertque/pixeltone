//! PixelPlayer PC desktop app: wires the Rust core to the Svelte UI through Tauri.

mod commands;
mod library;
#[cfg(windows)]
mod power;
mod settings;
#[cfg(windows)]
mod window_theme;

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use pp_core::OutputBackend;
use pp_engine::{Engine, EngineEvent};
use pp_library::Library;
use serde::Serialize;
use tauri::http::Response;
use tauri::{AppHandle, Emitter, Manager};
use tracing_subscriber::EnvFilter;

use crate::settings::AppSettings;

pub struct AppState {
    pub engine: Engine,
    pub backend: Arc<dyn OutputBackend>,
    pub library: Arc<Library>,
    pub settings: Mutex<AppSettings>,
    pub settings_path: PathBuf,
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
    match event {
        EngineEvent::State(snapshot) => {
            #[cfg(windows)]
            power::keep_awake(snapshot.status == pp_engine::PlaybackStatus::Playing);
            let _ = app.emit("player://state", snapshot);
        }
        EngineEvent::Position { position_ms } => {
            let _ = app.emit("player://position", PositionEvent { position_ms });
        }
        EngineEvent::Error { code, message } => {
            let _ = app.emit("player://error", ErrorEvent { code, message });
        }
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
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
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
            library::start(app.handle(), &library);
            app.manage(AppState {
                engine,
                backend,
                library,
                settings: Mutex::new(settings),
                settings_path,
            });
            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running PixelPlayer PC");
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
