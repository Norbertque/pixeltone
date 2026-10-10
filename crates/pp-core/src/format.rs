use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Integer or IEEE float samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SampleKind {
    Int,
    Float,
}

/// A sample container as WAVEFORMATEXTENSIBLE describes it: `container_bits` per sample in
/// the buffer, of which the top `valid_bits` carry signal (left-justified).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SampleFormat {
    pub kind: SampleKind,
    pub container_bits: u16,
    pub valid_bits: u16,
}

impl SampleFormat {
    pub const S16: Self = Self::int(16, 16);
    /// Packed 3-byte 24-bit.
    pub const S24: Self = Self::int(24, 24);
    /// 24 valid bits left-justified in a 32-bit slot.
    pub const S24_IN_32: Self = Self::int(32, 24);
    pub const S32: Self = Self::int(32, 32);
    pub const F32: Self = Self {
        kind: SampleKind::Float,
        container_bits: 32,
        valid_bits: 32,
    };

    /// Every container exclusive mode is probed for, smallest first.
    pub const PROBE_ORDER: [Self; 5] =
        [Self::S16, Self::S24, Self::S24_IN_32, Self::S32, Self::F32];

    pub const fn int(container_bits: u16, valid_bits: u16) -> Self {
        Self {
            kind: SampleKind::Int,
            container_bits,
            valid_bits,
        }
    }

    pub const fn bytes(self) -> usize {
        self.container_bits as usize / 8
    }

    /// Integer resolution this container reproduces exactly. A float32 container holds
    /// any integer sample of up to 24 bits without loss.
    pub const fn resolution(self) -> u16 {
        match self.kind {
            SampleKind::Int => self.valid_bits,
            SampleKind::Float => 24,
        }
    }

    pub fn label(self) -> String {
        match self.kind {
            SampleKind::Float => format!("{}-bit float", self.container_bits),
            SampleKind::Int if self.valid_bits == self.container_bits => {
                format!("{}-bit", self.valid_bits)
            }
            SampleKind::Int => format!("{}-in-{}-bit", self.valid_bits, self.container_bits),
        }
    }
}

/// What the device is opened with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OutputFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub sample: SampleFormat,
}

impl OutputFormat {
    pub const fn bytes_per_frame(&self) -> usize {
        self.sample.bytes() * self.channels as usize
    }
}

/// Decoded PCM as it leaves the decoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SourceFormat {
    pub sample_rate: u32,
    pub channels: u16,
    /// Significant bits of integer samples (16, 24 or 32). Informational for float.
    pub bits: u16,
    pub kind: SampleKind,
}

impl SourceFormat {
    /// Float decoder output carries at most 24 significant bits, and integer material of
    /// up to 24 bits survives int → float → int exactly, so floats negotiate as 24-bit.
    pub const fn effective_bits(&self) -> u16 {
        match self.kind {
            SampleKind::Int => self.bits,
            SampleKind::Float => 24,
        }
    }
}

/// Sample rates exclusive mode is probed for.
pub const PROBE_RATES: [u32; 10] = [
    44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000, 705_600, 768_000,
];

/// 44.1 kHz family (multiples of 11 025) vs 48 kHz family.
pub const fn same_rate_family(a: u32, b: u32) -> bool {
    a.is_multiple_of(11_025) == b.is_multiple_of(11_025)
}

/// "44.1 kHz", "96 kHz", "352.8 kHz".
pub fn rate_label(rate: u32) -> String {
    if rate.is_multiple_of(1000) {
        format!("{} kHz", rate / 1000)
    } else {
        let khz = rate as f64 / 1000.0;
        let s = format!("{khz:.1}");
        format!("{} kHz", s.trim_end_matches(".0"))
    }
}

/// Rate without the unit, for compact "44.1→48 kHz" labels.
pub fn rate_number(rate: u32) -> String {
    rate_label(rate).trim_end_matches(" kHz").to_string()
}

pub fn channels_label(channels: u16) -> String {
    match channels {
        1 => "mono".into(),
        2 => "stereo".into(),
        6 => "5.1".into(),
        8 => "7.1".into(),
        n => format!("{n} ch"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(rate_label(44_100), "44.1 kHz");
        assert_eq!(rate_label(96_000), "96 kHz");
        assert_eq!(rate_label(352_800), "352.8 kHz");
        assert_eq!(rate_number(48_000), "48");
        assert_eq!(SampleFormat::S24_IN_32.label(), "24-in-32-bit");
        assert_eq!(SampleFormat::S16.label(), "16-bit");
        assert_eq!(SampleFormat::F32.label(), "32-bit float");
    }

    #[test]
    fn float_container_resolves_24_bits() {
        assert_eq!(SampleFormat::F32.resolution(), 24);
        assert_eq!(SampleFormat::S24_IN_32.resolution(), 24);
        assert_eq!(SampleFormat::S32.resolution(), 32);
    }

    #[test]
    fn rate_families() {
        assert!(same_rate_family(44_100, 88_200));
        assert!(same_rate_family(48_000, 96_000));
        assert!(!same_rate_family(44_100, 48_000));
        assert!(same_rate_family(22_050, 176_400));
    }
}
