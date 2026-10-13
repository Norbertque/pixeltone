//! A 10-band graphic equaliser: octave-spaced peaking filters (RBJ cookbook biquads) run in
//! 64-bit float on the render side, so changes are heard at once. Off means untouched:
//! the samples never pass through it.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::format::{OutputFormat, SampleFormat, SampleKind};

/// Band centres in Hz.
pub const BANDS: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
/// Band gain limit, dB.
pub const MAX_DB: f32 = 12.0;
/// About an octave wide.
const Q: f64 = 1.41;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct EqSettings {
    pub enabled: bool,
    /// The preset the gains came from ("Custom" once edited).
    pub preset: String,
    /// Gain per band in dB, one per [`BANDS`] entry.
    pub gains_db: Vec<f32>,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: "Flat".into(),
            gains_db: vec![0.0; BANDS.len()],
        }
    }
}

impl EqSettings {
    /// On and not flat: the samples will change.
    pub fn active(&self) -> bool {
        self.enabled && self.gains_db.iter().any(|g| g.abs() >= 0.05)
    }

    /// Headroom for the boosts, so the sum stays below full scale.
    pub fn preamp_db(&self) -> f32 {
        -self.gains_db.iter().copied().fold(0.0, f32::max)
    }

    /// One finite gain per band, within ±[`MAX_DB`] (settings files can be edited by hand).
    pub fn normalized(mut self) -> Self {
        self.gains_db.resize(BANDS.len(), 0.0);
        for g in &mut self.gains_db {
            *g = if g.is_finite() {
                g.clamp(-MAX_DB, MAX_DB)
            } else {
                0.0
            };
        }
        self
    }
}

/// Biquad coefficients (normalised, a0 = 1).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Coeffs {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Coeffs {
    /// RBJ peaking EQ.
    fn peaking(rate: f64, freq: f64, gain_db: f64) -> Self {
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * freq / rate;
        let alpha = w0.sin() / (2.0 * Q);
        let cos = w0.cos();
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: -2.0 * cos / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }
}

/// The running equaliser for one stream.
pub struct Equalizer {
    channels: usize,
    rate: f64,
    preamp: f64,
    /// The bands that aren't flat: index into [`BANDS`] and their filter.
    sections: Vec<(usize, Coeffs)>,
    /// Transposed direct form II state (z1, z2) per section per channel.
    state: Vec<[f64; 2]>,
}

impl Equalizer {
    pub fn new(settings: &EqSettings, rate: u32, channels: usize) -> Self {
        let mut eq = Self {
            channels,
            rate: f64::from(rate),
            preamp: 1.0,
            sections: Vec::new(),
            state: Vec::new(),
        };
        eq.set(settings);
        eq
    }

    /// New gains; each band's filter state carries over, so changes don't click.
    pub fn set(&mut self, settings: &EqSettings) {
        let nyquist_guard = self.rate * 0.45;
        let sections: Vec<(usize, Coeffs)> = BANDS
            .iter()
            .zip(&settings.gains_db)
            .enumerate()
            .filter(|(_, (f, g))| g.abs() >= 0.05 && f64::from(**f) < nyquist_guard)
            .map(|(band, (f, g))| {
                let gain = f64::from(g.clamp(-MAX_DB, MAX_DB));
                (band, Coeffs::peaking(self.rate, f64::from(*f), gain))
            })
            .collect();
        let channels = self.channels.max(1);
        let mut state = vec![[0.0; 2]; sections.len() * channels];
        for (s, (band, _)) in sections.iter().enumerate() {
            if let Some(old) = self.sections.iter().position(|(b, _)| b == band) {
                state[s * channels..(s + 1) * channels]
                    .copy_from_slice(&self.state[old * channels..(old + 1) * channels]);
            }
        }
        self.sections = sections;
        self.state = state;
        self.preamp = 10f64.powf(f64::from(settings.preamp_db()) / 20.0);
    }

