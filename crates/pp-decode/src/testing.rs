//! Test fixtures FFmpeg can't make: FFmpeg has no DSD encoder, so DSF files are written here.

use std::path::Path;

/// DSF's fixed block size per channel.
const BLOCK: usize = 4096;

/// Writes a DSF file holding `channels[c]` as DSD bytes, oldest bit first (MSB-first).
/// The file stores them LSB-first in 4096-byte blocks per channel, the last one zero-padded
/// as real files are.
pub fn write_dsf(path: &Path, dsd_rate: u32, channels: &[Vec<u8>]) -> std::io::Result<()> {
    let n = channels.len();
    let bytes = channels.first().map_or(0, Vec::len);
    let blocks = bytes.div_ceil(BLOCK);
    let data_len = (blocks * BLOCK * n) as u64;
    let mut f = Vec::with_capacity(92 + data_len as usize);
    // "DSD " chunk: size, total file size, metadata offset (none).
    f.extend_from_slice(b"DSD ");
    f.extend_from_slice(&28u64.to_le_bytes());
    f.extend_from_slice(&(28 + 52 + 12 + data_len).to_le_bytes());
    f.extend_from_slice(&0u64.to_le_bytes());
    // "fmt " chunk.
    f.extend_from_slice(b"fmt ");
    f.extend_from_slice(&52u64.to_le_bytes());
    f.extend_from_slice(&1u32.to_le_bytes()); // format version
    f.extend_from_slice(&0u32.to_le_bytes()); // DSD raw
    f.extend_from_slice(&(if n == 1 { 1u32 } else { 2 }).to_le_bytes()); // mono / stereo
    f.extend_from_slice(&(n as u32).to_le_bytes());
    f.extend_from_slice(&dsd_rate.to_le_bytes());
    f.extend_from_slice(&1u32.to_le_bytes()); // 1 bit per sample, LSB first
    f.extend_from_slice(&(bytes as u64 * 8).to_le_bytes()); // samples per channel
    f.extend_from_slice(&(BLOCK as u32).to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    // "data" chunk: channel blocks in turn.
    f.extend_from_slice(b"data");
    f.extend_from_slice(&(12 + data_len).to_le_bytes());
    for b in 0..blocks {
        for ch in channels {
            let block = &ch[(b * BLOCK).min(bytes)..((b + 1) * BLOCK).min(bytes)];
            f.extend(block.iter().map(|v| v.reverse_bits()));
            f.resize(f.len() + BLOCK - block.len(), 0);
        }
    }
    std::fs::write(path, f)
}
