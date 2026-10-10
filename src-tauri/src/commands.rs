//! Tauri commands called from the UI (`ui/src/lib/api/tauri.ts`).

use std::path::PathBuf;
use std::sync::Arc;

use pp_core::{DeviceCapabilities, DeviceInfo, OutputBackend};
use pp_engine::{PlayerSnapshot, RepeatMode};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::AppState;
use crate::settings::{AppSettings, DevicePrefs};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// Package version, plus the CI build id for development builds.
    pub app_version: String,
    pub ffmpeg_version: String,
    pub ffmpeg_license: String,
    pub os: String,
    /// The output API in use ("wasapi" on Windows).
    pub output_backend: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    let version = env!("CARGO_PKG_VERSION");
    AppInfo {
        app_version: match option_env!("PP_BUILD_ID") {
            Some(build) if !build.is_empty() => format!("{version} ({build})"),
            _ => version.to_string(),
        },
        ffmpeg_version: pp_decode::ffmpeg_version(),
        ffmpeg_license: pp_decode::ffmpeg_license(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        output_backend: state.backend.name().to_string(),
    }
}

/// Tints the native title bar to the theme's surface colour (`#rrggbb`).
#[tauri::command]
pub fn set_window_theme(
    window: tauri::WebviewWindow,
    caption: String,
    dark: bool,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        crate::window_theme::apply(&window, &caption, dark)
    }
    #[cfg(not(windows))]
    {
        let _ = (window, caption, dark);
        Ok(())
    }
}

// ── Player ──────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlayerAction {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
}

#[tauri::command]
pub fn player_state(state: State<'_, AppState>) -> PlayerSnapshot {
    state.engine.snapshot()
}

#[tauri::command]
pub fn player_play_files(state: State<'_, AppState>, paths: Vec<String>, start: usize) {
    state
        .engine
        .play_files(paths.into_iter().map(PathBuf::from).collect(), start);
}

#[tauri::command]
pub fn player_action(state: State<'_, AppState>, action: PlayerAction) {
    let e = &state.engine;
    match action {
        PlayerAction::Play => e.play(),
        PlayerAction::Pause => e.pause(),
        PlayerAction::Toggle => e.toggle(),
        PlayerAction::Stop => e.stop(),
        PlayerAction::Next => e.next(),
        PlayerAction::Previous => e.previous(),
    }
}

#[tauri::command]
pub fn player_seek(state: State<'_, AppState>, ms: f64) {
    state.engine.seek(ms);
}

#[tauri::command]
pub fn player_set_repeat(state: State<'_, AppState>, mode: RepeatMode) {
    state.engine.set_repeat(mode);
}

// ── Output ──────────────────────────────────────────────────────────────────────────

async fn blocking<T: Send + 'static>(
    backend: Arc<dyn OutputBackend>,
    f: impl FnOnce(&dyn OutputBackend) -> Result<T, pp_core::OutputError> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || f(backend.as_ref()))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn output_devices(state: State<'_, AppState>) -> Result<Vec<DeviceInfo>, String> {
    blocking(state.backend.clone(), |b| b.list_devices()).await
}

#[tauri::command]
pub async fn output_capabilities(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<DeviceCapabilities, String> {
    blocking(state.backend.clone(), move |b| {
        b.capabilities(device_id.as_deref())
    })
    .await
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.lock().clone()
}

/// Replaces the output settings and the selected device's volume preferences.
#[tauri::command]
pub fn set_output_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    output: pp_engine::OutputSettings,
    prefs: DevicePrefs,
) -> Result<AppSettings, String> {
    let updated = {
        let mut s = state.settings.lock();
        let key = AppSettings::device_key(output.device_id.as_deref());
        s.output = output;
        s.devices.insert(key, prefs);
        s.clone()
    };
    updated
        .save(&state.settings_path)
        .map_err(|e| e.to_string())?;
    state.engine.set_output(updated.effective_output());
    crate::emit_settings(&app, &updated);
    Ok(updated)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareVolumeInfo {
    pub db: f32,
    pub min_db: f32,
    pub max_db: f32,
    pub step_db: f32,
    pub muted: bool,
}

/// The device's own volume control, if it has one.
#[tauri::command]
pub async fn hardware_volume(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<Option<HardwareVolumeInfo>, String> {
    blocking(state.backend.clone(), move |b| {
        let Some(v) = b.hardware_volume(device_id.as_deref())? else {
            return Ok(None);
        };
        let (min_db, max_db, step_db) = v.range_db();
        Ok(Some(HardwareVolumeInfo {
            db: v.get_db()?,
            min_db,
            max_db,
            step_db,
            muted: v.get_mute()?,
        }))
    })
    .await
}

#[tauri::command]
pub async fn set_hardware_volume(
    state: State<'_, AppState>,
    device_id: Option<String>,
    db: f32,
) -> Result<(), String> {
    blocking(state.backend.clone(), move |b| {
        match b.hardware_volume(device_id.as_deref())? {
            Some(v) => v.set_db(db),
            None => Ok(()),
        }
    })
    .await
}
