//! Decoding through FFmpeg into [`PcmBlock`]s.
//!
//! FFmpeg's integer sample formats are full-scale with `bits_per_raw_sample` significant
//! bits, which maps directly onto the left-justified `i32` representation used by
//! `pp-core`, so lossless sources come out untouched. Encoder delay and padding (MP3 LAME
//! headers, AAC edit lists, Opus pre-skip) are trimmed by FFmpeg, which is what makes
//! gapless playback possible.

use std::ffi::{CStr, CString};
use std::path::Path;
use std::sync::Once;

use pp_core::{PcmBlock, SampleKind, Samples, SourceFormat};
use rsmpeg::avcodec::AVCodecContext;
use rsmpeg::avformat::AVFormatContextInput;
use rsmpeg::avutil::{AVFrame, av_rescale_q};
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("cannot open {path}: {reason}")]
    Open { path: String, reason: String },
    #[error("{0} has no audio stream")]
    NoAudioStream(String),
    #[error("cannot initialise the {codec} decoder: {reason}")]
    Codec { codec: String, reason: String },
    #[error("decoding failed: {0}")]
    Decode(String),
    #[error("seeking failed: {0}")]
    Seek(String),
}

/// What FFmpeg reports about a track before decoding it.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackInfo {
    /// Decoder name, e.g. "flac", "alac", "mp3float", "ape", "wavpack", "dsd_lsbf_planar".
    pub codec: String,
    /// Container short name, e.g. "flac", "mov,mp4,m4a,3gp,3g2,mj2", "dsf".
    pub container: String,
    pub source: SourceFormat,
    /// Exact length in frames when the container states it.
    pub total_frames: Option<u64>,
    pub duration_ms: Option<u64>,
    pub lossless: bool,
    pub dsd: bool,
}

/// Tags as FFmpeg exposes them (stream tags win over container tags, which is where Ogg
/// and Opus keep theirs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub track: Option<String>,
    pub disc: Option<String>,
}

/// An embedded picture (front cover).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverArt {
    pub data: Vec<u8>,
    pub mime: &'static str,
}

/// MIME type from the image's magic bytes.
pub fn image_mime(data: &[u8]) -> Option<&'static str> {
    match data {
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some("image/webp"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [b'B', b'M', ..] => Some("image/bmp"),
        _ => None,
    }
}

/// FFmpeg's version string, e.g. "n8.1.3-17-gc389840894-20261010".
pub fn ffmpeg_version() -> String {
    rsmpeg::avutil::version_info()
        .to_string_lossy()
        .into_owned()
}

/// libavcodec's license, e.g. "LGPL version 2.1 or later".
pub fn ffmpeg_license() -> String {
    unsafe { CStr::from_ptr(ffi::avcodec_license()) }
        .to_string_lossy()
        .into_owned()
}

fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe { ffi::av_log_set_level(ffi::AV_LOG_ERROR as i32) });
}

/// Pulls PCM out of one file.
pub struct Decoder {
    input: AVFormatContextInput,
    codec: AVCodecContext,
    stream_index: usize,
    time_base: ffi::AVRational,
    info: TrackInfo,
    sent_eof: bool,
    finished: bool,
    /// After a seek, frames before this position are dropped (sample-accurate seeking).
    seek_target: Option<u64>,
    /// Position of the next frame `next_block` returns.
    position: u64,
}

