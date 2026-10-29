//! Remote control from phones (PixelPlayer): the app's side of `pp-remote`. Off unless
//! turned on in Settings → Remote control; the server only runs while it's on.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use pp_core::ShareMode;
use pp_core::gain::{FLOOR_DB, LOCKED_MAX_DB};
use pp_engine::{Engine, QueueItem};
use pp_library::{AlbumRow, AlbumSort, TrackRow};
use pp_remote::jam::GuestPermissions;
use pp_remote::protocol::{
    JamView, LibraryAlbum, LibraryAlbumDetail, LibraryResults, LibraryTrack,
};
use pp_remote::{Host, RemoteConfig, RemoteServer, RemoteVolume};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

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

    fn library_items(&self, ids: &[u32]) -> Vec<QueueItem> {
        let mut rows: Vec<Option<TrackRow>> = Vec::new();
        let found = self.state().library.tracks_by_id(ids).unwrap_or_default();
        // In the order asked, whatever order the library answers in.
        for id in ids {
            rows.push(found.iter().find(|r| r.id == *id).cloned());
        }
        rows.into_iter()
            .flatten()
            .map(crate::queue::item_from_row)
            .collect()
    }

    fn library_search(&self, query: &str, limit: u32) -> LibraryResults {
        let Ok(found) = self.state().library.search(query, limit) else {
            return LibraryResults::default();
        };
        LibraryResults {
            tracks: found.tracks.into_iter().map(library_track).collect(),
            albums: found.albums.into_iter().map(library_album).collect(),
        }
    }

    fn library_albums(&self, offset: u32, limit: u32) -> Vec<LibraryAlbum> {
        self.state()
            .library
            .albums(AlbumSort::Title, offset, limit)
            .map(|rows| rows.into_iter().map(library_album).collect())
            .unwrap_or_default()
    }

    fn library_album(&self, id: u32) -> Option<LibraryAlbumDetail> {
        let detail = self.state().library.album(id).ok()??;
        Some(LibraryAlbumDetail {
            album: library_album(detail.album),
            tracks: detail.tracks.into_iter().map(library_track).collect(),
        })
    }

    fn jam_changed(&self, jam: Option<&JamView>) {
        // While a jam runs, what plays isn't the owner's listening.
        self.state().history.set_jam(jam.is_some());
        let _ = self.app.emit("remote://jam", jam);
        if jam.is_none() {
            // The server may have run only for the jam. Not from here: this can be one of the
            // server's own threads, and stopping it waits for them.
            let app = self.app.clone();
            std::thread::spawn(move || {
                let enabled = app
                    .state::<AppState>()
                    .settings
                    .lock()
                    .preferences
                    .remote_enabled;
                apply(&app, enabled);
            });
        }
    }
}

fn some_text(text: String) -> Option<String> {
    (!text.is_empty()).then_some(text)
}

fn library_track(row: TrackRow) -> LibraryTrack {
    LibraryTrack {
        id: row.id,
        title: row.title,
        artist: some_text(row.artist),
        album: some_text(row.album),
        duration_ms: f64::from(row.duration_ms),
        cover_key: row.cover.as_deref().map(crate::library::cover_key),
    }
}

fn library_album(row: AlbumRow) -> LibraryAlbum {
    LibraryAlbum {
        id: row.id,
        title: row.title,
        artist: some_text(row.artist),
        year: row.year,
        track_count: row.track_count,
        cover_key: row.cover.as_deref().map(crate::library::cover_key),
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

/// The server's folder: its certificate, paired phones, a jam's phone tracks.
fn remote_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_local_data_dir()?.join("remote"))
}

const JAM_TRACKS: &str = "jam-tracks";

/// Where a jam's phone tracks are copied while it runs; the server empties it.
pub fn jam_tracks_dir(app: &AppHandle) -> Option<PathBuf> {
    remote_dir(app).ok().map(|d| d.join(JAM_TRACKS))
}

/// Starts or stops the server to match the setting. A jam keeps it running until it ends,
/// whatever the setting.
pub fn apply(app: &AppHandle, enabled: bool) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let mut slot = state.remote.lock();
    if !enabled {
        if slot.as_ref().is_some_and(|s| s.jam().is_some()) {
            return;
        }
        // Dropping it stops the server and withdraws the announcement.
        slot.take();
        return;
    }
    if slot.is_some() {
        return;
    }
    let dir = match remote_dir(app) {
        Ok(d) => d,
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
            jam_tracks_dir: dir.join(JAM_TRACKS),
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
            .and_then(|s| pp_remote::lan::lan_address().map(|ip| format!("{ip}:{}", s.port()))),
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

/// The jam that runs, as the PC's Jam panel shows it.
#[tauri::command]
pub fn jam_state(app: AppHandle) -> Option<JamView> {
    server(&app)?.jam()
}

/// Starts a jam; the server runs for it even with remote control off.
#[tauri::command]
pub fn jam_start(app: AppHandle) -> Result<JamView, String> {
    apply(&app, true);
    let server =
        server(&app).ok_or("The jam couldn't start: the remote control server didn't start.")?;
    Ok(server.jam_start())
}

/// Ends the jam; the server stops with it if it ran only for the jam (`jam_changed`).
#[tauri::command]
pub fn jam_end(app: AppHandle) {
    if let Some(s) = server(&app) {
        s.jam_end();
    }
}

#[tauri::command]
pub fn jam_permissions(app: AppHandle, permissions: GuestPermissions) {
    if let Some(s) = server(&app) {
        s.jam_permissions(permissions);
    }
}

#[tauri::command]
pub fn jam_remove_guest(app: AppHandle, guest_id: String) {
    if let Some(s) = server(&app) {
        s.jam_remove_guest(&guest_id);
    }
}

/// The owner's answer to a rejoin request; false when it isn't waiting any more.
#[tauri::command]
pub fn jam_answer(app: AppHandle, request_id: String, allow: bool) -> bool {
    server(&app).is_some_and(|s| s.jam_answer(&request_id, allow))
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
