//! An output backend that records everything it is sent. Used for end-to-end bit-exactness
//! tests and for running the engine without an audio device. By default it plays as fast as
//! audio arrives; [`CaptureBackend::paced`] plays in (scaled) real time instead, so tests can
//! pause or seek in the middle of a track.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use pp_core::{
    DeviceCapabilities, DeviceInfo, FormatCandidate, OpenRequest, OutputBackend, OutputError,
    OutputFormat, OutputStream, RenderFeed, SampleFormat, ShareMode, StreamControls, StreamStats,
};

/// Every stream the backend opened, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    pub mode: ShareMode,
    pub format: OutputFormat,
}

pub struct CaptureBackend {
    caps: DeviceCapabilities,
    /// Bytes the device "played", across all streams.
    pub captured: Arc<Mutex<Vec<u8>>>,
    pub opened: Arc<Mutex<Vec<Opened>>>,
    /// Make `open` fail in exclusive mode with this error (fallback tests).
    pub exclusive_error: Option<OutputError>,
    /// Playback speed relative to real time; `None` plays as fast as audio arrives.
    pace: Option<f64>,
    unplugged: Arc<AtomicBool>,
}

impl CaptureBackend {
    pub fn new(caps: DeviceCapabilities) -> Self {
        Self {
            caps,
            captured: Arc::default(),
            opened: Arc::default(),
            exclusive_error: None,
            pace: None,
            unplugged: Arc::default(),
        }
    }

    /// Plays at `speed` × real time, like a device clock would.
    pub fn paced(mut self, speed: f64) -> Self {
        self.pace = Some(speed);
        self
    }

    /// Simulates unplugging the device: open streams fail, and it can't be found or opened.
    pub fn unplug(&self) {
        self.unplugged.store(true, Ordering::Release);
    }

    pub fn replug(&self) {
        self.unplugged.store(false, Ordering::Release);
    }

    /// A DAC-like device: 16-bit, 24-in-32 and 32-bit stereo at 44.1–384 kHz, with a
    /// hardware volume control, and a 48 kHz float shared-mode mix format.
    pub fn typical_dac() -> DeviceCapabilities {
        let rates = vec![
            44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000,
        ];
        DeviceCapabilities {
            device: DeviceInfo {
                id: "capture".into(),
                name: "Capture DAC".into(),
                is_default: true,
            },
            mix_format: Some(OutputFormat {
                sample_rate: 48_000,
                channels: 2,
                sample: SampleFormat::F32,
            }),
            exclusive: [
                SampleFormat::S16,
                SampleFormat::S24_IN_32,
                SampleFormat::S32,
            ]
            .into_iter()
            .map(|sample| FormatCandidate {
                sample,
                channels: 2,
                sample_rates: rates.clone(),
            })
            .collect(),
            exclusive_error: None,
            hardware_volume: true,
            default_period_us: 10_000,
            min_period_us: 3_000,
        }
    }

    pub fn take_captured(&self) -> Vec<u8> {
        std::mem::take(&mut *self.captured.lock())
    }
}

impl OutputBackend for CaptureBackend {
    fn name(&self) -> &'static str {
        "capture"
    }

    fn list_devices(&self) -> Result<Vec<DeviceInfo>, OutputError> {
        if self.unplugged.load(Ordering::Acquire) {
            return Ok(Vec::new());
        }
        Ok(vec![self.caps.device.clone()])
    }

    fn capabilities(&self, _device_id: Option<&str>) -> Result<DeviceCapabilities, OutputError> {
        if self.unplugged.load(Ordering::Acquire) {
            return Err(OutputError::DeviceNotFound);
        }
        Ok(self.caps.clone())
    }

    fn open(
        &self,
        request: &OpenRequest,
        source: rtrb::Consumer<u8>,
    ) -> Result<Box<dyn OutputStream>, OutputError> {
        if self.unplugged.load(Ordering::Acquire) {
            return Err(OutputError::DeviceNotFound);
        }
        if request.mode == ShareMode::Exclusive
            && let Some(e) = &self.exclusive_error
        {
            return Err(e.clone());
        }
        self.opened.lock().push(Opened {
            mode: request.mode,
            format: request.format,
        });
        Ok(Box::new(CaptureStream::spawn(
            request.format,
            RenderFeed::new(source, request.controls.clone(), request.format),
            self.captured.clone(),
            request.controls.clone(),
            self.pace,
            self.unplugged.clone(),
        )))
    }
}

