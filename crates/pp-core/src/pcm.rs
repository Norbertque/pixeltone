//! Decoded PCM and its conversion into the device's byte layout.
//!
//! On the bit-perfect path the only transformations are lossless: integer samples are
//! widened into a larger container (left-justified), integer samples of up to 24 bits go
//! into float32 exactly, and mono is duplicated onto both front channels. Gain is applied
//! only when a [`SoftwareGain`] other than unity is set. Nothing here dithers.

use crate::format::{OutputFormat, SampleKind};
use crate::gain::SoftwareGain;

/// Interleaved samples in one of the two internal representations.
#[derive(Clone, Debug, PartialEq)]
pub enum Samples {
    /// Integer samples left-justified in i32 (FFmpeg's convention for every integer
    /// sample format), so a 16-bit sample `s` is stored as `s << 16`.
    Int(Vec<i32>),
    /// Float samples, nominally in [-1.0, 1.0).
    Float(Vec<f32>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcmBlock {
    pub channels: u16,
    pub samples: Samples,
}

impl PcmBlock {
    pub fn frames(&self) -> usize {
        let len = match &self.samples {
            Samples::Int(v) => v.len(),
            Samples::Float(v) => v.len(),
        };
        len / usize::from(self.channels.max(1))
    }

    /// Converts to float. Exact for integer samples of up to 24 significant bits.
    pub fn to_float(&self) -> PcmBlock {
        let samples = match &self.samples {
            Samples::Float(v) => Samples::Float(v.clone()),
            Samples::Int(v) => Samples::Float(v.iter().map(|&s| int_to_f32(s)).collect()),
        };
        PcmBlock {
            channels: self.channels,
            samples,
        }
    }
}

const I32_SCALE: f32 = 1.0 / 2_147_483_648.0;

#[inline]
fn int_to_f32(s: i32) -> f32 {
    s as f32 * I32_SCALE
}

/// Rounds a float sample to `valid_bits` and left-justifies it in an i32.
#[inline]
fn quantize(f: f32, valid_bits: u16) -> i32 {
    let scale = (1u64 << (valid_bits - 1)) as f64;
    let v = (f64::from(f) * scale).round().clamp(-scale, scale - 1.0) as i64;
    (v << (32 - valid_bits)) as i32
}

/// Packs decoded blocks into the device format: channel mapping, optional gain, and the
/// container conversion.
#[derive(Clone, Debug)]
pub struct Packer {
    out: OutputFormat,
    in_channels: u16,
    /// For every output channel, the input channel it carries (`None` = silence).
    map: Vec<Option<usize>>,
    gain_q16: Option<i64>,
    gain_linear: Option<f32>,
}

impl Packer {
    /// `in_channels` must not exceed the output channel count; downmix first with
    /// [`downmix`] when it does.
    pub fn new(out: OutputFormat, in_channels: u16, gain: SoftwareGain) -> Self {
        let in_ch = usize::from(in_channels.max(1));
        let map = (0..usize::from(out.channels))
            .map(|oc| match in_ch {
                1 if oc < 2 => Some(0),
                1 => None,
                n if oc < n => Some(oc),
                _ => None,
            })
            .collect();
        Self {
            out,
            in_channels: in_channels.max(1),
            map,
            gain_q16: gain.q16(),
            gain_linear: gain.linear(),
        }
    }

    pub fn output(&self) -> OutputFormat {
        self.out
    }

    pub fn set_gain(&mut self, gain: SoftwareGain) {
        self.gain_q16 = gain.q16();
        self.gain_linear = gain.linear();
    }

    /// Appends `block` to `dst` in the output byte layout (little-endian).
    pub fn pack(&self, block: &PcmBlock, dst: &mut Vec<u8>) {
        debug_assert_eq!(
            block.channels, self.in_channels,
            "packer built for another layout"
        );
        let in_ch = usize::from(block.channels.max(1));
        let frames = block.frames();
        dst.reserve(frames * self.out.bytes_per_frame());
        let sample = self.out.sample;

        match sample.kind {
            SampleKind::Int => {
                let valid = sample.valid_bits;
                let mask = (!0u32 << (32 - u32::from(valid))) as i32;
                let shift = 32 - u32::from(sample.container_bits);
                let bytes = sample.bytes();
                let write = |dst: &mut Vec<u8>, s: i32| {
                    let v = (s & mask) >> shift;
                    dst.extend_from_slice(&v.to_le_bytes()[..bytes]);
                };
                match &block.samples {
                    Samples::Int(data) => {
                        let gain = self.gain_q16;
                        self.for_each(frames, in_ch, |i| {
                            let s = i.map_or(0, |i| data[i]);
                            write(dst, gain.map_or(s, |g| ((i64::from(s) * g) >> 16) as i32));
                        });
                    }
                    Samples::Float(data) => {
                        let gain = self.gain_linear.unwrap_or(1.0);
                        self.for_each(frames, in_ch, |i| {
                            let s = i.map_or(0, |i| quantize(data[i] * gain, valid));
                            write(dst, s);
                        });
                    }
                }
            }
            SampleKind::Float => match &block.samples {
                Samples::Float(data) => {
                    let gain = self.gain_linear;
                    self.for_each(frames, in_ch, |i| {
                        let f = i.map_or(0.0, |i| gain.map_or(data[i], |g| data[i] * g));
                        dst.extend_from_slice(&f.to_le_bytes());
                    });
                }
                Samples::Int(data) => {
                    let gain = self.gain_q16;
                    self.for_each(frames, in_ch, |i| {
                        let s = i.map_or(0, |i| data[i]);
                        let s = gain.map_or(s, |g| ((i64::from(s) * g) >> 16) as i32);
                        dst.extend_from_slice(&int_to_f32(s).to_le_bytes());
                    });
                }
            },
        }
    }

    /// Calls `f` once per output sample, in interleaved order, with the index of the input
    /// sample it carries (`None` for silence).
    #[inline]
    fn for_each(&self, frames: usize, in_ch: usize, mut f: impl FnMut(Option<usize>)) {
        for frame in 0..frames {
            let base = frame * in_ch;
            for src in &self.map {
                f(src.map(|c| base + c));
            }
        }
    }
}

/// Folds surround layouts (FFmpeg default channel order) down to `out_channels`
/// (1 or 2) in float. This is a "converted" path, never bit-perfect.
pub fn downmix(block: &PcmBlock, out_channels: u16) -> PcmBlock {
    const C: f32 = std::f32::consts::FRAC_1_SQRT_2;
    let float = block.to_float();
    let Samples::Float(data) = &float.samples else {
        unreachable!()
    };
    let in_ch = usize::from(block.channels.max(1));
    let frames = block.frames();

    // (left weights, right weights) per input channel, by layout.
    let weights: Vec<(f32, f32)> = match in_ch {
        1 => vec![(1.0, 1.0)],
        2 => vec![(1.0, 0.0), (0.0, 1.0)],
        3 => vec![(1.0, 0.0), (0.0, 1.0), (C, C)],
        4 => vec![(1.0, 0.0), (0.0, 1.0), (C, 0.0), (0.0, C)],
        5 => vec![(1.0, 0.0), (0.0, 1.0), (C, C), (C, 0.0), (0.0, C)],
        6 => vec![
            (1.0, 0.0),
            (0.0, 1.0),
            (C, C),
            (0.0, 0.0),
            (C, 0.0),
            (0.0, C),
        ],
        _ => {
            // 7.1 and beyond: FL FR FC LFE BL BR SL SR …, alternate the rest left/right.
            let mut w = vec![(1.0, 0.0), (0.0, 1.0), (C, C), (0.0, 0.0)];
            for i in 4..in_ch {
                w.push(if i % 2 == 0 { (C, 0.0) } else { (0.0, C) });
            }
            w
        }
    };
    let norm_l: f32 = weights.iter().map(|w| w.0).sum::<f32>().max(1.0);
    let norm_r: f32 = weights.iter().map(|w| w.1).sum::<f32>().max(1.0);

    let out_ch = usize::from(out_channels.clamp(1, 2));
    let mut out = Vec::with_capacity(frames * out_ch);
    for frame in data.chunks_exact(in_ch) {
        let (mut l, mut r) = (0.0f32, 0.0f32);
        for (s, w) in frame.iter().zip(&weights) {
            l += s * w.0;
            r += s * w.1;
        }
        let (l, r) = (l / norm_l, r / norm_r);
        if out_ch == 1 {
            out.push((l + r) * 0.5);
        } else {
            out.push(l);
            out.push(r);
        }
    }
    PcmBlock {
        channels: out_ch as u16,
        samples: Samples::Float(out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::SampleFormat;

    fn out(sample: SampleFormat, channels: u16) -> OutputFormat {
        OutputFormat {
            sample_rate: 44_100,
            channels,
            sample,
        }
    }

    fn int_block(channels: u16, samples: Vec<i32>) -> PcmBlock {
        PcmBlock {
            channels,
            samples: Samples::Int(samples),
        }
    }

    fn pack(fmt: OutputFormat, block: &PcmBlock, gain: SoftwareGain) -> Vec<u8> {
        let mut dst = Vec::new();
        Packer::new(fmt, block.channels, gain).pack(block, &mut dst);
        dst
    }

    const S16: [i16; 6] = [0, 1, -1, i16::MAX, i16::MIN, 12_345];
    const S24: [i32; 6] = [0, 1, -1, 0x7F_FFFF, -0x80_0000, -1_234_567];

    fn s16_block() -> PcmBlock {
        let v: Vec<i32> = S16.iter().map(|&s| i32::from(s) << 16).collect();
        int_block(2, v)
    }

    fn s24_block() -> PcmBlock {
        int_block(2, S24.iter().map(|&s| s << 8).collect())
    }

    #[test]
    fn s16_into_s16_is_byte_identical() {
        let bytes = pack(out(SampleFormat::S16, 2), &s16_block(), SoftwareGain::UNITY);
        let expected: Vec<u8> = S16.iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn s16_widened_into_24_in_32_keeps_every_bit() {
        let bytes = pack(
            out(SampleFormat::S24_IN_32, 2),
            &s16_block(),
            SoftwareGain::UNITY,
        );
        let expected: Vec<u8> = S16
            .iter()
            .flat_map(|&s| (i32::from(s) << 16).to_le_bytes())
            .collect();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn s24_into_packed_24_is_byte_identical() {
        let bytes = pack(out(SampleFormat::S24, 2), &s24_block(), SoftwareGain::UNITY);
        let expected: Vec<u8> = S24
            .iter()
            .flat_map(|s| s.to_le_bytes()[..3].to_vec())
            .collect();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn s24_into_s32_is_left_justified() {
        let bytes = pack(out(SampleFormat::S32, 2), &s24_block(), SoftwareGain::UNITY);
        let expected: Vec<u8> = S24.iter().flat_map(|&s| (s << 8).to_le_bytes()).collect();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn s24_through_f32_round_trips_exactly() {
        let bytes = pack(out(SampleFormat::F32, 2), &s24_block(), SoftwareGain::UNITY);
        let floats: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        let back = pack(
            out(SampleFormat::S24, 2),
            &PcmBlock {
                channels: 2,
                samples: Samples::Float(floats),
            },
            SoftwareGain::UNITY,
        );
        let expected: Vec<u8> = S24
            .iter()
            .flat_map(|s| s.to_le_bytes()[..3].to_vec())
            .collect();
        assert_eq!(back, expected);
    }

    #[test]
    fn float_into_f32_is_bit_identical() {
        let data = vec![0.0f32, -0.0, 0.5, -1.0, 0.999_999_9, 1.0e-30];
        let block = PcmBlock {
            channels: 2,
            samples: Samples::Float(data.clone()),
        };
        let bytes = pack(out(SampleFormat::F32, 2), &block, SoftwareGain::UNITY);
        let expected: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn float_into_int_rounds_and_clamps() {
        let block = PcmBlock {
            channels: 1,
            samples: Samples::Float(vec![1.5, -1.5, 0.25]),
        };
        let bytes = pack(out(SampleFormat::S16, 1), &block, SoftwareGain::UNITY);
        let got: Vec<i16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect();
        assert_eq!(got, vec![i16::MAX, i16::MIN, 8192]);
    }

    #[test]
    fn mono_is_duplicated_onto_both_front_channels() {
        let block = int_block(1, vec![100 << 16, -7 << 16]);
        let bytes = pack(out(SampleFormat::S16, 2), &block, SoftwareGain::UNITY);
        let got: Vec<i16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect();
        assert_eq!(got, vec![100, 100, -7, -7]);
    }

    #[test]
    fn extra_device_channels_are_silent() {
        let block = int_block(2, vec![1 << 16, 2 << 16]);
        let bytes = pack(out(SampleFormat::S16, 4), &block, SoftwareGain::UNITY);
        let got: Vec<i16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect();
        assert_eq!(got, vec![1, 2, 0, 0]);
    }

    #[test]
    fn depth_reduction_truncates_without_dither() {
        let block = int_block(1, vec![0x12_3456 << 8, -0x12_3456 << 8]);
        let bytes = pack(out(SampleFormat::S16, 1), &block, SoftwareGain::UNITY);
        let got: Vec<i16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect();
        assert_eq!(got, vec![0x1234, (-0x12_3456i32 >> 8) as i16]);
    }

    #[test]
    fn gain_attenuates_in_the_32_bit_domain_and_keeps_low_bits() {
        let block = int_block(1, vec![1 << 16]); // 16-bit value 1
        let gain = SoftwareGain::new(-6.0206);
        let bytes = pack(out(SampleFormat::S32, 1), &block, gain);
        let got = i32::from_le_bytes(bytes[..4].try_into().unwrap());
        assert_eq!(
            got,
            1 << 15,
            "half of a 16-bit LSB survives in the 32-bit container"
        );
    }

    #[test]
    fn muted_gain_outputs_silence() {
        let bytes = pack(
            out(SampleFormat::S16, 2),
            &s16_block(),
            SoftwareGain::new(-80.0),
        );
        assert!(bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn downmix_51_to_stereo_keeps_centre_and_drops_lfe() {
        let fc = 0.5f32;
        let block = PcmBlock {
            channels: 6,
            samples: Samples::Float(vec![0.0, 0.0, fc, 0.9, 0.0, 0.0]),
        };
        let out = downmix(&block, 2);
        let Samples::Float(v) = out.samples else {
            panic!()
        };
        let norm = 1.0 + 2.0 * std::f32::consts::FRAC_1_SQRT_2;
        let expected = fc * std::f32::consts::FRAC_1_SQRT_2 / norm;
        assert!(
            (v[0] - expected).abs() < 1e-6 && (v[1] - expected).abs() < 1e-6,
            "{v:?}"
        );
    }
}
