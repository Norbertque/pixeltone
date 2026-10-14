//! The HTTPS + WebSocket server phones talk to.

use std::net::{IpAddr, Ipv6Addr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, FromRequestParts, Path, Query, Request, State};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use axum_server::Handle;
use axum_server::tls_rustls::RustlsConfig;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use pp_engine::{Engine, EngineEvent, QueueItem, QueueSnapshot};
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::art::{self, Art};
use crate::discovery::Advertisement;
use crate::lan;
use crate::pairing::{PairError, PairedDevice, Pairing};
use crate::protocol::{
    Command, LoadRequest, MAX_LOAD, Message, PROTOCOL, PairRequest, PairResponse, RemoteVolume,
    ServerInfo,
};
use crate::tls::Identity;

/// What the app lends the server.
pub trait Host: Send + Sync + 'static {
    fn engine(&self) -> &Engine;
    /// The volume control in charge, if any.
    fn volume(&self) -> Option<RemoteVolume>;
    fn set_volume(&self, db: f32) -> Result<(), String>;
    fn set_muted(&self, muted: bool) -> Result<(), String>;
    /// A library thumbnail (cover keys `lib/<hash>-<size>`), as JPEG bytes.
    fn library_art(&self, hash: &str, size: u32) -> Option<Vec<u8>>;
}

pub struct RemoteConfig {
    /// Shown on phones; usually the computer's name.
    pub name: String,
    /// The computer's network name (`<host>.local`).
    pub host_name: String,
    /// Stable per installation.
    pub id: String,
    pub version: String,
    /// Certificate and paired devices live here.
    pub data_dir: PathBuf,
    /// Preferred port (another is picked when it's taken); 0 for any.
    pub port: u16,
}

#[derive(Clone)]
struct Shared {
    host: Arc<dyn Host>,
    pairing: Arc<Pairing>,
    info: Arc<ServerInfo>,
    events: broadcast::Sender<Arc<str>>,
    art: Arc<Art>,
    last_volume: Arc<Mutex<Option<RemoteVolume>>>,
}

pub struct RemoteServer {
    shared: Shared,
    runtime: Option<tokio::runtime::Runtime>,
    handle: Handle<SocketAddr>,
    port: u16,
    _announcement: Option<Advertisement>,
}

/// Listens on every address, IPv4 and IPv6, on `port` (or any port when it's taken).
fn listen(port: u16) -> std::io::Result<TcpListener> {
    let bind = |port: u16| -> std::io::Result<TcpListener> {
        use socket2::{Domain, Socket, Type};
        let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;
        socket.set_only_v6(false)?;
        socket.bind(&SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port).into())?;
        socket.listen(64)?;
        Ok(socket.into())
    };
    let ipv4 = |port: u16| TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port));
    bind(port)
        .or_else(|_| ipv4(port))
        .or_else(|_| bind(0))
        .or_else(|_| ipv4(0))
}

