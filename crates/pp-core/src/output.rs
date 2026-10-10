//! The contract between the engine and an audio output backend.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

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

/// Live controls shared between the engine and a stream's render thread.
#[derive(Debug)]
pub struct StreamControls {
    /// Q16.16 software gain; negative means unity (samples untouched).
    gain_q16: AtomicI64,
    /// While set, an empty ring buffer counts as an underrun.
    expect_data: AtomicBool,
}

impl Default for StreamControls {
    fn default() -> Self {
        Self {
            gain_q16: AtomicI64::new(-1),
            expect_data: AtomicBool::new(false),
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
}

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
