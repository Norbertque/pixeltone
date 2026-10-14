//! The remote control end to end: TLS, pairing, the WebSocket, and a phone's queue played
//! from the phone's own HTTP server, bit for bit.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use pp_engine::testing::CaptureBackend;
use pp_engine::{Engine, EngineConfig, OutputSettings, PlaybackStatus};
use pp_remote::{Host, RemoteConfig, RemoteServer, RemoteVolume};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

struct TestHost {
    engine: Engine,
    volume: Mutex<RemoteVolume>,
}

impl Host for TestHost {
    fn engine(&self) -> &Engine {
        &self.engine
    }
    fn volume(&self) -> Option<RemoteVolume> {
        Some(self.volume.lock().clone())
    }
    fn set_volume(&self, db: f32) -> Result<(), String> {
        self.volume.lock().db = db;
        Ok(())
    }
    fn set_muted(&self, muted: bool) -> Result<(), String> {
        self.volume.lock().muted = muted;
        Ok(())
    }
    fn library_art(&self, _hash: &str, _size: u32) -> Option<Vec<u8>> {
        None
    }
}

struct Rig {
    server: Arc<RemoteServer>,
    host: Arc<TestHost>,
    backend: Arc<CaptureBackend>,
    _dir: tempfile::TempDir,
}

fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(CaptureBackend::new(CaptureBackend::typical_dac()));
    let (tx, rx) = mpsc::channel();
    let engine = Engine::with_config(
        backend.clone(),
        OutputSettings {
            exclusive: true,
            ..OutputSettings::default()
        },
        EngineConfig {
            preroll_ms: 0,
            ..EngineConfig::default()
        },
        move |e| {
            let _ = tx.send(e);
        },
    );
    let host = Arc::new(TestHost {
        engine,
        volume: Mutex::new(RemoteVolume {
            kind: "app".into(),
            db: 0.0,
            min: -60.0,
            max: 0.0,
            step: 0.5,
            muted: false,
        }),
    });
    let server = Arc::new(
        RemoteServer::start(
            RemoteConfig {
                name: "TEST-PC".into(),
                host_name: "test-pc".into(),
                id: "test-id".into(),
                version: "0.1.0".into(),
                data_dir: dir.path().to_path_buf(),
                port: 0,
            },
            host.clone(),
        )
        .unwrap(),
    );
    let publisher = server.clone();
    thread::spawn(move || {
        for event in rx {
            publisher.publish(&event);
        }
    });
    Rig {
        server,
        host,
        backend,
        _dir: dir,
    }
}

/// HTTPS without certificate checks: these tests check the fingerprint themselves.
fn https() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .disable_verification(true)
                .build(),
        )
        .http_status_as_error(false)
        .build()
        .into()
}

fn base(rig: &Rig) -> String {
    format!("https://127.0.0.1:{}", rig.server.port())
}

fn pair(rig: &Rig) -> String {
    let code = rig.server.pairing().new_code();
    let mut resp = https()
        .post(format!("{}/v1/pair", base(rig)))
        .send_json(serde_json::json!({"code": code, "deviceId": "phone-1", "deviceName": "Pixel"}))
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.body_mut().read_json().unwrap();
    body["token"].as_str().unwrap().to_string()
}

/// Accepts the server's certificate and remembers it, like a phone pinning it on pairing.
#[derive(Debug, Default)]
struct Pin(std::sync::Mutex<Option<Vec<u8>>>);

impl ServerCertVerifier for Pin {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        *self.0.lock().unwrap() = Some(end_entity.to_vec());
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>;

async fn connect(rig: &Rig, token: &str) -> (Socket, Vec<u8>) {
    let pin = Arc::new(Pin::default());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .dangerous()
    .with_custom_certificate_verifier(pin.clone())
    .with_no_client_auth();
    let tcp = tokio::net::TcpStream::connect(("127.0.0.1", rig.server.port()))
        .await
        .unwrap();
    let tls = tokio_rustls::TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from("localhost").unwrap(), tcp)
        .await
        .unwrap();
    let mut request = format!("wss://localhost:{}/v1/ws", rig.server.port())
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {token}").parse().unwrap());
    let (socket, _) = tokio_tungstenite::client_async(request, tls).await.unwrap();
    let cert = pin.0.lock().unwrap().clone().unwrap();
    (socket, cert)
}

/// The next message of `kind`, skipping others (positions, states).
async fn next_of(socket: &mut Socket, kind: &str) -> serde_json::Value {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let msg = tokio::time::timeout_at(deadline, socket.next())
            .await
            .unwrap_or_else(|_| panic!("no {kind} message"))
            .unwrap()
            .unwrap();
        if let Ws::Text(text) = msg {
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();
            if v["type"] == kind {
                return v;
            }
        }
    }
}

