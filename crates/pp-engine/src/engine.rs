//! The engine: a control thread that owns the queue and the output session, driven by
//! commands from the app and events from the decode worker.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use parking_lot::{Mutex, RwLock};
use pp_core::{
    DeviceCapabilities, OpenRequest, OutputBackend, OutputBadge, OutputError, OutputFormat,
    OutputStream, ShareMode, SoftwareGain, StreamControls, StreamStats,
};
use pp_decode::CoverArt;
use rtrb::{Producer, RingBuffer};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::pipeline::OutputPlan;
use crate::queue::{Queue, QueueItem, RepeatMode};
use crate::track::{LoadedTrack, StartedTrack, TrackMeta, load};
use crate::worker::{TaggedEvent, WorkerArgs, WorkerEvent, WorkerHandle};

/// How the user wants audio to leave the app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct OutputSettings {
    /// Endpoint id; `None` follows the Windows default device.
    pub device_id: Option<String>,
    /// Use WASAPI exclusive mode when the device allows it.
    pub exclusive: bool,
    /// Exclusive-mode timing: event-driven instead of polling.
    pub event_driven: bool,
    /// Device buffer length in milliseconds.
    pub buffer_ms: u32,
    /// Software attenuation for exclusive mode on devices without hardware volume (dB, ≤ 0).
    pub software_gain_db: f32,
    /// Full-scale output, bypassing the software gain stage (for an external amplifier).
    pub line_level: bool,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            device_id: None,
            exclusive: false,
            event_driven: false,
            buffer_ms: 50,
            software_gain_db: pp_core::gain::DEFAULT_DB,
            line_level: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Decoded audio buffered ahead of the device.
    pub ring_ms: u32,
    /// Silence played after (re)opening a device in exclusive mode, so the DAC's clock can
    /// lock to the new rate before music starts.
    pub preroll_ms: u32,
    /// Control loop period.
    pub tick: Duration,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            ring_ms: 1500,
            preroll_ms: 150,
            tick: Duration::from_millis(15),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OutputState {
    pub device_name: String,
    pub requested: ShareMode,
    pub active: ShareMode,
    /// Why exclusive mode isn't in use although it was requested.
    pub fallback_reason: Option<String>,
    pub badge: OutputBadge,
    pub stats: StreamStats,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlayerSnapshot {
    pub status: PlaybackStatus,
    pub track: Option<TrackMeta>,
    pub queue_index: Option<u32>,
    pub queue_length: u32,
    pub position_ms: f64,
    pub repeat: RepeatMode,
    pub output: Option<OutputState>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    /// Anything but the position changed.
    State(Box<PlayerSnapshot>),
    /// Periodic position while playing (the UI interpolates in between).
    Position { position_ms: f64 },
    /// Something the user should be told about.
    Error { code: String, message: String },
}

/// Embedded covers of recent tracks, served to the UI by key.
#[derive(Default)]
pub struct CoverStore {
    items: Mutex<VecDeque<(String, CoverArt)>>,
}

impl CoverStore {
    const CAPACITY: usize = 8;

    pub fn insert(&self, key: String, art: CoverArt) {
        let mut items = self.items.lock();
        items.retain(|(k, _)| *k != key);
        items.push_front((key, art));
        items.truncate(Self::CAPACITY);
    }

    pub fn get(&self, key: &str) -> Option<CoverArt> {
        self.items
            .lock()
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, a)| a.clone())
    }
}

enum Command {
    PlayItems {
        items: Vec<QueueItem>,
        start: usize,
    },
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    Seek {
        ms: f64,
    },
    SetRepeat(RepeatMode),
    SetOutput(OutputSettings),
    /// Forget probed device capabilities (devices changed).
    RefreshDevices,
    Shutdown,
}

pub struct Engine {
    tx: Sender<Command>,
    snapshot: Arc<RwLock<PlayerSnapshot>>,
    covers: Arc<CoverStore>,
    thread: Option<JoinHandle<()>>,
}

impl Engine {
    pub fn new(
        backend: Arc<dyn OutputBackend>,
        settings: OutputSettings,
        on_event: impl Fn(EngineEvent) + Send + Sync + 'static,
    ) -> Self {
        Self::with_config(backend, settings, EngineConfig::default(), on_event)
    }

