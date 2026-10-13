//! DSD over PCM (DoP 1.1): raw DSD carried untouched in 24-bit PCM frames, so a DAC that
//! understands DoP plays native DSD through an ordinary PCM interface.
//!
//! Each frame carries 16 DSD bits per channel: the top byte is a marker that alternates
//! 0x05 / 0xFA from frame to frame, then the older and the newer DSD byte (MSB = oldest
//! bit). The frame rate is the DSD bit rate / 16 (DSD64 → 176.4 kHz).

use crate::format::{OutputFormat, SampleFormat, SampleKind};

pub const MARKER_A: u8 = 0x05;
pub const MARKER_B: u8 = 0xFA;
/// The DSD idle pattern ("digital silence" for DSD modulators).
pub const IDLE: u8 = 0x69;

/// The DoP frame rate for a DSD bit rate (per channel).
pub const fn dop_rate(dsd_bit_rate: u32) -> u32 {
    dsd_bit_rate / 16
}

/// Containers DoP fits in: 24 valid integer bits or more.
pub fn carries_dop(sample: SampleFormat) -> bool {
    sample.kind == SampleKind::Int && sample.valid_bits >= 24 && sample.container_bits >= 24
}

/// Writes one 24-bit DoP word into a container slot (little-endian, left-justified).
fn put(out: &mut Vec<u8>, sample: SampleFormat, marker: u8, older: u8, newer: u8) {
    if sample.container_bits == 24 {
        out.extend_from_slice(&[newer, older, marker]);
    } else {
        out.extend_from_slice(&[0, newer, older, marker]);
    }
}

/// The marker byte of the last complete frame in `bytes` (device format), if any.
pub fn last_marker(bytes: &[u8], format: OutputFormat) -> Option<u8> {
    let bpf = format.bytes_per_frame();
    if bytes.len() < bpf {
        return None;
    }
    let frames = bytes.len() / bpf;
    let frame = &bytes[(frames - 1) * bpf..frames * bpf];
    Some(frame[format.sample.bytes() - 1])
}

/// The marker following `marker`.
pub const fn next_marker(marker: u8) -> u8 {
    if marker == MARKER_A {
        MARKER_B
    } else {
        MARKER_A
    }
}

/// Writes `frames` idle frames, continuing the marker sequence from `*marker`.
pub fn idle(out: &mut Vec<u8>, format: OutputFormat, frames: usize, marker: &mut u8) {
    out.reserve(frames * format.bytes_per_frame());
    for _ in 0..frames {
        for _ in 0..format.channels {
            put(out, format.sample, *marker, IDLE, IDLE);
        }
        *marker = next_marker(*marker);
    }
}

/// Fills a device buffer with idle frames, continuing the marker sequence from `*marker`.
pub fn idle_into(out: &mut [u8], format: OutputFormat, marker: &mut u8) {
    let sample_bytes = format.sample.bytes();
    for frame in out.chunks_exact_mut(format.bytes_per_frame()) {
        for slot in frame.chunks_exact_mut(sample_bytes) {
            let len = slot.len();
            slot[len - 3..].copy_from_slice(&[IDLE, IDLE, *marker]);
            slot[..len - 3].fill(0);
        }
        *marker = next_marker(*marker);
    }
}

/// Packs interleaved raw DSD (one byte per channel per step, MSB = oldest bit) into DoP.
pub struct DopPacker {
    format: OutputFormat,
    marker: u8,
    /// A byte per channel left over when a block ends on an odd step.
    carry: Vec<u8>,
}

