//! The contract between the engine and an audio output backend.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::eq::EqSettings;
use crate::format::OutputFormat;
use crate::gain::SoftwareGain;
use crate::negotiate::FormatCandidate;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ShareMode {
    /// Through the Windows mixer (resampled to its mix format).
    Shared,
    /// The app owns the device; samples reach the driver untouched.
    Exclusive,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeviceInfo {
    /// Stable endpoint id (the IMMDevice id on Windows).
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeviceCapabilities {
    pub device: DeviceInfo,
    /// The shared-mode mix format.
    pub mix_format: Option<OutputFormat>,
    /// Formats accepted in exclusive mode; empty when exclusive mode is unavailable.
    pub exclusive: Vec<FormatCandidate>,
    /// Why `exclusive` is empty, when it is.
    pub exclusive_error: Option<String>,
    /// The endpoint has a hardware volume control, so volume never touches the samples.
    pub hardware_volume: bool,
    pub default_period_us: u32,
    pub min_period_us: u32,
}

#[derive(Clone, Debug)]
pub struct OpenRequest {
    /// `None` opens the default output device.
    pub device_id: Option<String>,
    pub mode: ShareMode,
    pub format: OutputFormat,
    /// Requested device buffer length.
    pub buffer_ms: u32,
    /// Exclusive mode: event-driven timing instead of polling. Polling is the default
    /// because the standard Windows USB audio driver can stutter in event mode.
    pub event_driven: bool,
    pub controls: Arc<StreamControls>,
}

/// Transport fade requested from a stream's render thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Fade {
    /// Full level at once (playback starting from the beginning of a track).
    Audible = 0,
    /// Ramp up from the current level; the ramp waits for the first non-silent frame.
    In = 1,
    /// Ramp down to silence, then stop taking audio from the ring.
    Out = 2,
}

/// Live controls shared between the engine and a stream's render thread.
#[derive(Debug)]
pub struct StreamControls {
    /// Q16.16 software gain; negative means unity (samples untouched).
    gain_q16: AtomicI64,
    /// While set, an empty ring buffer counts as an underrun.
    expect_data: AtomicBool,
    fade: AtomicU8,
    /// Fade level change per frame (of [`crate::render::UNITY`]); 0 switches instantly.
    fade_step: AtomicU32,
    /// Set by the render side once a fade-out has reached the listener as silence.
    faded_out: AtomicBool,
    /// Equaliser settings, picked up by the render side when `eq_version` moves.
    eq: Mutex<Option<EqSettings>>,
    eq_version: AtomicU64,
    /// The stream carries DoP: silence must be DoP idle, and nothing may touch the samples.
    dop: AtomicBool,
}

impl Default for StreamControls {
    fn default() -> Self {
        Self {
            gain_q16: AtomicI64::new(-1),
            expect_data: AtomicBool::new(false),
            fade: AtomicU8::new(Fade::Audible as u8),
            fade_step: AtomicU32::new(0),
            faded_out: AtomicBool::new(false),
            eq: Mutex::new(None),
            eq_version: AtomicU64::new(0),
            dop: AtomicBool::new(false),
        }
    }
}

impl StreamControls {
    pub fn set_gain(&self, gain: SoftwareGain) {
        self.gain_q16
            .store(gain.q16().unwrap_or(-1), Ordering::Relaxed);
    }

    /// The gain to apply, or `None` at unity.
    pub fn gain_q16(&self) -> Option<i64> {
        let v = self.gain_q16.load(Ordering::Relaxed);
        (v >= 0).then_some(v)
    }

    pub fn set_expect_data(&self, expect: bool) {
        self.expect_data.store(expect, Ordering::Relaxed);
    }

    pub fn expect_data(&self) -> bool {
        self.expect_data.load(Ordering::Relaxed)
    }

    /// Requests a fade. A fade-out clears [`Self::faded_out`] until the silence is heard.
    pub fn set_fade(&self, fade: Fade) {
        if fade == Fade::Out {
            self.faded_out.store(false, Ordering::Release);
        }
        self.fade.store(fade as u8, Ordering::Release);
    }

    pub fn fade(&self) -> Fade {
        match self.fade.load(Ordering::Acquire) {
            1 => Fade::In,
            2 => Fade::Out,
            _ => Fade::Audible,
        }
    }

    /// Length of the next fade ramps in frames; 0 makes fades instant.
    pub fn set_fade_frames(&self, frames: u32) {
        let step = if frames == 0 {
            0
        } else {
            crate::render::UNITY.div_ceil(frames)
        };
        self.fade_step.store(step, Ordering::Release);
    }

    pub fn fade_step(&self) -> u32 {
        self.fade_step.load(Ordering::Acquire)
    }

    pub fn set_faded_out(&self, done: bool) {
        self.faded_out.store(done, Ordering::Release);
    }

    /// The last fade-out has been played to the end (the device is now playing silence).
    pub fn faded_out(&self) -> bool {
        self.faded_out.load(Ordering::Acquire)
    }

    /// New equaliser settings (`None` or inactive settings: off).
    pub fn set_eq(&self, eq: Option<EqSettings>) {
        *self.eq.lock().unwrap_or_else(|e| e.into_inner()) = eq;
        self.eq_version.fetch_add(1, Ordering::AcqRel);
    }

    pub fn eq_version(&self) -> u64 {
        self.eq_version.load(Ordering::Acquire)
    }

    /// The equaliser settings, unless the engine is changing them right now (the render
    /// thread never waits; it tries again on its next buffer).
    pub fn try_eq(&self) -> Option<Option<EqSettings>> {
        self.eq.try_lock().ok().map(|eq| eq.clone())
    }

    pub fn set_dop(&self, dop: bool) {
        self.dop.store(dop, Ordering::Release);
    }

    pub fn dop(&self) -> bool {
        self.dop.load(Ordering::Acquire)
    }
}

/// A change in the set of audio devices, as the backend reports it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export)]
pub enum DeviceChange {
    /// The system's default output device changed (to `id`, or to none at all).
    DefaultChanged {
        id: Option<String>,
    },
    Added {
        id: String,
    },
    Removed {
        id: String,
    },
    /// A device became available (`active`) or unavailable (unplugged, disabled).
    StateChanged {
        id: String,
        active: bool,
    },
}

impl DeviceChange {
    /// The device that went away, if this change means one did.
    pub fn lost(&self) -> Option<&str> {
        match self {
            Self::Removed { id } | Self::StateChanged { id, active: false } => Some(id),
            _ => None,
        }
    }
}

/// Keeps device-change notifications coming while it is alive.
pub type DeviceWatch = Box<dyn Send>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamStats {
    /// Device periods filled with silence because no audio was ready.
    pub underruns: u32,
    pub buffer_frames: u32,
    pub period_frames: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OutputError {
    #[error("the audio device was not found")]
    DeviceNotFound,
    #[error("the device is in use by another application in exclusive mode")]
    DeviceInUse,
    #[error(
        "exclusive mode is not allowed for this device; enable \"Allow applications to take \
         exclusive control of this device\" in the Windows Sound control panel"
    )]
    ExclusiveNotAllowed,
    #[error("the device does not accept {0}")]
    UnsupportedFormat(String),
    #[error("the audio device was disconnected")]
    DeviceInvalidated,
    #[error("{0}")]
    Other(String),
}

