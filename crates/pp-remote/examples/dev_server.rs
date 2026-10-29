//! A remote control server for trying phone clients without the Windows app. It plays into a
//! simulated device in real time and prints what happens:
//!
//! ```text
//! cargo run -p pp-remote --example dev_server [port]
//! ```
//!
//! Prints the port, the certificate fingerprint and a pairing code (a new one after each
//! pairing). Covers handed over by a phone are fetched like in the app. Its library is a few
//! test tones. Commands on its input: `jam` (starts one and prints its link), `end`,
//! `allow <request id>`, `decline <request id>`, `remove <guest id>`.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use parking_lot::Mutex;
use pp_engine::testing::CaptureBackend;
use pp_engine::{Engine, EngineConfig, EngineEvent, OutputSettings, QueueItem};
use pp_remote::protocol::{
    JamView, LibraryAlbum, LibraryAlbumDetail, LibraryResults, LibraryTrack,
};
use pp_remote::{Host, RemoteConfig, RemoteServer, RemoteVolume};

struct DevHost {
    engine: Engine,
    volume: Mutex<RemoteVolume>,
    library: Vec<Tone>,
}

/// A library track: a few seconds of a sine tone.
struct Tone {
    id: u32,
    title: String,
    album_id: u32,
    album: &'static str,
    path: PathBuf,
}

const ARTIST: &str = "Test Tones";
const ALBUMS: [(u32, &str); 2] = [(1, "Low Notes"), (2, "High Notes")];
const TONE_SECONDS: u32 = 20;

/// A 16-bit stereo WAV of a sine at `hz`.
fn write_tone(path: &Path, hz: f64) {
    let rate = 44_100u32;
    let frames = rate * TONE_SECONDS;
    let mut data = Vec::with_capacity(frames as usize * 4 + 44);
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(36 + frames * 4).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&rate.to_le_bytes());
    data.extend_from_slice(&(rate * 4).to_le_bytes());
    data.extend_from_slice(&4u16.to_le_bytes());
    data.extend_from_slice(&16u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&(frames * 4).to_le_bytes());
    for i in 0..frames {
        let t = f64::from(i) / f64::from(rate);
        let sample = ((t * hz * std::f64::consts::TAU).sin() * 8_000.0) as i16;
        data.extend_from_slice(&sample.to_le_bytes());
        data.extend_from_slice(&sample.to_le_bytes());
    }
    std::fs::write(path, data).expect("write a test tone");
}

fn tones(dir: &Path) -> Vec<Tone> {
    std::fs::create_dir_all(dir).expect("make the library folder");
    let notes = [
        (1, "A2", 110.0),
        (1, "E3", 164.81),
        (1, "A3", 220.0),
        (2, "A4", 440.0),
        (2, "E5", 659.25),
        (2, "A5", 880.0),
    ];
    notes
        .iter()
        .enumerate()
        .map(|(i, &(album_id, note, hz))| {
            let path = dir.join(format!("{note}.wav"));
            if !path.exists() {
                write_tone(&path, hz);
            }
            Tone {
                id: i as u32 + 1,
                title: format!("{note} ({hz} Hz)"),
                album_id,
                album: ALBUMS[album_id as usize - 1].1,
                path,
            }
        })
        .collect()
}

impl DevHost {
    fn track(t: &Tone) -> LibraryTrack {
        LibraryTrack {
            id: t.id,
            title: t.title.clone(),
            artist: Some(ARTIST.into()),
            album: Some(t.album.into()),
            duration_ms: f64::from(TONE_SECONDS * 1000),
            cover_key: None,
        }
    }

    fn album(&self, id: u32) -> Option<LibraryAlbum> {
        let (_, title) = ALBUMS.iter().find(|(a, _)| *a == id)?;
        Some(LibraryAlbum {
            id,
            title: (*title).into(),
            artist: Some(ARTIST.into()),
            year: Some(2026),
            track_count: self.library.iter().filter(|t| t.album_id == id).count() as u32,
            cover_key: None,
        })
    }
}

impl Host for DevHost {
    fn engine(&self) -> &Engine {
        &self.engine
    }

    fn volume(&self) -> Option<RemoteVolume> {
        Some(self.volume.lock().clone())
    }

    fn set_volume(&self, db: f32) -> Result<(), String> {
        let mut volume = self.volume.lock();
        volume.db = db.clamp(volume.min, volume.max);
        volume.muted = false;
        println!("volume {db} dB");
        Ok(())
    }

    fn set_muted(&self, muted: bool) -> Result<(), String> {
        self.volume.lock().muted = muted;
        println!("muted {muted}");
        Ok(())
    }

    fn library_art(&self, _hash: &str, _size: u32) -> Option<Vec<u8>> {
        None
    }

