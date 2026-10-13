//! The decode worker: fills the output ring buffer, moving from track to track without a
//! gap while the next track needs the same device format.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::Sender;
use parking_lot::Mutex;
use pp_core::dop::DopPacker;
use pp_core::{OutputFormat, SampleFormat, ShareMode, StreamControls};
use rtrb::Producer;

use crate::effects::{apply_gain, crossfade};
use crate::pipeline::{OutputPlan, Pipeline};
use crate::queue::Queue;
use crate::track::{LoadedTrack, StartedTrack, TrackMeta, load};

/// Bytes decoded ahead of the ring before the worker waits for space.
const PENDING_HIGH_WATER: usize = 512 * 1024;
/// Audio still queued for the device when the worker commits to the next track. The next
/// file is opened early (so the switch stays gapless), but which track it is gets decided
/// this late, so queue edits made near the end of a track still count.
const COMMIT_LOW_WATER_MS: u64 = 400;

pub enum WorkerEvent {
    /// `track` becomes audible once the device has played `at_frame` frames of this stream.
    TrackStarted {
        at_frame: u64,
        track: Box<StartedTrack>,
    },
    /// The stream ends at `at_frame`. `next` is a track that needs a different device format
    /// (the stream must be reopened); `None` means the queue is finished. `version` is the
    /// queue version this was decided on.
    Ended {
        at_frame: u64,
        next: Option<Box<LoadedTrack>>,
        version: u32,
    },
    TrackFailed {
        id: u32,
        error: String,
    },
}

/// Events are tagged with the worker generation so the engine can ignore stale ones.
pub type TaggedEvent = (u64, WorkerEvent);

pub struct WorkerArgs {
    pub generation: u64,
    pub producer: Producer<u8>,
    pub format: OutputFormat,
    pub plan: OutputPlan,
    pub queue: Arc<Mutex<Queue>>,
    pub first: LoadedTrack,
    /// Silence written before the first track (lets a DAC lock onto a new rate).
    pub preroll_frames: u64,
    pub events: Sender<TaggedEvent>,
    pub controls: Arc<StreamControls>,
}

pub struct WorkerHandle {
    stop: Arc<AtomicBool>,
    join: JoinHandle<Producer<u8>>,
}

impl WorkerHandle {
    pub fn spawn(args: WorkerArgs) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let join = thread::Builder::new()
            .name("pp-decode".into())
            .spawn(move || run(args, &flag))
            .expect("spawn decode worker");
        Self { stop, join }
    }

    /// Stops decoding and hands back the ring producer, unless the worker panicked.
    pub fn stop(self) -> Option<Producer<u8>> {
        self.stop.store(true, Ordering::Relaxed);
        self.join.join().ok()
    }
}

/// Turns a track into device bytes: decoded PCM through the pipeline (with ReplayGain), or
/// raw DSD packed as DoP.
enum Stage {
    Pcm(Box<Pipeline>),
    Dop(DopPacker),
}

impl Stage {
    fn for_track(track: &LoadedTrack, format: OutputFormat) -> Result<Self, String> {
        if track.negotiated.conversion.dop {
            Ok(Self::Dop(DopPacker::new(format)))
        } else {
            Pipeline::new(track.meta.source, format).map(|p| Self::Pcm(Box::new(p)))
        }
    }

    /// Whether `next` can follow through this stage without a gap.
    fn accepts(&self, next: &LoadedTrack) -> bool {
        match self {
            Self::Pcm(p) => !next.negotiated.conversion.dop && p.accepts(&next.meta.source),
            Self::Dop(_) => next.negotiated.conversion.dop,
        }
    }

    /// Appends the next piece of `track` to `out`; false at the end of the track.
    fn pull(&mut self, track: &mut LoadedTrack, out: &mut Vec<u8>, raw: &mut Vec<u8>) -> bool {
        let result = match self {
            Self::Pcm(p) => track.decoder.next_block().map(|block| {
                block.is_some_and(|mut block| {
                    if let Some(gain) = track.gain() {
                        apply_gain(&mut block, gain);
                    }
                    p.process(block, out);
                    true
                })
            }),
            Self::Dop(d) => {
                raw.clear();
                track.decoder.next_dsd(raw).inspect(|_| d.pack(raw, out))
            }
        };
        result.unwrap_or_else(|e| {
            tracing::warn!("decoding {} stopped early: {e}", track.meta.path);
            false
        })
    }

