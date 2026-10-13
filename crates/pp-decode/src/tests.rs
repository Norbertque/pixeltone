//! Decoder tests against fixtures generated on the fly with the fetched `ffmpeg` binary
//! (`PP_FFMPEG`, see `.cargo/config.toml`). The reference for bit-exactness is FFmpeg's own
//! decode of the same file, written out as raw little-endian PCM.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

fn ffmpeg() -> String {
    std::env::var("PP_FFMPEG").unwrap_or_else(|_| "ffmpeg".into())
}

fn run_ffmpeg(args: &[&str]) {
    let out = Command::new(ffmpeg())
        .args(["-hide_banner", "-v", "error", "-y"])
        .args(args)
        .output()
        .expect("run ffmpeg (set PP_FFMPEG or run scripts/fetch-ffmpeg.sh)");
    assert!(
        out.status.success(),
        "ffmpeg {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Two decorrelated channels: pink noise left, a 997 Hz tone right.
fn encode(dir: &Path, name: &str, rate: u32, seconds: &str, codec_args: &[&str]) -> PathBuf {
    let path = dir.join(name);
    let noise = format!("anoisesrc=d={seconds}:c=pink:r={rate}:a=0.3:seed=7");
    let tone = format!("sine=f=997:r={rate}:d={seconds}");
    let mut args = vec!["-f", "lavfi", "-i", &noise, "-f", "lavfi", "-i", &tone];
    args.extend(["-filter_complex", "[0][1]amerge=inputs=2"]);
    args.extend(codec_args);
    let p = path.to_str().unwrap().to_string();
    args.push(&p);
    run_ffmpeg(&args);
    path
}

/// FFmpeg's own decode of `path` as raw little-endian samples.
fn reference(path: &Path, raw_format: &str) -> Vec<u8> {
    let out = path.with_extension(format!("{raw_format}.raw"));
    run_ffmpeg(&[
        "-i",
        path.to_str().unwrap(),
        "-f",
        raw_format,
        out.to_str().unwrap(),
    ]);
    std::fs::read(out).unwrap()
}

fn decode_all(path: &Path) -> (Decoder, Vec<PcmBlock>) {
    let mut dec = Decoder::open(path).unwrap();
    let mut blocks = Vec::new();
    while let Some(b) = dec.next_block().unwrap() {
        blocks.push(b);
    }
    (dec, blocks)
}

fn ints(blocks: &[PcmBlock]) -> Vec<i32> {
    blocks
        .iter()
        .flat_map(|b| match &b.samples {
            Samples::Int(v) => v.clone(),
            Samples::Float(_) => panic!("expected integer samples"),
        })
        .collect()
}

fn floats(blocks: &[PcmBlock]) -> Vec<f32> {
    blocks
        .iter()
        .flat_map(|b| match &b.samples {
            Samples::Float(v) => v.clone(),
            Samples::Int(_) => panic!("expected float samples"),
        })
        .collect()
}

fn le_i32(bytes: &[u8]) -> Vec<i32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| i32::from_le_bytes(*b))
        .collect()
}

fn assert_int_bit_exact(path: &Path, expected_bits: &[u16]) -> Decoder {
    let (dec, blocks) = decode_all(path);
    let src = dec.source_format();
    assert_eq!(src.kind, SampleKind::Int, "{}", path.display());
    assert!(expected_bits.contains(&src.bits), "bits = {}", src.bits);
    assert!(dec.info().lossless);
    let got = ints(&blocks);
    let want = le_i32(&reference(path, "s32le"));
    assert_eq!(got.len(), want.len(), "sample count");
    assert!(
        got == want,
        "decoded samples differ from FFmpeg's reference"
    );
    dec
}

#[test]
fn flac_24_bit_is_bit_exact() {
    let dir = tempfile::tempdir().unwrap();
    let f = encode(
        dir.path(),
        "a.flac",
        96_000,
        "0.5",
        &[
            "-c:a",
            "flac",
            "-sample_fmt",
            "s32",
            "-bits_per_raw_sample",
            "24",
        ],
    );
    let dec = assert_int_bit_exact(&f, &[24]);
    assert_eq!(dec.info().codec, "flac");
    assert_eq!(dec.source_format().sample_rate, 96_000);
    assert_eq!(dec.source_format().channels, 2);
    assert_eq!(dec.info().total_frames, Some(48_000));
}

