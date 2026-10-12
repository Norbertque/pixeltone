//! Fixtures shared by the engine's integration tests: FFmpeg-generated test files, their
//! reference decodes, and an engine wired to a capture device.
#![allow(dead_code, unused_imports)] // each test binary uses a different subset

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use pp_core::{DeviceChange, OutputError, SampleFormat, ShareMode};
pub use pp_engine::testing::{CaptureBackend, Opened};
pub use pp_engine::{
    Engine, EngineConfig, EngineEvent, OutputSettings, PlaybackStatus, PlayerSnapshot, QueueItem,
    QueueSnapshot, RepeatMode,
};

pub fn ffmpeg(args: &[&str]) {
    let bin = std::env::var("PP_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let out = Command::new(bin)
        .args(["-hide_banner", "-v", "error", "-y"])
        .args(args)
        .output()
        .expect("run ffmpeg");
    assert!(
        out.status.success(),
        "ffmpeg {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Pink noise left, a tone right, so channel order mistakes show up.
pub fn fixture(dir: &Path, name: &str, rate: u32, seconds: &str, codec: &[&str]) -> PathBuf {
    let path = dir.join(name);
    let noise = format!("anoisesrc=d={seconds}:c=pink:r={rate}:a=0.3:seed=11");
    let tone = format!("sine=f=1234:r={rate}:d={seconds}");
    let mut args = vec![
        "-f",
        "lavfi",
        "-i",
        &noise,
        "-f",
        "lavfi",
        "-i",
        &tone,
        "-filter_complex",
        "[0][1]amerge=inputs=2",
    ];
    args.extend(codec);
    let p = path.to_str().unwrap().to_string();
    args.push(&p);
    ffmpeg(&args);
    path
}

/// FFmpeg's decode of `path` in raw format `fmt` (s16le, s32le, …).
pub fn reference(path: &Path, fmt: &str) -> Vec<u8> {
    let out = path.with_extension(format!("{fmt}.ref"));
    ffmpeg(&[
        "-i",
        path.to_str().unwrap(),
        "-f",
        fmt,
        out.to_str().unwrap(),
    ]);
    std::fs::read(out).unwrap()
}

pub struct Harness {
    pub engine: Engine,
    pub backend: Arc<CaptureBackend>,
    pub events: crossbeam_channel::Receiver<EngineEvent>,
}

pub fn harness(backend: CaptureBackend, exclusive: bool) -> Harness {
    let config = EngineConfig {
        preroll_ms: 0,
        ..EngineConfig::default()
    };
    harness_with(backend, exclusive, config)
}

pub fn harness_with(backend: CaptureBackend, exclusive: bool, config: EngineConfig) -> Harness {
    let settings = OutputSettings {
        exclusive,
        ..OutputSettings::default()
    };
    harness_full(backend, settings, config)
}

pub fn harness_full(
    backend: CaptureBackend,
    settings: OutputSettings,
    config: EngineConfig,
) -> Harness {
    let backend = Arc::new(backend);
    let (tx, rx) = crossbeam_channel::unbounded();
    let engine = Engine::with_config(backend.clone(), settings, config, move |e| {
        let _ = tx.send(e);
    });
    Harness {
        engine,
        backend,
        events: rx,
    }
}

impl Harness {
    /// Waits until playback has started and then stopped again; returns all events.
    pub fn play_to_end(&self, files: Vec<PathBuf>) -> Vec<EngineEvent> {
        self.engine.play_files(files, 0);
        self.wait_until_stopped()
    }

    /// Collects events until playback has been seen playing and then stopped.
    pub fn wait_until_stopped(&self) -> Vec<EngineEvent> {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut seen_playing = false;
        let mut events = Vec::new();
        while Instant::now() < deadline {
            let Ok(e) = self.events.recv_timeout(Duration::from_millis(100)) else {
                continue;
            };
            if let EngineEvent::State(s) = &e {
                if s.status == PlaybackStatus::Playing {
                    seen_playing = true;
                } else if seen_playing && s.status == PlaybackStatus::Stopped {
                    events.push(e);
                    return events;
                }
            }
            events.push(e);
        }
        panic!("playback did not finish; events: {events:?}");
    }

    /// Polls the engine until `cond` holds (or panics after a while).
    pub fn wait_for(&self, what: &str, cond: impl Fn(&PlayerSnapshot) -> bool) -> PlayerSnapshot {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let s = self.engine.snapshot();
            if cond(&s) {
                return s;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: {s:?}"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Id of the queued item playing `path`.
    pub fn queue_id(&self, path: &Path) -> u32 {
        self.engine
            .queue_entries(0, usize::MAX)
            .into_iter()
            .find(|e| e.path == path.display().to_string())
            .map(|e| e.id)
            .expect("queued")
    }

    pub fn queue_paths(&self) -> Vec<String> {
        self.engine
            .queue_entries(0, usize::MAX)
            .into_iter()
            .map(|e| e.path)
            .collect()
    }
}

pub fn last_badge(events: &[EngineEvent]) -> Option<pp_core::OutputBadge> {
    events.iter().rev().find_map(|e| match e {
        EngineEvent::State(s) => s.output.as_ref().map(|o| o.badge.clone()),
        _ => None,
    })
}

/// The tracks the engine reported, in order, without repeats.
pub fn track_sequence(events: &[EngineEvent]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for e in events {
        if let EngineEvent::State(s) = e
            && let Some(t) = &s.track
            && paths.last() != Some(&t.path)
        {
            paths.push(t.path.clone());
        }
    }
    paths
}

pub fn error_codes(events: &[EngineEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            EngineEvent::Error { code, .. } => Some(code.clone()),
            _ => None,
        })
        .collect()
}