    pub fn with_config(
        backend: Arc<dyn OutputBackend>,
        settings: OutputSettings,
        config: EngineConfig,
        on_event: impl Fn(EngineEvent) + Send + Sync + 'static,
    ) -> Self {
        let (tx, rx) = unbounded();
        let snapshot = Arc::new(RwLock::new(PlayerSnapshot::default()));
        let covers = Arc::new(CoverStore::default());
        let (worker_tx, worker_rx) = unbounded();
        let mut control = Control {
            backend,
            settings,
            config,
            queue: Arc::new(Mutex::new(Queue::default())),
            current: None,
            status: PlaybackStatus::Stopped,
            parked: None,
            session: None,
            caps: HashMap::new(),
            snapshot: snapshot.clone(),
            covers: covers.clone(),
            emit: Arc::new(on_event),
            worker_tx,
            worker_rx,
            generation: 0,
            last_position_emit: Instant::now(),
            last_state: None,
        };
        let thread = thread::Builder::new()
            .name("pp-engine".into())
            .spawn(move || control.run(&rx))
            .expect("spawn engine thread");
        Self {
            tx,
            snapshot,
            covers,
            thread: Some(thread),
        }
    }

    pub fn snapshot(&self) -> PlayerSnapshot {
        self.snapshot.read().clone()
    }

    pub fn covers(&self) -> Arc<CoverStore> {
        self.covers.clone()
    }

    fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }

    pub fn play_files(&self, paths: Vec<PathBuf>, start: usize) {
        self.play_items(paths.into_iter().map(QueueItem::from).collect(), start);
    }

    pub fn play_items(&self, items: Vec<QueueItem>, start: usize) {
        self.send(Command::PlayItems { items, start });
    }
    pub fn play(&self) {
        self.send(Command::Play);
    }
    pub fn pause(&self) {
        self.send(Command::Pause);
    }
    pub fn toggle(&self) {
        self.send(Command::Toggle);
    }
    pub fn stop(&self) {
        self.send(Command::Stop);
    }
    pub fn next(&self) {
        self.send(Command::Next);
    }
    pub fn previous(&self) {
        self.send(Command::Previous);
    }
    pub fn seek(&self, ms: f64) {
        self.send(Command::Seek { ms });
    }
    pub fn set_repeat(&self, mode: RepeatMode) {
        self.send(Command::SetRepeat(mode));
    }
    pub fn set_output(&self, settings: OutputSettings) {
        self.send(Command::SetOutput(settings));
    }
    pub fn refresh_devices(&self) {
        self.send(Command::RefreshDevices);
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.send(Command::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The device side of playback: one stream in one format.
struct Session {
    stream: Box<dyn OutputStream>,
    format: OutputFormat,
    plan: OutputPlan,
    controls: Arc<StreamControls>,
    device_name: String,
    fallback_reason: Option<String>,
    /// Applied software attenuation, if the gain stage is active.
    software_gain_db: Option<f32>,
    worker: Option<WorkerHandle>,
    /// Tracks queued in this stream; the first is the audible one.
    boundaries: Vec<Boundary>,
    /// Where the stream ends, and the track that needs a reopen afterwards.
    ended: Option<(u64, Option<Box<LoadedTrack>>)>,
}

struct Boundary {
    at_frame: u64,
    track: StartedTrack,
    badge: OutputBadge,
}

impl Session {
    fn gain(&self) -> SoftwareGain {
        self.software_gain_db
            .map(SoftwareGain::new)
            .unwrap_or(SoftwareGain::UNITY)
    }

    fn stop_worker(&mut self) -> Option<Producer<u8>> {
        self.worker.take().and_then(WorkerHandle::stop)
    }

    fn position_ms(&self, played: u64) -> Option<f64> {
        let b = self.boundaries.first()?;
        let frames = played.saturating_sub(b.at_frame);
        Some(b.track.start_ms as f64 + frames as f64 * 1000.0 / f64::from(self.format.sample_rate))
    }
}

struct PlanChoice {
    plan: OutputPlan,
    device_name: String,
    fallback_reason: Option<String>,
    software_gain_db: Option<f32>,
}

struct Parked {
    track: TrackMeta,
    position_ms: f64,
}

struct Control {
    backend: Arc<dyn OutputBackend>,
    settings: OutputSettings,
    config: EngineConfig,
    queue: Arc<Mutex<Queue>>,
    /// Queue position of the audible (or last) track.
    current: Option<usize>,
    status: PlaybackStatus,
    /// Track and position kept while no stream is open (stopped, paused after an error).
    parked: Option<Parked>,
    session: Option<Session>,
    caps: HashMap<String, DeviceCapabilities>,
    snapshot: Arc<RwLock<PlayerSnapshot>>,
    covers: Arc<CoverStore>,
    emit: Arc<dyn Fn(EngineEvent) + Send + Sync>,
    worker_tx: Sender<TaggedEvent>,
    worker_rx: Receiver<TaggedEvent>,
    generation: u64,
    last_position_emit: Instant,
    last_state: Option<PlayerSnapshot>,
}

impl Control {
    fn run(&mut self, rx: &Receiver<Command>) {
        loop {
            match rx.recv_timeout(self.config.tick) {
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.tick();
            self.publish();
        }
        self.close_session();
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::PlayItems { items, start } => {
                self.queue.lock().set(items);
                self.parked = None;
                self.start_at(start, 0.0, true, 0);
            }
            Command::Play => self.play(),
            Command::Pause => self.pause(),
            Command::Toggle => {
                if self.status == PlaybackStatus::Playing {
                    self.pause();
                } else {
                    self.play();
                }
            }
            Command::Stop => {
                self.park();
                self.close_session();
                self.status = PlaybackStatus::Stopped;
                if let Some(p) = &mut self.parked {
                    p.position_ms = 0.0;
                }
            }
            Command::Next => {
                let next = self.current.and_then(|c| self.queue.lock().next(c));
                match next {
                    Some(n) => self.start_at(n, 0.0, self.status == PlaybackStatus::Playing, 0),
                    None => self.finish_queue(),
                }
            }
            Command::Previous => {
                let Some(cur) = self.current else { return };
                let previous = self.queue.lock().previous(cur);
                if self.position_ms() > 3000.0 {
                    self.start_at(cur, 0.0, self.status == PlaybackStatus::Playing, 0);
                } else if let Some(p) = previous {
                    self.start_at(p, 0.0, self.status == PlaybackStatus::Playing, 0);
                }
            }
            Command::Seek { ms } => {
                if let Some(cur) = self.current {
                    let play = self.status == PlaybackStatus::Playing;
                    self.start_at(cur, ms.max(0.0), play, 0);
                }
            }
            Command::SetRepeat(mode) => self.queue.lock().repeat = mode,
            Command::SetOutput(settings) => self.set_output(settings),
            Command::RefreshDevices => self.caps.clear(),
            Command::Shutdown => {}
        }
    }

    fn play(&mut self) {
        if let Some(s) = &mut self.session {
            match s.stream.start() {
                Ok(()) => self.status = PlaybackStatus::Playing,
                Err(e) => self.on_device_error(e),
            }
            return;
        }
        let resume = self.parked.as_ref().map(|p| p.position_ms).unwrap_or(0.0);
        let pos = self.current.unwrap_or(0);
        if !self.queue.lock().is_empty() {
            self.start_at(pos, resume, true, 0);
        }
    }

    fn pause(&mut self) {
        if let Some(s) = &mut self.session
            && self.status == PlaybackStatus::Playing
        {
            if let Err(e) = s.stream.pause() {
                self.on_device_error(e);
                return;
            }
            self.status = PlaybackStatus::Paused;
        }
    }

    fn set_output(&mut self, settings: OutputSettings) {
        let previous = std::mem::replace(&mut self.settings, settings);
        if previous.device_id != self.settings.device_id {
            self.caps.clear();
        }
        let only_gain = OutputSettings {
            software_gain_db: self.settings.software_gain_db,
            ..previous.clone()
        } == self.settings;
        if let Some(s) = &mut self.session {
            if only_gain {
                if s.software_gain_db.is_some() {
                    s.software_gain_db = Some(self.settings.software_gain_db);
                    s.controls
                        .set_gain(SoftwareGain::new(self.settings.software_gain_db));
                    let gain = s.gain();
                    for b in &mut s.boundaries {
                        b.badge = OutputBadge::new(
                            s.plan.mode,
                            b.track.meta.source,
                            b.track.negotiated.format,
                            b.track.negotiated.conversion,
                            gain,
                        );
                    }
                }
                return;
            }
            // Device, mode or timing changed: reopen at the same spot.
            if let Some(cur) = self.current {
                let ms = self.position_ms();
                let play = self.status == PlaybackStatus::Playing;
                self.close_session();
                self.start_at(cur, ms, play, 0);
            }
        }
    }

    /// Chooses shared or exclusive mode for the configured device.
    fn plan(&mut self) -> Result<PlanChoice, OutputError> {
        let key = self.settings.device_id.clone().unwrap_or_default();
        let caps = match self.caps.get(&key) {
            Some(c) => c.clone(),
            None => {
                let c = self
                    .backend
                    .capabilities(self.settings.device_id.as_deref())?;
                self.caps.insert(key, c.clone());
                c
            }
        };
        let mut fallback_reason = None;
        let mode = if !self.settings.exclusive {
            ShareMode::Shared
        } else if caps.exclusive.is_empty() {
            fallback_reason = Some(
                caps.exclusive_error
                    .clone()
                    .unwrap_or_else(|| "the device offers no exclusive-mode formats".into()),
            );
            ShareMode::Shared
        } else {
            ShareMode::Exclusive
        };
        let gain_stage =
            mode == ShareMode::Exclusive && !caps.hardware_volume && !self.settings.line_level;
        Ok(PlanChoice {
            plan: OutputPlan {
                mode,
                exclusive: if mode == ShareMode::Exclusive {
                    caps.exclusive.clone()
                } else {
                    Vec::new()
                },
                mix_format: caps.mix_format,
                prefer_deepest: gain_stage,
            },
            device_name: caps.device.name.clone(),
            fallback_reason,
            software_gain_db: gain_stage.then_some(self.settings.software_gain_db),
        })
    }

    /// Starts queue position `pos` at `ms`, reusing the open stream when its format fits.
    fn start_at(&mut self, pos: usize, ms: f64, play: bool, attempt: usize) {
        let Some(item) = self.queue.lock().get(pos).cloned() else {
            self.finish_queue();
            return;
        };
        let choice = match self.plan() {
            Ok(c) => c,
            Err(e) => {
                self.report(e.code(), e.to_string());
                self.close_session();
                self.status = PlaybackStatus::Stopped;
                return;
            }
        };
        let track = match load(pos, &item, ms as u64, &choice.plan) {
            Ok(t) => t,
            Err(error) => {
                self.report(
                    "trackFailed",
                    format!("Can't play {}: {error}", item.path.display()),
                );
                let next = self.queue.lock().after(pos);
                let len = self.queue.lock().len();
                match next {
                    Some(n) if n != pos && attempt + 1 < len => {
                        self.start_at(n, 0.0, play, attempt + 1)
                    }
                    _ => self.finish_queue(),
                }
                return;
            }
        };
        self.current = Some(pos);

        let reusable = self
            .session
            .as_ref()
            .is_some_and(|s| s.plan == choice.plan && s.format == track.negotiated.format);
        if reusable {
            let s = self.session.as_mut().expect("checked");
            if let Some(producer) = s.stop_worker() {
                let flushed = s.stream.pause().and_then(|()| s.stream.flush());
                if flushed.is_ok() {
                    s.boundaries.clear();
                    s.ended = None;
                    self.generation += 1;
                    s.worker = Some(WorkerHandle::spawn(WorkerArgs {
                        generation: self.generation,
                        producer,
                        format: s.format,
                        plan: s.plan.clone(),
                        queue: self.queue.clone(),
                        first: track,
                        preroll_frames: 0,
                        events: self.worker_tx.clone(),
                        controls: s.controls.clone(),
                    }));
                    if play && let Err(e) = s.stream.start() {
                        self.on_device_error(e);
                        return;
                    }
                    self.status = if play {
                        PlaybackStatus::Playing
                    } else {
                        PlaybackStatus::Paused
                    };
                    return;
                }
            }
        }
        self.close_session();
        self.open_session(track, choice, play);
    }

    fn open_session(&mut self, mut track: LoadedTrack, choice: PlanChoice, play: bool) {
        let format = track.negotiated.format;
        let ring_frames =
            (u64::from(format.sample_rate) * u64::from(self.config.ring_ms) / 1000).max(4096);
        let (producer, consumer) = RingBuffer::new(ring_frames as usize * format.bytes_per_frame());
        let controls = Arc::new(StreamControls::default());
        if let Some(db) = choice.software_gain_db {
            controls.set_gain(SoftwareGain::new(db));
        }
        let request = OpenRequest {
            device_id: self.settings.device_id.clone(),
            mode: choice.plan.mode,
            format,
            buffer_ms: self.settings.buffer_ms,
            event_driven: self.settings.event_driven,
            controls: controls.clone(),
        };
        let mut stream = match self.backend.open(&request, consumer) {
            Ok(s) => s,
            Err(e) if choice.plan.mode == ShareMode::Exclusive && e.suggests_shared_fallback() => {
                tracing::warn!("exclusive mode unavailable ({e}); falling back to shared mode");
                let shared = PlanChoice {
                    plan: OutputPlan {
                        mode: ShareMode::Shared,
                        exclusive: Vec::new(),
                        mix_format: choice.plan.mix_format,
                        prefer_deepest: false,
                    },
                    device_name: choice.device_name,
                    fallback_reason: Some(e.to_string()),
                    software_gain_db: None,
                };
                match shared.plan.negotiate(&track.meta.source, track.meta.dsd) {
                    Some(n) => {
                        track.negotiated = n;
                        self.open_session(track, shared, play);
                    }
                    None => {
                        self.report(e.code(), e.to_string());
                        self.status = PlaybackStatus::Stopped;
                    }
                }
                return;
            }
            Err(e) => {
                self.report(e.code(), e.to_string());
                self.park_track(&track.meta, track.start_ms as f64);
                self.status = PlaybackStatus::Stopped;
                return;
            }
        };

        let preroll_frames = if choice.plan.mode == ShareMode::Exclusive {
            u64::from(format.sample_rate) * u64::from(self.config.preroll_ms) / 1000
        } else {
            0
        };
        self.generation += 1;
        let worker = WorkerHandle::spawn(WorkerArgs {
            generation: self.generation,
            producer,
            format,
            plan: choice.plan.clone(),
            queue: self.queue.clone(),
            first: track,
            preroll_frames,
            events: self.worker_tx.clone(),
            controls: controls.clone(),
        });
        if play && let Err(e) = stream.start() {
            self.report(e.code(), e.to_string());
        }
        self.session = Some(Session {
            stream,
            format,
            plan: choice.plan,
            controls,
            device_name: choice.device_name,
            fallback_reason: choice.fallback_reason,
            software_gain_db: choice.software_gain_db,
            worker: Some(worker),
            boundaries: Vec::new(),
            ended: None,
        });
        self.status = if play {
            PlaybackStatus::Playing
        } else {
            PlaybackStatus::Paused
        };
    }

    fn close_session(&mut self) {
        if let Some(mut s) = self.session.take() {
            s.stop_worker();
            let _ = s.stream.pause();
            // Dropping the stream releases the device.
        }
    }

    /// Remembers the audible track and position before the stream goes away.
    fn park(&mut self) {
        let parked = self.session.as_ref().and_then(|s| {
            let played = s.stream.played_frames();
            let b = s.boundaries.first()?;
            Some(Parked {
                track: b.track.meta.clone(),
                position_ms: s.position_ms(played).unwrap_or(0.0),
            })
        });
        if parked.is_some() {
            self.parked = parked;
        }
    }

    fn park_track(&mut self, meta: &TrackMeta, position_ms: f64) {
        self.parked = Some(Parked {
            track: meta.clone(),
            position_ms,
        });
    }

    fn finish_queue(&mut self) {
        self.park();
        self.close_session();
        self.status = PlaybackStatus::Stopped;
        if let Some(p) = &mut self.parked {
            p.position_ms = 0.0;
        }
    }

    fn on_device_error(&mut self, error: OutputError) {
        tracing::warn!("output error: {error}");
        self.report(error.code(), error.to_string());
        self.park();
        self.close_session();
        self.caps.clear();
        self.status = if self.current.is_some() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Stopped
        };
    }

    fn report(&self, code: &str, message: String) {
        (self.emit)(EngineEvent::Error {
            code: code.to_string(),
            message,
        });
    }

    fn position_ms(&self) -> f64 {
        match &self.session {
            Some(s) => s.position_ms(s.stream.played_frames()).unwrap_or(0.0),
            None => self.parked.as_ref().map(|p| p.position_ms).unwrap_or(0.0),
        }
    }

    fn tick(&mut self) {
        while let Ok((generation, event)) = self.worker_rx.try_recv() {
            if generation != self.generation {
                continue;
            }
            match event {
                WorkerEvent::TrackStarted {
                    at_frame,
                    mut track,
                } => {
                    if let (Some(key), Some(cover)) =
                        (track.meta.cover_key.clone(), track.cover.take())
                    {
                        self.covers.insert(key, cover);
                    }
                    if let Some(s) = &mut self.session {
                        let badge = OutputBadge::new(
                            s.plan.mode,
                            track.meta.source,
                            track.negotiated.format,
                            track.negotiated.conversion,
                            s.gain(),
                        );
                        s.boundaries.push(Boundary {
                            at_frame,
                            track: *track,
                            badge,
                        });
                    }
                }
                WorkerEvent::Ended { at_frame, next } => {
                    if let Some(s) = &mut self.session {
                        s.ended = Some((at_frame, next));
                    }
                }
                WorkerEvent::TrackFailed { pos, error } => {
                    let name = self
                        .queue
                        .lock()
                        .get(pos)
                        .map(|item| item.path.display().to_string())
                        .unwrap_or_default();
                    self.report("trackFailed", format!("Skipped {name}: {error}"));
                }
            }
        }

        let Some(s) = &mut self.session else { return };
        if let Some(error) = s.stream.take_error() {
            self.on_device_error(error);
            return;
        }
        let played = s.stream.played_frames();

        // The audible track is the last boundary the device has reached.
        if let Some(i) = s.boundaries.iter().rposition(|b| b.at_frame <= played)
            && i > 0
        {
            s.boundaries.drain(..i);
        }
        if let Some(b) = s.boundaries.first() {
            self.current = Some(b.track.pos);
        }

        let drained = s.ended.as_ref().is_some_and(|(end, _)| played >= *end);
        if drained {
            let (_, next) = s.ended.take().expect("checked");
            match next {
                Some(track) => {
                    // The next track needs another device format: reopen with a pre-roll.
                    let play = self.status == PlaybackStatus::Playing;
                    self.close_session();
                    match self.plan() {
                        Ok(choice) => {
                            self.current = Some(track.pos);
                            self.open_session(*track, choice, play);
                        }
                        Err(e) => self.on_device_error(e),
                    }
                }
                None => self.finish_queue(),
            }
        }
    }

    fn build_snapshot(&self) -> PlayerSnapshot {
        let queue = self.queue.lock();
        let mut snap = PlayerSnapshot {
            status: self.status,
            track: None,
            queue_index: self.current.map(|c| c as u32),
            queue_length: queue.len() as u32,
            position_ms: 0.0,
            repeat: queue.repeat,
            output: None,
        };
        drop(queue);
        match &self.session {
            Some(s) if !s.boundaries.is_empty() => {
                let b = &s.boundaries[0];
                snap.track = Some(b.track.meta.clone());
                snap.position_ms = s.position_ms(s.stream.played_frames()).unwrap_or(0.0);
                snap.output = Some(OutputState {
                    device_name: s.device_name.clone(),
                    requested: if self.settings.exclusive {
                        ShareMode::Exclusive
                    } else {
                        ShareMode::Shared
                    },
                    active: s.plan.mode,
                    fallback_reason: s.fallback_reason.clone(),
                    badge: b.badge.clone(),
                    stats: s.stream.stats(),
                });
            }
            _ => {
                if let Some(p) = &self.parked {
                    snap.track = Some(p.track.clone());
                    snap.position_ms = p.position_ms;
                }
            }
        }
        snap
    }

    /// Updates the shared snapshot and emits state/position events.
    fn publish(&mut self) {
        let snap = self.build_snapshot();
        *self.snapshot.write() = snap.clone();

        let mut comparable = snap.clone();
        comparable.position_ms = 0.0;
        if let Some(o) = &mut comparable.output {
            o.stats = StreamStats::default();
        }
        let changed = self.last_state.as_ref() != Some(&comparable);
        if changed {
            self.last_state = Some(comparable);
            (self.emit)(EngineEvent::State(Box::new(snap)));
            self.last_position_emit = Instant::now();
        } else if self.status == PlaybackStatus::Playing
            && self.last_position_emit.elapsed() >= Duration::from_millis(250)
        {
            self.last_position_emit = Instant::now();
            (self.emit)(EngineEvent::Position {
                position_ms: snap.position_ms,
            });
        }
    }
}
