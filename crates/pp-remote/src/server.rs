//! The HTTPS + WebSocket server phones talk to: paired phones (the owner's) and, while a jam
//! runs, its guests.

use std::net::{IpAddr, Ipv6Addr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
use crate::jam::{GuestPermissions, Jam, Join};
use crate::lan;
use crate::pairing::{PairError, PairedDevice, Pairing};
use crate::protocol::{
    Command, GuestView, JamView, JoinRequest, JoinResponse, JoinWaiting, LibraryAlbum,
    LibraryAlbumDetail, LibraryResults, LoadRequest, MAX_LOAD, Message, PROTOCOL, PairRequest,
    PairResponse, RemoteVolume, ServerInfo,
};
use crate::tls::Identity;

/// The page a jam's QR code links to: the app opens such links, and the page tells people
/// without it where to get it. Everything after `#` stays on the phone.
pub const JAM_PAGE: &str = "https://norbertque.github.io/pixeltone/jam/";

/// What the app lends the server.
pub trait Host: Send + Sync + 'static {
    fn engine(&self) -> &Engine;
    /// The volume control in charge, if any.
    fn volume(&self) -> Option<RemoteVolume>;
    fn set_volume(&self, db: f32) -> Result<(), String>;
    fn set_muted(&self, muted: bool) -> Result<(), String>;
    /// A library thumbnail (cover keys `lib/<hash>-<size>`), as JPEG bytes.
    fn library_art(&self, hash: &str, size: u32) -> Option<Vec<u8>>;
    /// Library tracks as queue items, in the order asked; unknown ids are left out.
    fn library_items(&self, ids: &[u32]) -> Vec<QueueItem> {
        let _ = ids;
        Vec::new()
    }
    fn library_search(&self, query: &str, limit: u32) -> LibraryResults {
        let _ = (query, limit);
        LibraryResults::default()
    }
    fn library_albums(&self, offset: u32, limit: u32) -> Vec<LibraryAlbum> {
        let _ = (offset, limit);
        Vec::new()
    }
    fn library_album(&self, id: u32) -> Option<LibraryAlbumDetail> {
        let _ = id;
        None
    }
    /// The jam changed (started, ended, guests, rejoin requests), for the PC's own screens.
    fn jam_changed(&self, jam: Option<&JamView>) {
        let _ = jam;
    }
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

/// What goes out to connected phones, and to whom.
enum Outbound {
    /// Everyone: the player's state and position, library changes, errors.
    All(String),
    /// Paired phones only: the volume, the jam as owners see it.
    Owners(String),
    /// One guest: their place in the jam.
    Guest { id: String, text: String },
    /// Ends guests' connections: one (removed), or all (`None`: the jam ended).
    CloseGuests {
        id: Option<String>,
        reason: &'static str,
    },
}

#[derive(Clone)]
struct Shared {
    host: Arc<dyn Host>,
    pairing: Arc<Pairing>,
    info: Arc<ServerInfo>,
    port: u16,
    events: broadcast::Sender<Arc<Outbound>>,
    art: Arc<Art>,
    last_volume: Arc<Mutex<Option<RemoteVolume>>>,
    jam: Arc<Mutex<Option<Jam>>>,
    /// The jam as last announced, to announce only changes.
    last_jam: Arc<Mutex<Option<JamView>>>,
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
            port,
            events,
            art: Arc::new(Art::new()),
            last_volume: Arc::new(Mutex::new(None)),
            jam: Arc::new(Mutex::new(None)),
            last_jam: Arc::new(Mutex::new(None)),
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
        runtime.spawn(watch(shared.clone()));
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
        let _ = self.shared.events.send(Arc::new(Outbound::All(text)));
    }

    pub fn library_changed(&self) {
        let _ = self
            .shared
            .events
            .send(Arc::new(Outbound::All(Message::LibraryChanged.json())));
    }

    /// The volume may have changed on the PC: tell phones if it did.
    pub fn volume_changed(&self) {
        broadcast_volume(&self.shared);
    }

    /// The jam that runs, as owners see it.
    pub fn jam(&self) -> Option<JamView> {
        jam_view(&self.shared)
    }

    /// Starts a jam (or keeps the one that runs).
    pub fn jam_start(&self) -> JamView {
        jam_start(&self.shared)
    }

    /// Ends the jam: guests lose access at once.
    pub fn jam_end(&self) {
        jam_end(&self.shared);
    }

    pub fn jam_permissions(&self, permissions: GuestPermissions) {
        jam_permissions(&self.shared, permissions);
    }

    pub fn jam_remove_guest(&self, guest_id: &str) {
        jam_remove_guest(&self.shared, guest_id);
    }

    /// The owner's answer to a rejoin request; false when it isn't waiting any more.
    pub fn jam_answer(&self, request_id: &str, allow: bool) -> bool {
        jam_answer(&self.shared, request_id, allow)
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
        let _ = shared.events.send(Arc::new(Outbound::Owners(text)));
    }
}