impl OutputError {
    /// Stable machine-readable code for the UI.
    pub fn code(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "deviceNotFound",
            Self::DeviceInUse => "deviceInUse",
            Self::ExclusiveNotAllowed => "exclusiveNotAllowed",
            Self::UnsupportedFormat(_) => "unsupportedFormat",
            Self::DeviceInvalidated => "deviceInvalidated",
            Self::Other(_) => "other",
        }
    }

    /// Errors after which the engine should fall back to shared mode.
    pub fn suggests_shared_fallback(&self) -> bool {
        matches!(
            self,
            Self::DeviceInUse | Self::ExclusiveNotAllowed | Self::UnsupportedFormat(_)
        )
    }
}

/// An audio output API: device discovery and opening streams.
pub trait OutputBackend: Send + Sync {
    fn name(&self) -> &'static str;

    fn list_devices(&self) -> Result<Vec<DeviceInfo>, OutputError>;

    /// Probes a device (`None` = default). Exclusive-mode probing may take a moment.
    fn capabilities(&self, device_id: Option<&str>) -> Result<DeviceCapabilities, OutputError>;

    /// Opens a stream that plays the bytes arriving through `source`, which must already be
    /// in `request.format`. The stream starts paused.
    fn open(
        &self,
        request: &OpenRequest,
        source: rtrb::Consumer<u8>,
    ) -> Result<Box<dyn OutputStream>, OutputError>;

    /// Hardware (endpoint) volume of a device, when it has one.
    fn hardware_volume(
        &self,
        _device_id: Option<&str>,
    ) -> Result<Option<Box<dyn HardwareVolume>>, OutputError> {
        Ok(None)
    }

    /// Reports device changes to `on_change` (from a backend thread) until the returned
    /// watch is dropped. `None` when the backend can't watch devices.
    fn watch_devices(
        &self,
        _on_change: Box<dyn Fn(DeviceChange) + Send + Sync>,
    ) -> Result<Option<DeviceWatch>, OutputError> {
        Ok(None)
    }
}

/// A running stream on one device in one format.
pub trait OutputStream: Send {
    fn format(&self) -> OutputFormat;
    fn start(&mut self) -> Result<(), OutputError>;
    fn pause(&mut self) -> Result<(), OutputError>;
    /// Discards everything queued (ring and device buffer) and resets the position. The
    /// producer must not write while this runs.
    fn flush(&mut self) -> Result<(), OutputError>;
    /// Frames the device has played since the stream was opened or last flushed.
    fn played_frames(&self) -> u64;
    fn stats(&self) -> StreamStats;
    /// A fatal error raised on the render thread (for example the device was unplugged).
    fn take_error(&mut self) -> Option<OutputError>;
}

/// Endpoint volume that changes the device's own gain, leaving samples untouched.
pub trait HardwareVolume: Send {
    /// (min, max, step) in dB.
    fn range_db(&self) -> (f32, f32, f32);
    fn get_db(&self) -> Result<f32, OutputError>;
    fn set_db(&self, db: f32) -> Result<(), OutputError>;
    fn get_mute(&self) -> Result<bool, OutputError>;
    fn set_mute(&self, mute: bool) -> Result<(), OutputError>;
}
