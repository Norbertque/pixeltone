//! Per-track conversion from decoded blocks to device bytes, and the output plan that picks
//! the device format for each track.

use pp_core::dop::carries_dop;
use pp_core::pcm::downmix;
use pp_core::{
    Conversion, FormatCandidate, Negotiated, OutputFormat, Packer, PcmBlock, Samples, ShareMode,
    SoftwareGain, SourceFormat, negotiate,
};

use crate::effects::ReplayGainMode;
use crate::resample::ResampleStage;

/// How the current device is driven, what it accepts, and the decode-side processing.
/// A change of plan means a new stream.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputPlan {
    pub mode: ShareMode,
    /// Exclusive-mode formats (empty in shared mode).
    pub exclusive: Vec<FormatCandidate>,
    /// Shared-mode mix format.
    pub mix_format: Option<OutputFormat>,
    /// Gain is applied somewhere: prefer deep containers so attenuation keeps low bits.
    pub prefer_deepest: bool,
    /// DSD may go out untouched as DoP.
    pub dop: bool,
    pub replay_gain: ReplayGainMode,
    pub replay_gain_preamp_db: f32,
    /// Crossfade length (shared mode only; 0 = gapless).
    pub crossfade_ms: u32,
    /// Crossfade between tracks of the same album too.
    pub crossfade_albums: bool,
}

impl Default for OutputPlan {
    fn default() -> Self {
        Self {
            mode: ShareMode::Shared,
            exclusive: Vec::new(),
            mix_format: None,
            prefer_deepest: false,
            dop: false,
            replay_gain: ReplayGainMode::Off,
            replay_gain_preamp_db: 0.0,
            crossfade_ms: 0,
            crossfade_albums: false,
        }
    }
}

impl OutputPlan {
    /// DSD as DoP: the DoP rate (DSD bit rate / 16) in a container with ≥ 24 valid bits and
    /// the source's channel count; 24-in-32 first, the layout most DACs expect.
    fn negotiate_dop(&self, source: &SourceFormat) -> Option<Negotiated> {
        // FFmpeg reports DSD at the bit rate / 8; DoP runs at the bit rate / 16.
        let rate = source.sample_rate / 2;
        self.exclusive
            .iter()
            .filter(|c| {
                c.channels == source.channels
                    && carries_dop(c.sample)
                    && c.sample_rates.contains(&rate)
            })
            .min_by_key(|c| (c.sample.container_bits != 32, c.sample.valid_bits != 24))
            .map(|c| Negotiated {
                format: OutputFormat {
                    sample_rate: rate,
                    channels: c.channels,
                    sample: c.sample,
                },
                conversion: Conversion {
                    dop: true,
                    ..Conversion::BIT_PERFECT
                },
            })
    }

    pub fn negotiate(&self, source: &SourceFormat, dsd: bool) -> Option<Negotiated> {
        if dsd
            && self.dop
            && self.mode == ShareMode::Exclusive
            && let Some(n) = self.negotiate_dop(source)
        {
            return Some(n);
        }
        let mut n = match self.mode {
            ShareMode::Exclusive => negotiate(source, &self.exclusive, self.prefer_deepest)?,
            ShareMode::Shared => {
                let mix = self.mix_format?;
                Negotiated {
                    format: mix,
                    conversion: Conversion {
                        resampled: mix.sample_rate != source.sample_rate,
                        depth_reduced: source.effective_bits() > mix.sample.resolution(),
                        downmixed: source.channels > mix.channels,
                        ..Conversion::BIT_PERFECT
                    },
                }
            }
        };
        n.conversion.dsd_to_pcm = dsd;
        Some(n)
    }
}

/// Decoded blocks → (downmix) → (resample) → packed device bytes.
pub struct Pipeline {
    source: SourceFormat,
    output: OutputFormat,
    downmix_to: Option<u16>,
    resampler: Option<ResampleStage>,
    packer: Packer,
}

impl Pipeline {
    pub fn new(source: SourceFormat, output: OutputFormat) -> Result<Self, String> {
        let downmix_to = (source.channels > output.channels).then_some(output.channels.min(2));
        let channels = downmix_to.unwrap_or(source.channels);
        let resampler = if source.sample_rate != output.sample_rate {
            Some(ResampleStage::new(
                source.sample_rate,
                output.sample_rate,
                channels,
            )?)
        } else {
            None
        };
        Ok(Self {
            source,
            output,
            downmix_to,
            resampler,
            packer: Packer::new(output, channels, SoftwareGain::UNITY),
        })
    }

    /// Whether a track in `source` format can continue through this pipeline unchanged.
    pub fn accepts(&self, source: &SourceFormat) -> bool {
        self.source == *source
    }

    pub fn output(&self) -> OutputFormat {
        self.output
    }

