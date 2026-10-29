//! Transport, queue and device behaviour end to end: fades around pause and seek (and
//! nowhere else), queue edits while playing, shuffle, restoring a saved queue, and devices
//! coming and going. The capture device runs in real time here so tests can act mid-track.

mod common;

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use common::*;

const RATE: u32 = 44_100;
/// Pause/resume and seek fade lengths in frames at [`RATE`] (engine: 80 and 25 ms).
const PAUSE_FADE: usize = 3528;
const SEEK_FADE: usize = 1102;

fn s16(dir: &Path, name: &str, seconds: &str) -> PathBuf {
    fixture(
        dir,
        name,
        RATE,
        seconds,
        &["-c:a", "flac", "-sample_fmt", "s16"],
    )
}

/// Stereo 16-bit frames.
fn frames(bytes: &[u8]) -> Vec<[i16; 2]> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| {
            [
                i16::from_le_bytes([c[0], c[1]]),
                i16::from_le_bytes([c[2], c[3]]),
            ]
        })
        .collect()
}

fn ms_to_frame(ms: f64) -> usize {
    (ms * f64::from(RATE) / 1000.0) as usize
}

/// First and last index where the two differ.
fn diff_span(a: &[[i16; 2]], b: &[[i16; 2]]) -> Option<(usize, usize)> {
    let first = a.iter().zip(b).position(|(x, y)| x != y)?;
    let last = a.iter().zip(b).rposition(|(x, y)| x != y)?;
    Some((first, last))
}

/// A fade only ever makes samples quieter.
fn assert_never_louder(captured: &[[i16; 2]], reference: &[[i16; 2]]) {
    for (i, (c, r)) in captured.iter().zip(reference).enumerate() {
        for ch in 0..2 {
            assert!(
                c[ch].unsigned_abs() <= r[ch].unsigned_abs() + 1,
                "frame {i} ch {ch}: {} louder than {}",
                c[ch],
                r[ch]
            );
        }
    }
}

fn realtime() -> CaptureBackend {
    CaptureBackend::new(CaptureBackend::typical_dac()).paced(1.0)
}

#[test]
fn pause_and_resume_fade_only_around_the_pause() {
    let dir = tempfile::tempdir().unwrap();
    let f = s16(dir.path(), "a.flac", "2");
    let h = harness(realtime(), true);
    h.engine.play_files(vec![f.clone()], 0);
    h.wait_for("half a second played", |s| {
        s.status == PlaybackStatus::Playing && s.position_ms >= 500.0
    });
    h.engine.pause();
    let paused = h.wait_for("paused", |s| s.status == PlaybackStatus::Paused);
    thread::sleep(Duration::from_millis(150));
    assert_eq!(
        h.engine.snapshot().position_ms,
        paused.position_ms,
        "position holds"
    );
    h.engine.play();
    h.wait_until_stopped();

    let captured = frames(&h.backend.take_captured());
    let reference = frames(&reference(&f, "s16le"));
    assert_eq!(
        captured.len(),
        reference.len(),
        "every frame played exactly once"
    );
    let (first, last) = diff_span(&captured, &reference).expect("the pause faded");
    assert!(
        last - first < 2 * PAUSE_FADE,
        "only the two ramps differ: {first}..={last}"
    );
    let pause_frame = ms_to_frame(paused.position_ms);
    assert!(
        first < pause_frame && pause_frame <= last + 1,
        "the ramps surround the pause at {pause_frame}: {first}..={last}"
    );
    assert_never_louder(&captured[first..=last], &reference[first..=last]);
    assert_eq!(captured[pause_frame - 1], [0, 0], "faded all the way out");
}

#[test]
fn without_fades_a_pause_leaves_every_sample_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let f = s16(dir.path(), "a.flac", "1.5");
    let settings = OutputSettings {
        exclusive: true,
        fades: false,
        ..OutputSettings::default()
    };
    let config = EngineConfig {
        preroll_ms: 0,
        ..EngineConfig::default()
    };
    let h = harness_full(realtime(), settings, config);
    h.engine.play_files(vec![f.clone()], 0);
    h.wait_for("playing", |s| s.position_ms >= 400.0);
    h.engine.pause();
    h.wait_for("paused", |s| s.status == PlaybackStatus::Paused);
    h.engine.play();
    h.wait_until_stopped();
    assert!(h.backend.take_captured() == reference(&f, "s16le"));
}