#[test]
fn flac_16_bit_is_bit_exact() {
    let dir = tempfile::tempdir().unwrap();
    let f = encode(
        dir.path(),
        "a.flac",
        44_100,
        "0.5",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    assert_int_bit_exact(&f, &[16]);
}

#[test]
fn alac_24_bit_is_bit_exact() {
    let dir = tempfile::tempdir().unwrap();
    let f = encode(
        dir.path(),
        "a.m4a",
        192_000,
        "0.25",
        &[
            "-c:a",
            "alac",
            "-sample_fmt",
            "s32p",
            "-bits_per_raw_sample",
            "24",
        ],
    );
    assert_int_bit_exact(&f, &[24]);
}

#[test]
fn wavpack_is_bit_exact() {
    let dir = tempfile::tempdir().unwrap();
    let f = encode(
        dir.path(),
        "a.wv",
        88_200,
        "0.25",
        &["-c:a", "wavpack", "-sample_fmt", "s32p"],
    );
    assert_int_bit_exact(&f, &[24, 32]);
}

#[test]
fn wav_24_bit_is_bit_exact() {
    let dir = tempfile::tempdir().unwrap();
    let f = encode(dir.path(), "a.wav", 48_000, "0.25", &["-c:a", "pcm_s24le"]);
    assert_int_bit_exact(&f, &[24]);
}

#[test]
fn float_wav_decodes_to_identical_floats() {
    let dir = tempfile::tempdir().unwrap();
    let f = encode(dir.path(), "a.wav", 48_000, "0.25", &["-c:a", "pcm_f32le"]);
    let (dec, blocks) = decode_all(&f);
    assert_eq!(dec.source_format().kind, SampleKind::Float);
    let got = floats(&blocks);
    let want: Vec<f32> = reference(&f, "f32le")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    assert_eq!(got.len(), want.len());
    assert!(
        got.iter()
            .zip(&want)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
}

#[test]
fn seek_is_sample_accurate() {
    let dir = tempfile::tempdir().unwrap();
    let f = encode(
        dir.path(),
        "a.flac",
        44_100,
        "1",
        &["-c:a", "flac", "-sample_fmt", "s16"],
    );
    let (_, blocks) = decode_all(&f);
    let all = ints(&blocks);

    let target = 30_123u64;
    let mut dec = Decoder::open(&f).unwrap();
    dec.seek(target).unwrap();
    let mut after = Vec::new();
    while let Some(b) = dec.next_block().unwrap() {
        after.extend(ints(&[b]));
    }
    assert_eq!(after.as_slice(), &all[target as usize * 2..]);
}

/// Encodes exactly `frames` frames of a tone and checks the decoder returns exactly that
/// many — FFmpeg must trim encoder delay and padding for gapless playback.
fn assert_gapless(name: &str, rate: u32, codec_args: &[&str]) {
    let dir = tempfile::tempdir().unwrap();
    let frames = 2 * rate + 123;
    let path = dir.path().join(name);
    let src = format!("sine=f=440:r={rate}");
    let trim = format!("atrim=end_sample={frames}");
    let mut args = vec!["-f", "lavfi", "-i", &src, "-af", &trim];
    args.extend(codec_args);
    let p = path.to_str().unwrap().to_string();
    args.push(&p);
    run_ffmpeg(&args);

    let (dec, blocks) = decode_all(&path);
    let decoded: usize = blocks.iter().map(PcmBlock::frames).sum();
    assert_eq!(
        decoded as u32,
        frames,
        "{name}: decoded {decoded} frames, encoded {frames} ({})",
        dec.info().codec
    );
}

#[test]
fn mp3_is_gapless() {
    assert_gapless("a.mp3", 44_100, &["-c:a", "libmp3lame", "-b:a", "192k"]);
}

/// FFmpeg's own AAC encoder doesn't record the end padding in a way its MP4 demuxer
/// applies (iTunes-encoded files do, via the edit list), so here the requirement is only
/// that we match FFmpeg's decode exactly, priming samples removed.
#[test]
fn aac_in_m4a_matches_ffmpeg() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.m4a");
    let p = path.to_str().unwrap().to_string();
    run_ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "sine=f=440:r=44100",
        "-af",
        "atrim=end_sample=88323",
        "-c:a",
        "aac",
        "-b:a",
        "192k",
        &p,
    ]);
    let (_, blocks) = decode_all(&path);
    let decoded: usize = blocks.iter().map(PcmBlock::frames).sum();
    let reference_frames = reference(&path, "f32le").len() / 4;
    assert_eq!(decoded, reference_frames);
    assert!(
        decoded < 88_323 + 1024,
        "priming samples must be trimmed ({decoded})"
    );
}

#[test]
fn opus_is_gapless() {
    assert_gapless("a.opus", 48_000, &["-c:a", "libopus", "-b:a", "128k"]);
}

#[test]
fn vorbis_is_gapless() {
    assert_gapless("a.ogg", 44_100, &["-c:a", "libvorbis", "-q:a", "5"]);
}