    /// The end of a track: the resampler's tail. (DoP carries over into the next track.)
    fn end_track(&mut self, out: &mut Vec<u8>) {
        if let Self::Pcm(p) = self {
            p.finish(out);
        }
    }

    /// The end of the stream: DoP pads an odd last byte.
    fn end_stream(&mut self, out: &mut Vec<u8>) {
        if let Self::Dop(d) = self {
            d.finish(out);
        }
    }

    /// Silence of the stream's own kind: DoP idle keeps a DAC in DSD mode.
    fn silence(&mut self, frames: usize, bpf: usize, out: &mut Vec<u8>) {
        match self {
            Self::Dop(d) => d.idle(frames, out),
            Self::Pcm(_) => out.resize(out.len() + frames * bpf, 0),
        }
    }
}

/// Where decoded bytes go: `pending` (on its way to the ring), counted in `frames`. With
/// crossfading on, the newest `hold_limit` bytes of each track wait in `hold` first, to be
/// mixed into the start of the next one.
struct Out {
    pending: Vec<u8>,
    hold: Vec<u8>,
    hold_limit: usize,
    frames: u64,
    bpf: usize,
}

impl Out {
    fn emit(&mut self, bytes: &[u8]) {
        self.pending.extend_from_slice(bytes);
        self.frames += (bytes.len() / self.bpf) as u64;
    }

    fn push(&mut self, bytes: &[u8]) {
        if self.hold_limit == 0 {
            self.emit(bytes);
            return;
        }
        self.hold.extend_from_slice(bytes);
        if self.hold.len() > self.hold_limit {
            let spill = (self.hold.len() - self.hold_limit) / self.bpf * self.bpf;
            self.pending.extend(self.hold.drain(..spill));
            self.frames += (spill / self.bpf) as u64;
        }
    }

    /// Lets the held tail through unmixed.
    fn release(&mut self) {
        let held = std::mem::take(&mut self.hold);
        self.emit(&held);
    }
}

/// Consecutive tracks of one album (kept gapless unless the user wants them crossfaded).
fn same_album(a: &TrackMeta, b: &TrackMeta) -> bool {
    a.album.as_deref().is_some_and(|album| !album.is_empty()) && a.album == b.album
}