    /// Filters interleaved samples in place.
    pub fn process(&mut self, samples: &mut [f64]) {
        let channels = self.channels.max(1);
        for (i, x) in samples.iter_mut().enumerate() {
            let ch = i % channels;
            let mut v = *x * self.preamp;
            for (s, (_, c)) in self.sections.iter().enumerate() {
                let z = &mut self.state[s * channels + ch];
                let y = c.b0 * v + z[0];
                z[0] = c.b1 * v - c.a1 * y + z[1];
                z[1] = c.b2 * v - c.a2 * y;
                v = y;
            }
            *x = v;
        }
    }
}

/// Device-format bytes → samples in −1.0..1.0.
pub fn decode(bytes: &[u8], sample: SampleFormat, out: &mut Vec<f64>) {
    out.clear();
    match (sample.kind, sample.container_bits) {
        (SampleKind::Float, _) => out.extend(
            bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| f64::from(f32::from_le_bytes(*c))),
        ),
        (SampleKind::Int, 16) => out.extend(
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| f64::from(i16::from_le_bytes(*c)) / 32_768.0),
        ),
        (SampleKind::Int, 24) => {
            out.extend(
                bytes.as_chunks::<3>().0.iter().map(|c| {
                    f64::from(i32::from_le_bytes([0, c[0], c[1], c[2]])) / 2_147_483_648.0
                }),
            )
        }
        (SampleKind::Int, _) => out.extend(
            bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| f64::from(i32::from_le_bytes(*c)) / 2_147_483_648.0),
        ),
    }
}

/// Samples → device-format bytes, rounded to the valid bits and clipped to full scale.
pub fn encode(samples: &[f64], sample: SampleFormat, bytes: &mut [u8]) {
    match (sample.kind, sample.container_bits) {
        (SampleKind::Float, _) => {
            for (c, v) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(samples) {
                *c = (v.clamp(-1.0, 1.0) as f32).to_le_bytes();
            }
        }
        (SampleKind::Int, container) => {
            let valid = u32::from(sample.valid_bits.clamp(8, 32));
            let scale = f64::from(1u32 << (valid - 1));
            let (lo, hi) = (-scale, scale - 1.0);
            let shift = 32 - valid;
            let to_i32 = |v: f64| ((v * scale).round().clamp(lo, hi) as i64 as i32) << shift;
            match container {
                16 => {
                    for (c, v) in bytes.as_chunks_mut::<2>().0.iter_mut().zip(samples) {
                        *c = ((to_i32(*v) >> 16) as i16).to_le_bytes();
                    }
                }
                24 => {
                    for (c, v) in bytes.as_chunks_mut::<3>().0.iter_mut().zip(samples) {
                        let b = to_i32(*v).to_le_bytes();
                        c.copy_from_slice(&b[1..]);
                    }
                }
                _ => {
                    for (c, v) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(samples) {
                        *c = to_i32(*v).to_le_bytes();
                    }
                }
            }
        }
    }
}

