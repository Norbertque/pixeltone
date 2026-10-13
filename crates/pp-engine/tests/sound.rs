//! M5 end to end: DSD over DoP, ReplayGain, the equaliser and crossfading, through real
//! files, the engine and a capture device.

mod common;

use std::path::{Path, PathBuf};

use common::*;
use pp_core::dop::{DopPacker, MARKER_A, MARKER_B};
use pp_core::{EqSettings, OutputFormat};
use pp_engine::ReplayGainMode;

fn config() -> EngineConfig {
    EngineConfig {
        preroll_ms: 0,
        ..EngineConfig::default()
    }
}

fn exclusive(settings: OutputSettings) -> OutputSettings {
    OutputSettings {
        exclusive: true,
        ..settings
    }
}

/// Stereo DSD64 with known bytes; returns the file and the interleaved MSB-first source.
fn dsf(dir: &Path, bytes: usize) -> (PathBuf, Vec<u8>) {
    let left: Vec<u8> = (0..bytes as u32).map(|i| (i * 7 + 3) as u8).collect();
    let right: Vec<u8> = (0..bytes as u32).map(|i| (i * 13 + 5) as u8).collect();
    let path = dir.join("tone.dsf");
    pp_decode::testing::write_dsf(&path, 2_822_400, &[left.clone(), right.clone()]).unwrap();
    let interleaved = (0..bytes).flat_map(|i| [left[i], right[i]]).collect();
    (path, interleaved)
}

const DOP: OutputFormat = OutputFormat {
    sample_rate: 176_400,
    channels: 2,
    sample: SampleFormat::S24_IN_32,
};

fn markers(bytes: &[u8]) -> Vec<u8> {
    bytes.as_chunks::<8>().0.iter().map(|f| f[3]).collect()
}

#[test]
fn dsd_reaches_a_dop_dac_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let (path, source) = dsf(dir.path(), 6000);
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        exclusive(OutputSettings::default()),
        config(),
    );
    let events = h.play_to_end(vec![path]);

    let opened = h.backend.opened.lock().clone();
    assert_eq!(opened[0].format, DOP);
    let badge = last_badge(&events).unwrap();
    assert!(badge.bit_perfect);
    assert_eq!(badge.label, "Exclusive • DSD64 over DoP • Bit-perfect");

    let mut expected = Vec::new();
    let mut packer = DopPacker::new(DOP);
    packer.pack(&source, &mut expected);
    packer.finish(&mut expected);
    let captured = h.backend.take_captured();
    assert_eq!(
        captured.len(),
        3000 * 8,
        "two DSD bytes per channel per frame"
    );
    assert!(captured == expected, "every DSD bit arrives as sent");
}

#[test]
fn dop_preroll_is_idle_that_keeps_the_marker_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let (path, _) = dsf(dir.path(), 4000);
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        exclusive(OutputSettings::default()),
        EngineConfig {
            preroll_ms: 150,
            ..EngineConfig::default()
        },
    );
    h.play_to_end(vec![path]);
    let captured = h.backend.take_captured();
    let preroll = 176_400 * 150 / 1000;
    let m = markers(&captured);
    assert_eq!(m.len(), preroll + 2000);
    assert!(
        m.iter()
            .enumerate()
            .all(|(i, &x)| x == if i % 2 == 0 { MARKER_A } else { MARKER_B }),
        "markers alternate from the first idle frame through the music"
    );
    assert!(
        captured[..preroll * 8]
            .as_chunks::<4>()
            .0
            .iter()
            .all(|s| s[1] == 0x69 && s[2] == 0x69)
    );
}

#[test]
fn without_hardware_volume_dsd_is_converted_to_pcm_instead() {
    let dir = tempfile::tempdir().unwrap();
    let (path, _) = dsf(dir.path(), 6000);
    let mut caps = CaptureBackend::typical_dac();
    caps.hardware_volume = false;
    let h = harness_full(
        CaptureBackend::new(caps),
        exclusive(OutputSettings::default()),
        config(),
    );
    let events = h.play_to_end(vec![path]);
    let badge = last_badge(&events).unwrap();
    assert!(badge.conversion.dsd_to_pcm && !badge.conversion.dop);
    assert!(!badge.bit_perfect);
    assert_eq!(h.backend.opened.lock()[0].format.sample_rate, 352_800);
}

