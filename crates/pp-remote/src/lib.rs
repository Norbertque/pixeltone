//! Remote control: phones on the same network find the PC (mDNS), pair once with a code
//! shown on the PC, then follow and drive the player over TLS, or hand their own queue
//! over to it ("play on the PC", like Spotify Connect). See `docs/remote-protocol.md`.
//!
//! No Tauri here: the app lends the engine and its volume rules through [`Host`].

mod art;
mod discovery;
pub mod jam;
pub mod lan;
pub mod pairing;
pub mod protocol;
mod server;
mod tls;

pub use pairing::{PairedDevice, Pairing};
pub use protocol::{PROTOCOL, RemoteVolume, ServerInfo};
pub use server::{Host, RemoteConfig, RemoteServer};