#[test]
fn seeking_fades_out_and_lands_on_the_exact_frame() {
    let dir = tempfile::tempdir().unwrap();
    let f = s16(dir.path(), "a.flac", "3");
    let h = harness(realtime(), true);
    h.engine.play_files(vec![f.clone()], 0);
    h.wait_for("playing", |s| s.position_ms >= 300.0);
    h.engine.seek(2000.0);
    h.wait_until_stopped();

    let captured = frames(&h.backend.take_captured());
    let reference = frames(&reference(&f, "s16le"));
    let target = ms_to_frame(2000.0);
    let tail = reference.len() - target;
    assert!(captured.len() > tail + ms_to_frame(300.0));
    let (head, after) = captured.split_at(captured.len() - tail);

    // Before the seek: the reference from the start, ending in a fade-out.
    let clean = head.len() - SEEK_FADE;
    assert!(head[..clean] == reference[..clean]);
    assert_never_louder(&head[clean..], &reference[clean..head.len()]);
    assert_eq!(head[head.len() - 1], [0, 0]);
    // After: exactly the reference from the target frame, fading in.
    assert!(after[SEEK_FADE..] == reference[target + SEEK_FADE..]);
    assert_never_louder(&after[..SEEK_FADE], &reference[target..target + SEEK_FADE]);
}

#[test]
fn queue_edits_while_playing_change_what_comes_next() {
    let dir = tempfile::tempdir().unwrap();
    let [a, b, c, d, x] =
        ["a", "b", "c", "d", "x"].map(|n| s16(dir.path(), &format!("{n}.flac"), "1"));
    let h = harness(realtime(), true);
    h.engine
        .play_files(vec![a.clone(), b.clone(), c.clone(), d.clone()], 0);
    h.wait_for("a playing", |s| s.status == PlaybackStatus::Playing);

    // The worker has already opened b to stay gapless; it must notice the edits.
    h.engine.enqueue(vec![QueueItem::from(x.clone())], true);
    h.engine.remove_from_queue(vec![h.wait_id(&c)]);
    h.engine.move_in_queue(h.wait_id(&d), 1);
    let expected: Vec<String> = [&a, &d, &x, &b].map(|p| p.display().to_string()).into();
    let s = h.wait_for("edits applied", |_| h.queue_paths() == expected);
    assert_eq!(s.queue_index, Some(0));

    let events = h.wait_until_stopped();
    assert_eq!(track_sequence(&events), expected);
}

#[test]
fn withdrawing_the_playing_track_moves_on_to_the_next_that_stays() {
    let dir = tempfile::tempdir().unwrap();
    let [a, b, c] = ["a", "b", "c"].map(|n| s16(dir.path(), &format!("{n}.flac"), "1"));
    let h = harness(realtime(), true);
    h.engine
        .play_files(vec![a.clone(), b.clone(), c.clone()], 0);
    h.wait_for("a playing", |s| s.status == PlaybackStatus::Playing);

    h.engine
        .withdraw_from_queue(vec![h.wait_id(&a), h.wait_id(&b)]);
    let s = h.wait_for("c playing", |s| {
        s.status == PlaybackStatus::Playing
            && s.track.as_ref().is_some_and(|t| Path::new(&t.path) == c)
    });
    assert_eq!(s.queue_length, 1);

    // Nothing after it: playback stops and nothing is current.
    h.engine.withdraw_from_queue(vec![h.wait_id(&c)]);
    let s = h.wait_for("stopped", |s| s.status == PlaybackStatus::Stopped);
    assert!(s.current_id.is_none());
    assert!(h.queue_paths().is_empty());
}