impl Decoder {
    pub fn open(path: &Path) -> Result<Self, DecodeError> {
        init();
        let display = path.display().to_string();
        let url = path
            .to_str()
            .map(|p| format!("file:{p}"))
            .and_then(|p| CString::new(p).ok())
            .ok_or_else(|| DecodeError::Open {
                path: display.clone(),
                reason: "the path is not valid UTF-8".into(),
            })?;

        let input = AVFormatContextInput::open(&url).map_err(|e| DecodeError::Open {
            path: display.clone(),
            reason: e.to_string(),
        })?;

        let (stream_index, decoder) = input
            .find_best_stream(ffi::AVMEDIA_TYPE_AUDIO)
            .map_err(|e| DecodeError::Open {
                path: display.clone(),
                reason: e.to_string(),
            })?
            .ok_or_else(|| DecodeError::NoAudioStream(display.clone()))?;

        let codec_name = decoder.name().to_string_lossy().into_owned();
        // The stream borrows `input`; keep that borrow in this block so `input` can move later.
        let (time_base, stream_duration, codec_id, raw_bits, codec) = {
            let stream = &input.streams()[stream_index];
            let codecpar = stream.codecpar();
            let mut codec = AVCodecContext::new(&decoder);
            codec
                .apply_codecpar(&codecpar)
                .map_err(|e| DecodeError::Codec {
                    codec: codec_name.clone(),
                    reason: e.to_string(),
                })?;
            (
                stream.time_base,
                stream.duration,
                codecpar.codec_id,
                codecpar.bits_per_raw_sample,
                codec,
            )
        };
        let mut codec = codec;
        codec.open(None).map_err(|e| DecodeError::Codec {
            codec: codec_name.clone(),
            reason: e.to_string(),
        })?;

        let sample_rate = codec.sample_rate.max(0) as u32;
        let channels = codec.ch_layout().nb_channels.max(1) as u16;
        let packed = unsafe { ffi::av_get_packed_sample_fmt(codec.sample_fmt) };
        let (kind, container_bits) = match packed {
            ffi::AV_SAMPLE_FMT_U8 => (SampleKind::Int, 8),
            ffi::AV_SAMPLE_FMT_S16 => (SampleKind::Int, 16),
            ffi::AV_SAMPLE_FMT_S32 => (SampleKind::Int, 32),
            ffi::AV_SAMPLE_FMT_S64 => (SampleKind::Int, 32),
            ffi::AV_SAMPLE_FMT_DBL => (SampleKind::Float, 64),
            _ => (SampleKind::Float, 32),
        };
        let bits = match kind {
            SampleKind::Int if raw_bits > 0 && raw_bits < container_bits => raw_bits as u16,
            _ => container_bits as u16,
        };

        let descriptor = unsafe { ffi::avcodec_descriptor_get(codec_id) };
        let lossless = !descriptor.is_null()
            && unsafe { (*descriptor).props } & ffi::AV_CODEC_PROP_LOSSLESS as i32 != 0;
        let dsd = codec_name.starts_with("dsd_");

        let total_frames = (stream_duration > 0 && sample_rate > 0).then(|| {
            av_rescale_q(
                stream_duration,
                time_base,
                ffi::AVRational {
                    num: 1,
                    den: sample_rate as i32,
                },
            ) as u64
        });
        let duration_ms = total_frames
            .map(|f| f * 1000 / u64::from(sample_rate.max(1)))
            .or_else(|| (input.duration > 0).then(|| input.duration as u64 / 1000));
        let container = input.iformat().name().to_string_lossy().into_owned();

        Ok(Self {
            input,
            codec,
            stream_index,
            time_base,
            info: TrackInfo {
                codec: codec_name,
                container,
                source: SourceFormat {
                    sample_rate,
                    channels,
                    bits,
                    kind,
                },
                total_frames,
                duration_ms,
                lossless: lossless || dsd,
                dsd,
            },
            sent_eof: false,
            finished: false,
            seek_target: None,
            position: 0,
        })
    }

    pub fn info(&self) -> &TrackInfo {
        &self.info
    }

    pub fn source_format(&self) -> SourceFormat {
        self.info.source
    }

