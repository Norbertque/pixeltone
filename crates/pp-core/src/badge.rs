//! The Now Playing output badge: "Exclusive • 24-bit/96 kHz • Bit-perfect".

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::format::{
    OutputFormat, SampleKind, SourceFormat, channels_label, rate_label, rate_number,
};
use crate::gain::SoftwareGain;
use crate::negotiate::Conversion;
use crate::output::ShareMode;

/// Sound processing the user turned on, which also changes the samples.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Effects {
    /// The equaliser is on.
    pub eq: bool,
    /// ReplayGain applied to this track, in dB.
    pub replay_gain_db: Option<f32>,
}

impl Effects {
    pub const NONE: Self = Self {
        eq: false,
        replay_gain_db: None,
    };

    pub fn any(&self) -> bool {
        self.eq || self.replay_gain_db.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OutputBadge {
    pub mode: ShareMode,
    pub source: SourceFormat,
    pub output: OutputFormat,
    pub conversion: Conversion,
    /// Software attenuation, when the gain stage is active.
    pub gain_db: Option<f32>,
    pub effects: Effects,
    pub bit_perfect: bool,
    pub label: String,
}

/// "DSD64" for a DSD source (whose decoded rate is the bit rate / 8).
fn dsd_name(source: &SourceFormat) -> String {
    format!(
        "DSD{}",
        (u64::from(source.sample_rate) * 8 + 22_050) / 44_100
    )
}

impl OutputBadge {
    pub fn new(
        mode: ShareMode,
        source: SourceFormat,
        output: OutputFormat,
        conversion: Conversion,
        gain: SoftwareGain,
    ) -> Self {
        Self::build(mode, source, output, conversion, gain, Effects::NONE)
    }

    /// The same badge with the user's sound processing taken into account.
    pub fn with_effects(self, effects: Effects) -> Self {
        let gain = self.gain_db.map_or(SoftwareGain::UNITY, SoftwareGain::new);
        Self::build(
            self.mode,
            self.source,
            self.output,
            self.conversion,
            gain,
            effects,
        )
    }

    fn build(
        mode: ShareMode,
        source: SourceFormat,
        output: OutputFormat,
        conversion: Conversion,
        gain: SoftwareGain,
        effects: Effects,
    ) -> Self {
        let gain_db = (!gain.is_unity()).then_some(gain.db);
        let bit_perfect = mode == ShareMode::Exclusive
            && conversion.is_bit_perfect()
            && gain_db.is_none()
            && !effects.any();

        let depth = {
            let out = match output.sample.kind {
                SampleKind::Float => output.sample.label(),
                SampleKind::Int => format!("{}-bit", output.sample.valid_bits),
            };
            if conversion.dsd_to_pcm {
                format!("DSD→{out}")
            } else if conversion.depth_reduced && source.kind == SampleKind::Int {
                format!("{}→{out}", source.bits)
            } else {
                out
            }
        };
        let rate = if output.sample_rate == source.sample_rate {
            rate_label(output.sample_rate)
        } else {
            format!(
                "{}→{}",
                rate_number(source.sample_rate),
                rate_label(output.sample_rate)
            )
        };
        let mut parts = vec![
            match mode {
                ShareMode::Exclusive => "Exclusive".to_string(),
                ShareMode::Shared => "Shared".to_string(),
            },
            if conversion.dop {
                format!("{} over DoP", dsd_name(&source))
            } else {
                format!("{depth}/{rate}")
            },
        ];
        if conversion.downmixed {
            parts.push(format!(
                "{}→{}",
                channels_label(source.channels),
                channels_label(output.channels)
            ));
        }
        if effects.eq {
            parts.push("EQ".to_string());
        }
        if let Some(db) = effects.replay_gain_db {
            let sign = if db < 0.0 { "−" } else { "+" };
            parts.push(format!("ReplayGain {sign}{:.1} dB", db.abs()));
        }
        parts.push(match (mode, gain_db) {
            (ShareMode::Shared, _) => "Windows mixer".to_string(),
            (ShareMode::Exclusive, Some(db)) => format!("−{:.0} dB", -db),
            (ShareMode::Exclusive, None) if bit_perfect => "Bit-perfect".to_string(),
            (ShareMode::Exclusive, None) => "Converted".to_string(),
        });

        Self {
            mode,
            source,
            output,
            conversion,
            gain_db,
            effects,
            bit_perfect,
            label: parts.join(" • "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::SampleFormat;

    fn src(rate: u32, bits: u16) -> SourceFormat {
        SourceFormat {
            sample_rate: rate,
            channels: 2,
            bits,
            kind: SampleKind::Int,
        }
    }

    fn out(rate: u32, sample: SampleFormat) -> OutputFormat {
        OutputFormat {
            sample_rate: rate,
            channels: 2,
            sample,
        }
    }

    #[test]
    fn bit_perfect_label() {
        let b = OutputBadge::new(
            ShareMode::Exclusive,
            src(96_000, 24),
            out(96_000, SampleFormat::S24_IN_32),
            Conversion::BIT_PERFECT,
            SoftwareGain::UNITY,
        );
        assert!(b.bit_perfect);
        assert_eq!(b.label, "Exclusive • 24-bit/96 kHz • Bit-perfect");
    }

    #[test]
    fn widened_container_still_bit_perfect() {
        let b = OutputBadge::new(
            ShareMode::Exclusive,
            src(44_100, 16),
            out(44_100, SampleFormat::S32),
            Conversion::BIT_PERFECT,
            SoftwareGain::UNITY,
        );
        assert!(b.bit_perfect);
        assert_eq!(b.label, "Exclusive • 32-bit/44.1 kHz • Bit-perfect");
    }

    #[test]
    fn resampled_label() {
        let conversion = Conversion {
            resampled: true,
            ..Conversion::BIT_PERFECT
        };
        let b = OutputBadge::new(
            ShareMode::Exclusive,
            src(44_100, 16),
            out(88_200, SampleFormat::S24),
            conversion,
            SoftwareGain::UNITY,
        );
        assert!(!b.bit_perfect);
        assert_eq!(b.label, "Exclusive • 24-bit/44.1→88.2 kHz • Converted");
    }

    #[test]
    fn gain_label() {
        let b = OutputBadge::new(
            ShareMode::Exclusive,
            src(44_100, 16),
            out(44_100, SampleFormat::S32),
            Conversion::BIT_PERFECT,
            SoftwareGain::new(-30.0),
        );
        assert!(!b.bit_perfect);
        assert_eq!(b.label, "Exclusive • 32-bit/44.1 kHz • −30 dB");
    }

    #[test]
    fn depth_reduction_and_downmix_label() {
        let mut source = src(48_000, 24);
        source.channels = 6;
        let conversion = Conversion {
            depth_reduced: true,
            downmixed: true,
            ..Conversion::BIT_PERFECT
        };
        let b = OutputBadge::new(
            ShareMode::Exclusive,
            source,
            out(48_000, SampleFormat::S16),
            conversion,
            SoftwareGain::UNITY,
        );
        assert_eq!(
            b.label,
            "Exclusive • 24→16-bit/48 kHz • 5.1→stereo • Converted"
        );
    }

    #[test]
    fn dsd_converted_to_pcm_is_never_bit_perfect() {
        let source = SourceFormat {
            sample_rate: 352_800,
            channels: 2,
            bits: 32,
            kind: SampleKind::Float,
        };
        let conversion = Conversion {
            dsd_to_pcm: true,
            ..Conversion::BIT_PERFECT
        };
        let b = OutputBadge::new(
            ShareMode::Exclusive,
            source,
            out(352_800, SampleFormat::S32),
            conversion,
            SoftwareGain::UNITY,
        );
        assert!(!b.bit_perfect);
        assert_eq!(b.label, "Exclusive • DSD→32-bit/352.8 kHz • Converted");
    }

    #[test]
    fn dop_is_bit_perfect_dsd() {
        let source = SourceFormat {
            sample_rate: 352_800,
            channels: 2,
            bits: 32,
            kind: SampleKind::Float,
        };
        let conversion = Conversion {
            dop: true,
            ..Conversion::BIT_PERFECT
        };
        let b = OutputBadge::new(
            ShareMode::Exclusive,
            source,
            out(176_400, SampleFormat::S24_IN_32),
            conversion,
            SoftwareGain::UNITY,
        );
        assert!(b.bit_perfect);
        assert_eq!(b.label, "Exclusive • DSD64 over DoP • Bit-perfect");
    }

    #[test]
    fn effects_make_it_converted_and_say_so() {
        let b = OutputBadge::new(
            ShareMode::Exclusive,
            src(44_100, 16),
            out(44_100, SampleFormat::S32),
            Conversion::BIT_PERFECT,
            SoftwareGain::UNITY,
        )
        .with_effects(Effects {
            eq: true,
            replay_gain_db: Some(-6.24),
        });
        assert!(!b.bit_perfect);
        assert_eq!(
            b.label,
            "Exclusive • 32-bit/44.1 kHz • EQ • ReplayGain −6.2 dB • Converted"
        );
        let back = b.with_effects(Effects::NONE);
        assert!(back.bit_perfect);
    }

    #[test]
    fn shared_label() {
        let conversion = Conversion {
            resampled: true,
            ..Conversion::BIT_PERFECT
        };
        let b = OutputBadge::new(
            ShareMode::Shared,
            src(44_100, 16),
            out(48_000, SampleFormat::F32),
            conversion,
            SoftwareGain::UNITY,
        );
        assert!(!b.bit_perfect);
        assert_eq!(b.label, "Shared • 32-bit float/44.1→48 kHz • Windows mixer");
    }
}
