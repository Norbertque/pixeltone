//! Per-track conversion from decoded blocks to device bytes, and the output plan that picks
//! the device format for each track.

use pp_core::pcm::downmix;
use pp_core::{
    Conversion, FormatCandidate, Negotiated, OutputFormat, Packer, PcmBlock, Samples, ShareMode,
    SoftwareGain, SourceFormat, negotiate,
};

use crate::resample::ResampleStage;

/// How the current device is driven, and what it accepts.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputPlan {
    pub mode: ShareMode,
    /// Exclusive-mode formats (empty in shared mode).
    pub exclusive: Vec<FormatCandidate>,
    /// Shared-mode mix format.
    pub mix_format: Option<OutputFormat>,
    /// A software gain stage is in use: prefer deep containers so attenuation keeps low bits.
    pub prefer_deepest: bool,
}

impl OutputPlan {
    pub fn negotiate(&self, source: &SourceFormat, dsd: bool) -> Option<Negotiated> {
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
                        dsd_to_pcm: false,
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
            prefer_deepest: false,
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
            prefer_deepest: false,
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
