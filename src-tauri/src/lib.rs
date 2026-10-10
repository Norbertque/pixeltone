//! PixelPlayer PC desktop app: wires the Rust core to the Svelte UI through Tauri.

mod commands;
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
use serde::Serialize;
use tauri::http::Response;
use tauri::{AppHandle, Emitter, Manager};
use tracing_subscriber::EnvFilter;

use crate::settings::AppSettings;

pub struct AppState {
    pub engine: Engine,
    pub backend: Arc<dyn OutputBackend>,
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
        // Embedded cover art of recent tracks: cover://localhost/<key>.
        .register_uri_scheme_protocol("cover", |ctx, request| {
            let key = request.uri().path().trim_start_matches('/').to_string();
            let art = ctx
                .app_handle()
                .try_state::<AppState>()
                .and_then(|s| s.engine.covers().get(&key));
            match art {
                Some(art) => Response::builder()
                    .header("Content-Type", art.mime)
                    .header("Cache-Control", "max-age=31536000, immutable")
                    // The UI reads cover pixels on a canvas to pick the theme colour.
                    .header("Access-Control-Allow-Origin", "*")
                    .body(art.data)
                    .unwrap_or_default(),
                None => Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap_or_default(),
            }
        })
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = AppSettings::load(&settings_path);
            let backend = output_backend();
            let handle = app.handle().clone();
            let engine = Engine::new(backend.clone(), settings.effective_output(), move |event| {
                forward(&handle, event)
            });
            app.manage(AppState {
                engine,
                backend,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running PixelPlayer PC");
}