/// Runs `eq` over device bytes in place.
pub fn apply(eq: &mut Equalizer, bytes: &mut [u8], format: OutputFormat, scratch: &mut Vec<f64>) {
    decode(bytes, format.sample, scratch);
    eq.process(scratch);
    encode(scratch, format.sample, bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(gains: [f32; 10]) -> EqSettings {
        EqSettings {
            enabled: true,
            preset: "Custom".into(),
            gains_db: gains.to_vec(),
        }
    }

    /// Steady-state gain (dB) of a sine at `freq` through the equaliser.
    fn response(eq: &mut Equalizer, rate: u32, freq: f64) -> f64 {
        let n = rate as usize;
        let mut x: Vec<f64> = (0..n)
            .map(|i| 0.25 * (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(rate)).sin())
            .collect();
        eq.process(&mut x);
        let tail = &x[n / 2..];
        let rms = (tail.iter().map(|v| v * v).sum::<f64>() / tail.len() as f64).sqrt();
        20.0 * (rms / (0.25 / 2f64.sqrt())).log10()
    }

    #[test]
    fn a_band_boosts_its_own_frequency_and_leaves_others_alone() {
        let mut gains = [0.0; 10];
        gains[5] = 6.0; // 1 kHz
        let s = settings(gains);
        // Pre-amp −6 dB plus the +6 dB band: unity at 1 kHz, −6 dB far away.
        let mut eq = Equalizer::new(&s, 48_000, 1);
        assert!((response(&mut eq, 48_000, 1000.0) - 0.0).abs() < 0.3);
        let mut eq = Equalizer::new(&s, 48_000, 1);
        assert!((response(&mut eq, 48_000, 60.0) + 6.0).abs() < 0.3);
        let mut eq = Equalizer::new(&s, 48_000, 1);
        assert!((response(&mut eq, 48_000, 15_000.0) + 6.0).abs() < 0.3);
    }

    #[test]
    fn changing_the_gains_keeps_the_filters_running() {
        let mut gains = [0.0; 10];
        gains[2] = 4.0;
        let s = settings(gains);
        let input: Vec<f64> = (0..4000)
            .map(|i| 0.3 * (2.0 * std::f64::consts::PI * 130.0 * i as f64 / 48_000.0).sin())
            .collect();
        let mut whole = input.clone();
        Equalizer::new(&s, 48_000, 2).process(&mut whole);

        let mut split = input.clone();
        let mut eq = Equalizer::new(&s, 48_000, 2);
        eq.process(&mut split[..2000]);
        eq.set(&s);
        eq.process(&mut split[2000..]);
        assert!(whole == split, "re-applying the same gains is seamless");

        // A band appearing in front of it keeps the 125 Hz filter's state.
        let before = eq.state.clone();
        let mut more = gains;
        more[0] = 3.0;
        eq.set(&settings(more));
        assert_eq!(eq.sections.len(), 2);
        assert_eq!(eq.state[2..4], before[..2]);
        assert_eq!(eq.state[..2], [[0.0; 2]; 2]);
    }

    #[test]
    fn flat_and_off_are_inactive() {
        assert!(!EqSettings::default().active());
        assert!(!settings([0.0; 10]).active());
        let mut s = settings([3.0; 10]);
        assert!(s.active());
        s.enabled = false;
        assert!(!s.active());
        assert_eq!(
            settings([3.0, -2.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]).preamp_db(),
            -5.0
        );
        let odd = EqSettings {
            gains_db: vec![40.0, f32::NAN, -3.0],
            ..EqSettings::default()
        };
        let fixed = odd.normalized().gains_db;
        assert_eq!(fixed.len(), BANDS.len());
        assert_eq!(fixed[..3], [MAX_DB, 0.0, -3.0]);
    }

    #[test]
    fn decode_encode_round_trip_every_container() {
        for sample in SampleFormat::PROBE_ORDER {
            let format_bytes = sample.bytes();
            let values = [0.5, -0.25, 0.999, -1.0];
            let mut bytes = vec![0u8; values.len() * format_bytes];
            encode(&values, sample, &mut bytes);
            let mut back = Vec::new();
            decode(&bytes, sample, &mut back);
            let tolerance = if sample.kind == SampleKind::Float {
                1e-7
            } else {
                2.0 / f64::from(1u32 << (sample.valid_bits - 1))
            };
            for (a, b) in values.iter().zip(&back) {
                assert!((a - b).abs() <= tolerance, "{sample:?}: {a} vs {b}");
            }
        }
        // 24-in-32 keeps the low byte clear; full scale clips instead of wrapping.
        let mut bytes = vec![0u8; 8];
        encode(&[0.5, 2.0], SampleFormat::S24_IN_32, &mut bytes);
        assert_eq!(bytes[0], 0);
        assert_eq!(
            i32::from_le_bytes(bytes[4..].try_into().unwrap()),
            0x7FFF_FF00
        );
    }
}
