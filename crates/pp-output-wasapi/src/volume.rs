//! Endpoint volume: the device's own volume control. Changing it never touches samples, so
//! exclusive mode stays bit-perfect.

use pp_core::OutputError;
use pp_core::output::HardwareVolume;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::core::{GUID, HSTRING};

/// `ENDPOINT_HARDWARE_SUPPORT_VOLUME` from endpointvolume.h.
const HARDWARE_SUPPORT_VOLUME: u32 = 0x1;

fn other(e: windows::core::Error) -> OutputError {
    OutputError::Other(e.message())
}

/// Opens the endpoint volume of a device (`None` = default output).
pub fn endpoint_volume(device_id: Option<&str>) -> Result<IAudioEndpointVolume, OutputError> {
    crate::backend::init_com();
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(other)?;
        let device = match device_id {
            Some(id) => enumerator
                .GetDevice(&HSTRING::from(id))
                .map_err(|_| OutputError::DeviceNotFound)?,
            None => enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(other)?,
        };
        device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(other)
    }
}

/// Whether the endpoint's volume is applied in hardware (by the device), not in software.
pub fn has_hardware_volume(device_id: Option<&str>) -> bool {
    endpoint_volume(device_id)
        .and_then(|v| unsafe { v.QueryHardwareSupport() }.map_err(other))
        .map(|mask| mask & HARDWARE_SUPPORT_VOLUME != 0)
        .unwrap_or(false)
}

/// Re-opens the endpoint on every call, so the handle can move between threads freely.
pub struct EndpointVolume {
    device_id: Option<String>,
}

impl EndpointVolume {
    pub fn new(device_id: Option<&str>) -> Self {
        Self {
            device_id: device_id.map(str::to_string),
        }
    }

    fn open(&self) -> Result<IAudioEndpointVolume, OutputError> {
        endpoint_volume(self.device_id.as_deref())
    }
}

impl HardwareVolume for EndpointVolume {
    fn range_db(&self) -> (f32, f32, f32) {
        let (mut min, mut max, mut step) = (-96.0f32, 0.0f32, 0.5f32);
        if let Ok(v) = self.open() {
            let _ = unsafe { v.GetVolumeRange(&mut min, &mut max, &mut step) };
        }
        (min, max, step)
    }

    fn get_db(&self) -> Result<f32, OutputError> {
        unsafe { self.open()?.GetMasterVolumeLevel() }.map_err(other)
    }

    fn set_db(&self, db: f32) -> Result<(), OutputError> {
        let (min, max, _) = self.range_db();
        unsafe {
            self.open()?
                .SetMasterVolumeLevel(db.clamp(min, max), &GUID::zeroed())
        }
        .map_err(other)
    }

    fn get_mute(&self) -> Result<bool, OutputError> {
        unsafe { self.open()?.GetMute() }
            .map(|b| b.as_bool())
            .map_err(other)
    }

    fn set_mute(&self, mute: bool) -> Result<(), OutputError> {
        unsafe { self.open()?.SetMute(mute, &GUID::zeroed()) }.map_err(other)
    }
}