#[test]
fn shuffle_keeps_the_current_track_and_unshuffle_restores_the_order() {
    let dir = tempfile::tempdir().unwrap();
    let files: Vec<PathBuf> = (0..12)
        .map(|i| s16(dir.path(), &format!("{i:02}.flac"), "1"))
        .collect();
    let original: Vec<String> = files.iter().map(|p| p.display().to_string()).collect();
    let h = harness(realtime(), true);
    let items = files.iter().cloned().map(QueueItem::from).collect();
    h.engine.play_items(items, Some(4), Some(true));
    let s = h.wait_for("shuffled", |s| s.shuffle && s.track.is_some());
    assert_eq!(s.track.unwrap().path, original[4]);
    let shuffled = h.queue_paths();
    assert_eq!(shuffled[0], original[4], "the chosen track leads");
    assert_ne!(shuffled, original);

    h.engine.set_shuffle(false);
    let s = h.wait_for("unshuffled", |s| !s.shuffle);
    assert_eq!(h.queue_paths(), original);
    assert_eq!(s.queue_index, Some(4), "still on the same track");
    h.engine.stop();
}

#[test]
fn a_restored_queue_waits_paused_and_resumes_at_its_position() {
    let dir = tempfile::tempdir().unwrap();
    let a = s16(dir.path(), "a.flac", "1.5");
    let b = s16(dir.path(), "b.flac", "0.5");
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), true);
    h.engine.restore(QueueSnapshot {
        items: vec![QueueItem::from(a.clone()), QueueItem::from(b.clone())],
        original: None,
        current: Some(0),
        position_ms: 700.0,
        repeat: RepeatMode::Off,
    });
    let s = h.wait_for("restored", |s| s.track.is_some());
    assert_eq!(s.status, PlaybackStatus::Paused);
    assert_eq!(s.position_ms, 700.0);
    assert_eq!(s.queue_length, 2);
    assert!(
        h.backend.opened.lock().is_empty(),
        "the device stays closed until play"
    );
    let saved = h.engine.queue_snapshot();
    assert_eq!(saved.current, Some(0));
    assert_eq!(saved.position_ms, 700.0);

    h.engine.play();
    h.wait_until_stopped();
    let captured = frames(&h.backend.take_captured());
    let mut expected = frames(&reference(&a, "s16le"));
    expected.drain(..ms_to_frame(700.0));
    expected.extend(frames(&reference(&b, "s16le")));
    assert_eq!(captured.len(), expected.len());
    assert!(captured[PAUSE_FADE..] == expected[PAUSE_FADE..]);
    assert_never_louder(&captured[..PAUSE_FADE], &expected[..PAUSE_FADE]);
}

#[test]
fn unplugging_pauses_and_play_resumes_where_it_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let f = s16(dir.path(), "a.flac", "2");
    let h = harness(realtime(), true);
    h.engine.play_files(vec![f.clone()], 0);
    h.wait_for("playing", |s| s.position_ms >= 500.0);
    h.backend.unplug();
    let paused = h.wait_for("paused by the unplug", |s| {
        s.status == PlaybackStatus::Paused
    });
    assert!(paused.track.is_some(), "the track stays");
    let before = h.backend.take_captured().len() / 4;

    h.backend.replug();
    h.engine.play();
    let events = h.wait_until_stopped();
    assert!(error_codes(&events).contains(&"deviceInvalidated".to_string()));
    assert_eq!(h.backend.opened.lock().len(), 2);

    let after = frames(&h.backend.take_captured());
    let reference = frames(&reference(&f, "s16le"));
    let resume = ms_to_frame(paused.position_ms.floor());
    assert!(
        resume.abs_diff(before) <= 50,
        "resumes where it stopped: {resume} vs {before}"
    );
    assert_eq!(after.len(), reference.len() - resume);
    assert!(after[PAUSE_FADE..] == reference[resume + PAUSE_FADE..]);
}

#[test]
fn a_new_default_device_takes_over_playback() {
    let dir = tempfile::tempdir().unwrap();
    let f = s16(dir.path(), "a.flac", "2");
    let h = harness(realtime(), true);
    h.engine.play_files(vec![f.clone()], 0);
    h.wait_for("playing", |s| s.position_ms >= 300.0);
    h.engine.device_changed(DeviceChange::DefaultChanged {
        id: Some("headphones".into()),
    });
    h.wait_until_stopped();
    assert_eq!(
        h.backend.opened.lock().len(),
        2,
        "reopened on the new default"
    );
    // Playback carried on to the end.
    let captured = frames(&h.backend.take_captured());
    let reference = frames(&reference(&f, "s16le"));
    let last_second = ms_to_frame(1000.0);
    assert!(captured[captured.len() - last_second..] == reference[reference.len() - last_second..]);
}

