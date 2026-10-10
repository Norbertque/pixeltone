//! An output backend that records everything it is sent, as fast as it arrives. Used for
//! end-to-end bit-exactness tests and for running the engine without an audio device.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use pp_core::gain::apply_q16_in_place;
use pp_core::{
    DeviceCapabilities, DeviceInfo, FormatCandidate, OpenRequest, OutputBackend, OutputError,
    OutputFormat, OutputStream, SampleFormat, ShareMode, StreamControls, StreamStats,
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
}

impl CaptureBackend {
    pub fn new(caps: DeviceCapabilities) -> Self {
        Self {
            caps,
            captured: Arc::default(),
            opened: Arc::default(),
            exclusive_error: None,
        }
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
        Ok(vec![self.caps.device.clone()])
    }

    fn capabilities(&self, _device_id: Option<&str>) -> Result<DeviceCapabilities, OutputError> {
        Ok(self.caps.clone())
    }

    fn open(
        &self,
        request: &OpenRequest,
        source: rtrb::Consumer<u8>,
    ) -> Result<Box<dyn OutputStream>, OutputError> {
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
            source,
            self.captured.clone(),
            request.controls.clone(),
        )))
    }
}

struct Shared {
    running: AtomicBool,
    flush: AtomicBool,
    stop: AtomicBool,
    played: AtomicU64,
}

struct CaptureStream {
    format: OutputFormat,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl CaptureStream {
    fn spawn(
        format: OutputFormat,
        mut source: rtrb::Consumer<u8>,
        captured: Arc<Mutex<Vec<u8>>>,
        controls: Arc<StreamControls>,
    ) -> Self {
        let shared = Arc::new(Shared {
            running: AtomicBool::new(false),
            flush: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            played: AtomicU64::new(0),
        });
        let s = shared.clone();
        let bpf = format.bytes_per_frame();
        let thread = thread::spawn(move || {
            let mut scratch = vec![0u8; 64 * 1024 / bpf * bpf];
            while !s.stop.load(Ordering::Acquire) {
                if s.flush.load(Ordering::Acquire) {
                    let n = source.slots();
                    if let Ok(chunk) = source.read_chunk(n) {
                        chunk.commit_all();
                    }
                    s.played.store(0, Ordering::Release);
                    s.flush.store(false, Ordering::Release);
                    continue;
                }
                if !s.running.load(Ordering::Acquire) {
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                let frames = (source.slots() / bpf).min(scratch.len() / bpf);
                if frames == 0 {
                    thread::sleep(Duration::from_micros(200));
                    continue;
                }
                let bytes = frames * bpf;
                let (filled, _) = source.pop_partial_slice(&mut scratch[..bytes]);
                let filled = filled.len();
                if let Some(q16) = controls.gain_q16() {
                    apply_q16_in_place(&mut scratch[..filled], format.sample, q16);
                }
                captured.lock().extend_from_slice(&scratch[..filled]);
                s.played.fetch_add((filled / bpf) as u64, Ordering::AcqRel);
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
        None
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
