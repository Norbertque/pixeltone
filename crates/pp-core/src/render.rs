//! The render side of a stream: takes device-format bytes from the ring buffer, applies the
//! software gain and transport fades, and pads with silence. Every output backend renders
//! through this, so gain and fades behave the same everywhere and are tested once.
//!
//! Fades only exist around pause, seek and skip. Outside a ramp, bytes pass through
//! untouched (or through the software gain, when it is active).

use std::sync::Arc;

use crate::format::OutputFormat;
use crate::gain::apply_q16_in_place;
use crate::output::{Fade, StreamControls};

/// Full level of a fade, in Q16.
pub const UNITY: u32 = 1 << 16;

/// What one [`RenderFeed::fill`] produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fill {
    /// Frames taken from the ring, at the start of the buffer; the rest is silence.
    pub ring_frames: usize,
    /// The stream has faded out: from this frame on, the buffer is fade-out silence.
    pub silent_from: Option<usize>,
}

pub struct RenderFeed {
    source: rtrb::Consumer<u8>,
    controls: Arc<StreamControls>,
    format: OutputFormat,
    bpf: usize,
    /// Fade level, 0 (silent) ..= [`UNITY`].
    level: u32,
}

impl RenderFeed {
    pub fn new(
        source: rtrb::Consumer<u8>,
        controls: Arc<StreamControls>,
        format: OutputFormat,
    ) -> Self {
        let level = match controls.fade() {
            Fade::Audible => UNITY,
            Fade::In | Fade::Out => 0,
        };
        Self {
            source,
            controls,
            format,
            bpf: format.bytes_per_frame(),
            level,
        }
    }

    pub fn available_frames(&self) -> usize {
        self.source.slots() / self.bpf
    }

    /// Drops everything waiting in the ring (a flush).
    pub fn discard(&mut self) {
        let n = self.source.slots();
        if let Ok(chunk) = self.source.read_chunk(n) {
            chunk.commit_all();
        }
    }

    /// Fills `out` (whole frames) for the device.
    pub fn fill(&mut self, out: &mut [u8]) -> Fill {
        let bpf = self.bpf;
        let frames = out.len() / bpf;
        let out = &mut out[..frames * bpf];
        let fade = self.controls.fade();
        let step = self.controls.fade_step();
        match fade {
            Fade::Audible => self.level = UNITY,
            Fade::In if step == 0 => self.level = UNITY,
            Fade::Out if step == 0 => self.level = 0,
            _ => {}
        }

        let ready = self.available_frames().min(frames);
        if fade == Fade::Out && (self.level == 0 || ready == 0) {
            // Silent (or nothing left to fade): hold, leaving the ring for the resume.
            self.level = 0;
            out.fill(0);
            return Fill {
                ring_frames: 0,
                silent_from: Some(0),
            };
        }
        // A fade-out takes only the frames its ramp needs.
        let take = if fade == Fade::Out {
            ready.min(self.level.div_ceil(step) as usize)
        } else {
            ready
        };
        let (taken, _) = self.source.pop_partial_slice(&mut out[..take * bpf]);
        let taken = taken.len();
        let audio = &mut out[..taken];
        if let Some(q16) = self.controls.gain_q16() {
            apply_q16_in_place(audio, self.format.sample, q16);
        }

        let mut silent_from = None;
        match fade {
            Fade::Out => {
                for frame in audio.chunks_exact_mut(bpf) {
                    self.level = self.level.saturating_sub(step);
                    scale(frame, self.format, self.level);
                }
                if self.level == 0 {
                    silent_from = Some(taken / bpf);
                }
            }
            Fade::In | Fade::Audible if self.level < UNITY => {
                for frame in audio.chunks_exact_mut(bpf) {
                    if self.level >= UNITY {
                        break;
                    }
                    // The ramp starts with the music, not with leading or pre-roll silence.
                    if self.level == 0 && frame.iter().all(|&b| b == 0) {
                        continue;
                    }
                    self.level = (self.level + step).min(UNITY);
                    scale(frame, self.format, self.level);
                }
            }
            _ => {}
        }
        out[taken..].fill(0);
        Fill {
            ring_frames: taken / bpf,
            silent_from,
        }
    }
}

/// Raised-cosine fade curve: Q16 gain for a Q16 level.
pub fn fade_gain(level: u32) -> i64 {
    if level >= UNITY {
        return i64::from(UNITY);
    }
    let x = f64::from(level) / f64::from(UNITY);
    let g = 0.5 - 0.5 * (std::f64::consts::PI * x).cos();
    (g * f64::from(UNITY)).round() as i64
}

