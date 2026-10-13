//! Per-track sound processing done while decoding: ReplayGain and crossfading. Both are
//! opt-in; with them off, decoded blocks pass through untouched.

use pp_core::{PcmBlock, Samples};
use pp_decode::ReplayGainTags;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ReplayGainMode {
    #[default]
    Off,
    /// Each track at the same loudness.
    Track,
    /// Albums at the same loudness, keeping the differences between their tracks.
    Album,
}

/// The gain (dB) ReplayGain asks for: the chosen value (falling back to the other one),
/// plus `preamp_db`, never more than the tagged peak allows. `None` when off or untagged.
pub fn replay_gain_db(mode: ReplayGainMode, preamp_db: f32, tags: &ReplayGainTags) -> Option<f32> {
    let (gain, peak) = match mode {
        ReplayGainMode::Off => return None,
        ReplayGainMode::Track => (
            tags.track_gain_db.or(tags.album_gain_db),
            tags.track_peak.or(tags.album_peak),
        ),
        ReplayGainMode::Album => (
            tags.album_gain_db.or(tags.track_gain_db),
            tags.album_peak.or(tags.track_peak),
        ),
    };
    let mut db = gain? + preamp_db;
    if let Some(peak) = peak.filter(|p| *p > 0.0) {
        db = db.min(-20.0 * peak.log10());
    }
    (db.abs() >= 0.01).then_some(db)
}

/// Multiplies a decoded block by `factor` (linear), clipping at full scale.
pub fn apply_gain(block: &mut PcmBlock, factor: f32) {
    match &mut block.samples {
        Samples::Int(v) => {
            let f = f64::from(factor);
            for s in v.iter_mut() {
                *s = (f64::from(*s) * f)
                    .round()
                    .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
            }
        }
        Samples::Float(v) => {
            for s in v.iter_mut() {
                *s = (*s * factor).clamp(-1.0, 1.0);
            }
        }
    }
}

/// Mixes the end of one track into the start of the next (interleaved f32 bytes, equal
/// power): `outgoing` fades out while `incoming` fades in over their common length. Returns
/// the mixed bytes; whatever `incoming` has beyond the overlap follows unchanged.
pub fn crossfade(outgoing: &[u8], incoming: &[u8], channels: usize) -> Vec<u8> {
    let frame = 4 * channels.max(1);
    let overlap = outgoing.len().min(incoming.len()) / frame;
    let mut out = Vec::with_capacity(incoming.len().max(outgoing.len()));
    for i in 0..overlap {
        let t = (i as f32 + 0.5) / overlap as f32;
        let (fade_out, fade_in) = (
            (t * std::f32::consts::FRAC_PI_2).cos(),
            (t * std::f32::consts::FRAC_PI_2).sin(),
        );
        for c in 0..channels {
            let at = i * frame + c * 4;
            let a = f32::from_le_bytes(outgoing[at..at + 4].try_into().unwrap_or_default());
            let b = f32::from_le_bytes(incoming[at..at + 4].try_into().unwrap_or_default());
            out.extend_from_slice(&(a * fade_out + b * fade_in).clamp(-1.0, 1.0).to_le_bytes());
        }
    }
    out.extend_from_slice(&incoming[overlap * frame..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(track: Option<f32>, album: Option<f32>, peak: Option<f32>) -> ReplayGainTags {
        ReplayGainTags {
            track_gain_db: track,
            track_peak: peak,
            album_gain_db: album,
            album_peak: None,
        }
    }

    #[test]
    fn picks_the_mode_falls_back_and_respects_the_peak() {
        let t = tags(Some(-6.0), Some(-8.0), None);
        assert_eq!(replay_gain_db(ReplayGainMode::Off, 0.0, &t), None);
        assert_eq!(replay_gain_db(ReplayGainMode::Track, 0.0, &t), Some(-6.0));
        assert_eq!(replay_gain_db(ReplayGainMode::Album, 0.0, &t), Some(-8.0));
        assert_eq!(
            replay_gain_db(ReplayGainMode::Album, 0.0, &tags(Some(-6.0), None, None)),
            Some(-6.0)
        );
        assert_eq!(replay_gain_db(ReplayGainMode::Track, 2.0, &t), Some(-4.0));
        assert_eq!(
            replay_gain_db(ReplayGainMode::Track, 0.0, &tags(None, None, None)),
            None
        );
        // +3 dB asked, but the peak (0.9 ≈ −0.92 dBFS) only allows +0.92 dB.
        let boost = replay_gain_db(
            ReplayGainMode::Track,
            0.0,
            &tags(Some(3.0), None, Some(0.9)),
        )
        .unwrap();
        assert!((boost - 0.915).abs() < 0.01, "{boost}");
    }

    #[test]
    fn gain_scales_ints_and_floats_and_clips() {
        let mut ints = PcmBlock {
            channels: 1,
            samples: Samples::Int(vec![1 << 20, -(1 << 20), i32::MAX]),
        };
        apply_gain(&mut ints, 0.5);
        assert_eq!(
            ints.samples,
            Samples::Int(vec![
                1 << 19,
                -(1 << 19),
                (i32::MAX as f64 * 0.5).round() as i32
            ])
        );
        apply_gain(&mut ints, 8.0);
        let Samples::Int(v) = &ints.samples else {
            unreachable!()
        };
        assert_eq!(v[2], i32::MAX, "clips instead of wrapping");

        let mut floats = PcmBlock {
            channels: 1,
            samples: Samples::Float(vec![0.5, -0.9]),
        };
        apply_gain(&mut floats, 2.0);
        assert_eq!(floats.samples, Samples::Float(vec![1.0, -1.0]));
    }

    #[test]
    fn crossfade_keeps_constant_power_and_the_rest_of_the_next_track() {
        let a: Vec<u8> = (0..100).flat_map(|_| 0.5f32.to_le_bytes()).collect();
        let b: Vec<u8> = (0..150).flat_map(|_| 0.5f32.to_le_bytes()).collect();
        let mixed = crossfade(&a, &b, 1);
        assert_eq!(
            mixed.len(),
            b.len(),
            "the overlap replaces both, then the next track goes on"
        );
        let v: Vec<f32> = mixed
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect();
        // Correlated material peaks at sqrt(2) × in the middle of an equal-power fade.
        assert!(v[0] > 0.49 && v[0] < 0.55);
        assert!(v[50] > 0.69 && v[50] < 0.72);
        assert!(v[99] > 0.49 && v[99] < 0.55);
        assert!(v[100..].iter().all(|x| *x == 0.5));
    }
}