fn run(args: WorkerArgs, stop: &AtomicBool) -> Producer<u8> {
    let WorkerArgs {
        generation,
        mut producer,
        format,
        plan,
        queue,
        first,
        preroll_frames,
        events,
        controls,
    } = args;
    let send = |e: WorkerEvent| {
        let _ = events.send((generation, e));
    };
    let bpf = format.bytes_per_frame();
    let low_water = (u64::from(format.sample_rate) * COMMIT_LOW_WATER_MS / 1000) as usize * bpf;
    let low_water = low_water.min(producer.buffer().capacity() / 2);
    // Crossfading: shared mode only (exclusive mode stays bit-perfect), on float output.
    let hold_limit = if plan.mode == ShareMode::Shared
        && plan.crossfade_ms > 0
        && format.sample == SampleFormat::F32
    {
        (u64::from(format.sample_rate) * u64::from(plan.crossfade_ms) / 1000) as usize * bpf
    } else {
        0
    };
    let mut out = Out {
        pending: Vec::with_capacity(PENDING_HIGH_WATER),
        hold: Vec::new(),
        hold_limit,
        frames: 0,
        bpf,
    };

    let mut stage = match Stage::for_track(&first, format) {
        Ok(s) => s,
        Err(error) => {
            send(WorkerEvent::TrackFailed {
                id: first.id,
                error,
            });
            send(WorkerEvent::Ended {
                at_frame: 0,
                next: None,
                version: queue.lock().version(),
            });
            return producer;
        }
    };
    let mut scratch = Vec::new();
    let mut raw = Vec::new();
    stage.silence(preroll_frames as usize, bpf, &mut scratch);
    out.emit(&scratch);

    let mut track = first;
    let mut id = track.id;
    send(WorkerEvent::TrackStarted {
        at_frame: out.frames,
        track: Box::new(track.started()),
    });

    while !stop.load(Ordering::Relaxed) {
        if write_pending(&mut producer, &mut out.pending) > 0 {
            controls.set_expect_data(true);
        }
        if out.pending.len() >= PENDING_HIGH_WATER {
            thread::sleep(Duration::from_millis(5));
            continue;
        }

        scratch.clear();
        if stage.pull(&mut track, &mut scratch, &mut raw) {
            out.push(&scratch);
            continue;
        }

        // End of the current track.
        scratch.clear();
        stage.end_track(&mut scratch);
        out.push(&scratch);

        // Open the successor now, commit to it shortly before the device needs it.
        let expected = queue.lock().after(id);
        let mut next = next_playable(&queue, id, &plan, &send);
        if !drain_to_low_water(&mut producer, &mut out.pending, stop, &controls, low_water) {
            return producer;
        }
        if queue.lock().after(id) != expected {
            next = next_playable(&queue, id, &plan, &send);
        }
        let version = queue.lock().version();
        match next {
            Some(mut next) if next.negotiated.format == format => {
                if !stage.accepts(&next) {
                    match Stage::for_track(&next, format) {
                        Ok(s) => stage = s,
                        Err(error) => {
                            send(WorkerEvent::TrackFailed { id: next.id, error });
                            out.release();
                            finish_stream(&mut producer, &mut out.pending, stop, &controls);
                            send(WorkerEvent::Ended {
                                at_frame: out.frames,
                                next: None,
                                version,
                            });
                            return producer;
                        }
                    }
                }
                let mix = !out.hold.is_empty()
                    && (plan.crossfade_albums || !same_album(&track.meta, &next.meta));
                if !mix {
                    out.release();
                }
                // The next track becomes audible where the mix (or the next track) begins.
                id = next.id;
                send(WorkerEvent::TrackStarted {
                    at_frame: out.frames,
                    track: Box::new(next.started()),
                });
                if mix {
                    let mut head = Vec::with_capacity(out.hold.len());
                    while head.len() < out.hold.len() && stage.pull(&mut next, &mut head, &mut raw)
                    {
                    }
                    let tail = std::mem::take(&mut out.hold);
                    out.push(&crossfade(&tail, &head, usize::from(format.channels)));
                }
                track = next;
            }
            other => {
                out.release();
                scratch.clear();
                stage.end_stream(&mut scratch);
                out.emit(&scratch);
                finish_stream(&mut producer, &mut out.pending, stop, &controls);
                send(WorkerEvent::Ended {
                    at_frame: out.frames,
                    next: other.map(Box::new),
                    version,
                });
                return producer;
            }
        }
    }
    producer
}

/// The next track after `id` that opens, skipping (and reporting) broken files.
fn next_playable(
    queue: &Mutex<Queue>,
    mut id: u32,
    plan: &OutputPlan,
    send: &impl Fn(WorkerEvent),
) -> Option<LoadedTrack> {
    let attempts = queue.lock().len();
    for _ in 0..attempts {
        let (next, item) = {
            let q = queue.lock();
            let next = q.after(id)?;
            (next, q.get(next)?.clone())
        };
        match load(next, &item, 0, plan) {
            Ok(track) => return Some(track),
            Err(error) => {
                send(WorkerEvent::TrackFailed { id: next, error });
                if next == id {
                    return None;
                }
                id = next;
            }
        }
    }
    None
}

/// Writes everything decoded and waits until the ring holds at most `low_water` bytes.
/// Returns false when the worker was stopped meanwhile.
fn drain_to_low_water(
    producer: &mut Producer<u8>,
    pending: &mut Vec<u8>,
    stop: &AtomicBool,
    controls: &StreamControls,
    low_water: usize,
) -> bool {
    loop {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        if write_pending(producer, pending) > 0 {
            controls.set_expect_data(true);
        }
        let queued = producer.buffer().capacity() - producer.slots();
        if pending.is_empty() && queued <= low_water {
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Writes all remaining bytes (unless stopped) once nothing more will be decoded.
fn finish_stream(
    producer: &mut Producer<u8>,
    pending: &mut Vec<u8>,
    stop: &AtomicBool,
    controls: &StreamControls,
) {
    controls.set_expect_data(false);
    while !pending.is_empty() && !stop.load(Ordering::Relaxed) {
        if write_pending(producer, pending) == 0 {
            thread::sleep(Duration::from_millis(5));
        }
    }
}

fn write_pending(producer: &mut Producer<u8>, pending: &mut Vec<u8>) -> usize {
    if pending.is_empty() {
        return 0;
    }
    let (pushed, _) = producer.push_partial_slice(pending);
    let n = pushed.len();
    pending.drain(..n);
    n
}