fn scale(frame: &mut [u8], format: OutputFormat, level: u32) {
    if level < UNITY {
        apply_q16_in_place(frame, format.sample, fade_gain(level));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::SampleFormat;

    const FORMAT: OutputFormat = OutputFormat {
        sample_rate: 48_000,
        channels: 2,
        sample: SampleFormat::S16,
    };

    /// A feed over `frames` stereo frames whose samples are all `value`.
    fn feed(frames: usize, value: i16) -> (RenderFeed, Arc<StreamControls>) {
        let (mut producer, consumer) = rtrb::RingBuffer::new(frames * 4 + 64);
        let bytes: Vec<u8> = (0..frames * 2).flat_map(|_| value.to_le_bytes()).collect();
        assert_eq!(producer.push_partial_slice(&bytes).0.len(), bytes.len());
        let controls = Arc::new(StreamControls::default());
        (
            RenderFeed::new(consumer, controls.clone(), FORMAT),
            controls,
        )
    }

    fn samples(bytes: &[u8]) -> Vec<i16> {
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c))
            .collect()
    }

    #[test]
    fn passes_audio_through_untouched_without_a_fade() {
        let (mut feed, _) = feed(100, 12_345);
        let mut out = vec![0xAAu8; 150 * 4];
        let fill = feed.fill(&mut out);
        assert_eq!(fill.ring_frames, 100);
        assert_eq!(fill.silent_from, None);
        let s = samples(&out);
        assert!(s[..200].iter().all(|&v| v == 12_345));
        assert!(s[200..].iter().all(|&v| v == 0), "padding is silence");
    }

    #[test]
    fn fades_out_over_the_ramp_and_keeps_the_rest_in_the_ring() {
        let (mut feed, controls) = feed(1000, 10_000);
        controls.set_fade_frames(100);
        controls.set_fade(Fade::Out);
        let mut out = vec![0u8; 400 * 4];
        let fill = feed.fill(&mut out);
        assert_eq!(fill.ring_frames, 100, "only the ramp is taken");
        assert_eq!(fill.silent_from, Some(100));
        let s = samples(&out);
        // Monotonically falling, ending in silence.
        assert!(s[0] < 10_000 && s[0] > 9_900, "first ramp frame {}", s[0]);
        assert!(s.windows(2).take(199).all(|w| w[0] >= w[1]));
        assert_eq!(s[198], 0);
        assert!(s[200..].iter().all(|&v| v == 0));
        assert_eq!(feed.available_frames(), 900);

        // Held silent without consuming.
        let fill = feed.fill(&mut out);
        assert_eq!(fill.ring_frames, 0);
        assert_eq!(fill.silent_from, Some(0));
        assert_eq!(feed.available_frames(), 900);
    }

    #[test]
    fn fades_back_in_from_where_it_stopped() {
        let (mut feed, controls) = feed(1000, 10_000);
        controls.set_fade_frames(100);
        controls.set_fade(Fade::Out);
        let mut out = vec![0u8; 400 * 4];
        feed.fill(&mut out);

        controls.set_fade(Fade::In);
        let fill = feed.fill(&mut out);
        assert_eq!(fill.ring_frames, 400);
        assert_eq!(fill.silent_from, None);
        let s = samples(&out);
        assert!(s[0] > 0 && s[0] < 100, "ramp starts low: {}", s[0]);
        assert!(s.windows(2).take(199).all(|w| w[0] <= w[1]));
        assert_eq!(s[198], 10_000, "full level after the ramp");
        assert!(s[200..].iter().all(|&v| v == 10_000), "then untouched");
    }

    #[test]
    fn a_fade_in_waits_for_the_music() {
        let (mut producer, consumer) = rtrb::RingBuffer::new(4096);
        let mut bytes = vec![0u8; 50 * 4]; // pre-roll silence
        bytes.extend((0..100 * 2).flat_map(|_| 8_000i16.to_le_bytes()));
        assert_eq!(producer.push_partial_slice(&bytes).0.len(), bytes.len());
        let controls = Arc::new(StreamControls::default());
        controls.set_fade_frames(20);
        controls.set_fade(Fade::In);
        let mut feed = RenderFeed::new(consumer, controls, FORMAT);
        let mut out = vec![0u8; 150 * 4];
        feed.fill(&mut out);
        let s = samples(&out);
        assert!(s[..100].iter().all(|&v| v == 0));
        assert!(
            s[100] > 0 && s[100] < 8_000,
            "ramp starts at the first music frame"
        );
        assert_eq!(s[100 + 38], 8_000);
    }

    #[test]
    fn instant_fades_when_fading_is_off() {
        let (mut feed, controls) = feed(100, 5_000);
        controls.set_fade(Fade::Out); // fade_frames 0
        let mut out = vec![0u8; 10 * 4];
        let fill = feed.fill(&mut out);
        assert_eq!(fill.silent_from, Some(0));
        assert_eq!(feed.available_frames(), 100);
        controls.set_fade(Fade::In);
        feed.fill(&mut out);
        assert!(samples(&out).iter().all(|&v| v == 5_000));
    }

    #[test]
    fn fading_out_an_empty_ring_is_immediately_silent() {
        let (mut feed, controls) = feed(0, 0);
        controls.set_fade_frames(100);
        controls.set_fade(Fade::Out);
        let mut out = vec![0u8; 10 * 4];
        assert_eq!(feed.fill(&mut out).silent_from, Some(0));
    }

    #[test]
    fn software_gain_and_fade_combine() {
        let (mut feed, controls) = feed(1000, 16_000);
        controls.set_gain(crate::SoftwareGain::new(-6.0206));
        let mut out = vec![0u8; 10 * 4];
        feed.fill(&mut out);
        assert!(samples(&out).iter().all(|&v| v == 8_000));
        controls.set_fade_frames(10);
        controls.set_fade(Fade::Out);
        feed.fill(&mut out);
        let s = samples(&out);
        assert!(s[0] < 8_000 && s[0] > 7_500);
        assert_eq!(s[18], 0);
    }

    #[test]
    fn curve_is_monotonic_from_zero_to_unity() {
        assert_eq!(fade_gain(0), 0);
        assert_eq!(fade_gain(UNITY), i64::from(UNITY));
        let mut last = 0;
        for l in (0..=UNITY).step_by(256) {
            let g = fade_gain(l);
            assert!(g >= last);
            last = g;
        }
    }
}