    fn library_items(&self, ids: &[u32]) -> Vec<QueueItem> {
        ids.iter()
            .filter_map(|id| self.library.iter().find(|t| t.id == *id))
            .map(|t| QueueItem {
                title: Some(t.title.clone()),
                artist: Some(ARTIST.into()),
                album: Some(t.album.into()),
                duration_ms: Some(f64::from(TONE_SECONDS * 1000)),
                track_id: Some(t.id),
                ..QueueItem::from(t.path.clone())
            })
            .collect()
    }

    fn library_search(&self, query: &str, limit: u32) -> LibraryResults {
        let query = query.to_lowercase();
        let hit = |text: &str| text.to_lowercase().contains(&query);
        LibraryResults {
            tracks: self
                .library
                .iter()
                .filter(|t| hit(&t.title) || hit(t.album) || hit(ARTIST))
                .take(limit as usize)
                .map(Self::track)
                .collect(),
            albums: ALBUMS
                .iter()
                .filter(|(_, title)| hit(title) || hit(ARTIST))
                .filter_map(|(id, _)| self.album(*id))
                .collect(),
        }
    }

    fn library_albums(&self, offset: u32, limit: u32) -> Vec<LibraryAlbum> {
        ALBUMS
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .filter_map(|(id, _)| self.album(*id))
            .collect()
    }

    fn library_album(&self, id: u32) -> Option<LibraryAlbumDetail> {
        Some(LibraryAlbumDetail {
            album: self.album(id)?,
            tracks: self
                .library
                .iter()
                .filter(|t| t.album_id == id)
                .map(Self::track)
                .collect(),
        })
    }

    fn jam_changed(&self, jam: Option<&JamView>) {
        match jam {
            None => println!("jam none"),
            Some(jam) => {
                let guests: Vec<String> = jam
                    .info
                    .guests
                    .iter()
                    .map(|g| format!("{} ({})", g.name, g.id))
                    .collect();
                println!("jam guests [{}]", guests.join(", "));
                for r in &jam.info.requests {
                    println!("jam request {} from {}", r.id, r.name);
                }
            }
        }
    }
}

/// The owner at the PC, typing.
fn console(server: &RemoteServer) {
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { return };
        let mut words = line.split_whitespace();
        match (words.next(), words.next()) {
            (Some("jam"), _) => println!("jam link {}", server.jam_start().link),
            (Some("end"), _) => server.jam_end(),
            (Some("allow"), Some(id)) => println!("answered {}", server.jam_answer(id, true)),
            (Some("decline"), Some(id)) => println!("answered {}", server.jam_answer(id, false)),
            (Some("remove"), Some(id)) => server.jam_remove_guest(id),
            (None, _) => {}
            _ => println!("commands: jam, end, allow <id>, decline <id>, remove <id>"),
        }
    }
}

fn main() {
    let port = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(47_821);
    let backend = Arc::new(CaptureBackend::new(CaptureBackend::typical_dac()).paced(1.0));
    let (tx, rx) = mpsc::channel();
    let engine = Engine::with_config(
        backend,
        OutputSettings {
            exclusive: true,
            ..OutputSettings::default()
        },
        EngineConfig::default(),
        move |e| {
            let _ = tx.send(e);
        },
    );
    let host = Arc::new(DevHost {
        engine,
        volume: Mutex::new(RemoteVolume {
            kind: "app".into(),
            db: -20.0,
            min: -60.0,
            max: 0.0,
            step: 0.5,
            muted: false,
        }),
        library: tones(&std::env::temp_dir().join("pixeltone-dev-server-library")),
    });
    let server = Arc::new(
        RemoteServer::start(
            RemoteConfig {
                name: "DEV-PC".into(),
                host_name: "dev-pc".into(),
                id: "dev-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                data_dir: std::env::temp_dir().join("pixeltone-dev-server"),
                port,
                jam_tracks_dir: std::env::temp_dir().join("pixeltone-dev-server-jam"),
            },
            host,
        )
        .expect("the server starts"),
    );
    println!("port {}", server.port());
    println!("fingerprint {}", server.info().fingerprint);

    let publisher = server.clone();
    thread::spawn(move || {
        for event in rx {
            match &event {
                EngineEvent::State(s) => println!(
                    "state {:?} {:?} {}",
                    s.status,
                    s.track.as_ref().map(|t| &t.title),
                    s.track.as_ref().map_or("", |t| t.path.as_str())
                ),
                EngineEvent::Error { message, .. } => println!("error {message}"),
                EngineEvent::Position { .. } => {}
            }
            publisher.publish(&event);
        }
    });

    let typing = server.clone();
    thread::spawn(move || console(&typing));

    loop {
        if server.pairing().current_code().is_none() {
            println!("code {}", server.pairing().new_code());
        }
        thread::sleep(Duration::from_millis(200));
    }
}
