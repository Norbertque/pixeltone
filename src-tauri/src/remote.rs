//! Remote control from phones (PixelPlayer): the app's side of `pp-remote`. Off unless
//! turned on in Settings → Remote control; the server only runs while it's on.

use std::net::UdpSocket;
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use pp_core::ShareMode;
use pp_core::gain::{FLOOR_DB, LOCKED_MAX_DB};
use pp_engine::Engine;
use pp_remote::{Host, RemoteConfig, RemoteServer, RemoteVolume};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::AppState;
use crate::settings::AppSettings;

/// The port phones try first (another is used when it's taken; mDNS says which).
pub const PORT: u16 = 47_821;

struct AppHost {
    app: AppHandle,
    /// Level to return to when the app's own volume is unmuted.
    before_mute: Mutex<f32>,
}

impl AppHost {
    fn state(&self) -> &AppState {
        self.app.state::<AppState>().inner()
    }

    /// Saves changed settings and applies them, as Settings would.
    fn update(&self, change: impl FnOnce(&mut AppSettings)) -> Result<(), String> {
        let state = self.state();
        let updated = {
            let mut s = state.settings.lock();
            change(&mut s);
            s.clone()
        };
        updated
            .save(&state.settings_path)
            .map_err(|e| e.to_string())?;
        state.engine.set_output(updated.effective_output());
        crate::emit_settings(&self.app, &updated);
        Ok(())
    }
}

impl Host for AppHost {
    fn engine(&self) -> &Engine {
        &self.state().engine
    }

    /// The same rules as the UI's `volume.ts`.
    fn volume(&self) -> Option<RemoteVolume> {
        let state = self.state();
        let settings = state.settings.lock().clone();
        let exclusive = match state.engine.snapshot().output {
            Some(o) => o.active == ShareMode::Exclusive,
            None => settings.output.exclusive,
        };
        let app_volume = |db: f32, max: f32, kind: &str| RemoteVolume {
            kind: kind.into(),
            db: db.min(max),
            min: FLOOR_DB,
            max,
            step: 0.5,
            muted: db <= FLOOR_DB,
        };
        if !exclusive {
            return Some(app_volume(settings.output.shared_gain_db, 0.0, "app"));
        }
        let device_id = settings.output.device_id.as_deref();
        let prefs = settings.prefs(device_id);
        if !prefs.software_volume
            && let Ok(Some(hw)) = state.backend.hardware_volume(device_id)
        {
            let (min, max, step) = hw.range_db();
            return Some(RemoteVolume {
                kind: "device".into(),
                db: hw.get_db().unwrap_or(max),
                min,
                max,
                step: if step > 0.0 { step } else { 0.5 },
                muted: hw.get_mute().unwrap_or(false),
            });
        }
        if prefs.line_level {
            return Some(RemoteVolume {
                kind: "fixed".into(),
                db: 0.0,
                min: 0.0,
                max: 0.0,
                step: 1.0,
                muted: false,
            });
        }
        let max = if prefs.loudness_acknowledged {
            0.0
        } else {
            LOCKED_MAX_DB
        };
        Some(app_volume(prefs.software_gain_db, max, "software"))
    }

    fn set_volume(&self, db: f32) -> Result<(), String> {
        let Some(v) = self.volume() else {
            return Ok(());
        };
        let db = if db.is_finite() {
            db.clamp(v.min, v.max)
        } else {
            v.db
        };
        match v.kind.as_str() {
            "app" => self.update(|s| s.output.shared_gain_db = db),
            "software" => self.update(|s| {
                let key = AppSettings::device_key(s.output.device_id.as_deref());
                let mut prefs = s.prefs(s.output.device_id.as_deref());
                prefs.software_gain_db = db;
                s.devices.insert(key, prefs);
            }),
            "device" => {
                let state = self.state();
                let device_id = state.settings.lock().output.device_id.clone();
                match state
                    .backend
                    .hardware_volume(device_id.as_deref())
                    .map_err(|e| e.to_string())?
                {
                    Some(hw) => hw.set_db(db).map_err(|e| e.to_string()),
                    None => Ok(()),
                }
            }
            _ => Err("The output is at a fixed line level.".into()),
        }
    }

    fn set_muted(&self, muted: bool) -> Result<(), String> {
        let Some(v) = self.volume() else {
            return Ok(());
        };
        match v.kind.as_str() {
            "device" => {
                let state = self.state();
                let device_id = state.settings.lock().output.device_id.clone();
                match state
                    .backend
                    .hardware_volume(device_id.as_deref())
                    .map_err(|e| e.to_string())?
                {
                    Some(hw) => hw.set_mute(muted).map_err(|e| e.to_string()),
                    None => Ok(()),
                }
            }
            "fixed" => Ok(()),
            _ if muted == v.muted => Ok(()),
            _ if muted => {
                *self.before_mute.lock() = v.db;
                self.set_volume(FLOOR_DB)
            }
            _ => {
                let back = *self.before_mute.lock();
                self.set_volume(back.min(v.max))
            }
        }
    }