#[test]
fn reads_tags_and_cover_art() {
    let dir = tempfile::tempdir().unwrap();
    let cover = dir.path().join("cover.png");
    run_ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "color=c=0x7e57c2:s=64x64:d=1",
        "-frames:v",
        "1",
        cover.to_str().unwrap(),
    ]);
    let path = dir.path().join("tagged.flac");
    run_ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "sine=f=440:r=44100:d=0.2",
        "-i",
        cover.to_str().unwrap(),
        "-map",
        "0:a",
        "-map",
        "1:v",
        "-c:a",
        "flac",
        "-c:v",
        "png",
        "-disposition:v",
        "attached_pic",
        "-metadata",
        "title=Neon Tidepool",
        "-metadata",
        "artist=Aurora Lane",
        "-metadata",
        "album=Pixel Dreams",
        path.to_str().unwrap(),
    ]);
    let dec = Decoder::open(&path).unwrap();
    let tags = dec.tags();
    assert_eq!(tags.title.as_deref(), Some("Neon Tidepool"));
    assert_eq!(tags.artist.as_deref(), Some("Aurora Lane"));
    assert_eq!(tags.album.as_deref(), Some("Pixel Dreams"));
    let art = dec.cover_art().expect("embedded cover");
    assert_eq!(art.mime, "image/png");
}

#[test]
fn reports_ffmpeg_version() {
    assert!(!ffmpeg_version().is_empty());
    assert!(ffmpeg_license().contains("GPL"));
}

/// 1.5 DSF blocks of known bytes per channel, as DSD64.
fn dsf_fixture(dir: &Path) -> (PathBuf, Vec<Vec<u8>>) {
    let path = dir.join("tone.dsf");
    let left: Vec<u8> = (0..6000u32).map(|i| (i * 7 + 3) as u8).collect();
    let right: Vec<u8> = (0..6000u32).map(|i| (i * 13 + 5) as u8).collect();
    testing::write_dsf(&path, 2_822_400, &[left.clone(), right.clone()]).unwrap();
    (path, vec![left, right])
}

fn interleave(channels: &[Vec<u8>], from: usize) -> Vec<u8> {
    (from..channels[0].len())
        .flat_map(|i| channels.iter().map(move |c| c[i]))
        .collect()
}

#[test]
fn raw_dsd_comes_out_untouched_and_without_block_padding() {
    let dir = tempfile::tempdir().unwrap();
    let (path, channels) = dsf_fixture(dir.path());
    let mut dec = Decoder::open(&path).unwrap();
    assert!(dec.info().dsd);
    assert_eq!(dec.dsd_bit_rate(), Some(2_822_400));
    // FFmpeg reports whole DSF blocks as the length but trims the last block's padding.
    assert!(dec.info().total_frames.is_some_and(|t| t >= 6000));
    let mut out = Vec::new();
    while dec.next_dsd(&mut out).unwrap() {}
    assert_eq!(out.len(), 12_000);
    assert!(
        out == interleave(&channels, 0),
        "MSB-first, interleaved, padding dropped"
    );
}

#[test]
fn raw_dsd_seeks_to_the_byte() {
    let dir = tempfile::tempdir().unwrap();
    let (path, channels) = dsf_fixture(dir.path());
    let mut dec = Decoder::open(&path).unwrap();
    dec.seek_dsd(5000).unwrap();
    let mut out = Vec::new();
    while dec.next_dsd(&mut out).unwrap() {}
    assert!(out == interleave(&channels, 5000));
}

#[test]
fn dsd_still_decodes_to_pcm() {
    let dir = tempfile::tempdir().unwrap();
    let (path, _) = dsf_fixture(dir.path());
    let (dec, blocks) = decode_all(&path);
    assert_eq!(dec.info().source.sample_rate, 352_800);
    assert!(!blocks.is_empty());
    assert!(matches!(blocks[0].samples, Samples::Float(_)));
}

#[test]
fn reads_replaygain_tags() {
    let dir = tempfile::tempdir().unwrap();
    let path = encode(
        dir.path(),
        "rg.flac",
        44_100,
        "0.2",
        &[
            "-metadata",
            "REPLAYGAIN_TRACK_GAIN=-6.50 dB",
            "-metadata",
            "REPLAYGAIN_TRACK_PEAK=0.950000",
            "-metadata",
            "REPLAYGAIN_ALBUM_GAIN=-7.25 dB",
        ],
    );
    let rg = Decoder::open(&path).unwrap().tags().replay_gain;
    assert_eq!(rg.track_gain_db, Some(-6.5));
    assert_eq!(rg.track_peak, Some(0.95));
    assert_eq!(rg.album_gain_db, Some(-7.25));
    assert_eq!(rg.album_peak, None);
    assert_eq!(leading_number("+2.10 dB"), Some(2.1));
    assert_eq!(leading_number("garbage"), None);
}
