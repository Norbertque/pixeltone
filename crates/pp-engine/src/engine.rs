//! The engine: a control thread that owns the queue and the output session, driven by
//! commands from the app and events from the decode worker.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use parking_lot::{Mutex, RwLock};
use pp_core::{
    DeviceCapabilities, DeviceChange, Effects, EqSettings, Fade, OpenRequest, OutputBackend,
    OutputBadge, OutputError, OutputFormat, OutputStream, ShareMode, SoftwareGain, StreamControls,
    StreamStats,
};
use pp_decode::CoverArt;
use rtrb::{Producer, RingBuffer};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::effects::ReplayGainMode;
use crate::pipeline::OutputPlan;
use crate::queue::{Queue, QueueEntry, QueueItem, QueueSnapshot, RepeatMode};
use crate::track::{LoadedTrack, StartedTrack, TrackMeta, load, read_meta};
use crate::worker::{TaggedEvent, WorkerArgs, WorkerEvent, WorkerHandle};

/// Fade length for pause, resume and stop.
const PAUSE_FADE_MS: u32 = 80;
/// Fade length for seeking and skipping.
const SEEK_FADE_MS: u32 = 25;

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
    /// Exclusive mode: attenuate in the app (`software_gain_db`) even though the device has
    /// a volume control of its own, e.g. when its quietest setting is still too loud.
    pub software_volume: bool,
    /// The app's own volume in shared mode (dB, ≤ 0), applied before Windows mixes.
    pub shared_gain_db: f32,
    /// Short fades on pause, resume, seek and skip, so they don't click.
    pub fades: bool,
    /// Send DSD as DoP (exclusive mode; DACs that take the rate and have their own volume).
    pub dop: bool,
    /// The 10-band equaliser (any mode; the badge then says "Converted").
    pub eq: EqSettings,
    /// Levels tracks by their ReplayGain tags (any mode; the badge then says "Converted").
    pub replay_gain: ReplayGainMode,
    /// Added to ReplayGain values, dB.
    pub replay_gain_preamp_db: f32,
    /// Crossfade between tracks in shared mode (0 = off, gapless).
    pub crossfade_ms: u32,
    /// Crossfade between tracks of the same album too.
    pub crossfade_albums: bool,
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
            software_volume: false,
            shared_gain_db: 0.0,
            fades: true,
            dop: true,
            eq: EqSettings::default(),
            replay_gain: ReplayGainMode::Off,
            replay_gain_preamp_db: 0.0,
            crossfade_ms: 0,
            crossfade_albums: false,
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
    /// Queue item id of the current track.
    pub current_id: Option<u32>,
    pub queue_index: Option<u32>,
    pub queue_length: u32,
    /// Changes whenever the queue's items or their order change.
    pub queue_version: u32,
    pub position_ms: f64,
    pub repeat: RepeatMode,
    pub shuffle: bool,
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
        start: Option<usize>,
        shuffle: Option<bool>,
    },
    Enqueue {
        items: Vec<QueueItem>,
        next: bool,
    },
    Remove(Vec<u32>),
    Move {
        id: u32,
        to: usize,
    },
    ClearQueue,
    JumpTo(u32),
    SetShuffle(bool),
    Restore(Box<QueueSnapshot>),
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
    DeviceChanged(DeviceChange),
    /// Forget probed device capabilities (devices changed).
    RefreshDevices,
    Shutdown,
}

pub struct Engine {
    tx: Sender<Command>,
    snapshot: Arc<RwLock<PlayerSnapshot>>,
    queue: Arc<Mutex<Queue>>,
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
        let queue = Arc::new(Mutex::new(Queue::default()));
        let covers = Arc::new(CoverStore::default());
        let (worker_tx, worker_rx) = unbounded();
        let mut control = Control {
            backend,
            settings,
            config,
            queue: queue.clone(),
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
            queue,
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

    /// Queue rows `offset..offset + limit`, in play order.
    pub fn queue_entries(&self, offset: usize, limit: usize) -> Vec<QueueEntry> {
        self.queue.lock().entries(offset, limit)
    }

    /// The queue, current track and position, for saving across restarts.
    pub fn queue_snapshot(&self) -> QueueSnapshot {
        let (current, position) = {
            let s = self.snapshot.read();
            (s.current_id, s.position_ms)
        };
        self.queue.lock().snapshot(current, position)
    }

    fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }

    pub fn play_files(&self, paths: Vec<PathBuf>, start: usize) {
        self.play_items(
            paths.into_iter().map(QueueItem::from).collect(),
            Some(start),
            None,
        );
    }

    /// Replaces the queue and plays it from item `start`. `shuffle` turns shuffle on or off
    /// (`None` keeps the current mode); shuffled without a `start`, a random track leads.
    pub fn play_items(&self, items: Vec<QueueItem>, start: Option<usize>, shuffle: Option<bool>) {
        self.send(Command::PlayItems {
            items,
            start,
            shuffle,
        });
    }

    /// "Play next" (`next`) or "Add to queue".
    pub fn enqueue(&self, items: Vec<QueueItem>, next: bool) {
        self.send(Command::Enqueue { items, next });
    }

    pub fn remove_from_queue(&self, ids: Vec<u32>) {
        self.send(Command::Remove(ids));
    }

    pub fn move_in_queue(&self, id: u32, to: usize) {
        self.send(Command::Move { id, to });
    }

    /// Removes everything but the current track.
    pub fn clear_queue(&self) {
        self.send(Command::ClearQueue);
    }

    pub fn jump_to(&self, id: u32) {
        self.send(Command::JumpTo(id));
    }

    pub fn set_shuffle(&self, on: bool) {
        self.send(Command::SetShuffle(on));
    }

