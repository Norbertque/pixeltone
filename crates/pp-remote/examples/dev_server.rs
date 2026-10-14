//! A remote control server for trying phone clients without the Windows app. It plays into a
//! simulated device in real time and prints what happens:
//!
//! ```text
//! cargo run -p pp-remote --example dev_server [port]
//! ```
//!
//! Prints the port, the certificate fingerprint and a pairing code (a new one after each
//! pairing). Covers handed over by a phone are fetched like in the app.

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use parking_lot::Mutex;
use pp_engine::testing::CaptureBackend;
use pp_engine::{Engine, EngineConfig, EngineEvent, OutputSettings};
use pp_remote::{Host, RemoteConfig, RemoteServer, RemoteVolume};

struct DevHost {
    engine: Engine,
    volume: Mutex<RemoteVolume>,
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

    loop {
        if server.pairing().current_code().is_none() {
            println!("code {}", server.pairing().new_code());
        }
        thread::sleep(Duration::from_millis(200));
    }
}
