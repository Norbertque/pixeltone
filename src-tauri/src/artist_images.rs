//! Artist pictures from Deezer, fetched in the background for artists that have none yet: a
//! few lookups a second (Deezer allows 50 per 5 s), the artists with the most tracks first.
//! Misses are asked again after 30 days; while offline, the next scan or a later round
//! picks up where this one stopped.

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use pp_deezer::{Clues, Deezer, DeezerError, worth_looking_up};
use pp_library::Library;

const PAUSE: Duration = Duration::from_millis(250);
const QUOTA_PAUSE: Duration = Duration::from_secs(10);
/// Another round this long after the last, for artists added while offline.
const IDLE: Duration = Duration::from_secs(6 * 3600);
const RETRY_MISSES_AFTER_S: i64 = 30 * 86_400;
const BATCH: u32 = 100;
/// How often the UI hears about new pictures while a round runs.
const NOTIFY_EVERY: Duration = Duration::from_secs(4);

enum Msg {
    Kick,
    Enable(bool),
}

pub struct ArtistImages {
    tx: Sender<Msg>,
}

impl ArtistImages {
    /// Starts the background worker; `on_change` runs when pictures appeared or went away.
    pub fn start(
        library: Arc<Library>,
        enabled: bool,
        on_change: impl Fn() + Send + 'static,
    ) -> Self {
        let (tx, rx) = unbounded();
        thread::Builder::new()
            .name("artist-images".into())
            .spawn(move || run(&library, enabled, &rx, &on_change))
            .expect("spawn the artist picture worker");
        Self { tx }
    }

    /// Looks for pictures of artists that have none (after a scan).
    pub fn kick(&self) {
        let _ = self.tx.send(Msg::Kick);
    }

    /// Off also removes the pictures found so far.
    pub fn set_enabled(&self, on: bool) {
        let _ = self.tx.send(Msg::Enable(on));
    }
}

struct Worker<'a> {
    library: &'a Library,
    deezer: Deezer,
    rx: &'a Receiver<Msg>,
    on_change: &'a dyn Fn(),
    enabled: bool,
    due: bool,
}

fn run(library: &Library, enabled: bool, rx: &Receiver<Msg>, on_change: &dyn Fn()) {
    let mut w = Worker {
        library,
        deezer: Deezer::new(pp_deezer::API),
        rx,
        on_change,
        enabled,
        due: true,
    };
    loop {
        if w.due && w.enabled {
            w.due = false;
            w.round();
        }
        match rx.recv_timeout(IDLE) {
            Ok(msg) => w.handle(msg),
            Err(RecvTimeoutError::Timeout) => w.due = true,
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

impl Worker<'_> {
    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Kick => self.due = true,
            Msg::Enable(on) if on == self.enabled => {}
            Msg::Enable(true) => {
                self.enabled = true;
                self.due = true;
            }
            Msg::Enable(false) => {
                self.enabled = false;
                match self.library.forget_artist_images() {
                    Ok(()) => (self.on_change)(),
                    Err(e) => tracing::warn!("can't remove artist pictures: {e}"),
                }
            }
        }
    }

    /// Looks artists up until none are left, the network fails, or the user turns it off.
    fn round(&mut self) {
        let mut fresh = 0;
        let mut told = Instant::now();
        loop {
            let retry_before = pp_library::scan::now_secs() - RETRY_MISSES_AFTER_S;
            let todo = match self.library.artists_without_images(retry_before, BATCH) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("artist pictures: {e}");
                    break;
                }
            };
            if todo.is_empty() {
                break;
            }
            let mut progress = false;
            for (name, key) in todo {
                // Settings changes take effect at once, not after the round.
                while let Ok(msg) = self.rx.try_recv() {
                    self.handle(msg);
                }
                if !self.enabled {
                    return;
                }
                let found = if worth_looking_up(&name) {
                    match self.lookup(&name, &key) {
                        Ok(found) => found,
                        Err(DeezerError::Quota) => {
                            thread::sleep(QUOTA_PAUSE);
                            progress = true; // asked again in the next batch
                            continue;
                        }
                        Err(e) => {
                            tracing::info!("artist pictures paused: {e}");
                            if fresh > 0 {
                                (self.on_change)();
                            }
                            return;
                        }
                    }
                } else {
                    None
                };
                let saved = self
                    .library
                    .set_artist_image(&key, found.as_deref())
                    // A picture that won't decode counts as none (asked again in 30 days).
                    .or_else(|_| self.library.set_artist_image(&key, None));
                match saved {
                    Ok(()) => {
                        progress = true;
                        fresh += usize::from(found.is_some());
                    }
                    Err(e) => tracing::warn!("artist picture for {name}: {e}"),
                }
                if fresh > 0 && told.elapsed() >= NOTIFY_EVERY {
                    (self.on_change)();
                    fresh = 0;
                    told = Instant::now();
                }
                thread::sleep(PAUSE);
            }
            if !progress {
                break; // the database refuses every write: don't spin
            }
        }
        if fresh > 0 {
            (self.on_change)();
        }
    }

    /// The picture of the artist `name` (library key `key`), checked against their titles.
    fn lookup(&self, name: &str, key: &str) -> Result<Option<Vec<u8>>, DeezerError> {
        let (albums, tracks) = self.library.artist_titles(key, 3).unwrap_or_default();
        match self.deezer.find(name, &Clues { albums, tracks })? {
            Some(picture) => self.deezer.download(&picture.url).map(Some),
            None => Ok(None),
        }
    }
}