    /// Rebuilds a saved queue, paused at its saved position.
    pub fn restore(&self, snapshot: QueueSnapshot) {
        self.send(Command::Restore(Box::new(snapshot)));
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
    /// A device was added, removed or became the default (from the backend's watcher).
    pub fn device_changed(&self, change: DeviceChange) {
        self.send(Command::DeviceChanged(change));
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
    /// The endpoint actually opened (also when following the default device).
    device_id: String,
    device_name: String,
    fallback_reason: Option<String>,
    /// Applied software attenuation, if the gain stage is active.
    software_gain_db: Option<f32>,
    worker: Option<WorkerHandle>,
    /// Tracks queued in this stream; the first is the audible one.
    boundaries: Vec<Boundary>,
    /// Where the stream ends, the track that needs a reopen afterwards, and the queue
    /// version that was decided on.
    ended: Option<(u64, Option<Box<LoadedTrack>>, u32)>,
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

    /// The badge for a track in this stream, including the user's sound processing.
    fn badge(&self, track: &StartedTrack, eq: &EqSettings) -> OutputBadge {
        let dop = track.negotiated.conversion.dop;
        OutputBadge::new(
            self.plan.mode,
            track.meta.source,
            track.negotiated.format,
            track.negotiated.conversion,
            self.gain(),
        )
        .with_effects(Effects {
            eq: !dop && eq.active(),
            replay_gain_db: track.replay_gain_db,
        })
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
    device_id: String,
    device_name: String,
    fallback_reason: Option<String>,
    software_gain_db: Option<f32>,
}

/// Track and position shown while no stream plays them.
struct Parked {
    track: TrackMeta,
    position_ms: f64,
}

struct Control {
    backend: Arc<dyn OutputBackend>,
    settings: OutputSettings,
    config: EngineConfig,
    queue: Arc<Mutex<Queue>>,
    /// Queue id of the audible (or last) track.
    current: Option<u32>,
    status: PlaybackStatus,
    /// Track and position kept while no stream plays them (stopped, restored, paused after
    /// an error, or about to start).
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

/// Seed for shuffling.
fn seed() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos
        ^ COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

fn fade_frames(format: OutputFormat, ms: u32, enabled: bool) -> u32 {
    if enabled {
        (u64::from(format.sample_rate) * u64::from(ms) / 1000) as u32
    } else {
        0
    }
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
        self.close_session(PAUSE_FADE_MS);
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::PlayItems {
                items,
                start,
                shuffle,
            } => {
                let first = {
                    let mut q = self.queue.lock();
                    let shuffle = shuffle.unwrap_or_else(|| q.is_shuffled());
                    let ids = q.set(items);
                    let first = start.and_then(|i| ids.get(i).copied());
                    if shuffle {
                        q.shuffle(first, seed());
                        q.id_at(0)
                    } else {
                        first.or_else(|| ids.first().copied())
                    }
                };
                self.parked = None;
                match first {
                    Some(id) => self.start_at(id, 0.0, true, 0),
                    None => self.finish_queue(SEEK_FADE_MS),
                }
            }
            Command::Enqueue { items, next } => {
                let ids = {
                    let mut q = self.queue.lock();
                    if next {
                        q.insert_after(self.current, items)
                    } else {
                        q.append(items)
                    }
                };
                // Nothing loaded yet: show the first new track, ready to play.
                if self.current.is_none()
                    && let Some(&first) = ids.first()
                {
                    self.current = Some(first);
                    self.park_item(first, 0.0);
                }
            }
            Command::Remove(ids) => {
                self.queue.lock().remove(&ids, self.current);
            }
            Command::Move { id, to } => {
                self.queue.lock().move_to(id, to);
            }
            Command::ClearQueue => self.queue.lock().clear_except(self.current),
            Command::JumpTo(id) => {
                if self.queue.lock().index_of(id).is_some() {
                    self.start_at(id, 0.0, true, 0);
                }
            }
            Command::SetShuffle(on) => {
                let mut q = self.queue.lock();
                if !on {
                    q.unshuffle();
                } else if !q.is_shuffled() {
                    q.shuffle(self.current, seed());
                }
            }
            Command::Restore(snapshot) => self.restore(*snapshot),
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
                self.close_session(PAUSE_FADE_MS);
                self.status = PlaybackStatus::Stopped;
                if let Some(p) = &mut self.parked {
                    p.position_ms = 0.0;
                }
            }
            Command::Next => {
                let next = self.current.and_then(|c| self.queue.lock().next(c));
                match next {
                    Some(n) => self.skip_to(n),
                    None => self.finish_queue(SEEK_FADE_MS),
                }
            }
            Command::Previous => {
                let Some(cur) = self.current else { return };
                let previous = self.queue.lock().previous(cur);
                if self.position_ms() > 3000.0 {
                    self.skip_to(cur);
                } else if let Some(p) = previous {
                    self.skip_to(p);
                }
            }
            Command::Seek { ms } => {
                let ms = ms.max(0.0);
                if let Some(cur) = self.current {
                    if self.session.is_some() {
                        let play = self.status == PlaybackStatus::Playing;
                        self.start_at(cur, ms, play, 0);
                    } else if let Some(p) = &mut self.parked {
                        // Nothing open: just move where play will start.
                        p.position_ms = ms;
                    }
                }
            }
            Command::SetRepeat(mode) => self.queue.lock().repeat = mode,
            Command::SetOutput(settings) => self.set_output(settings),
            Command::DeviceChanged(change) => self.device_changed(change),
            Command::RefreshDevices => self.caps.clear(),
            Command::Shutdown => {}
        }
    }

    fn play(&mut self) {
        if let Some(s) = &mut self.session {
            // DoP can't be faded: it would stop being DSD.
            let fades = self.settings.fades && !s.controls.dop();
            s.controls
                .set_fade_frames(fade_frames(s.format, PAUSE_FADE_MS, fades));
            s.controls
                .set_fade(if fades { Fade::In } else { Fade::Audible });
            match s.stream.start() {
                Ok(()) => self.status = PlaybackStatus::Playing,
                Err(e) => self.on_device_error(e),
            }
            return;
        }
        let resume = self.parked.as_ref().map(|p| p.position_ms).unwrap_or(0.0);
        let id = self.current.or_else(|| self.queue.lock().id_at(0));
        if let Some(id) = id {
            self.start_at(id, resume, true, 0);
        }
    }

    /// Next/previous: plays from the top of `id` if a stream is open; otherwise only moves
    /// there, so the device isn't opened (or taken in exclusive mode) before Play.
    fn skip_to(&mut self, id: u32) {
        if self.session.is_some() {
            let play = self.status == PlaybackStatus::Playing;
            self.start_at(id, 0.0, play, 0);
        } else {
            self.current = Some(id);
            self.park_item(id, 0.0);
        }
    }

    fn pause(&mut self) {
        if self.session.is_none() || self.status != PlaybackStatus::Playing {
            return;
        }
        self.fade_out(PAUSE_FADE_MS);
        let s = self.session.as_mut().expect("checked");
        if let Err(e) = s.stream.pause() {
            self.on_device_error(e);
            return;
        }
        self.status = PlaybackStatus::Paused;
    }

    /// Fades the playing stream out and waits until the device has played the silence.
    fn fade_out(&self, ms: u32) {
        let Some(s) = &self.session else { return };
        if self.status != PlaybackStatus::Playing
            || !self.settings.fades
            || ms == 0
            || s.controls.dop()
        {
            return;
        }
        s.controls.set_fade_frames(fade_frames(s.format, ms, true));
        s.controls.set_fade(Fade::Out);
        let deadline =
            Instant::now() + Duration::from_millis(u64::from(ms + self.settings.buffer_ms) + 250);
        while !s.controls.faded_out() {
            if Instant::now() > deadline {
                tracing::debug!("fade-out not confirmed by the device; continuing");
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn set_output(&mut self, settings: OutputSettings) {
        let previous = std::mem::replace(&mut self.settings, settings);
        if previous.device_id != self.settings.device_id {
            self.caps.clear();
        }
        // Volume, fades and the equaliser apply to the open stream as they are.
        let live = OutputSettings {
            software_gain_db: self.settings.software_gain_db,
            shared_gain_db: self.settings.shared_gain_db,
            fades: self.settings.fades,
            eq: self.settings.eq.clone(),
            ..previous.clone()
        } == self.settings;
        if let Some(s) = &mut self.session {
            if live {
                if s.software_gain_db.is_some() {
                    let db = match s.plan.mode {
                        ShareMode::Shared => self.settings.shared_gain_db,
                        ShareMode::Exclusive => self.settings.software_gain_db,
                    };
                    s.software_gain_db = Some(db);
                    s.controls.set_gain(SoftwareGain::new(db));
                }
                if !s.controls.dop() {
                    s.controls.set_eq(Some(self.settings.eq.clone()));
                }
                let badges: Vec<OutputBadge> = s
                    .boundaries
                    .iter()
                    .map(|b| s.badge(&b.track, &self.settings.eq))
                    .collect();
                for (b, badge) in s.boundaries.iter_mut().zip(badges) {
                    b.badge = badge;
                }
                return;
            }
            // Device, mode or timing changed: reopen at the same spot.
            self.reopen();
        }
    }

    /// Closes the stream and opens it again (new device or settings) where it was.
    fn reopen(&mut self) {
        if let Some(cur) = self.current {
            let ms = self.position_ms();
            let play = self.status == PlaybackStatus::Playing;
            self.park();
            self.close_session(SEEK_FADE_MS);
            self.start_at(cur, ms, play, 0);
        }
    }

    fn device_changed(&mut self, change: DeviceChange) {
        // Formats and volume controls may differ now; probe again when needed.
        self.caps.clear();
        let Some(s) = &self.session else { return };
        let device = s.device_id.clone();
        if change.lost() == Some(device.as_str()) {
            self.on_device_error(OutputError::DeviceInvalidated);
            return;
        }
        if let DeviceChange::DefaultChanged { id } = &change
            && self.settings.device_id.is_none()
            && id.as_deref() != Some(device.as_str())
        {
            // Following the system default. If the old device is still there the user
            // switched devices, so playback moves along; if it went away (headphones
            // unplugged), pause like any disconnect instead of blasting the speakers.
            let still_there = self
                .backend
                .list_devices()
                .is_ok_and(|list| list.iter().any(|d| d.id == device));
            if still_there {
                self.reopen();
            } else {
                self.on_device_error(OutputError::DeviceInvalidated);
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
        let s = &self.settings;
        // Exclusive mode attenuates in the app only where the device can't (or the user
        // prefers it); shared mode always has the app's own volume.
        let software_gain_db = match mode {
            ShareMode::Shared => Some(s.shared_gain_db),
            ShareMode::Exclusive if s.line_level => None,
            ShareMode::Exclusive if caps.hardware_volume && !s.software_volume => None,
            ShareMode::Exclusive => Some(s.software_gain_db),
        };
        let gain_stage = mode == ShareMode::Exclusive && software_gain_db.is_some();
        Ok(PlanChoice {
            plan: OutputPlan {
                mode,
                exclusive: if mode == ShareMode::Exclusive {
                    caps.exclusive.clone()
                } else {
                    Vec::new()
                },
                mix_format: caps.mix_format,
                prefer_deepest: gain_stage || s.replay_gain != ReplayGainMode::Off,
                // DoP can't be attenuated: only where the DAC has its own volume (or is fed
                // to an amplifier at line level).
                dop: s.dop && mode == ShareMode::Exclusive && !gain_stage,
                replay_gain: s.replay_gain,
                replay_gain_preamp_db: s.replay_gain_preamp_db,
                crossfade_ms: if mode == ShareMode::Shared {
                    s.crossfade_ms.min(12_000)
                } else {
                    0
                },
                crossfade_albums: s.crossfade_albums,
            },
            device_id: caps.device.id.clone(),
            device_name: caps.device.name.clone(),
            fallback_reason,
            software_gain_db,
        })
    }

    /// Starts queue item `id` at `ms`, reusing the open stream when its format fits.
    fn start_at(&mut self, id: u32, ms: f64, play: bool, attempt: usize) {
        let Some(item) = self.queue.lock().get(id).cloned() else {
            self.finish_queue(SEEK_FADE_MS);
            return;
        };
        let choice = match self.plan() {
            Ok(c) => c,
            Err(e) => {
                self.report(e.code(), e.to_string());
                self.park();
                self.close_session(0);
                self.status = if self.parked.is_some() {
                    PlaybackStatus::Paused
                } else {
                    PlaybackStatus::Stopped
                };
                return;
            }
        };
        let mut track = match load(id, &item, ms as u64, &choice.plan) {
            Ok(t) => t,
            Err(error) => {
                self.report(
                    "trackFailed",
                    format!("Can't play {}: {error}", item.path.display()),
                );
                let (next, len) = {
                    let q = self.queue.lock();
                    (q.after(id), q.len())
                };
                match next {
                    Some(n) if n != id && attempt + 1 < len => {
                        self.start_at(n, 0.0, play, attempt + 1)
                    }
                    _ => self.finish_queue(SEEK_FADE_MS),
                }
                return;
            }
        };
        self.current = Some(id);
        // Shown until the device reaches the track; its cover must be servable by then.
        if let (Some(key), Some(cover)) = (track.meta.cover_key.clone(), track.cover.take()) {
            self.covers.insert(key, cover);
        }
        self.park_track(&track.meta, ms);

        let reusable = self
            .session
            .as_ref()
            .is_some_and(|s| s.plan == choice.plan && s.format == track.negotiated.format);
        if reusable {
            self.fade_out(SEEK_FADE_MS);
            let s = self.session.as_mut().expect("checked");
            let fades = self.settings.fades && !s.controls.dop();
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
                    // Mid-track starts fade in; a track's own beginning needs no fade.
                    s.controls
                        .set_fade_frames(fade_frames(s.format, SEEK_FADE_MS, fades));
                    s.controls.set_fade(if fades && ms > 0.0 {
                        Fade::In
                    } else {
                        Fade::Audible
                    });
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
        self.close_session(SEEK_FADE_MS);
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
        // DoP is DSD in disguise: no gain, EQ or fades may touch it.
        let dop = track.negotiated.conversion.dop;
        controls.set_dop(dop);
        controls.set_eq((!dop).then(|| self.settings.eq.clone()));
        let fades = self.settings.fades && !dop;
        controls.set_fade_frames(fade_frames(format, PAUSE_FADE_MS, fades));
        controls.set_fade(if fades && track.start_ms > 0 {
            Fade::In
        } else {
            Fade::Audible
        });
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
                        prefer_deepest: false,
                        dop: false,
                        crossfade_ms: self.settings.crossfade_ms.min(12_000),
                        ..choice.plan.clone()
                    },
                    device_id: choice.device_id,
                    device_name: choice.device_name,
                    fallback_reason: Some(e.to_string()),
                    software_gain_db: Some(self.settings.shared_gain_db),
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
                self.status = PlaybackStatus::Paused;
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
            device_id: choice.device_id,
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

    /// Releases the device, fading out first (by `fade_ms`) if music is playing.
    fn close_session(&mut self, fade_ms: u32) {
        self.fade_out(fade_ms);
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

    /// Shows a queue item that isn't playing (restored or freshly queued).
    fn park_item(&mut self, id: u32, position_ms: f64) {
        let Some(item) = self.queue.lock().get(id).cloned() else {
            return;
        };
        match read_meta(&item) {
            Ok((meta, cover)) => {
                if let (Some(key), Some(cover)) = (meta.cover_key.clone(), cover) {
                    self.covers.insert(key, cover);
                }
                self.park_track(&meta, position_ms);
            }
            Err(e) => {
                tracing::debug!("can't read {}: {e}", item.path.display());
                self.parked = None;
            }
        }
    }

    fn restore(&mut self, snapshot: QueueSnapshot) {
        self.close_session(0);
        let position = snapshot.position_ms.max(0.0);
        let current = self.queue.lock().restore(snapshot);
        self.current = current;
        self.parked = None;
        if let Some(id) = current {
            self.park_item(id, position);
        }
        self.status = if self.parked.is_some() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Stopped
        };
    }

    fn finish_queue(&mut self, fade_ms: u32) {
        self.park();
        self.close_session(fade_ms);
        self.status = PlaybackStatus::Stopped;
        if let Some(p) = &mut self.parked {
            p.position_ms = 0.0;
        }
    }

    fn on_device_error(&mut self, error: OutputError) {
        tracing::warn!("output error: {error}");
        self.report(error.code(), error.to_string());
        self.park();
        self.close_session(0);
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
            Some(s) => s
                .position_ms(s.stream.played_frames())
                .or_else(|| self.parked.as_ref().map(|p| p.position_ms))
                .unwrap_or(0.0),
            None => self.parked.as_ref().map(|p| p.position_ms).unwrap_or(0.0),
        }
    }

    fn tick(&mut self) {
        let mut started = false;
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
                        let badge = s.badge(&track, &self.settings.eq);
                        s.boundaries.push(Boundary {
                            at_frame,
                            track: *track,
                            badge,
                        });
                        started = true;
                    }
                }
                WorkerEvent::Ended {
                    at_frame,
                    next,
                    version,
                } => {
                    if let Some(s) = &mut self.session {
                        s.ended = Some((at_frame, next, version));
                    }
                }
                WorkerEvent::TrackFailed { id, error } => {
                    let name = self
                        .queue
                        .lock()
                        .get(id)
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
            self.current = Some(b.track.id);
        }

        let drained = s.ended.as_ref().is_some_and(|(end, ..)| played >= *end);
        if drained {
            let (_, next, version) = s.ended.take().expect("checked");
            // A track shorter than a tick can start and drain between two ticks; announce
            // it (title, badge) before the session closes, or it would never be published.
            if started {
                self.publish();
            }
            let play = self.status == PlaybackStatus::Playing;
            // The queue may have been edited since the worker decided what comes next.
            let edited = self.queue.lock().version() != version;
            let successor = if edited {
                self.current.and_then(|c| self.queue.lock().after(c))
            } else {
                next.as_ref().map(|t| t.id)
            };
            match (next, successor) {
                (Some(track), Some(id)) if track.id == id => {
                    // The next track needs another device format: reopen with a pre-roll.
                    self.close_session(0);
                    match self.plan() {
                        Ok(choice) => {
                            self.current = Some(track.id);
                            self.park_track(&track.meta, 0.0);
                            self.open_session(*track, choice, play);
                        }
                        Err(e) => self.on_device_error(e),
                    }
                }
                (_, Some(id)) => {
                    self.close_session(0);
                    self.start_at(id, 0.0, play, 0);
                }
                (_, None) => self.finish_queue(0),
            }
        }
    }

    fn build_snapshot(&self) -> PlayerSnapshot {
        let queue = self.queue.lock();
        let mut snap = PlayerSnapshot {
            status: self.status,
            track: None,
            current_id: self.current,
            queue_index: self
                .current
                .and_then(|id| queue.index_of(id))
                .map(|i| i as u32),
            queue_length: queue.len() as u32,
            queue_version: queue.version(),
            position_ms: 0.0,
            repeat: queue.repeat,
            shuffle: queue.is_shuffled(),
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
