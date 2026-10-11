//! Playback engine: queue, gapless decode pipeline and output sessions.
//!
//! Threads: the **engine** thread owns the queue and the output session and reacts to
//! commands; a **decode worker** per session fills a lock-free ring buffer with bytes
//! already in the device format; the backend's **render** thread moves them to the device.
//! Track changes are frame-accurate markers in that byte stream, so position and
//! "now playing" follow what the device has actually played.

mod engine;
pub mod pipeline;
pub mod queue;
mod resample;
pub mod testing;
pub mod track;
mod worker;

pub use engine::{
    CoverStore, Engine, EngineConfig, EngineEvent, OutputSettings, OutputState, PlaybackStatus,
    PlayerSnapshot,
};
pub use queue::{QueueItem, RepeatMode};
pub use track::TrackMeta;