/// Catches what changes by itself: the volume set outside the app (the Windows volume, the
/// keyboard), and rejoin requests running out.
async fn watch(shared: Shared) {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tick.tick().await;
        let s = shared.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if s.events.receiver_count() > 0 {
                broadcast_volume(&s);
            }
            announce_jam(&s);
        })
        .await;
    }
}

/* ---------------------------------- Jam ---------------------------------- */

/// Percent-encodes everything but unreserved characters (RFC 3986).
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The link a jam's QR code shows: this PC's address, its certificate and the jam's secret,
/// all after `#` (the page never sees them).
fn jam_link(s: &Shared, secret: &str) -> String {
    let host = lan::lan_address().unwrap_or_else(|| "127.0.0.1".into());
    format!(
        "{JAM_PAGE}#v={PROTOCOL}&h={}&p={}&f={}&s={}&n={}",
        encode(&host),
        s.port,
        s.info.fingerprint,
        encode(secret),
        encode(&s.info.name)
    )
}

fn jam_view(s: &Shared) -> Option<JamView> {
    let mut jam = s.jam.lock();
    let jam = jam.as_mut()?;
    let secret = jam.secret().to_owned();
    let info = jam.info(Instant::now());
    Some(JamView {
        link: jam_link(s, &secret),
        info,
    })
}

fn guest_view(s: &Shared, guest_id: &str) -> Option<GuestView> {
    let jam = s.jam.lock();
    let jam = jam.as_ref()?;
    let guest = jam.guest_by_id(guest_id)?;
    Some(GuestView {
        guest_id: guest.id.clone(),
        name: guest.name.clone(),
        permissions: jam.permissions,
    })
}

/// Tells owners and the PC's screens about the jam, if it changed since the last time.
fn announce_jam(s: &Shared) {
    let view = jam_view(s);
    {
        let mut last = s.last_jam.lock();
        if *last == view {
            return;
        }
        *last = view.clone();
    }
    let _ = s.events.send(Arc::new(Outbound::Owners(
        Message::Jam { jam: view.as_ref() }.json(),
    )));
    s.host.jam_changed(view.as_ref());
}

/// Every guest's view again (after the permissions changed).
fn tell_guests(s: &Shared) {
    let views: Vec<GuestView> = {
        let mut jam = s.jam.lock();
        let Some(jam) = jam.as_mut() else {
            return;
        };
        jam.info(Instant::now())
            .guests
            .into_iter()
            .map(|g| GuestView {
                guest_id: g.id,
                name: g.name,
                permissions: jam.permissions,
            })
            .collect()
    };
    for view in views {
        let text = Message::Guest { guest: &view }.json();
        let _ = s.events.send(Arc::new(Outbound::Guest {
            id: view.guest_id,
            text,
        }));
    }
}

fn jam_start(s: &Shared) -> JamView {
    {
        let mut jam = s.jam.lock();
        if jam.is_none() {
            *jam = Some(Jam::new());
        }
    }
    announce_jam(s);
    jam_view(s).expect("a jam was just started")
}

fn jam_end(s: &Shared) {
    if s.jam.lock().take().is_some() {
        let _ = s.events.send(Arc::new(Outbound::CloseGuests {
            id: None,
            reason: "ended",
        }));
        announce_jam(s);
    }
}

fn jam_permissions(s: &Shared, permissions: GuestPermissions) {
    if let Some(jam) = s.jam.lock().as_mut() {
        jam.permissions = permissions;
    }
    announce_jam(s);
    tell_guests(s);
}

fn jam_remove_guest(s: &Shared, guest_id: &str) {
    let Some(guest) = s.jam.lock().as_mut().and_then(|j| j.remove(guest_id)) else {
        return;
    };
    // Their tracks still to come go with them.
    let engine = s.host.engine();
    let current = engine.snapshot().queue_index.map(|i| i as usize);
    let upcoming: Vec<u32> = engine
        .queue_entries(0, usize::MAX)
        .into_iter()
        .enumerate()
        .filter(|(i, e)| {
            e.guest_id.as_deref() == Some(guest.id.as_str()) && current.is_none_or(|c| *i > c)
        })
        .map(|(_, e)| e.id)
        .collect();
    if !upcoming.is_empty() {
        engine.remove_from_queue(upcoming);
    }
    let _ = s.events.send(Arc::new(Outbound::CloseGuests {
        id: Some(guest.id),
        reason: "removed",
    }));
    announce_jam(s);
}