impl DopPacker {
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            marker: MARKER_A,
            carry: Vec::new(),
        }
    }

    pub fn format(&self) -> OutputFormat {
        self.format
    }

    /// Packs `dsd` (interleaved bytes for `format.channels` channels).
    pub fn pack(&mut self, dsd: &[u8], out: &mut Vec<u8>) {
        let channels = usize::from(self.format.channels);
        let mut bytes: Vec<u8>;
        let data = if self.carry.is_empty() {
            dsd
        } else {
            bytes = std::mem::take(&mut self.carry);
            bytes.extend_from_slice(dsd);
            &bytes
        };
        let step = channels * 2;
        let whole = data.len() / step * step;
        out.reserve(whole / step * self.format.bytes_per_frame());
        for pair in data[..whole].chunks_exact(step) {
            let (older, newer) = pair.split_at(channels);
            for c in 0..channels {
                put(out, self.format.sample, self.marker, older[c], newer[c]);
            }
            self.marker = next_marker(self.marker);
        }
        self.carry = data[whole..].to_vec();
    }

    /// Ends the stream: a trailing odd byte is padded with idle.
    pub fn finish(&mut self, out: &mut Vec<u8>) {
        if self.carry.is_empty() {
            return;
        }
        let channels = usize::from(self.format.channels);
        let older = std::mem::take(&mut self.carry);
        for c in 0..channels {
            put(
                out,
                self.format.sample,
                self.marker,
                older.get(c).copied().unwrap_or(IDLE),
                IDLE,
            );
        }
        self.marker = next_marker(self.marker);
    }

    /// Idle frames that continue this packer's marker sequence (pre-roll silence).
    pub fn idle(&mut self, frames: usize, out: &mut Vec<u8>) {
        idle(out, self.format, frames, &mut self.marker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S24_32: OutputFormat = OutputFormat {
        sample_rate: 176_400,
        channels: 2,
        sample: SampleFormat::S24_IN_32,
    };

    fn words(bytes: &[u8]) -> Vec<u32> {
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c) >> 8)
            .collect()
    }

    #[test]
    fn packs_two_dsd_bytes_per_channel_with_alternating_markers() {
        let mut p = DopPacker::new(S24_32);
        let mut out = Vec::new();
        // L R L R | L R L R
        p.pack(&[0x11, 0x21, 0x12, 0x22, 0x13, 0x23, 0x14, 0x24], &mut out);
        assert_eq!(
            words(&out),
            [0x05_1112, 0x05_2122, 0xFA_1314, 0xFA_2324],
            "marker, older byte, newer byte"
        );
        assert!(
            out.as_chunks::<4>().0.iter().all(|c| c[0] == 0),
            "low byte stays zero"
        );
    }

    #[test]
    fn odd_blocks_carry_over_without_breaking_the_sequence() {
        let mut p = DopPacker::new(S24_32);
        let mut out = Vec::new();
        p.pack(&[0xA0, 0xB0, 0xA1, 0xB1, 0xA2, 0xB2], &mut out);
        p.pack(&[0xA3, 0xB3], &mut out);
        assert_eq!(words(&out), [0x05_A0A1, 0x05_B0B1, 0xFA_A2A3, 0xFA_B2B3]);
        p.pack(&[0xA4, 0xB4], &mut out);
        p.finish(&mut out);
        assert_eq!(&words(&out)[4..], [0x05_A469, 0x05_B469]);
    }

    #[test]
    fn packed_24_bit_layout_and_idle_frames() {
        let fmt = OutputFormat {
            sample: SampleFormat::S24,
            ..S24_32
        };
        let mut p = DopPacker::new(fmt);
        let mut out = Vec::new();
        p.idle(2, &mut out);
        p.pack(&[1, 2, 3, 4], &mut out);
        assert_eq!(
            out,
            [
                0x69, 0x69, 0x05, 0x69, 0x69, 0x05, // idle, marker A
                0x69, 0x69, 0xFA, 0x69, 0x69, 0xFA, // idle, marker B
                3, 1, 0x05, 4, 2, 0x05, // data continues the sequence
            ]
        );
        assert_eq!(last_marker(&out, fmt), Some(0x05));
        let mut marker = next_marker(0x05);
        let mut tail = Vec::new();
        idle(&mut tail, fmt, 1, &mut marker);
        assert_eq!(tail[2], 0xFA);
    }

    #[test]
    fn which_containers_and_rates_carry_dop() {
        assert!(carries_dop(SampleFormat::S24));
        assert!(carries_dop(SampleFormat::S24_IN_32));
        assert!(carries_dop(SampleFormat::S32));
        assert!(!carries_dop(SampleFormat::S16));
        assert!(!carries_dop(SampleFormat::F32));
        assert_eq!(dop_rate(2_822_400), 176_400);
        assert_eq!(dop_rate(5_644_800), 352_800);
    }
}
