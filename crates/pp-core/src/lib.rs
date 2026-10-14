//! Platform-independent core of the Pixeltone audio path.
//!
//! Everything that decides *what* reaches the DAC lives here and is unit-tested on any
//! host: the sample formats, the exclusive-mode format negotiator, PCM packing with the
//! optional software gain stage, the Now Playing badge, and the traits an output
//! backend (WASAPI on Windows, capture/null in tests) implements.

pub mod badge;
pub mod dop;
pub mod eq;
pub mod format;
pub mod gain;
pub mod negotiate;
pub mod output;
pub mod pcm;
pub mod render;

pub use badge::{Effects, OutputBadge};
pub use eq::EqSettings;
pub use format::{OutputFormat, SampleFormat, SampleKind, SourceFormat};
pub use gain::SoftwareGain;
pub use negotiate::{Conversion, FormatCandidate, Negotiated, negotiate};
pub use output::{
    DeviceCapabilities, DeviceChange, DeviceInfo, DeviceWatch, Fade, OpenRequest, OutputBackend,
    OutputError, OutputStream, ShareMode, StreamControls, StreamStats,
};
pub use pcm::{Packer, PcmBlock, Samples};
pub use render::{Fill, RenderFeed};
