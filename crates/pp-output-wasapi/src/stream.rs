//! A WASAPI stream: a render thread (MMCSS "Pro Audio") that moves bytes from the ring
//! buffer into the device, in shared or exclusive mode.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use pp_core::{
    OpenRequest, OutputError, OutputFormat, OutputStream, RenderFeed, ShareMode, StreamStats,
};
use wasapi::{AudioClient, Direction, Handle, StreamMode, WasapiError, calculate_period_100ns};
use windows::Win32::Media::Audio::AUDCLNT_E_BUFFER_SIZE_NOT_ALIGNED;
use windows::Win32::System::Threading::{
    AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW,
};
use windows::core::w;

use crate::backend::{find_device, init_com};
use crate::convert::{describe, map_error, to_wave_format};

#[derive(Default)]
struct Shared {
    running: AtomicBool,
    flush: AtomicBool,
    stop: AtomicBool,
    /// Ring frames the device has played since open/flush.
    played: AtomicU64,
    underruns: AtomicU32,
    buffer_frames: AtomicU32,
    period_frames: AtomicU32,
    error: Mutex<Option<OutputError>>,
}

pub struct WasapiStream {
    format: OutputFormat,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl WasapiStream {
    /// Opens the device on a new render thread; errors from initialisation are returned here.
    pub fn open(request: OpenRequest, source: rtrb::Consumer<u8>) -> Result<Self, OutputError> {
        let shared = Arc::new(Shared::default());
        let (ready_tx, ready_rx) = mpsc::channel();
        let format = request.format;
        let s = shared.clone();
        let thread = thread::Builder::new()
            .name("pp-render".into())
            .spawn(move || render_thread(request, source, s, ready_tx))
            .map_err(|e| OutputError::Other(e.to_string()))?;
        match ready_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => Ok(Self {
                format,
                shared,
                thread: Some(thread),
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                shared.stop.store(true, Ordering::Release);
                Err(OutputError::Other(
                    "the audio device did not respond".into(),
                ))
            }
        }
    }
}

impl OutputStream for WasapiStream {
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
            if let Some(e) = self.shared.error.lock().clone() {
                return Err(e);
            }
            if Instant::now() > deadline {
                return Err(OutputError::Other(
                    "the audio device did not respond to a flush".into(),
                ));
            }
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    fn played_frames(&self) -> u64 {
        self.shared.played.load(Ordering::Acquire)
    }

    fn stats(&self) -> StreamStats {
        StreamStats {
            underruns: self.shared.underruns.load(Ordering::Relaxed),
            buffer_frames: self.shared.buffer_frames.load(Ordering::Relaxed),
            period_frames: self.shared.period_frames.load(Ordering::Relaxed),
        }
    }

    fn take_error(&mut self) -> Option<OutputError> {
        self.shared.error.lock().take()
    }
}

impl Drop for WasapiStream {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Raises the thread's scheduling class to "Pro Audio" while alive.
struct Mmcss(Option<windows::Win32::Foundation::HANDLE>);

impl Mmcss {
    fn pro_audio() -> Self {
        let mut task = 0u32;
        Self(unsafe { AvSetMmThreadCharacteristicsW(w!("Pro Audio"), &mut task) }.ok())
    }
}

impl Drop for Mmcss {
    fn drop(&mut self) {
        if let Some(h) = self.0 {
            let _ = unsafe { AvRevertMmThreadCharacteristics(h) };
        }
    }
}

struct Device {
    client: AudioClient,
    render: wasapi::AudioRenderClient,
    clock: wasapi::AudioClock,
    event: Option<Handle>,
    buffer_frames: u32,
    period: Duration,
}

/// One device write: `ring` frames from the ring buffer, then `total - ring` of silence.
struct Segment {
    device_start: u64,
    ring_start: u64,
    ring: u64,
    total: u64,
}

/// Maps the device clock to frames of the ring buffer actually heard, skipping silence the
/// render thread inserted (pre-fill, underruns).
#[derive(Default)]
struct Accounting {
    segments: VecDeque<Segment>,
    device_written: u64,
    ring_consumed: u64,
    ring_played: u64,
}

impl Accounting {
    fn wrote(&mut self, ring: u64, total: u64) {
        self.segments.push_back(Segment {
            device_start: self.device_written,
            ring_start: self.ring_consumed,
            ring,
            total,
        });
        self.device_written += total;
        self.ring_consumed += ring;
    }

