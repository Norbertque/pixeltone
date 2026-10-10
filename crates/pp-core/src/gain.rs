//! Software gain stage for exclusive-mode devices without a hardware volume control.
//!
//! Policy (same as the Android USB mode): first use starts at [`DEFAULT_DB`], levels above
//! [`LOCKED_MAX_DB`] stay locked until the user acknowledges the loudness warning, and
//! 0 dB skips the multiply entirely — the only level the badge calls bit-perfect.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::format::{SampleFormat, SampleKind};

/// Level applied the first time a volume-less device is used.
pub const DEFAULT_DB: f32 = -30.0;
/// Highest level allowed before the loudness warning is acknowledged.
pub const LOCKED_MAX_DB: f32 = -10.0;
/// At or below this the output is muted.
pub const FLOOR_DB: f32 = -60.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SoftwareGain {
    pub db: f32,
}

impl Default for SoftwareGain {
    fn default() -> Self {
        Self::UNITY
    }
}

impl SoftwareGain {
    pub const UNITY: Self = Self { db: 0.0 };

    /// Attenuation only: positive values are clamped to unity.
    pub fn new(db: f32) -> Self {
        Self {
            db: if db.is_finite() { db.min(0.0) } else { 0.0 },
        }
    }

    pub fn is_unity(&self) -> bool {
        self.db >= 0.0
    }

    pub fn is_muted(&self) -> bool {
        self.db <= FLOOR_DB
    }

    /// Q16.16 multiplier for the 32-bit integer domain; `None` at unity.
    pub fn q16(&self) -> Option<i64> {
        if self.is_unity() {
            None
        } else if self.is_muted() {
            Some(0)
        } else {
            Some((10f64.powf(f64::from(self.db) / 20.0) * 65536.0).round() as i64)
        }
    }

    /// Linear multiplier for float samples; `None` at unity.
    pub fn linear(&self) -> Option<f32> {
        if self.is_unity() {
            None
        } else if self.is_muted() {
            Some(0.0)
        } else {
            Some(10f64.powf(f64::from(self.db) / 20.0) as f32)
        }
    }
}

/// Applies a Q16.16 gain in place to samples already in `sample`'s container layout
/// (little-endian). Used by output render threads so volume changes act immediately.
pub fn apply_q16_in_place(bytes: &mut [u8], sample: SampleFormat, q16: i64) {
    match (sample.kind, sample.container_bits) {
        (SampleKind::Float, _) => {
            let g = q16 as f32 / 65536.0;
            for c in bytes.as_chunks_mut::<4>().0 {
                *c = (f32::from_le_bytes(*c) * g).to_le_bytes();
            }
        }
        (SampleKind::Int, 16) => {
            for c in bytes.as_chunks_mut::<2>().0 {
                let v = i64::from(i16::from_le_bytes(*c));
                *c = (((v * q16) >> 16) as i16).to_le_bytes();
            }
        }
        (SampleKind::Int, 24) => {
            for c in bytes.as_chunks_mut::<3>().0 {
                let v = i64::from(i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8);
                let out = (((v * q16) >> 16) as i32).to_le_bytes();
                c.copy_from_slice(&out[..3]);
            }
        }
        (SampleKind::Int, _) => {
            let mask = (!0u32 << (32 - u32::from(sample.valid_bits.clamp(1, 32)))) as i32;
            for c in bytes.as_chunks_mut::<4>().0 {
                let v = i64::from(i32::from_le_bytes(*c));
                *c = (((v * q16) >> 16) as i32 & mask).to_le_bytes();
            }
        }
    }
}

/// Clamps a requested level to the policy.
pub fn clamp_to_policy(db: f32, warning_acknowledged: bool) -> f32 {
    let max = if warning_acknowledged {
        0.0
    } else {
        LOCKED_MAX_DB
    };
    if db.is_finite() {
        db.clamp(FLOOR_DB, max)
    } else {
        DEFAULT_DB
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_skips_the_multiply() {
        assert_eq!(SoftwareGain::UNITY.q16(), None);
        assert_eq!(SoftwareGain::new(3.0).q16(), None);
        assert_eq!(SoftwareGain::UNITY.linear(), None);
    }

    #[test]
    fn floor_mutes() {
        assert_eq!(SoftwareGain::new(FLOOR_DB).q16(), Some(0));
        assert_eq!(SoftwareGain::new(-90.0).linear(), Some(0.0));
    }

    #[test]
    fn minus_six_db_is_about_half() {
        let q = SoftwareGain::new(-6.0206).q16().unwrap();
        assert!((q - 32768).abs() <= 1, "q16 = {q}");
    }

    #[test]
    fn in_place_gain_halves_every_container() {
        let half = SoftwareGain::new(-6.0206).q16().unwrap();

        let mut s16: Vec<u8> = [1000i16, -1000]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        apply_q16_in_place(&mut s16, SampleFormat::S16, half);
        assert_eq!(
            s16,
            [500i16, -500]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>()
        );

        let mut s24: Vec<u8> = [1000i32, -1000]
            .iter()
            .flat_map(|v| v.to_le_bytes()[..3].to_vec())
            .collect();
        apply_q16_in_place(&mut s24, SampleFormat::S24, half);
        let expected: Vec<u8> = [500i32, -500]
            .iter()
            .flat_map(|v| v.to_le_bytes()[..3].to_vec())
            .collect();
        assert_eq!(s24, expected);

        // 24 valid bits in a 32-bit slot stay left-justified (low byte zero).
        let mut s2432: Vec<u8> = (1001i32 << 8).to_le_bytes().to_vec();
        apply_q16_in_place(&mut s2432, SampleFormat::S24_IN_32, half);
        let v = i32::from_le_bytes(s2432.try_into().unwrap());
        assert_eq!(v & 0xFF, 0);
        assert_eq!(v >> 8, 500);

        let mut f: Vec<u8> = 0.5f32.to_le_bytes().to_vec();
        apply_q16_in_place(&mut f, SampleFormat::F32, half);
        assert!((f32::from_le_bytes(f.try_into().unwrap()) - 0.25).abs() < 1e-4);
    }

    #[test]
    fn policy_caps_until_acknowledged() {
        assert_eq!(clamp_to_policy(-3.0, false), LOCKED_MAX_DB);
        assert_eq!(clamp_to_policy(-3.0, true), -3.0);
        assert_eq!(clamp_to_policy(5.0, true), 0.0);
        assert_eq!(clamp_to_policy(-100.0, true), FLOOR_DB);
        assert_eq!(clamp_to_policy(f32::NAN, true), DEFAULT_DB);
    }
}