#[test]
fn replaygain_scales_each_track_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "rg.flac",
        44_100,
        "0.5",
        &[
            "-c:a",
            "flac",
            "-sample_fmt",
            "s16",
            "-metadata",
            "REPLAYGAIN_TRACK_GAIN=-6.02 dB",
        ],
    );
    let settings = exclusive(OutputSettings {
        replay_gain: ReplayGainMode::Track,
        ..OutputSettings::default()
    });
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        settings,
        config(),
    );
    let events = h.play_to_end(vec![f.clone()]);

    // Gain wants the deepest container, so the attenuated low bits survive.
    assert_eq!(h.backend.opened.lock()[0].format.sample, SampleFormat::S32);
    let badge = last_badge(&events).unwrap();
    assert!(!badge.bit_perfect);
    assert_eq!(badge.effects.replay_gain_db, Some(-6.02));
    assert!(
        badge.label.contains("ReplayGain −6.0 dB"),
        "{}",
        badge.label
    );

    let factor = f64::from(10f32.powf(-6.02 / 20.0));
    let expected: Vec<i32> = reference(&f, "s16le")
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| (f64::from(i32::from(i16::from_le_bytes(*c)) << 16) * factor).round() as i32)
        .collect();
    let captured: Vec<i32> = h
        .backend
        .take_captured()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| i32::from_le_bytes(*c))
        .collect();
    assert!(captured == expected);
}

#[test]
fn the_equaliser_changes_the_sound_only_when_active() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "eq.flac",
        44_100,
        "0.5",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );

    // On but flat: untouched.
    let flat = exclusive(OutputSettings {
        eq: EqSettings {
            enabled: true,
            ..EqSettings::default()
        },
        ..OutputSettings::default()
    });
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        flat,
        config(),
    );
    let events = h.play_to_end(vec![f.clone()]);
    assert!(last_badge(&events).unwrap().bit_perfect);
    assert!(h.backend.take_captured() == reference(&f, "s16le"));

    let mut gains = vec![0.0; 10];
    gains[5] = 6.0;
    let boosted = exclusive(OutputSettings {
        eq: EqSettings {
            enabled: true,
            preset: "Custom".into(),
            gains_db: gains,
        },
        ..OutputSettings::default()
    });
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        boosted,
        config(),
    );
    let events = h.play_to_end(vec![f.clone()]);
    let badge = last_badge(&events).unwrap();
    assert!(!badge.bit_perfect);
    assert!(badge.label.contains("EQ"), "{}", badge.label);
    assert!(h.backend.take_captured() != reference(&f, "s16le"));
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

/// Two one-second 48 kHz tracks (shared mode plays them without resampling).
fn two_tracks(dir: &Path, same_album: bool) -> (PathBuf, PathBuf) {
    let a = fixture(
        dir,
        "a.flac",
        48_000,
        "1",
        &[
            "-c:a",
            "flac",
            "-sample_fmt",
            "s16",
            "-metadata",
            "album=One",
        ],
    );
    let album_b = if same_album { "album=One" } else { "album=Two" };
    let b = fixture(
        dir,
        "b.flac",
        48_000,
        "1",
        &["-c:a", "flac", "-sample_fmt", "s16", "-metadata", album_b],
    );
    (a, b)
}

#[test]
fn crossfade_overlaps_tracks_in_shared_mode() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_tracks(dir.path(), false);
    let settings = OutputSettings {
        crossfade_ms: 250,
        ..OutputSettings::default()
    };
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        settings,
        config(),
    );
    let events = h.play_to_end(vec![a.clone(), b.clone()]);
    assert_eq!(track_sequence(&events).len(), 2);

    let captured = floats(&h.backend.take_captured());
    let (ra, rb) = (
        floats(&reference(&a, "f32le")),
        floats(&reference(&b, "f32le")),
    );
    let overlap = 12_000 * 2; // 250 ms of stereo samples
    assert_eq!(captured.len(), ra.len() + rb.len() - overlap);
    let mix_start = ra.len() - overlap;
    assert!(
        captured[..mix_start] == ra[..mix_start],
        "A untouched before the mix"
    );
    assert!(
        captured[mix_start + overlap..] == rb[overlap..],
        "B untouched after it"
    );
    assert!(
        captured[mix_start..mix_start + overlap] != ra[mix_start..],
        "the overlap is a mix"
    );
}

#[test]
fn tracks_of_one_album_stay_gapless_unless_asked() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_tracks(dir.path(), true);
    let settings = OutputSettings {
        crossfade_ms: 250,
        ..OutputSettings::default()
    };
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        settings,
        config(),
    );
    h.play_to_end(vec![a.clone(), b.clone()]);
    let mut expected = reference(&a, "f32le");
    expected.extend(reference(&b, "f32le"));
    assert!(h.backend.take_captured() == expected);
}

#[test]
fn exclusive_mode_never_crossfades() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_tracks(dir.path(), false);
    let settings = exclusive(OutputSettings {
        crossfade_ms: 250,
        ..OutputSettings::default()
    });
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        settings,
        config(),
    );
    h.play_to_end(vec![a.clone(), b.clone()]);
    let mut expected = reference(&a, "s16le");
    expected.extend(reference(&b, "s16le"));
    assert!(
        h.backend.take_captured() == expected,
        "bit-perfect and gapless"
    );
}