    fn played(&mut self, device_played: u64) -> u64 {
        while let Some(front) = self.segments.front() {
            if front.device_start + front.total <= device_played {
                self.ring_played = front.ring_start + front.ring;
                self.segments.pop_front();
            } else {
                break;
            }
        }
        if let Some(front) = self.segments.front()
            && device_played > front.device_start
        {
            self.ring_played =
                front.ring_start + front.ring.min(device_played - front.device_start);
        }
        self.ring_played
    }
}

fn open_device(request: &OpenRequest) -> Result<Device, OutputError> {
    let device = find_device(request.device_id.as_deref())?;
    let mut client = device.get_iaudioclient().map_err(map_error)?;
    let wanted = to_wave_format(&request.format);
    let exclusive = request.mode == ShareMode::Exclusive;
    let event_driven = !exclusive || request.event_driven;

    let format = if exclusive {
        client
            .is_supported_exclusive_with_quirks(&wanted)
            .map_err(|_| OutputError::UnsupportedFormat(describe(&request.format)))?
    } else {
        wanted
    };

    let (default_period, _min) = client.get_device_period().map_err(map_error)?;
    let buffer_hns = i64::from(request.buffer_ms.max(10)) * 10_000;
    let mode_for = |period: i64| match (exclusive, event_driven) {
        (true, true) => StreamMode::EventsExclusive { period_hns: period },
        (true, false) => StreamMode::PollingExclusive {
            buffer_duration_hns: buffer_hns.max(period * 3),
            period_hns: period,
        },
        (false, _) => StreamMode::EventsShared {
            autoconvert: false,
            buffer_duration_hns: buffer_hns,
        },
    };
    let period = if exclusive {
        client
            .calculate_aligned_period_near(default_period, Some(128), &format)
            .map_err(map_error)?
    } else {
        default_period
    };

    match client.initialize_client(&format, &Direction::Render, &mode_for(period)) {
        Ok(()) => {}
        Err(WasapiError::Windows(e)) if e.code() == AUDCLNT_E_BUFFER_SIZE_NOT_ALIGNED => {
            // The driver wants a different period: use the buffer size it suggested.
            let frames = client.get_buffer_size().map_err(map_error)?;
            let aligned =
                calculate_period_100ns(i64::from(frames), i64::from(format.get_samplespersec()));
            client = device.get_iaudioclient().map_err(map_error)?;
            client
                .initialize_client(&format, &Direction::Render, &mode_for(aligned))
                .map_err(map_error)?;
        }
        Err(e) => return Err(map_error(e)),
    }

    let event = if event_driven {
        Some(client.set_get_eventhandle().map_err(map_error)?)
    } else {
        None
    };
    let render = client.get_audiorenderclient().map_err(map_error)?;
    let clock = client.get_audioclock().map_err(map_error)?;
    let buffer_frames = client.get_buffer_size().map_err(map_error)?;
    let period = Duration::from_nanos(u64::try_from(default_period).unwrap_or(100_000) * 100);
    Ok(Device {
        client,
        render,
        clock,
        event,
        buffer_frames,
        period,
    })
}

fn render_thread(
    request: OpenRequest,
    source: rtrb::Consumer<u8>,
    shared: Arc<Shared>,
    ready: mpsc::Sender<Result<(), OutputError>>,
) {
    init_com();
    let _mmcss = Mmcss::pro_audio();
    let dev = match open_device(&request) {
        Ok(d) => d,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    shared
        .buffer_frames
        .store(dev.buffer_frames, Ordering::Relaxed);
    let rate = u64::from(request.format.sample_rate);
    let period_frames = (dev.period.as_secs_f64() * rate as f64).round() as u32;
    shared.period_frames.store(period_frames, Ordering::Relaxed);
    let _ = ready.send(Ok(()));

    let mut feed = RenderFeed::new(source, request.controls.clone(), request.format);
    if let Err(e) = run(&request, &dev, &mut feed, &shared) {
        tracing::warn!("render thread stopped: {e}");
        *shared.error.lock() = Some(e);
    }
    let _ = dev.client.stop_stream();
}

fn run(
    request: &OpenRequest,
    dev: &Device,
    feed: &mut RenderFeed,
    shared: &Shared,
) -> Result<(), OutputError> {
    let format = request.format;
    let controls = &request.controls;
    let bpf = format.bytes_per_frame();
    let rate = u64::from(format.sample_rate);
    let frequency = dev.clock.get_frequency().map_err(map_error)?.max(1);
    let mut scratch = vec![0u8; dev.buffer_frames as usize * bpf];
    let mut accounting = Accounting::default();
    let mut started = false;
    // Device frame where fade-out silence begins; the fade is over once that is played.
    let mut silence_at: Option<u64> = None;

    // Exclusive event mode must be pre-filled with a full buffer before starting.
    let prefill = |accounting: &mut Accounting, scratch: &mut [u8]| -> Result<(), OutputError> {
        let frames = dev
            .client
            .get_available_space_in_frames()
            .map_err(map_error)? as usize;
        scratch[..frames * bpf].fill(0);
        dev.render
            .write_to_device(frames, &scratch[..frames * bpf], None)
            .map_err(map_error)?;
        accounting.wrote(0, frames as u64);
        Ok(())
    };
    prefill(&mut accounting, &mut scratch)?;

    while !shared.stop.load(Ordering::Acquire) {
        if shared.flush.load(Ordering::Acquire) {
            if started {
                dev.client.stop_stream().map_err(map_error)?;
                started = false;
            }
            dev.client.reset_stream().map_err(map_error)?;
            feed.discard();
            silence_at = None;
            accounting = Accounting::default();
            shared.played.store(0, Ordering::Release);
            prefill(&mut accounting, &mut scratch)?;
            shared.flush.store(false, Ordering::Release);
            continue;
        }

        let want_running = shared.running.load(Ordering::Acquire);
        if want_running != started {
            if want_running {
                dev.client.start_stream().map_err(map_error)?;
            } else {
                dev.client.stop_stream().map_err(map_error)?;
            }
            started = want_running;
        }
        if !started {
            thread::sleep(Duration::from_millis(5));
            continue;
        }

        match &dev.event {
            Some(event) => {
                if event.wait_for_event(200).is_err() {
                    continue;
                }
            }
            None => thread::sleep(dev.period / 2),
        }

        let space = dev
            .client
            .get_available_space_in_frames()
            .map_err(map_error)? as usize;
        if space > 0 {
            let bytes = space * bpf;
            let fill = feed.fill(&mut scratch[..bytes]);
            if fill.ring_frames < space && fill.silent_from.is_none() && controls.expect_data() {
                shared.underruns.fetch_add(1, Ordering::Relaxed);
            }
            dev.render
                .write_to_device(space, &scratch[..bytes], None)
                .map_err(map_error)?;
            match fill.silent_from {
                Some(frame) => {
                    silence_at.get_or_insert(accounting.device_written + frame as u64);
                }
                None => silence_at = None,
            }
            accounting.wrote(fill.ring_frames as u64, space as u64);
        }

        let (position, _) = dev.clock.get_position().map_err(map_error)?;
        let device_played =
            (u128::from(position) * u128::from(rate) / u128::from(frequency)) as u64;
        shared
            .played
            .store(accounting.played(device_played), Ordering::Release);
        if silence_at.is_some_and(|at| device_played >= at) {
            controls.set_faded_out(true);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Accounting;

    #[test]
    fn silence_is_not_counted_as_played_audio() {
        let mut a = Accounting::default();
        a.wrote(0, 480); // pre-fill
        a.wrote(480, 480);
        a.wrote(100, 480); // underrun: 380 frames of padding
        assert_eq!(a.played(240), 0);
        assert_eq!(a.played(480 + 200), 200);
        assert_eq!(a.played(960 + 50), 530);
        assert_eq!(a.played(960 + 400), 580);
        assert_eq!(a.played(5000), 580);
    }
}