#[derive(Default)]
struct Shared {
    running: AtomicBool,
    flush: AtomicBool,
    stop: AtomicBool,
    played: AtomicU64,
    error: Mutex<Option<OutputError>>,
}

struct CaptureStream {
    format: OutputFormat,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl CaptureStream {
    fn spawn(
        format: OutputFormat,
        mut feed: RenderFeed,
        captured: Arc<Mutex<Vec<u8>>>,
        controls: Arc<StreamControls>,
        pace: Option<f64>,
        unplugged: Arc<AtomicBool>,
    ) -> Self {
        let shared = Arc::new(Shared::default());
        let s = shared.clone();
        let bpf = format.bytes_per_frame();
        let thread = thread::spawn(move || {
            let mut scratch = vec![0u8; 64 * 1024 / bpf * bpf];
            let scratch_frames = scratch.len() / bpf;
            let rate = f64::from(format.sample_rate) * pace.unwrap_or(1.0);
            // (start, frames rendered since) while running in paced mode.
            let mut clock: Option<(Instant, u64)> = None;
            while !s.stop.load(Ordering::Acquire) {
                if unplugged.load(Ordering::Acquire) {
                    *s.error.lock() = Some(OutputError::DeviceInvalidated);
                    break;
                }
                if s.flush.load(Ordering::Acquire) {
                    feed.discard();
                    s.played.store(0, Ordering::Release);
                    s.flush.store(false, Ordering::Release);
                    continue;
                }
                if !s.running.load(Ordering::Acquire) {
                    clock = None;
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                let want = match pace {
                    None => scratch_frames,
                    Some(_) => {
                        let (start, done) = *clock.get_or_insert((Instant::now(), 0));
                        let due = (start.elapsed().as_secs_f64() * rate) as u64;
                        due.saturating_sub(done).min(scratch_frames as u64) as usize
                    }
                };
                if want == 0 {
                    thread::sleep(Duration::from_micros(500));
                    continue;
                }
                let fill = feed.fill(&mut scratch[..want * bpf]);
                if fill.silent_from.is_some() {
                    controls.set_faded_out(true);
                }
                if pace.is_none() && fill.ring_frames == 0 {
                    thread::sleep(Duration::from_micros(200));
                    continue;
                }
                if let Some((_, done)) = &mut clock {
                    *done += want as u64;
                }
                captured
                    .lock()
                    .extend_from_slice(&scratch[..fill.ring_frames * bpf]);
                s.played
                    .fetch_add(fill.ring_frames as u64, Ordering::AcqRel);
            }
        });
        Self {
            format,
            shared,
            thread: Some(thread),
        }
    }
}

impl OutputStream for CaptureStream {
    fn format(&self) -> OutputFormat {
        self.format
    }

    fn start(&mut self) -> Result<(), OutputError> {
        self.shared.running.store(true, Ordering::Release);
        Ok(())
    }

    fn pause(&mut self) -> Result<(), OutputError> {
        self.shared.running.store(false, Ordering::Release);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), OutputError> {
        self.shared.flush.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.shared.flush.load(Ordering::Acquire) {
            if Instant::now() > deadline {
                return Err(OutputError::Other("flush timed out".into()));
            }
            thread::sleep(Duration::from_micros(200));
        }
        Ok(())
    }

    fn played_frames(&self) -> u64 {
        self.shared.played.load(Ordering::Acquire)
    }

    fn stats(&self) -> StreamStats {
        StreamStats::default()
    }

    fn take_error(&mut self) -> Option<OutputError> {
        self.shared.error.lock().take()
    }
}

impl Drop for CaptureStream {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
