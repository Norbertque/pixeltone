//! End-to-end: real files through decoder, negotiator, worker and engine into a capture
//! device, compared byte for byte with FFmpeg's own decode.

mod common;

use std::time::Duration;

use common::*;

#[test]
fn flac_24_bit_reaches_the_device_bit_perfect() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "a.flac",
        96_000,
        "1",
        &[
            "-c:a",
            "flac",
            "-sample_fmt",
            "s32",
            "-bits_per_raw_sample",
            "24",
        ],
    );
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), true);
    let events = h.play_to_end(vec![f.clone()]);

    let opened = h.backend.opened.lock().clone();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].mode, ShareMode::Exclusive);
    assert_eq!(opened[0].format.sample, SampleFormat::S24_IN_32);
    assert_eq!(opened[0].format.sample_rate, 96_000);

    // FFmpeg's s32le decode of 24-bit material is left-justified: the same bytes.
    let captured = h.backend.take_captured();
    let want = reference(&f, "s32le");
    assert_eq!(captured.len(), want.len(), "length");
    assert!(captured == want, "device bytes differ from FFmpeg's decode");

    let badge = last_badge(&events).expect("badge");
    assert!(badge.bit_perfect, "{}", badge.label);
    assert_eq!(badge.label, "Exclusive • 24-bit/96 kHz • Bit-perfect");
}

#[test]
fn flac_16_bit_uses_a_16_bit_container_bit_perfect() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "a.flac",
        44_100,
        "1",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), true);
    h.play_to_end(vec![f.clone()]);
    assert_eq!(h.backend.opened.lock()[0].format.sample, SampleFormat::S16);
    assert!(h.backend.take_captured() == reference(&f, "s16le"));
}

#[test]
fn alac_and_wavpack_are_bit_perfect() {
    let dir = tempfile::tempdir().unwrap();
    let alac = fixture(
        dir.path(),
        "a.m4a",
        192_000,
        "0.5",
        &[
            "-c:a",
            "alac",
            "-sample_fmt",
            "s32p",
            "-bits_per_raw_sample",
            "24",
        ],
    );
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), true);
    h.play_to_end(vec![alac.clone()]);
    assert!(
        h.backend.take_captured() == reference(&alac, "s32le"),
        "ALAC"
    );

    let wv = fixture(
        dir.path(),
        "a.wv",
        88_200,
        "0.5",
        &["-c:a", "wavpack", "-sample_fmt", "s16p"],
    );
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), true);
    h.play_to_end(vec![wv.clone()]);
    assert!(
        h.backend.take_captured() == reference(&wv, "s16le"),
        "WavPack"
    );
}

#[test]
fn same_format_tracks_play_gaplessly_in_one_stream() {
    let dir = tempfile::tempdir().unwrap();
    let a = fixture(
        dir.path(),
        "a.flac",
        44_100,
        "0.7",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let b = fixture(
        dir.path(),
        "b.flac",
        44_100,
        "0.4",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), true);
    h.play_to_end(vec![a.clone(), b.clone()]);

    assert_eq!(
        h.backend.opened.lock().len(),
        1,
        "one stream for both tracks"
    );
    let mut want = reference(&a, "s16le");
    want.extend(reference(&b, "s16le"));
    assert!(h.backend.take_captured() == want, "no gap, no overlap");
}

#[test]
fn a_rate_change_reopens_the_device() {
    let dir = tempfile::tempdir().unwrap();
    let a = fixture(
        dir.path(),
        "a.flac",
        44_100,
        "0.3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let b = fixture(
        dir.path(),
        "b.flac",
        96_000,
        "0.3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), true);
    h.play_to_end(vec![a, b]);
    let rates: Vec<u32> = h
        .backend
        .opened
        .lock()
        .iter()
        .map(|o: &Opened| o.format.sample_rate)
        .collect();
    assert_eq!(rates, vec![44_100, 96_000]);
}

#[test]
fn exclusive_failure_falls_back_to_shared_with_a_reason() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "a.flac",
        44_100,
        "0.3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let mut backend = CaptureBackend::new(CaptureBackend::typical_dac());
    backend.exclusive_error = Some(OutputError::DeviceInUse);
    let h = harness(backend, true);
    let events = h.play_to_end(vec![f]);

    let opened = h.backend.opened.lock().clone();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].mode, ShareMode::Shared);
    assert_eq!(
        opened[0].format.sample_rate, 48_000,
        "resampled to the mix format"
    );

    let reason = events.iter().find_map(|e| match e {
        EngineEvent::State(s) => s.output.as_ref().and_then(|o| o.fallback_reason.clone()),
        _ => None,
    });
    assert!(reason.unwrap_or_default().contains("in use"));
    // 0.3 s at 48 kHz, stereo float.
    let frames = h.backend.take_captured().len() / 8;
    assert!((frames as i64 - 14_400).abs() <= 2, "frames = {frames}");
}

#[test]
fn software_gain_engages_on_devices_without_hardware_volume() {
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
    let h = harness(CaptureBackend::new(caps), true);
    let events = h.play_to_end(vec![f.clone()]);

    // The deepest container keeps the attenuated low bits.
    assert_eq!(h.backend.opened.lock()[0].format.sample, SampleFormat::S32);
    let badge = last_badge(&events).expect("badge");
    assert!(!badge.bit_perfect);
    assert_eq!(badge.label, "Exclusive • 32-bit/44.1 kHz • −30 dB");

    // Exactly the Q16 multiply of the 16-bit samples widened into 32 bits.
    let q16 = pp_core::SoftwareGain::new(-30.0).q16().unwrap();
    let captured = h.backend.take_captured();
    let expected: Vec<u8> = reference(&f, "s32le")
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|c| (((i64::from(i32::from_le_bytes(*c)) * q16) >> 16) as i32).to_le_bytes())
        .collect();
    assert!(captured == expected, "software gain output differs");
}

/// The capture device plays faster than real time, so with a long control tick the whole
/// track starts and drains between two ticks. Its state must still be published once.
#[test]
fn a_track_that_ends_within_one_tick_still_reports_its_output() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(
        dir.path(),
        "a.flac",
        44_100,
        "0.3",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let config = EngineConfig {
        preroll_ms: 0,
        tick: Duration::from_millis(500),
        ..EngineConfig::default()
    };
    let h = harness_with(
        CaptureBackend::new(CaptureBackend::typical_dac()),
        true,
        config,
    );
    let events = h.play_to_end(vec![f]);
    let badge = last_badge(&events).expect("the track's output state was never published");
    assert!(badge.bit_perfect);
}
