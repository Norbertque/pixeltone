//! Play history: turns what the engine reports into finished listens. A listen counts once
//! the user has heard half the track or four minutes, and at least 30 seconds (the usual
//! scrobbling rule); time paused or skipped over doesn't count.

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use pp_engine::{PlaybackStatus, PlayerSnapshot};
use pp_library::{Library, PlayRecord};

const MIN_LISTEN: Duration = Duration::from_secs(30);
const ENOUGH: Duration = Duration::from_secs(240);

struct Listen {
    path: String,
    title: String,
    artist: String,
    album: String,
    duration: Option<Duration>,
    started_at: i64,
    heard: Duration,
    playing_since: Option<Instant>,
    last_position_ms: f64,
}

impl Listen {
    fn total(&self) -> Duration {
        self.heard + self.playing_since.map_or(Duration::ZERO, |t| t.elapsed())
    }

    fn counts(&self) -> bool {
        let total = self.total();
        let needed = self.duration.map_or(ENOUGH, |d| (d / 2).min(ENOUGH));
        total >= MIN_LISTEN && total >= needed
    }

    fn record(&self) -> PlayRecord {
        PlayRecord {
            path: self.path.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            played_at: self.started_at,
            ms_played: self.total().as_millis().min(u128::from(u32::MAX)) as u32,
        }
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub struct History {
    library: Arc<Library>,
    current: Mutex<Option<Listen>>,
}

impl History {
    pub fn new(library: Arc<Library>) -> Self {
        Self {
            library,
            current: Mutex::new(None),
        }
    }

    /// Records a finished listen without blocking the caller (the engine thread).
    fn finish(&self, listen: Option<Listen>) {
        let Some(listen) = listen.filter(Listen::counts) else {
            return;
        };
        let record = listen.record();
        let library = self.library.clone();
        std::thread::spawn(move || {
            if let Err(e) = library.record_play(&record) {
                tracing::warn!("can't record a play: {e}");
            }
        });
    }

    fn start(s: &PlayerSnapshot) -> Option<Listen> {
        let t = s.track.as_ref()?;
        // A phone's track (remote control) isn't part of this library's history.
        if t.path.starts_with("http://") {
            return None;
        }
        Some(Listen {
            path: t.path.clone(),
            title: t.title.clone(),
            artist: t.artist.clone().unwrap_or_default(),
            album: t.album.clone().unwrap_or_default(),
            duration: t.duration_ms.map(|ms| Duration::from_millis(ms as u64)),
            started_at: unix_now(),
            heard: Duration::ZERO,
            playing_since: None,
            last_position_ms: s.position_ms,
        })
    }

    pub fn on_state(&self, s: &PlayerSnapshot) {
        let mut current = self.current.lock();
        let path = s.track.as_ref().map(|t| t.path.as_str());
        if current.as_ref().map(|l| l.path.as_str()) != path {
            self.finish(current.take());
            *current = Self::start(s);
        }
        if let Some(l) = current.as_mut() {
            let playing = s.status == PlaybackStatus::Playing;
            match (playing, l.playing_since) {
                (true, None) => l.playing_since = Some(Instant::now()),
                (false, Some(since)) => {
                    l.heard += since.elapsed();
                    l.playing_since = None;
                }
                _ => {}
            }
            l.last_position_ms = s.position_ms;
        }
    }

    /// Position updates: a jump back to the start (repeat-one, "previous") begins a new listen.
    pub fn on_position(&self, position_ms: f64) {
        let mut current = self.current.lock();
        let Some(l) = current.as_mut() else { return };
        let restarted = position_ms < 1500.0 && l.last_position_ms > 10_000.0;
        l.last_position_ms = position_ms;
        if restarted && l.counts() {
            let playing = l.playing_since.is_some();
            let mut fresh = Listen {
                path: l.path.clone(),
                title: l.title.clone(),
                artist: l.artist.clone(),
                album: l.album.clone(),
                duration: l.duration,
                started_at: unix_now(),
                heard: Duration::ZERO,
                playing_since: playing.then(Instant::now),
                last_position_ms: position_ms,
            };
            std::mem::swap(l, &mut fresh);
            self.finish(Some(fresh));
        }
    }

    /// The app is closing: keep a listen that already counts.
    pub fn flush(&self) {
        let listen = self.current.lock().take();
        if let Some(l) = listen.filter(Listen::counts)
            && let Err(e) = self.library.record_play(&l.record())
        {
            tracing::warn!("can't record a play: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listen(duration_s: Option<u64>, heard_s: u64) -> Listen {
        Listen {
            path: "a".into(),
            title: "t".into(),
            artist: String::new(),
            album: String::new(),
            duration: duration_s.map(Duration::from_secs),
            started_at: 0,
            heard: Duration::from_secs(heard_s),
            playing_since: None,
            last_position_ms: 0.0,
        }
    }

    #[test]
    fn half_the_track_or_four_minutes_and_at_least_thirty_seconds() {
        assert!(!listen(Some(200), 99).counts());
        assert!(listen(Some(200), 100).counts());
        assert!(
            listen(Some(3600), 240).counts(),
            "four minutes is enough for long tracks"
        );
        assert!(!listen(Some(40), 25).counts(), "under 30 s never counts");
        assert!(listen(None, 240).counts());
        assert!(!listen(None, 200).counts());
    }
}