    /// Frame position of the next block.
    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn tags(&self) -> Tags {
        let stream_meta = self.input.streams()[self.stream_index].metadata();
        let format_meta = self.input.metadata();
        let get = |key: &CStr| -> Option<String> {
            [stream_meta.as_ref(), format_meta.as_ref()]
                .into_iter()
                .flatten()
                .find_map(|dict| dict.get(key, None, 0))
                .map(|e| e.value().to_string_lossy().trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Tags {
            title: get(c"title"),
            artist: get(c"artist"),
            album: get(c"album"),
            album_artist: get(c"album_artist"),
            genre: get(c"genre"),
            date: get(c"date"),
            track: get(c"track"),
            disc: get(c"disc"),
        }
    }

    /// The first embedded picture, if any (FFmpeg exposes covers as attached-picture streams).
    pub fn cover_art(&self) -> Option<CoverArt> {
        self.input.streams().iter().find_map(|stream| {
            if stream.disposition & ffi::AV_DISPOSITION_ATTACHED_PIC as i32 == 0 {
                return None;
            }
            let pic = &stream.attached_pic;
            if pic.data.is_null() || pic.size <= 0 {
                return None;
            }
            let data = unsafe { std::slice::from_raw_parts(pic.data, pic.size as usize) }.to_vec();
            let mime = image_mime(&data)?;
            Some(CoverArt { data, mime })
        })
    }

    /// The next block of interleaved PCM, or `None` at the end of the track.
    pub fn next_block(&mut self) -> Result<Option<PcmBlock>, DecodeError> {
        if self.finished {
            return Ok(None);
        }
        loop {
            match self.codec.receive_frame() {
                Ok(frame) => {
                    if let Some(block) = self.accept(&frame)? {
                        return Ok(Some(block));
                    }
                    continue;
                }
                Err(RsmpegError::DecoderDrainError) => {}
                Err(RsmpegError::DecoderFlushedError) => {
                    self.finished = true;
                    return Ok(None);
                }
                // A corrupt frame: skip it and keep going rather than stopping the track.
                Err(e) => {
                    tracing::warn!("skipping undecodable frame: {e}");
                    continue;
                }
            }

            if self.sent_eof {
                // The decoder asked for input after the flush; nothing more will come.
                self.finished = true;
                return Ok(None);
            }
            match self.input.read_packet() {
                Ok(Some(packet)) => {
                    if packet.stream_index as usize != self.stream_index {
                        continue;
                    }
                    match self.codec.send_packet(Some(&packet)) {
                        Ok(()) => {}
                        Err(RsmpegError::DecoderFlushedError) => {
                            self.finished = true;
                            return Ok(None);
                        }
                        Err(e) => tracing::warn!("skipping corrupt packet: {e}"),
                    }
                }
                Ok(None) => {
                    self.sent_eof = true;
                    let _ = self.codec.send_packet(None);
                }
                Err(e) => return Err(DecodeError::Decode(e.to_string())),
            }
        }
    }

    /// Seeks so the next block starts exactly at `frame`.
    pub fn seek(&mut self, frame: u64) -> Result<(), DecodeError> {
        let rate = self.info.source.sample_rate.max(1) as i32;
        let ts = av_rescale_q(
            frame as i64,
            ffi::AVRational { num: 1, den: rate },
            self.time_base,
        );
        self.input
            .seek(
                self.stream_index as i32,
                ts,
                ffi::AVSEEK_FLAG_BACKWARD as i32,
            )
            .map_err(|e| DecodeError::Seek(e.to_string()))?;
        self.codec.flush_buffers();
        self.sent_eof = false;
        self.finished = false;
        self.seek_target = Some(frame);
        self.position = frame;
        Ok(())
    }

    /// Converts a decoded frame, trimming it if a seek target lies inside it.
    fn accept(&mut self, frame: &AVFrame) -> Result<Option<PcmBlock>, DecodeError> {
        let block = frame_to_block(frame)?;
        let frames = block.frames() as u64;
        let channels = usize::from(block.channels);

        let Some(target) = self.seek_target else {
            self.position += frames;
            return Ok(Some(block));
        };

        let ts = if frame.pts != ffi::AV_NOPTS_VALUE {
            frame.pts
        } else {
            frame.best_effort_timestamp
        };
        if ts == ffi::AV_NOPTS_VALUE {
            // Can't refine without timestamps; resume here.
            self.seek_target = None;
            self.position = target + frames;
            return Ok(Some(block));
        }
        let rate = self.info.source.sample_rate.max(1) as i32;
        let start =
            av_rescale_q(ts, self.time_base, ffi::AVRational { num: 1, den: rate }).max(0) as u64;
        let end = start + frames;
        if end <= target {
            return Ok(None);
        }
        self.seek_target = None;
        let skip = target.saturating_sub(start) as usize * channels;
        self.position = end;
        if skip == 0 {
            return Ok(Some(block));
        }
        let samples = match block.samples {
            Samples::Int(v) => Samples::Int(v[skip..].to_vec()),
            Samples::Float(v) => Samples::Float(v[skip..].to_vec()),
        };
        Ok(Some(PcmBlock {
            channels: block.channels,
            samples,
        }))
    }
}

/// Interleaves one FFmpeg audio frame into a [`PcmBlock`].
fn frame_to_block(frame: &AVFrame) -> Result<PcmBlock, DecodeError> {
    let channels = frame.ch_layout.nb_channels.max(1) as usize;
    let n = frame.nb_samples.max(0) as usize;
    let format = frame.format;
    let planar = unsafe { ffi::av_sample_fmt_is_planar(format) } != 0;
    let packed = unsafe { ffi::av_get_packed_sample_fmt(format) };
    let data = frame.extended_data;
    if data.is_null() {
        return Err(DecodeError::Decode("frame without data".into()));
    }

    /// Reads sample `i` of channel `ch` as `T`.
    ///
    /// Safety: `data` must be the frame's `extended_data` holding `n` samples of `T` per
    /// channel, planar or interleaved as `planar` says.
    #[inline]
    unsafe fn read<T: Copy>(
        data: *mut *mut u8,
        planar: bool,
        channels: usize,
        ch: usize,
        i: usize,
    ) -> T {
        unsafe {
            if planar {
                *(*data.add(ch) as *const T).add(i)
            } else {
                *(*data as *const T).add(i * channels + ch)
            }
        }
    }

    let total = n * channels;
    macro_rules! collect {
        ($t:ty, $map:expr) => {{
            let mut out = Vec::with_capacity(total);
            for i in 0..n {
                for ch in 0..channels {
                    let v: $t = unsafe { read::<$t>(data, planar, channels, ch, i) };
                    out.push($map(v));
                }
            }
            out
        }};
    }

    let samples = match packed {
        ffi::AV_SAMPLE_FMT_U8 => Samples::Int(collect!(u8, |v: u8| (i32::from(v) - 128) << 24)),
        ffi::AV_SAMPLE_FMT_S16 => Samples::Int(collect!(i16, |v: i16| i32::from(v) << 16)),
        ffi::AV_SAMPLE_FMT_S32 => Samples::Int(collect!(i32, |v: i32| v)),
        ffi::AV_SAMPLE_FMT_S64 => Samples::Int(collect!(i64, |v: i64| (v >> 32) as i32)),
        ffi::AV_SAMPLE_FMT_FLT => Samples::Float(collect!(f32, |v: f32| v)),
        ffi::AV_SAMPLE_FMT_DBL => Samples::Float(collect!(f64, |v: f64| v as f32)),
        other => {
            return Err(DecodeError::Decode(format!(
                "unsupported sample format {other}"
            )));
        }
    };
    Ok(PcmBlock {
        channels: channels as u16,
        samples,
    })
}

#[cfg(test)]
mod tests;