impl RemoteServer {
    pub fn start(config: RemoteConfig, host: Arc<dyn Host>) -> Result<Self, String> {
        let identity = Identity::load_or_create(&config.data_dir, &config.name)?;
        let tls = RustlsConfig::from_config(identity.server_config()?);
        let listener = listen(config.port).map_err(|e| format!("can't listen: {e}"))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let info = ServerInfo {
            app: "Pixeltone",
            id: config.id.clone(),
            name: config.name.clone(),
            version: config.version.clone(),
            protocol: PROTOCOL,
            fingerprint: identity.fingerprint(),
        };
        let (events, _) = broadcast::channel(256);
        let shared = Shared {
            host,
            pairing: Arc::new(Pairing::open(config.data_dir.join("remote-devices.json"))),
            info: Arc::new(info),
            events,
            art: Arc::new(Art::new()),
            last_volume: Arc::new(Mutex::new(None)),
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("pp-remote")
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let handle = Handle::new();
        let server = {
            let _entered = runtime.enter();
            axum_server::from_tcp_rustls(listener, tls)
                .map_err(|e| e.to_string())?
                .handle(handle.clone())
        };
        let app = router(shared.clone());
        runtime.spawn(async move {
            let service = app.into_make_service_with_connect_info::<SocketAddr>();
            if let Err(e) = server.serve(service).await {
                tracing::warn!("remote control server stopped: {e}");
            }
        });
        runtime.spawn(watch_volume(shared.clone()));
        let announcement = Advertisement::start(
            &config.name,
            &config.host_name,
            port,
            &[
                ("id", &config.id),
                ("v", &PROTOCOL.to_string()),
                ("name", &config.name),
            ],
        )
        .inspect_err(|e| tracing::warn!("{e}"))
        .ok();
        tracing::info!("remote control listening on port {port}");
        Ok(Self {
            shared,
            runtime: Some(runtime),
            handle,
            port,
            _announcement: announcement,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn info(&self) -> &ServerInfo {
        &self.shared.info
    }

    pub fn pairing(&self) -> &Pairing {
        &self.shared.pairing
    }

    /// Passes an engine event on to connected phones.
    pub fn publish(&self, event: &EngineEvent) {
        let text = match event {
            EngineEvent::State(state) => {
                self.prefetch_covers(state.queue_index);
                Message::State { state }.json()
            }
            EngineEvent::Position { position_ms } => Message::Position {
                position_ms: *position_ms,
            }
            .json(),
            EngineEvent::Error { code, message } => Message::Error { code, message }.json(),
        };
        let _ = self.shared.events.send(text.into());
    }

    pub fn library_changed(&self) {
        let _ = self
            .shared
            .events
            .send(Message::LibraryChanged.json().into());
    }

    /// The volume may have changed on the PC: tell phones if it did.
    pub fn volume_changed(&self) {
        broadcast_volume(&self.shared);
    }

    /// Covers of a phone's tracks: the current one and the next two.
    fn prefetch_covers(&self, index: Option<u32>) {
        let (Some(index), Some(runtime)) = (index, &self.runtime) else {
            return;
        };
        let shared = self.shared.clone();
        runtime.spawn_blocking(move || {
            let engine = shared.host.engine();
            for entry in engine.queue_entries(index as usize, 3) {
                if let Some(key) = entry.cover_key.filter(|k| k.starts_with(art::PREFIX)) {
                    shared.art.ensure(&key, &engine.covers());
                }
            }
        });
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        self.handle
            .graceful_shutdown(Some(Duration::from_millis(500)));
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_secs(2));
        }
    }
}

fn broadcast_volume(shared: &Shared) {
    let volume = shared.host.volume();
    let mut last = shared.last_volume.lock();
    if *last != volume {
        let text = Message::Volume {
            volume: volume.as_ref(),
        }
        .json();
        *last = volume;
        let _ = shared.events.send(text.into());
    }
}

/// Catches volume changes made outside the app (the Windows volume, the keyboard).
async fn watch_volume(shared: Shared) {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tick.tick().await;
        if shared.events.receiver_count() > 0 {
            let s = shared.clone();
            let _ = tokio::task::spawn_blocking(move || broadcast_volume(&s)).await;
        }
    }
}

fn router(shared: Shared) -> Router {
    Router::new()
        .route("/v1/info", get(info))
        .route("/v1/pair", post(pair))
        .route("/v1/pair", delete(unpair))
        .route("/v1/ws", get(ws))
        .route("/v1/queue", get(queue))
        .route("/v1/cover/{*key}", get(cover))
        .layer(middleware::from_fn(local_only))
        .with_state(shared)
}

/// Refuses anyone outside the local network, whatever else they have.
async fn local_only(request: Request, next: Next) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip());
    match peer {
        Some(ip) if lan::is_local(ip) => next.run(request).await,
        _ => StatusCode::FORBIDDEN.into_response(),
    }
}

/// A request from a paired phone (`Authorization: Bearer <token>`).
struct Paired(PairedDevice);

