//! Media keys and the Windows media flyout, kept in step with the player.

use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use pp_engine::{PlaybackStatus, PlayerSnapshot, RepeatMode};
use pp_media_controls::{MediaAction, MediaControls, Repeat, Status, Track};
use tauri::{AppHandle, Manager};

use crate::AppState;

/// How often the flyout's seek bar is moved along while playing.
const TIMELINE_INTERVAL: Duration = Duration::from_secs(1);

pub enum Update {
    State(Box<PlayerSnapshot>),
    Position(f64),
}

pub struct Media {
    tx: Sender<Update>,
}

impl Media {
    pub fn send(&self, update: Update) {
        let _ = self.tx.send(update);
    }
}

/// Attaches the controls to the main window. `None` when Windows refuses (they're optional).
pub fn start(app: &AppHandle) -> Option<Media> {
    let window = app.get_webview_window("main")?;
    let hwnd = window.hwnd().ok()?.0 as isize;
    let handle = app.clone();
    let controls = MediaControls::new(hwnd, move |action| act(&handle, action))
        .inspect_err(|e| tracing::warn!("media controls unavailable: {e}"))
        .ok()?;
    let (tx, rx) = unbounded();
    let app = app.clone();
    thread::Builder::new()
        .name("pp-media".into())
        .spawn(move || {
            pp_media_controls::init_thread();
            run(&app, &controls, &rx);
        })
        .ok()?;
    Some(Media { tx })
}

fn act(app: &AppHandle, action: MediaAction) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let e = &state.engine;
    match action {
        MediaAction::Play => e.play(),
        MediaAction::Pause => e.pause(),
        MediaAction::Stop => e.stop(),
        MediaAction::Next => e.next(),
        MediaAction::Previous => e.previous(),
        MediaAction::Seek(ms) => e.seek(ms),
        MediaAction::Shuffle(on) => e.set_shuffle(on),
        MediaAction::Repeat(r) => e.set_repeat(match r {
            Repeat::Off => RepeatMode::Off,
            Repeat::All => RepeatMode::All,
            Repeat::One => RepeatMode::One,
        }),
    }
}

/// Cover bytes for a track's cover key (library thumbnail or embedded picture).
fn cover(app: &AppHandle, key: &str) -> Option<Vec<u8>> {
    let state = app.try_state::<AppState>()?;
    match key.strip_prefix("lib/") {
        Some(hash) => std::fs::read(state.library.art().existing(hash, 300)?).ok(),
        None => state.engine.covers().get(key).map(|art| art.data),
    }
}

fn run(app: &AppHandle, controls: &MediaControls, rx: &Receiver<Update>) {
    let mut duration = 0.0;
    let mut playing = false;
    let mut shown: Option<(String, Option<String>)> = None;
    let mut last_timeline = Instant::now();
    while let Ok(update) = rx.recv() {
        let result = match update {
            Update::State(s) => {
                duration = s.track.as_ref().and_then(|t| t.duration_ms).unwrap_or(0.0);
                playing = s.status == PlaybackStatus::Playing;
                let key = s
                    .track
                    .as_ref()
                    .map(|t| (t.path.clone(), t.cover_key.clone()));
                if key != shown {
                    let track = s.track.as_ref().map(|t| Track {
                        title: t.title.clone(),
                        artist: t.artist.clone(),
                        album: t.album.clone(),
                        cover: t.cover_key.as_deref().and_then(|k| cover(app, k)),
                    });
                    if let Err(e) = controls.set_track(track.as_ref()) {
                        tracing::debug!("media flyout: {e}");
                    }
                    shown = key;
                }
                let index = s.queue_index.unwrap_or(0);
                let can_next = index + 1 < s.queue_length || s.repeat == RepeatMode::All;
                let status = match s.status {
                    PlaybackStatus::Playing => Status::Playing,
                    PlaybackStatus::Paused => Status::Paused,
                    PlaybackStatus::Stopped => Status::Stopped,
                };
                let repeat = match s.repeat {
                    RepeatMode::Off => Repeat::Off,
                    RepeatMode::All => Repeat::All,
                    RepeatMode::One => Repeat::One,
                };
                last_timeline = Instant::now();
                controls
                    .set_status(status, s.track.is_some(), can_next)
                    .and_then(|()| controls.set_modes(s.shuffle, repeat))
                    .and_then(|()| controls.set_timeline(s.position_ms, duration))
            }
            Update::Position(ms) => {
                if !playing || last_timeline.elapsed() < TIMELINE_INTERVAL {
                    continue;
                }
                last_timeline = Instant::now();
                controls.set_timeline(ms, duration)
            }
        };
        if let Err(e) = result {
            tracing::debug!("media flyout: {e}");
        }
    }
}