#[test]
fn losing_the_default_device_pauses_instead_of_moving_on() {
    let dir = tempfile::tempdir().unwrap();
    let f = s16(dir.path(), "a.flac", "2");
    let h = harness(realtime(), true);
    h.engine.play_files(vec![f], 0);
    h.wait_for("playing", |s| s.position_ms >= 300.0);
    // Headphones unplugged: Windows makes the speakers the default.
    h.backend.unplug();
    h.engine.device_changed(DeviceChange::DefaultChanged {
        id: Some("speakers".into()),
    });
    h.wait_for("paused", |s| s.status == PlaybackStatus::Paused);
    thread::sleep(Duration::from_millis(100));
    assert_eq!(h.backend.opened.lock().len(), 1, "nothing reopened");
}

#[test]
fn removing_the_device_in_use_pauses_with_a_message() {
    let dir = tempfile::tempdir().unwrap();
    let f = s16(dir.path(), "a.flac", "2");
    let h = harness(realtime(), true);
    h.engine.play_files(vec![f], 0);
    h.wait_for("playing", |s| s.position_ms >= 200.0);
    h.engine.device_changed(DeviceChange::Removed {
        id: "capture".into(),
    });
    h.wait_for("paused", |s| s.status == PlaybackStatus::Paused);
    let codes: Vec<String> = h
        .events
        .try_iter()
        .filter_map(|e| match e {
            EngineEvent::Error { code, .. } => Some(code),
            _ => None,
        })
        .collect();
    assert_eq!(codes, ["deviceInvalidated"]);
}

trait QueueIds {
    fn wait_id(&self, path: &Path) -> u32;
}

impl QueueIds for Harness {
    fn wait_id(&self, path: &Path) -> u32 {
        self.queue_id(path)
    }
}

#[test]
fn skipping_before_play_leaves_the_device_closed() {
    let dir = tempfile::tempdir().unwrap();
    let a = s16(dir.path(), "a.flac", "0.5");
    let b = s16(dir.path(), "b.flac", "0.5");
    let h = harness(CaptureBackend::new(CaptureBackend::typical_dac()), true);
    h.engine.restore(QueueSnapshot {
        items: vec![QueueItem::from(a.clone()), QueueItem::from(b.clone())],
        current: Some(0),
        position_ms: 200.0,
        ..QueueSnapshot::default()
    });
    h.wait_for("restored", |s| s.track.is_some());
    h.engine.next();
    let s = h.wait_for("moved to b", |s| {
        s.track
            .as_ref()
            .is_some_and(|t| t.path == b.display().to_string())
    });
    assert_eq!(s.status, PlaybackStatus::Paused);
    assert_eq!(s.position_ms, 0.0);
    assert_eq!(s.queue_index, Some(1));
    assert!(
        h.backend.opened.lock().is_empty(),
        "skipping must not open the device"
    );

    h.engine.play();
    h.wait_until_stopped();
    assert!(
        h.backend.take_captured() == reference(&b, "s16le"),
        "b from its start"
    );
}

#[test]
fn reload_reopens_the_current_track_where_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let f = s16(dir.path(), "a.flac", "2");
    let h = harness(realtime(), true);
    h.engine.play_files(vec![f.clone()], 0);
    h.wait_for("playing", |s| s.position_ms >= 300.0);
    h.engine.reload();
    h.wait_until_stopped();
    assert_eq!(h.backend.opened.lock().len(), 2, "reopened once");
    // It carried on from where it was: the end arrives as it was.
    let captured = frames(&h.backend.take_captured());
    let reference = frames(&reference(&f, "s16le"));
    let last_second = ms_to_frame(1000.0);
    assert!(captured[captured.len() - last_second..] == reference[reference.len() - last_second..]);
}