impl FromRequestParts<Shared> for Paired {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        shared: &Shared,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;
        shared
            .pairing
            .authorize(token.trim())
            .map(Paired)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

async fn info(State(s): State<Shared>) -> Json<ServerInfo> {
    Json((*s.info).clone())
}

async fn pair(State(s): State<Shared>, Json(request): Json<PairRequest>) -> Response {
    match s
        .pairing
        .pair(&request.code, &request.device_id, &request.device_name)
    {
        Ok(token) => Json(PairResponse {
            token,
            server: (*s.info).clone(),
        })
        .into_response(),
        Err(e) => {
            let status = match e {
                PairError::WrongCode => StatusCode::FORBIDDEN,
                PairError::Closed | PairError::TooManyTries => StatusCode::GONE,
                PairError::Store(_) => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, e.to_string()).into_response()
        }
    }
}

async fn unpair(State(s): State<Shared>, Paired(device): Paired) -> StatusCode {
    match s.pairing.forget(&device.id) {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    200
}

async fn queue(State(s): State<Shared>, _: Paired, Query(page): Query<Page>) -> Response {
    Json(
        s.host
            .engine()
            .queue_entries(page.offset, page.limit.min(1000)),
    )
    .into_response()
}

async fn cover(State(s): State<Shared>, _: Paired, Path(key): Path<String>) -> Response {
    let found = tokio::task::spawn_blocking(move || {
        if let Some(rest) = key.strip_prefix("lib/") {
            let (hash, size) = rest.rsplit_once('-').unwrap_or((rest, "300"));
            let size = size.parse().unwrap_or(300);
            return s.host.library_art(hash, size).map(|d| ("image/jpeg", d));
        }
        let covers = s.host.engine().covers();
        if key.starts_with(art::PREFIX) {
            s.art.ensure(&key, &covers);
        }
        covers.get(&key).map(|a| (a.mime, a.data))
    })
    .await
    .ok()
    .flatten();
    match found {
        Some((mime, data)) => ([(header::CONTENT_TYPE, mime)], data).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn ws(
    State(s): State<Shared>,
    Paired(device): Paired,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(move |socket| session(s, socket, lan::canonical(peer.ip()), device))
}

async fn session(s: Shared, socket: WebSocket, peer: IpAddr, device: PairedDevice) {
    tracing::info!("remote: {} connected from {peer}", device.name);
    let (mut tx, mut rx) = socket.split();
    let mut events = s.events.subscribe();
    let greeting = [
        Message::Hello { server: &s.info }.json(),
        Message::State {
            state: &s.host.engine().snapshot(),
        }
        .json(),
        Message::Volume {
            volume: s.host.volume().as_ref(),
        }
        .json(),
    ];
    for text in greeting {
        if tx.send(WsMessage::Text(text.into())).await.is_err() {
            return;
        }
    }
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(text) => {
                    if tx.send(WsMessage::Text(text.as_ref().into())).await.is_err() {
                        break;
                    }
                }
                // Fell behind: the latest state says it all.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let state = Message::State { state: &s.host.engine().snapshot() }.json();
                    if tx.send(WsMessage::Text(state.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = rx.next() => match incoming {
                Some(Ok(WsMessage::Text(text))) => {
                    if let Err(message) = run(&s, peer, text.as_str()).await {
                        let error = Message::Error { code: "command", message: &message }.json();
                        if tx.send(WsMessage::Text(error.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Some(Ok(WsMessage::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
        }
    }
    tracing::info!("remote: {} disconnected", device.name);
}

async fn run(s: &Shared, peer: IpAddr, text: &str) -> Result<(), String> {
    let command: Command = serde_json::from_str(text).map_err(|e| format!("not a command: {e}"))?;
    let engine = s.host.engine();
    match command {
        Command::Play => engine.play(),
        Command::Pause => engine.pause(),
        Command::Toggle => engine.toggle(),
        Command::Stop => engine.stop(),
        Command::Next => engine.next(),
        Command::Previous => engine.previous(),
        Command::Seek { ms } => engine.seek(ms.max(0.0)),
        Command::SetRepeat { mode } => engine.set_repeat(mode),
        Command::SetShuffle { on } => engine.set_shuffle(on),
        Command::Jump { id } => engine.jump_to(id),
        Command::SetVolume { db } => {
            let s = s.clone();
            tokio::task::spawn_blocking(move || {
                let result = s.host.set_volume(db);
                broadcast_volume(&s);
                result
            })
            .await
            .map_err(|e| e.to_string())??;
        }
        Command::SetMuted { muted } => {
            let s = s.clone();
            tokio::task::spawn_blocking(move || {
                let result = s.host.set_muted(muted);
                broadcast_volume(&s);
                result
            })
            .await
            .map_err(|e| e.to_string())??;
        }
        Command::Load(request) => {
            let s = s.clone();
            tokio::task::spawn_blocking(move || load(&s, peer, request))
                .await
                .map_err(|e| e.to_string())??;
        }
    }
    Ok(())
}

/// Takes over a phone's queue: its files, served by the phone itself, from `position_ms`
/// into the track at `start`.
fn load(s: &Shared, peer: IpAddr, request: LoadRequest) -> Result<(), String> {
    if request.items.is_empty() {
        return Err("Nothing to play.".into());
    }
    if request.items.len() > MAX_LOAD {
        return Err(format!("At most {MAX_LOAD} tracks at a time."));
    }
    let start = request.start.min(request.items.len() - 1);
    let mut items = Vec::with_capacity(request.items.len());
    for item in request.items {
        if !lan::served_by(&item.url, peer) {
            return Err("Tracks must come from the phone that sends them, over HTTP.".into());
        }
        let cover_key = item
            .art_url
            .filter(|url| lan::served_by(url, peer))
            .map(|url| s.art.key_for(&url));
        items.push(QueueItem {
            path: PathBuf::from(item.url),
            cover_key,
            title: item.title.filter(|t| !t.trim().is_empty()),
            artist: item.artist,
            album: item.album,
            duration_ms: item.duration_ms.filter(|d| *d > 0.0),
            track_id: None,
            remote: true,
        });
    }
    let engine = s.host.engine();
    // The first cover now, so Now Playing has it from the start.
    if let Some(key) = &items[start].cover_key {
        s.art.ensure(key, &engine.covers());
    }
    engine.restore(QueueSnapshot {
        items,
        original: None,
        current: Some(start as u32),
        position_ms: request.position_ms.max(0.0),
        repeat: request.repeat.unwrap_or_default(),
    });
    if request.play {
        engine.play();
    }
    Ok(())
}