fn jam_answer(s: &Shared, request_id: &str, allow: bool) -> bool {
    let answered = s
        .jam
        .lock()
        .as_mut()
        .is_some_and(|j| j.answer(request_id, allow, Instant::now()));
    announce_jam(s);
    answered
}

/// Library tracks to the end of the queue, with the guest's name on them in a jam.
fn add_tracks(s: &Shared, track_ids: &[u32], guest: Option<&GuestView>) -> Result<(), String> {
    let mut items = s.host.library_items(track_ids);
    if items.is_empty() {
        return Err("Those tracks aren't in the library.".into());
    }
    if let Some(guest) = guest {
        for item in &mut items {
            item.added_by = Some(guest.name.clone());
            item.guest_id = Some(guest.guest_id.clone());
        }
    }
    s.host.engine().enqueue(items, false);
    Ok(())
}

/// Whether queue item `id` was added by this guest.
fn owns(s: &Shared, guest: &GuestView, id: u32) -> bool {
    s.host
        .engine()
        .queue_entries(0, usize::MAX)
        .iter()
        .any(|e| e.id == id && e.guest_id.as_deref() == Some(guest.guest_id.as_str()))
}

/* -------------------------------- Requests -------------------------------- */

fn router(shared: Shared) -> Router {
    Router::new()
        .route("/v1/info", get(info))
        .route("/v1/pair", post(pair))
        .route("/v1/pair", delete(unpair))
        .route("/v1/ws", get(ws))
        .route("/v1/queue", get(queue))
        .route("/v1/cover/{*key}", get(cover))
        .route("/v1/jam/join", post(join))
        .route("/v1/jam/requests/{id}", get(join_request))
        .route("/v1/library/search", get(library_search))
        .route("/v1/library/albums", get(library_albums))
        .route("/v1/library/albums/{id}", get(library_album))
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

fn bearer(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
}

/// A request from a paired phone (`Authorization: Bearer <token>`).
struct Paired(PairedDevice);

impl FromRequestParts<Shared> for Paired {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        shared: &Shared,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer(parts).ok_or(StatusCode::UNAUTHORIZED)?;
        shared
            .pairing
            .authorize(token)
            .map(Paired)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

/// Who's asking: a paired phone (the owner's), or a guest of the jam that runs.
enum Caller {
    Owner(PairedDevice),
    Guest(GuestView),
}

impl FromRequestParts<Shared> for Caller {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        shared: &Shared,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer(parts).ok_or(StatusCode::UNAUTHORIZED)?;
        if let Some(device) = shared.pairing.authorize(token) {
            return Ok(Caller::Owner(device));
        }
        let jam = shared.jam.lock();
        jam.as_ref()
            .and_then(|j| {
                j.guest(token).map(|g| GuestView {
                    guest_id: g.id.clone(),
                    name: g.name.clone(),
                    permissions: j.permissions,
                })
            })
            .map(Caller::Guest)
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

fn join_response(s: &Shared, outcome: Join) -> Response {
    match outcome {
        Join::Joined { guest_id, token } => Json(JoinResponse {
            token: &token,
            guest_id: &guest_id,
            server: &s.info,
        })
        .into_response(),
        Join::Waiting { request_id } => (
            StatusCode::ACCEPTED,
            Json(JoinWaiting {
                request_id: &request_id,
            }),
        )
            .into_response(),
        Join::Declined => (StatusCode::FORBIDDEN, "The owner said no.").into_response(),
        Join::Full => (StatusCode::CONFLICT, "The jam is full.").into_response(),
        Join::WrongSecret => (StatusCode::GONE, "This jam has ended.").into_response(),
    }
}

async fn join(State(s): State<Shared>, Json(request): Json<JoinRequest>) -> Response {
    let outcome = match s.jam.lock().as_mut() {
        Some(jam) => jam.join(
            &request.secret,
            &request.device_id,
            &request.name,
            Instant::now(),
        ),
        None => Join::WrongSecret,
    };
    let response = join_response(&s, outcome);
    announce_jam(&s);
    response
}

/// Where a rejoin request stands (asked again every few seconds while it waits).
async fn join_request(State(s): State<Shared>, Path(id): Path<String>) -> Response {
    let outcome = s
        .jam
        .lock()
        .as_mut()
        .and_then(|j| j.poll(&id, Instant::now()));
    let response = match outcome {
        Some(outcome) => join_response(&s, outcome),
        None => (StatusCode::GONE, "The request ran out, or the jam ended.").into_response(),
    };
    announce_jam(&s);
    response
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

async fn queue(State(s): State<Shared>, _: Caller, Query(page): Query<Page>) -> Response {
    Json(
        s.host
            .engine()
            .queue_entries(page.offset, page.limit.min(1000)),
    )
    .into_response()
}

#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    q: String,
    #[serde(default = "default_search_limit")]
    limit: u32,
}

fn default_search_limit() -> u32 {
    50
}

async fn library_search(
    State(s): State<Shared>,
    _: Caller,
    Query(search): Query<Search>,
) -> Response {
    let found = tokio::task::spawn_blocking(move || {
        s.host.library_search(&search.q, search.limit.min(200))
    })
    .await
    .unwrap_or_default();
    Json(found).into_response()
}

async fn library_albums(State(s): State<Shared>, _: Caller, Query(page): Query<Page>) -> Response {
    let offset = u32::try_from(page.offset).unwrap_or(u32::MAX);
    let limit = page.limit.min(1000) as u32;
    let albums = tokio::task::spawn_blocking(move || s.host.library_albums(offset, limit))
        .await
        .unwrap_or_default();
    Json(albums).into_response()
}

async fn library_album(State(s): State<Shared>, _: Caller, Path(id): Path<u32>) -> Response {
    match tokio::task::spawn_blocking(move || s.host.library_album(id))
        .await
        .ok()
        .flatten()
    {
        Some(album) => Json(album).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn cover(State(s): State<Shared>, _: Caller, Path(key): Path<String>) -> Response {
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
    caller: Caller,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(move |socket| session(s, socket, lan::canonical(peer.ip()), caller))
}

async fn session(s: Shared, socket: WebSocket, peer: IpAddr, caller: Caller) {
    let (who, me) = match &caller {
        Caller::Owner(device) => (device.name.clone(), None),
        Caller::Guest(guest) => (
            format!("{} (guest)", guest.name),
            Some(guest.guest_id.clone()),
        ),
    };
    tracing::info!("remote: {who} connected from {peer}");
    let (mut tx, mut rx) = socket.split();
    let mut events = s.events.subscribe();
    let mut greeting = vec![
        Message::Hello { server: &s.info }.json(),
        Message::State {
            state: &s.host.engine().snapshot(),
        }
        .json(),
    ];
    match &caller {
        Caller::Owner(_) => {
            greeting.push(
                Message::Volume {
                    volume: s.host.volume().as_ref(),
                }
                .json(),
            );
            greeting.push(
                Message::Jam {
                    jam: jam_view(&s).as_ref(),
                }
                .json(),
            );
        }
        Caller::Guest(guest) => greeting.push(Message::Guest { guest }.json()),
    }
    for text in greeting {
        if tx.send(WsMessage::Text(text.into())).await.is_err() {
            return;
        }
    }
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(out) => {
                    let text = match (&*out, &me) {
                        (Outbound::All(text), _) | (Outbound::Owners(text), None) => Some(text.clone()),
                        (Outbound::Guest { id, text }, Some(me)) if id == me => Some(text.clone()),
                        (Outbound::CloseGuests { id, reason }, Some(me)) if id.as_ref().is_none_or(|id| id == me) => {
                            let _ = tx.send(WsMessage::Text(Message::JamClosed { reason }.json().into())).await;
                            break;
                        }
                        _ => None,
                    };
                    if let Some(text) = text
                        && tx.send(WsMessage::Text(text.into())).await.is_err()
                    {
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
                    // A guest's rights may have changed since they connected: as they are now.
                    let now = match &caller {
                        Caller::Owner(device) => Some(Caller::Owner(device.clone())),
                        Caller::Guest(guest) => guest_view(&s, &guest.guest_id).map(Caller::Guest),
                    };
                    let Some(now) = now else {
                        break;
                    };
                    match run(&s, peer, &now, text.as_str()).await {
                        Ok(After::Stay) => {}
                        Ok(After::Leave) => break,
                        Err(message) => {
                            let error = Message::Error { code: "command", message: &message }.json();
                            if tx.send(WsMessage::Text(error.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Some(Ok(WsMessage::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
        }
    }
    tracing::info!("remote: {who} disconnected");
}

/// What a command means for the connection that sent it.
enum After {
    Stay,
    /// A guest left the jam.
    Leave,
}

async fn run(s: &Shared, peer: IpAddr, caller: &Caller, text: &str) -> Result<After, String> {
    let command: Command = serde_json::from_str(text).map_err(|e| format!("not a command: {e}"))?;
    let s = s.clone();
    match caller {
        Caller::Owner(_) => tokio::task::spawn_blocking(move || owner_command(&s, peer, command))
            .await
            .map_err(|e| e.to_string())?,
        Caller::Guest(guest) => {
            let guest = guest.clone();
            tokio::task::spawn_blocking(move || guest_command(&s, &guest, command))
                .await
                .map_err(|e| e.to_string())?
        }
    }
}

/// The player's own commands, the same for owners and (if allowed) guests.
fn playback(engine: &Engine, command: &Command) -> bool {
    match *command {
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
        _ => return false,
    }
    true
}

fn owner_command(s: &Shared, peer: IpAddr, command: Command) -> Result<After, String> {
    let engine = s.host.engine();
    if playback(engine, &command) {
        return Ok(After::Stay);
    }
    match command {
        Command::SetVolume { db } => {
            let result = s.host.set_volume(db);
            broadcast_volume(s);
            result?;
        }
        Command::SetMuted { muted } => {
            let result = s.host.set_muted(muted);
            broadcast_volume(s);
            result?;
        }
        Command::Load(request) => load(s, peer, request)?,
        Command::JamStart => {
            jam_start(s);
        }
        Command::JamEnd => jam_end(s),
        Command::JamPermissions { permissions } => jam_permissions(s, permissions),
        Command::JamRemoveGuest { guest_id } => jam_remove_guest(s, &guest_id),
        Command::JamAnswer { request_id, allow } => {
            if !jam_answer(s, &request_id, allow) {
                return Err("That request isn't waiting any more.".into());
            }
        }
        Command::QueueAdd { track_ids } => add_tracks(s, &track_ids, None)?,
        Command::QueueRemove { id } => engine.remove_from_queue(vec![id]),
        Command::QueueMove { id, to } => engine.move_in_queue(id, to),
        Command::JamLeave => return Err("Only guests leave a jam.".into()),
        _ => {}
    }
    Ok(After::Stay)
}

fn guest_command(s: &Shared, guest: &GuestView, command: Command) -> Result<After, String> {
    let engine = s.host.engine();
    let allowed = guest.permissions;
    match command {
        Command::Play
        | Command::Pause
        | Command::Toggle
        | Command::Stop
        | Command::Next
        | Command::Previous
        | Command::Seek { .. }
        | Command::Jump { .. } => {
            if !allowed.control_playback {
                return Err("The owner hasn't let guests control playback.".into());
            }
            playback(engine, &command);
        }
        Command::QueueAdd { track_ids } => {
            if !allowed.add_tracks {
                return Err("The owner hasn't let guests add tracks.".into());
            }
            add_tracks(s, &track_ids, Some(guest))?;
        }
        Command::QueueRemove { id } | Command::QueueMove { id, .. }
            if !allowed.change_own_tracks =>
        {
            let _ = id;
            return Err("The owner hasn't let guests change their tracks.".into());
        }
        Command::QueueRemove { id } => {
            if !owns(s, guest, id) {
                return Err("That track isn't yours.".into());
            }
            engine.remove_from_queue(vec![id]);
        }
        Command::QueueMove { id, to } => {
            if !owns(s, guest, id) {
                return Err("That track isn't yours.".into());
            }
            engine.move_in_queue(id, to);
        }
        Command::JamLeave => {
            if let Some(jam) = s.jam.lock().as_mut() {
                jam.leave(&guest.guest_id);
            }
            announce_jam(s);
            return Ok(After::Leave);
        }
        _ => return Err("Only the owner can do that.".into()),
    }
    Ok(After::Stay)
}

/// Takes over a phone's queue: its files, served by the phone itself, from `position_ms`
/// into the track at `start`. While a jam runs the queue is everyone's: the phone's tracks
/// join its end instead.
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
            added_by: None,
            guest_id: None,
        });
    }
    let engine = s.host.engine();
    if s.jam.lock().is_some() {
        engine.enqueue(items, false);
        return Ok(());
    }
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