    fn library_art(&self, hash: &str, size: u32) -> Option<Vec<u8>> {
        let path = self.state().library.art().existing(hash, size)?;
        std::fs::read(path).ok()
    }
}

fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Pixeltone".into())
}

/// A random id for this installation, made once.
fn installation_id(dir: &Path) -> String {
    let path = dir.join("remote-id.txt");
    if let Ok(id) = std::fs::read_to_string(&path)
        && !id.trim().is_empty()
    {
        return id.trim().to_string();
    }
    let mut bytes = [0u8; 16];
    let _ = getrandom::fill(&mut bytes);
    let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(&path, &id);
    id
}

/// The address other devices on the network reach this PC at (no packet is sent).
fn lan_address() -> Option<String> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("192.0.2.1", 9)).ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

/// Starts or stops the server to match the setting.
pub fn apply(app: &AppHandle, enabled: bool) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let mut slot = state.remote.lock();
    if !enabled {
        // Dropping it stops the server and withdraws the announcement.
        slot.take();
        return;
    }
    if slot.is_some() {
        return;
    }
    let dir = match app.path().app_local_data_dir() {
        Ok(d) => d.join("remote"),
        Err(e) => {
            tracing::warn!("remote control: {e}");
            return;
        }
    };
    let name = computer_name();
    let host = Arc::new(AppHost {
        app: app.clone(),
        before_mute: Mutex::new(-20.0),
    });
    match RemoteServer::start(
        RemoteConfig {
            name: name.clone(),
            host_name: name,
            id: installation_id(&dir),
            version: app.package_info().version.to_string(),
            data_dir: dir,
            port: PORT,
        },
        host,
    ) {
        Ok(server) => *slot = Some(Arc::new(server)),
        Err(e) => tracing::warn!("remote control didn't start: {e}"),
    }
}

/// The running server, if any (cheap: for event forwarding).
pub fn server(app: &AppHandle) -> Option<Arc<RemoteServer>> {
    app.try_state::<AppState>()?.remote.lock().clone()
}

pub fn volume_changed(app: &AppHandle) {
    if let Some(s) = server(app) {
        s.volume_changed();
    }
}

pub fn library_changed(app: &AppHandle) {
    if let Some(s) = server(app) {
        s.library_changed();
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDevice {
    pub id: String,
    pub name: String,
    pub paired_at: u64,
    pub last_seen: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus {
    pub enabled: bool,
    pub running: bool,
    /// How phones list this PC.
    pub name: String,
    /// `ip:port`, for phones that can't find the PC by itself.
    pub address: Option<String>,
    /// The pairing code on show, and for how many more seconds.
    pub code: Option<String>,
    pub code_seconds: u32,
    pub devices: Vec<RemoteDevice>,
}

fn status(app: &AppHandle, state: &AppState) -> RemoteStatus {
    let enabled = state.settings.lock().preferences.remote_enabled;
    let server = server(app);
    let (code, code_seconds) = server
        .as_ref()
        .and_then(|s| s.pairing().current_code())
        .map_or((None, 0), |(c, left)| (Some(c), left.as_secs() as u32));
    RemoteStatus {
        enabled,
        running: server.is_some(),
        name: computer_name(),
        address: server
            .as_ref()
            .and_then(|s| lan_address().map(|ip| format!("{ip}:{}", s.port()))),
        code,
        code_seconds,
        devices: server
            .map(|s| {
                s.pairing()
                    .devices()
                    .into_iter()
                    .map(|d| RemoteDevice {
                        id: d.id,
                        name: d.name,
                        paired_at: d.paired_at,
                        last_seen: d.last_seen,
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

#[tauri::command]
pub fn remote_status(app: AppHandle, state: State<'_, AppState>) -> RemoteStatus {
    status(&app, &state)
}

/// Shows a new pairing code (pairing stays open until it's used or expires).
#[tauri::command]
pub fn remote_pair(app: AppHandle, state: State<'_, AppState>) -> Result<RemoteStatus, String> {
    let server = server(&app).ok_or("Turn on remote control first.")?;
    server.pairing().new_code();
    Ok(status(&app, &state))
}

#[tauri::command]
pub fn remote_stop_pairing(app: AppHandle, state: State<'_, AppState>) -> RemoteStatus {
    if let Some(s) = server(&app) {
        s.pairing().close();
    }
    status(&app, &state)
}

#[tauri::command]
pub fn remote_forget(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<RemoteStatus, String> {
    if let Some(s) = server(&app) {
        s.pairing().forget(&id)?;
    }
    Ok(status(&app, &state))
}
