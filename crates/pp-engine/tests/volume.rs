//! Which volume control applies where: the device's own, the app's attenuation in exclusive
//! mode, line level, and the app's volume in shared mode.

mod common;

use common::*;

fn config() -> EngineConfig {
    EngineConfig {
        preroll_ms: 0,
        ..EngineConfig::default()
    }
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

#[test]
fn the_app_volume_scales_shared_mode() {
    let dir = tempfile::tempdir().unwrap();
    // 48 kHz: the mix format's rate, so nothing but the volume changes the samples.
    let f = fixture(
        dir.path(),
        "a.flac",
        48_000,
        "0.3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let settings = OutputSettings {
        shared_gain_db: -20.0,
        ..OutputSettings::default()
    };
    let h = harness_full(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        settings,
        config(),
    );
    let events = h.play_to_end(vec![f.clone()]);

    let badge = last_badge(&events).unwrap();
    assert_eq!(badge.gain_db, Some(-20.0));
    assert!(badge.label.ends_with("Windows mixer"), "{}", badge.label);
    let g = SoftwareGain::new(-20.0).q16().unwrap() as f32 / 65536.0;
    let expected: Vec<f32> = floats(&reference(&f, "f32le"))
        .iter()
        .map(|v| v * g)
        .collect();
    assert!(floats(&h.backend.take_captured()) == expected);
}

#[test]
fn full_app_volume_leaves_shared_mode_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "a.flac",
        48_000,
        "0.3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), false);
    let events = h.play_to_end(vec![f.clone()]);
    assert_eq!(last_badge(&events).unwrap().gain_db, None);
    assert!(h.backend.take_captured() == reference(&f, "f32le"));
}

#[test]
fn the_app_can_attenuate_even_where_the_device_has_its_own_volume() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "a.flac",
        44_100,
        "0.3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let caps = CaptureBackend::typical_dac();
    assert!(caps.hardware_volume);
    let settings = OutputSettings {
        exclusive: true,
        software_volume: true,
        software_gain_db: -30.0,
        ..OutputSettings::default()
    };
    let h = harness_full(CaptureBackend::new(caps), settings, config());
    let events = h.play_to_end(vec![f.clone()]);
    assert_eq!(h.backend.opened.lock()[0].format.sample, SampleFormat::S32);
    assert_eq!(
        last_badge(&events).unwrap().label,
        "Exclusive • 32-bit/44.1 kHz • −30 dB"
    );
}

#[test]
fn line_level_stays_bit_perfect_without_a_device_volume() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "a.flac",
        44_100,
        "0.3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let mut caps = CaptureBackend::typical_dac();
    caps.hardware_volume = false;
    let settings = OutputSettings {
        exclusive: true,
        line_level: true,
        ..OutputSettings::default()
    };
    let h = harness_full(CaptureBackend::new(caps), settings, config());
    let events = h.play_to_end(vec![f.clone()]);
    assert!(last_badge(&events).unwrap().bit_perfect);
    assert!(h.backend.take_captured() == reference(&f, "s16le"));
}

#[test]
fn volume_changes_apply_without_reopening_the_device() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "a.flac",
        48_000,
        "3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let h = harness(
        CaptureBackend::new(CaptureBackend::typical_dac()).paced(1.0),
        false,
    );
    h.engine.play_files(vec![f], 0);
    h.wait_for("playing", |s| {
        s.status == PlaybackStatus::Playing && s.output.is_some()
    });

    h.engine.set_output(OutputSettings {
        shared_gain_db: -12.0,
        ..OutputSettings::default()
    });
    h.wait_for("the new volume", |s| {
        s.output
            .as_ref()
            .is_some_and(|o| o.badge.gain_db == Some(-12.0))
    });
    assert_eq!(
        h.backend.opened.lock().len(),
        1,
        "no reopen for a volume change"
    );
    h.engine.stop();
}
