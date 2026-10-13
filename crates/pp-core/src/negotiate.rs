//! Exclusive-mode format negotiation — the same decision order as the Android USB
//! exclusive mode (`FormatNegotiator.kt` in the PixelPlayer fork).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::format::{OutputFormat, SampleFormat, SourceFormat, same_rate_family};

/// One sample container and channel count the device accepts in exclusive mode, with
/// every sample rate it accepts for that combination.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FormatCandidate {
    pub sample: SampleFormat,
    pub channels: u16,
    pub sample_rates: Vec<u32>,
}

/// What had to happen to the samples on the way to the device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Conversion {
    pub resampled: bool,
    pub depth_reduced: bool,
    pub downmixed: bool,
    /// DSD decoded to PCM (as opposed to DoP, which carries DSD untouched).
    pub dsd_to_pcm: bool,
    /// DSD carried untouched inside PCM frames (DoP); not a conversion of the signal.
    pub dop: bool,
}

impl Conversion {
    pub const BIT_PERFECT: Self = Self {
        resampled: false,
        depth_reduced: false,
        downmixed: false,
        dsd_to_pcm: false,
        dop: false,
    };

    pub const fn is_bit_perfect(&self) -> bool {
        !self.resampled && !self.depth_reduced && !self.downmixed && !self.dsd_to_pcm
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Negotiated {
    pub format: OutputFormat,
    pub conversion: Conversion,
}

/// Picks the device format for `source`, preferring in order: no downmix, no resampling,
/// no depth reduction, a container of the same kind (int/float), the closest channel
/// count, the tightest adequate depth, then the least destructive rate.
///
/// `prefer_deepest`: choose the deepest adequate container instead of the tightest. Used
/// when a software gain stage attenuates, so the low-order bits survive.
pub fn negotiate(
    source: &SourceFormat,
    candidates: &[FormatCandidate],
    prefer_deepest: bool,
) -> Option<Negotiated> {
    candidates
        .iter()
        .filter(|c| !c.sample_rates.is_empty() && c.channels > 0)
        .map(|c| {
            let (rate, resampled) = choose_rate(source.sample_rate, &c.sample_rates);
            let conversion = Conversion {
                resampled,
                depth_reduced: source.effective_bits() > c.sample.resolution(),
                downmixed: source.channels > c.channels,
                ..Conversion::BIT_PERFECT
            };
            let format = OutputFormat {
                sample_rate: rate,
                channels: c.channels,
                sample: c.sample,
            };
            (format, conversion)
        })
        .min_by_key(|(format, conversion)| {
            (
                conversion.downmixed,
                conversion.resampled,
                conversion.depth_reduced,
                format.sample.kind != source.kind,
                format.channels.abs_diff(source.channels),
                depth_overhead(source, format.sample, prefer_deepest),
                rate_distance(source.sample_rate, format.sample_rate),
                format.sample.container_bits,
            )
        })
        .map(|(format, conversion)| Negotiated { format, conversion })
}

/// Exact rate when available; otherwise the smallest same-family rate above the source
/// (44.1 kHz material stays on the 44.1 kHz clock family whenever possible), then the
/// smallest higher rate of any family, then the highest rate below.
pub fn choose_rate(source_rate: u32, rates: &[u32]) -> (u32, bool) {
    if rates.contains(&source_rate) {
        return (source_rate, false);
    }
    let higher = || rates.iter().copied().filter(move |&r| r > source_rate);
    if let Some(r) = higher().filter(|&r| same_rate_family(r, source_rate)).min() {
        return (r, true);
    }
    if let Some(r) = higher().min() {
        return (r, true);
    }
    (rates.iter().copied().max().unwrap_or(source_rate), true)
}

fn depth_overhead(source: &SourceFormat, sample: SampleFormat, prefer_deepest: bool) -> u32 {
    let resolution = u32::from(sample.resolution());
    let wanted = u32::from(source.effective_bits());
    if resolution < wanted {
        // Inadequate depth sorts after any adequate one; among inadequate, deeper is better.
        return 1000 + (wanted - resolution);
    }
    if prefer_deepest {
        64 - resolution
    } else {
        resolution - wanted
    }
}

fn rate_distance(source_rate: u32, chosen: u32) -> u64 {
    if chosen >= source_rate {
        u64::from(chosen - source_rate)
    } else {
        // Upsampling is preferable to downsampling at equal distance.
        1_000_000_000 + u64::from(source_rate - chosen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::SampleKind;

    const HIFI_RATES: [u32; 6] = [44_100, 48_000, 88_200, 96_000, 176_400, 192_000];

    fn cand(sample: SampleFormat, rates: &[u32]) -> FormatCandidate {
        FormatCandidate {
            sample,
            channels: 2,
            sample_rates: rates.to_vec(),
        }
    }

    fn src(rate: u32, bits: u16, channels: u16, kind: SampleKind) -> SourceFormat {
        SourceFormat {
            sample_rate: rate,
            channels,
            bits,
            kind,
        }
    }

    fn int(rate: u32, bits: u16) -> SourceFormat {
        src(rate, bits, 2, SampleKind::Int)
    }

    /// A typical USB DAC: 16-bit and 32-bit containers, 44.1–192 kHz.
    fn typical_dac() -> Vec<FormatCandidate> {
        vec![
            cand(SampleFormat::S16, &HIFI_RATES),
            cand(SampleFormat::S32, &HIFI_RATES),
        ]
    }

    #[test]
    fn cd_audio_on_a_matching_dac_is_bit_perfect_at_44100() {
        let n = negotiate(&int(44_100, 16), &typical_dac(), false).unwrap();
        assert_eq!(n.format.sample_rate, 44_100);
        assert_eq!(n.format.sample, SampleFormat::S16);
        assert!(n.conversion.is_bit_perfect());
    }

    #[test]
    fn float_source_picks_the_32_bit_container_bit_perfectly() {
        let n = negotiate(
            &src(96_000, 32, 2, SampleKind::Float),
            &typical_dac(),
            false,
        )
        .unwrap();
        assert_eq!(n.format.sample_rate, 96_000);
        assert_eq!(n.format.sample, SampleFormat::S32);
        assert!(n.conversion.is_bit_perfect());
    }

    #[test]
    fn float_source_prefers_a_float_container_when_offered() {
        let dac = vec![
            cand(SampleFormat::S24_IN_32, &HIFI_RATES),
            cand(SampleFormat::F32, &HIFI_RATES),
        ];
        let n = negotiate(&src(44_100, 32, 2, SampleKind::Float), &dac, false).unwrap();
        assert_eq!(n.format.sample, SampleFormat::F32);
        assert!(n.conversion.is_bit_perfect());
    }

    #[test]
    fn int_source_prefers_an_int_container_over_float() {
        let dac = vec![
            cand(SampleFormat::F32, &HIFI_RATES),
            cand(SampleFormat::S24_IN_32, &HIFI_RATES),
        ];
        let n = negotiate(&int(96_000, 24), &dac, false).unwrap();
        assert_eq!(n.format.sample, SampleFormat::S24_IN_32);
    }

    #[test]
    fn twenty_four_bit_source_prefers_tightest_adequate_depth() {
        let dac = vec![
            cand(SampleFormat::S24, &HIFI_RATES),
            cand(SampleFormat::S32, &HIFI_RATES),
        ];
        let n = negotiate(&int(96_000, 24), &dac, false).unwrap();
        assert_eq!(n.format.sample, SampleFormat::S24);
        assert!(n.conversion.is_bit_perfect());
    }

    #[test]
    fn missing_rate_resamples_within_the_same_family_upwards() {
        let dac = vec![cand(SampleFormat::S24, &[48_000, 88_200, 96_000])];
        let n = negotiate(&int(44_100, 16), &dac, false).unwrap();
        // 44.1k material goes to 88.2k (same family), not 48k (nearer but cross-family).
        assert_eq!(n.format.sample_rate, 88_200);
        assert!(n.conversion.resampled);
        assert!(!n.conversion.is_bit_perfect());
    }

    #[test]
    fn rate_above_the_maximum_downsamples_to_the_highest_supported() {
        let dac = vec![cand(SampleFormat::S24, &[44_100, 48_000, 96_000])];
        let n = negotiate(&int(192_000, 24), &dac, false).unwrap();
        assert_eq!(n.format.sample_rate, 96_000);
        assert!(n.conversion.resampled);
    }

    #[test]
    fn twenty_four_bit_source_on_a_16_bit_only_dac_reports_depth_reduction() {
        let dac = vec![cand(SampleFormat::S16, &HIFI_RATES)];
        let n = negotiate(&int(96_000, 24), &dac, false).unwrap();
        assert_eq!(n.format.sample, SampleFormat::S16);
        assert!(n.conversion.depth_reduced);
        assert!(!n.conversion.resampled);
    }

    #[test]
    fn surround_source_on_a_stereo_dac_reports_downmix() {
        let n = negotiate(&src(48_000, 16, 6, SampleKind::Int), &typical_dac(), false).unwrap();
        assert!(n.conversion.downmixed);
        assert!(!n.conversion.resampled);
    }

    #[test]
    fn mono_source_on_a_stereo_dac_stays_bit_perfect() {
        let n = negotiate(&src(48_000, 16, 1, SampleKind::Int), &typical_dac(), false).unwrap();
        assert_eq!(n.format.channels, 2);
        assert!(n.conversion.is_bit_perfect());
    }

    #[test]
    fn exact_channel_count_wins_over_wider_layouts() {
        // Even when the wider layout has the tighter container.
        let dac = vec![
            FormatCandidate {
                channels: 8,
                ..cand(SampleFormat::S16, &HIFI_RATES)
            },
            cand(SampleFormat::S32, &HIFI_RATES),
        ];
        let n = negotiate(&int(44_100, 16), &dac, false).unwrap();
        assert_eq!(n.format.channels, 2);
        assert_eq!(n.format.sample, SampleFormat::S32);
    }

    #[test]
    fn no_formats_yields_none() {
        assert!(negotiate(&int(44_100, 16), &[], false).is_none());
        let empty_rates = vec![cand(SampleFormat::S16, &[])];
        assert!(negotiate(&int(44_100, 16), &empty_rates, false).is_none());
    }

    #[test]
    fn prefers_exact_rate_over_tighter_depth() {
        let dac = vec![
            cand(SampleFormat::S24, &[48_000]),
            cand(SampleFormat::S32, &[44_100, 48_000]),
        ];
        let n = negotiate(&int(44_100, 24), &dac, false).unwrap();
        assert_eq!(n.format.sample, SampleFormat::S32);
        assert!(n.conversion.is_bit_perfect());
    }

    #[test]
    fn prefer_deepest_picks_the_deepest_adequate_container() {
        // Volume-less DAC: the software gain stage wants headroom, so a 16-bit source lands
        // on the 32-bit container; widening is lossless, so still bit-perfect at unity.
        let n = negotiate(&int(44_100, 16), &typical_dac(), true).unwrap();
        assert_eq!(n.format.sample, SampleFormat::S32);
        assert_eq!(n.format.sample_rate, 44_100);
        assert!(n.conversion.is_bit_perfect());
    }

    #[test]
    fn prefer_deepest_never_trades_an_exact_rate_for_depth() {
        let dac = vec![
            cand(SampleFormat::S16, &[44_100]),
            cand(SampleFormat::S32, &[48_000]),
        ];
        let n = negotiate(&int(44_100, 16), &dac, true).unwrap();
        assert_eq!(n.format.sample, SampleFormat::S16);
        assert!(!n.conversion.resampled);
    }

    #[test]
    fn packed_and_padded_24_bit_tie_breaks_to_the_smaller_container() {
        let dac = vec![
            cand(SampleFormat::S24_IN_32, &HIFI_RATES),
            cand(SampleFormat::S24, &HIFI_RATES),
        ];
        let n = negotiate(&int(96_000, 24), &dac, false).unwrap();
        assert_eq!(n.format.sample, SampleFormat::S24);
    }

    #[test]
    fn choose_rate_table() {
        let rates = [44_100, 48_000, 88_200, 96_000];
        assert_eq!(choose_rate(44_100, &rates), (44_100, false));
        assert_eq!(choose_rate(22_050, &rates), (44_100, true));
        assert_eq!(choose_rate(176_400, &rates), (96_000, true));
        assert_eq!(choose_rate(32_000, &rates), (48_000, true));
    }
}
