//! WASAPI output for Windows: device discovery, exclusive-mode capability probing, shared
//! and exclusive streams, and endpoint (hardware) volume. Compiles to nothing elsewhere.
#![cfg(windows)]

mod backend;
mod convert;
mod stream;
mod volume;

pub use backend::WasapiBackend;
