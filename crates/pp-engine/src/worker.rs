//! The decode worker: fills the output ring buffer, moving from track to track without a
//! gap while the next track needs the same device format.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::Sender;
use parking_lot::Mutex;
use pp_core::{OutputFormat, StreamControls};
use rtrb::Producer;

use crate::pipeline::{OutputPlan, Pipeline};
use crate::queue::Queue;
use crate::track::{LoadedTrack, StartedTrack, load};

/// Bytes decoded ahead of the ring before the worker waits for space.
const PENDING_HIGH_WATER: usize = 512 * 1024;

pub enum WorkerEvent {
    /// `track` becomes audible once the device has played `at_frame` frames of this stream.
    TrackStarted {
        at_frame: u64,
        track: Box<StartedTrack>,
    },
    /// The stream ends at `at_frame`. `next` is a track that needs a different device format
    /// (the stream must be reopened); `None` means the queue is finished.
    Ended {
        at_frame: u64,
        next: Option<Box<LoadedTrack>>,
    },
    TrackFailed {
        pos: usize,
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
    let mut pending: Vec<u8> = Vec::with_capacity(PENDING_HIGH_WATER);
    let mut frames: u64 = preroll_frames;
    pending.resize(preroll_frames as usize * bpf, 0);

    let mut pipeline = match Pipeline::new(first.meta.source, format) {
        Ok(p) => p,
        Err(error) => {
            send(WorkerEvent::TrackFailed {
                pos: first.pos,
                error,
            });
            send(WorkerEvent::Ended {
                at_frame: frames,
                next: None,
            });
            return producer;
        }
    };
    let mut track = first;
    let mut pos = track.pos;
    send(WorkerEvent::TrackStarted {
        at_frame: frames,
        track: Box::new(track.started()),
    });

    while !stop.load(Ordering::Relaxed) {
        if write_pending(&mut producer, &mut pending) > 0 {
            controls.set_expect_data(true);
        }
        if pending.len() >= PENDING_HIGH_WATER {
            thread::sleep(Duration::from_millis(5));
            continue;
        }

        let block = match track.decoder.next_block() {
            Ok(block) => block,
            Err(e) => {
                tracing::warn!("decoding {} stopped early: {e}", track.meta.path);
                None
            }
        };
        if let Some(block) = block {
            let before = pending.len();
            pipeline.process(block, &mut pending);
            frames += ((pending.len() - before) / bpf) as u64;
            continue;
        }

        // End of the current track.
        let before = pending.len();
        pipeline.finish(&mut pending);
        frames += ((pending.len() - before) / bpf) as u64;

        let next = next_playable(&queue, pos, &plan, &send);
        match next {
            Some(mut next) if next.negotiated.format == format => {
                if !pipeline.accepts(&next.meta.source) {
                    match Pipeline::new(next.meta.source, format) {
                        Ok(p) => pipeline = p,
                        Err(error) => {
                            send(WorkerEvent::TrackFailed {
                                pos: next.pos,
                                error,
                            });
                            finish_stream(&mut producer, &mut pending, stop, &controls);
                            send(WorkerEvent::Ended {
                                at_frame: frames,
                                next: None,
                            });
                            return producer;
                        }
                    }
                }
                pos = next.pos;
                send(WorkerEvent::TrackStarted {
                    at_frame: frames,
                    track: Box::new(next.started()),
                });
                track = next;
            }
            other => {
                finish_stream(&mut producer, &mut pending, stop, &controls);
                send(WorkerEvent::Ended {
                    at_frame: frames,
                    next: other.map(Box::new),
                });
                return producer;
            }
        }
    }
    producer
}

/// The next track after `pos` that opens, skipping (and reporting) broken files.
fn next_playable(
    queue: &Mutex<Queue>,
    mut pos: usize,
    plan: &OutputPlan,
    send: &impl Fn(WorkerEvent),
) -> Option<LoadedTrack> {
    let attempts = queue.lock().len();
    for _ in 0..attempts {
        let (next, item) = {
            let q = queue.lock();
            let next = q.after(pos)?;
            (next, q.get(next)?.clone())
        };
        match load(next, &item, 0, plan) {
            Ok(track) => return Some(track),
            Err(error) => {
                send(WorkerEvent::TrackFailed { pos: next, error });
                if next == pos {
                    return None;
                }
                pos = next;
            }
        }
    }
    None
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
