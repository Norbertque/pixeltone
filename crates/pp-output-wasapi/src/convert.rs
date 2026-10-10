//! Conversions between WASAPI types and `pp-core` types, and error mapping.

use pp_core::{OutputError, OutputFormat, SampleFormat, SampleKind};
use wasapi::{SampleType, WasapiError, WaveFormat};
use windows::Win32::Media::Audio::{
    AUDCLNT_E_DEVICE_IN_USE, AUDCLNT_E_DEVICE_INVALIDATED, AUDCLNT_E_EXCLUSIVE_MODE_NOT_ALLOWED,
    AUDCLNT_E_UNSUPPORTED_FORMAT,
};

pub fn to_wave_format(format: &OutputFormat) -> WaveFormat {
    let kind = match format.sample.kind {
        SampleKind::Int => SampleType::Int,
        SampleKind::Float => SampleType::Float,
    };
    WaveFormat::new(
        usize::from(format.sample.container_bits),
        usize::from(format.sample.valid_bits),
        &kind,
        format.sample_rate as usize,
        usize::from(format.channels),
        None,
    )
}

pub fn from_wave_format(fmt: &WaveFormat) -> Option<OutputFormat> {
    let kind = match fmt.get_subformat().ok()? {
        SampleType::Int => SampleKind::Int,
        SampleType::Float => SampleKind::Float,
    };
    let container_bits = fmt.get_bitspersample();
    let valid = fmt.get_validbitspersample();
    Some(OutputFormat {
        sample_rate: fmt.get_samplespersec(),
        channels: fmt.get_nchannels(),
        sample: SampleFormat {
            kind,
            container_bits,
            valid_bits: if valid == 0 { container_bits } else { valid },
        },
    })
}

pub fn describe(format: &OutputFormat) -> String {
    format!(
        "{} {} Hz, {} channels",
        format.sample.label(),
        format.sample_rate,
        format.channels
    )
}

pub fn map_error(e: WasapiError) -> OutputError {
    match e {
        WasapiError::Windows(w) => {
            let code = w.code();
            if code == AUDCLNT_E_DEVICE_IN_USE {
                OutputError::DeviceInUse
            } else if code == AUDCLNT_E_EXCLUSIVE_MODE_NOT_ALLOWED {
                OutputError::ExclusiveNotAllowed
            } else if code == AUDCLNT_E_UNSUPPORTED_FORMAT {
                OutputError::UnsupportedFormat("the requested format".into())
            } else if code == AUDCLNT_E_DEVICE_INVALIDATED {
                OutputError::DeviceInvalidated
            } else {
                OutputError::Other(format!("{} (0x{:08X})", w.message(), code.0 as u32))
            }
        }
        WasapiError::DeviceNotFound(_) => OutputError::DeviceNotFound,
        WasapiError::UnsupportedFormat => {
            OutputError::UnsupportedFormat("the requested format".into())
        }
        other => OutputError::Other(other.to_string()),
    }
}