async fn send(socket: &mut Socket, command: serde_json::Value) {
    socket
        .send(Ws::Text(command.to_string().into()))
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn pairing_then_following_and_driving_the_player() {
    let rig = rig();
    let agent = https();

    let info: serde_json::Value = agent
        .get(format!("{}/v1/info", base(&rig)))
        .call()
        .unwrap()
        .body_mut()
        .read_json()
        .unwrap();
    assert_eq!(info["name"], "TEST-PC");
    assert_eq!(info["protocol"], 1);

    // Nothing without pairing.
    let resp = agent
        .get(format!("{}/v1/queue", base(&rig)))
        .call()
        .unwrap();
    assert_eq!(resp.status(), 401);
    let closed = agent
        .post(format!("{}/v1/pair", base(&rig)))
        .send_json(serde_json::json!({"code": "123456", "deviceId": "x", "deviceName": "X"}))
        .unwrap();
    assert_eq!(closed.status(), 410, "no code shown on the PC");
    let code = rig.server.pairing().new_code();
    let wrong = if code == "000000" { "000001" } else { "000000" };
    let resp = agent
        .post(format!("{}/v1/pair", base(&rig)))
        .send_json(serde_json::json!({"code": wrong, "deviceId": "x", "deviceName": "X"}))
        .unwrap();
    assert_eq!(resp.status(), 403);
    rig.server.pairing().close();

    let token = pair(&rig);
    let resp = agent
        .get(format!("{}/v1/queue", base(&rig)))
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .unwrap();
    assert_eq!(resp.status(), 200);

    let (mut socket, cert) = connect(&rig, &token).await;
    let fingerprint: String = Sha256::digest(&cert)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        info["fingerprint"], fingerprint,
        "the certificate a phone pins is the one served"
    );

    let hello = next_of(&mut socket, "hello").await;
    assert_eq!(hello["server"]["id"], "test-id");
    let state = next_of(&mut socket, "state").await;
    assert_eq!(state["state"]["status"], "stopped");
    let volume = next_of(&mut socket, "volume").await;
    assert_eq!(volume["volume"]["db"], 0.0);

    send(
        &mut socket,
        serde_json::json!({"cmd": "setVolume", "db": -12.5}),
    )
    .await;
    let volume = next_of(&mut socket, "volume").await;
    assert_eq!(volume["volume"]["db"], -12.5);
    assert_eq!(rig.host.volume.lock().db, -12.5);

    send(&mut socket, serde_json::json!({"cmd": "eject"})).await;
    let error = next_of(&mut socket, "error").await;
    assert!(error["message"].as_str().unwrap().contains("not a command"));
}

/// Serves one file over plain HTTP with Range support, like the phone's file server.
fn phone_server(file: PathBuf) -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = seen.clone();
    thread::spawn(move || {
        let data = std::fs::read(&file).unwrap();
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let data = data.clone();
            let log = log.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                if reader.read_line(&mut first).is_err() {
                    return;
                }
                let mut from = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) <= 2 {
                        break;
                    }
                    let lower = line.to_ascii_lowercase();
                    if let Some(range) = lower.strip_prefix("range: bytes=") {
                        from = range.split('-').next().unwrap().trim().parse().unwrap_or(0);
                    }
                }
                log.lock().unwrap().push(first.trim().to_string());
                let from = from.min(data.len());
                let body = &data[from..];
                let head = if from > 0 {
                    format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {from}-{}/{}\r\n",
                        data.len() - 1,
                        data.len()
                    )
                } else {
                    "HTTP/1.1 200 OK\r\n".to_string()
                };
                let _ = write!(
                    stream,
                    "{head}Content-Type: audio/flac\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(body);
                let _ = stream.read(&mut [0u8; 1]);
            });
        }
    });
    (base, seen)
}

fn ffmpeg(args: &[&str]) -> Vec<u8> {
    let bin = std::env::var("PP_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let out = Command::new(bin)
        .args(["-hide_banner", "-v", "error", "-y"])
        .args(args)
        .output()
        .expect("run ffmpeg");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn flac(dir: &Path) -> PathBuf {
    let path = dir.join("song.flac");
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "sine=f=440:r=44100:d=0.6",
        "-ac",
        "2",
        "-c:a",
        "flac",
        "-sample_fmt",
        "s16",
        path.to_str().unwrap(),
    ]);
    path
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_hands_its_queue_over_and_it_plays_bit_perfect() {
    let rig = rig();
    let dir = tempfile::tempdir().unwrap();
    let file = flac(dir.path());
    let (phone, requests) = phone_server(file.clone());
    let token = pair(&rig);
    let (mut socket, _) = connect(&rig, &token).await;

    // Only the phone's own files: another address is refused.
    send(
        &mut socket,
        serde_json::json!({"cmd": "load", "items": [{"url": "http://127.0.0.2:9/x.flac"}]}),
    )
    .await;
    let error = next_of(&mut socket, "error").await;
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("must come from the phone")
    );

    send(
        &mut socket,
        serde_json::json!({
            "cmd": "load",
            "items": [{"url": format!("{phone}/file/1?auth=secret"), "title": "From the phone"}],
            "start": 0,
            "positionMs": 0
        }),
    )
    .await;

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut seen_playing = false;
    loop {
        let s = rig.host.engine.snapshot();
        if s.status == PlaybackStatus::Playing {
            seen_playing = true;
            assert_eq!(s.track.as_ref().unwrap().title, "From the phone");
        }
        if seen_playing && s.status == PlaybackStatus::Stopped {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "playback didn't finish: {:?}",
            s.status
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let reference = ffmpeg(&["-i", file.to_str().unwrap(), "-f", "s16le", "-"]);
    let captured = rig.backend.take_captured();
    assert_eq!(rig.backend.opened.lock()[0].format.sample_rate, 44_100);
    assert!(
        captured == reference,
        "the phone's file reaches the DAC untouched"
    );
    assert!(
        requests
            .lock()
            .unwrap()
            .iter()
            .all(|r| r.starts_with("GET /file/1?auth=secret")),
        "{:?}",
        requests.lock().unwrap()
    );
}