    pub fn process(&mut self, block: PcmBlock, out: &mut Vec<u8>) {
        let mut block = block;
        if let Some(ch) = self.downmix_to {
            block = downmix(&block, ch);
        }
        if let Some(rs) = &mut self.resampler {
            let float = block.to_float();
            let Samples::Float(input) = &float.samples else {
                unreachable!()
            };
            let mut resampled = Vec::with_capacity(input.len() * 2);
            rs.process(input, &mut resampled);
            block = PcmBlock {
                channels: float.channels,
                samples: Samples::Float(resampled),
            };
        }
        self.packer.pack(&block, out);
    }

    /// Flushes the resampler at the end of the stream.
    pub fn finish(&mut self, out: &mut Vec<u8>) {
        if let Some(rs) = &mut self.resampler {
            let mut tail = Vec::new();
            rs.finish(&mut tail);
            let channels = self.downmix_to.unwrap_or(self.source.channels);
            self.packer.pack(
                &PcmBlock {
                    channels,
                    samples: Samples::Float(tail),
                },
                out,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use pp_core::{SampleFormat, SampleKind};

    use super::*;

    fn src(rate: u32, bits: u16, channels: u16) -> SourceFormat {
        SourceFormat {
            sample_rate: rate,
            channels,
            bits,
            kind: SampleKind::Int,
        }
    }

    #[test]
    fn shared_mode_reports_resampling_to_the_mix_rate() {
        let plan = OutputPlan {
            mode: ShareMode::Shared,
            exclusive: vec![],
            mix_format: Some(OutputFormat {
                sample_rate: 48_000,
                channels: 2,
                sample: SampleFormat::F32,
            }),
            ..OutputPlan::default()
        };
        let n = plan.negotiate(&src(44_100, 16, 2), false).unwrap();
        assert!(n.conversion.resampled);
        assert_eq!(n.format.sample, SampleFormat::F32);
    }

    #[test]
    fn dsd_is_flagged_as_converted() {
        let plan = OutputPlan {
            mode: ShareMode::Exclusive,
            exclusive: vec![FormatCandidate {
                sample: SampleFormat::S32,
                channels: 2,
                sample_rates: vec![352_800],
            }],
            mix_format: None,
            ..OutputPlan::default()
        };
        let source = SourceFormat {
            sample_rate: 352_800,
            channels: 2,
            bits: 32,
            kind: SampleKind::Float,
        };
        let n = plan.negotiate(&source, true).unwrap();
        assert!(!n.conversion.is_bit_perfect());
    }

    #[test]
    fn dsd_goes_out_as_dop_when_the_device_takes_the_rate() {
        let candidate = |sample, rates: Vec<u32>| FormatCandidate {
            sample,
            channels: 2,
            sample_rates: rates,
        };
        let dsd64 = SourceFormat {
            sample_rate: 352_800,
            channels: 2,
            bits: 32,
            kind: SampleKind::Float,
        };
        let plan = OutputPlan {
            mode: ShareMode::Exclusive,
            exclusive: vec![
                candidate(SampleFormat::S16, vec![176_400]),
                candidate(SampleFormat::S24, vec![176_400]),
                candidate(SampleFormat::S24_IN_32, vec![176_400, 352_800]),
                candidate(SampleFormat::S32, vec![176_400, 352_800]),
            ],
            dop: true,
            ..OutputPlan::default()
        };
        let n = plan.negotiate(&dsd64, true).unwrap();
        assert!(n.conversion.dop && n.conversion.is_bit_perfect());
        assert_eq!(n.format.sample_rate, 176_400);
        assert_eq!(n.format.sample, SampleFormat::S24_IN_32);

        // DoP off, or no rate for it: decoded to PCM instead.
        let off = OutputPlan {
            dop: false,
            ..plan.clone()
        };
        assert!(off.negotiate(&dsd64, true).unwrap().conversion.dsd_to_pcm);
        let dsd512 = SourceFormat {
            sample_rate: 2_822_400,
            ..dsd64
        };
        let n = plan.negotiate(&dsd512, true).unwrap();
        assert!(!n.conversion.dop && n.conversion.dsd_to_pcm);
    }

    #[test]
    fn same_rate_pipeline_is_a_straight_pack() {
        let out = OutputFormat {
            sample_rate: 44_100,
            channels: 2,
            sample: SampleFormat::S16,
        };
        let mut p = Pipeline::new(src(44_100, 16, 2), out).unwrap();
        let mut bytes = Vec::new();
        p.process(
            PcmBlock {
                channels: 2,
                samples: Samples::Int(vec![1 << 16, -1 << 16]),
            },
            &mut bytes,
        );
        p.finish(&mut bytes);
        assert_eq!(
            bytes,
            [1i16, -1]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>()
        );
    }
}
