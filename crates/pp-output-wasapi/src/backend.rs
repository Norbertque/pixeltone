//! Device discovery and capability probing.

use pp_core::format::PROBE_RATES;
use pp_core::output::HardwareVolume;
use pp_core::{
    DeviceCapabilities, DeviceInfo, FormatCandidate, OpenRequest, OutputBackend, OutputError,
    OutputFormat, OutputStream, SampleFormat,
};
use wasapi::{Device, DeviceEnumerator, Direction, ShareMode as WShareMode, WasapiError};
use windows::Win32::Media::Audio::{AUDCLNT_E_DEVICE_IN_USE, AUDCLNT_E_EXCLUSIVE_MODE_NOT_ALLOWED};

use crate::convert::{from_wave_format, map_error, to_wave_format};
use crate::stream::WasapiStream;
use crate::volume::{EndpointVolume, has_hardware_volume};

/// Joins the multithreaded COM apartment. Harmless when already initialised; on a
/// single-threaded-apartment thread the existing apartment is kept.
pub fn init_com() {
    let _ = wasapi::initialize_mta();
}

pub fn find_device(device_id: Option<&str>) -> Result<Device, OutputError> {
    init_com();
    let enumerator = DeviceEnumerator::new().map_err(map_error)?;
    match device_id {
        Some(id) => enumerator
            .get_device(id)
            .map_err(|_| OutputError::DeviceNotFound),
        None => enumerator
            .get_default_device(&Direction::Render)
            .map_err(map_error),
    }
}

#[derive(Debug, Default)]
pub struct WasapiBackend;

impl WasapiBackend {
    pub fn new() -> Self {
        Self
    }
}

impl OutputBackend for WasapiBackend {
    fn name(&self) -> &'static str {
        "wasapi"
    }

    fn list_devices(&self) -> Result<Vec<DeviceInfo>, OutputError> {
        init_com();
        let enumerator = DeviceEnumerator::new().map_err(map_error)?;
        let default_id = enumerator
            .get_default_device(&Direction::Render)
            .and_then(|d| d.get_id())
            .ok();
        let collection = enumerator
            .get_device_collection(&Direction::Render)
            .map_err(map_error)?;
        let mut devices = Vec::new();
        for device in &collection {
            let Ok(device) = device else { continue };
            let Ok(id) = device.get_id() else { continue };
            let name = device.get_friendlyname().unwrap_or_else(|_| id.clone());
            devices.push(DeviceInfo {
                is_default: default_id.as_deref() == Some(id.as_str()),
                id,
                name,
            });
        }
        Ok(devices)
    }

    fn capabilities(&self, device_id: Option<&str>) -> Result<DeviceCapabilities, OutputError> {
        let device = find_device(device_id)?;
        let id = device.get_id().map_err(map_error)?;
        let name = device.get_friendlyname().unwrap_or_else(|_| id.clone());
        let default_id = DeviceEnumerator::new()
            .and_then(|e| e.get_default_device(&Direction::Render))
            .and_then(|d| d.get_id())
            .ok();
        let client = device.get_iaudioclient().map_err(map_error)?;
        let mix_format = client
            .get_mixformat()
            .ok()
            .as_ref()
            .and_then(from_wave_format);
        let (default_period, min_period) = client.get_device_period().map_err(map_error)?;

        let (exclusive, exclusive_error) = probe_exclusive(&device, mix_format);

        Ok(DeviceCapabilities {
            device: DeviceInfo {
                is_default: default_id.as_deref() == Some(id.as_str()),
                id: id.clone(),
                name,
            },
            mix_format,
            exclusive,
            exclusive_error,
            hardware_volume: has_hardware_volume(Some(&id)),
            default_period_us: (default_period / 10) as u32,
            min_period_us: (min_period / 10) as u32,
        })
    }

    fn open(
        &self,
        request: &OpenRequest,
        source: rtrb::Consumer<u8>,
    ) -> Result<Box<dyn OutputStream>, OutputError> {
        Ok(Box::new(WasapiStream::open(request.clone(), source)?))
    }

    fn hardware_volume(
        &self,
        device_id: Option<&str>,
    ) -> Result<Option<Box<dyn HardwareVolume>>, OutputError> {
        if has_hardware_volume(device_id) {
            Ok(Some(Box::new(EndpointVolume::new(device_id))))
        } else {
            Ok(None)
        }
    }
}

/// Asks the driver which formats it accepts in exclusive mode. Returns an explanation
/// instead when exclusive mode is unavailable altogether.
fn probe_exclusive(
    device: &Device,
    mix: Option<OutputFormat>,
) -> (Vec<FormatCandidate>, Option<String>) {
    let Ok(client) = device.get_iaudioclient() else {
        return (Vec::new(), Some("the device could not be opened".into()));
    };

    // A plain query first: it reports policy problems that format probing would hide.
    let probe = to_wave_format(&OutputFormat {
        sample_rate: mix.map(|m| m.sample_rate).unwrap_or(48_000),
        channels: 2,
        sample: SampleFormat::S16,
    });
    if let Err(WasapiError::Windows(e)) = client.is_supported(&probe, &WShareMode::Exclusive) {
        if e.code() == AUDCLNT_E_EXCLUSIVE_MODE_NOT_ALLOWED {
            return (
                Vec::new(),
                Some(OutputError::ExclusiveNotAllowed.to_string()),
            );
        }
        if e.code() == AUDCLNT_E_DEVICE_IN_USE {
            return (Vec::new(), Some(OutputError::DeviceInUse.to_string()));
        }
    }

    let mut channel_counts = vec![2u16];
    if let Some(m) = mix
        && m.channels != 2
    {
        channel_counts.push(m.channels);
    }
    let mut found = Vec::new();
    for channels in channel_counts {
        for sample in SampleFormat::PROBE_ORDER {
            let sample_rates: Vec<u32> = PROBE_RATES
                .into_iter()
                .filter(|&rate| {
                    let fmt = to_wave_format(&OutputFormat {
                        sample_rate: rate,
                        channels,
                        sample,
                    });
                    client.is_supported_exclusive_with_quirks(&fmt).is_ok()
                })
                .collect();
            if !sample_rates.is_empty() {
                found.push(FormatCandidate {
                    sample,
                    channels,
                    sample_rates,
                });
            }
        }
    }
    let error = found
        .is_empty()
        .then(|| "the driver accepts none of the standard formats in exclusive mode".to_string());
    (found, error)
}
