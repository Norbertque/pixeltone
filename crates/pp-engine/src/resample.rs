//! Streaming sample-rate conversion (rubato's FFT resampler). Only used on converted paths:
//! shared mode, or exclusive mode when the device lacks the source rate.

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Indexing, Resampler};

const CHUNK: usize = 1024;

pub struct ResampleStage {
    rs: Fft<f32>,
    channels: usize,
    ratio: f64,
    /// Interleaved input not yet consumed.
    input: Vec<f32>,
    out_scratch: Vec<f32>,
    /// Output frames still to drop to compensate for the resampler's delay.
    trim: usize,
    frames_in: u64,
    frames_out: u64,
}

impl ResampleStage {
    pub fn new(from: u32, to: u32, channels: u16) -> Result<Self, String> {
        let channels = usize::from(channels.max(1));
        let rs = Fft::<f32>::new(
            from as usize,
            to as usize,
            CHUNK,
            channels,
            FixedSync::Input,
        )
        .map_err(|e| format!("resampler {from}→{to}: {e}"))?;
        let trim = rs.output_delay();
        Ok(Self {
            rs,
            channels,
            ratio: f64::from(to) / f64::from(from),
            input: Vec::new(),
            out_scratch: Vec::new(),
            trim,
            frames_in: 0,
            frames_out: 0,
        })
    }

    /// Feeds interleaved samples and appends whatever output is ready to `out`.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.input.extend_from_slice(input);
        self.frames_in += (input.len() / self.channels) as u64;
        loop {
            let need = self.rs.input_frames_next();
            if self.input.len() / self.channels < need {
                break;
            }
            let used = self.run(None, out);
            self.input.drain(..used * self.channels);
        }
    }

    /// End of stream: processes the remainder and flushes the delay line, so the output
    /// length matches the input length times the ratio.
    pub fn finish(&mut self, out: &mut Vec<f32>) {
        let expected = (self.frames_in as f64 * self.ratio).round() as u64;
        let left = self.input.len() / self.channels;
        if left > 0 {
            self.run(Some(left), out);
            self.input.clear();
        }
        let mut guard = 0;
        while self.frames_out < expected && guard < 64 {
            self.run(Some(0), out);
            guard += 1;
        }
        if self.frames_out > expected {
            let extra = (self.frames_out - expected) as usize * self.channels;
            out.truncate(out.len().saturating_sub(extra));
            self.frames_out = expected;
        }
    }

    /// One `process_into_buffer` call; returns the input frames consumed.
    fn run(&mut self, partial: Option<usize>, out: &mut Vec<f32>) -> usize {
        let ch = self.channels;
        let need = self.rs.input_frames_next();
        let produce = self.rs.output_frames_next();
        self.out_scratch.resize(produce * ch, 0.0);

        let available = (self.input.len() / ch).min(need);
        let mut padded;
        let in_slice: &[f32] = if available < need {
            padded = self.input[..available * ch].to_vec();
            padded.resize(need * ch, 0.0);
            &padded
        } else {
            &self.input[..need * ch]
        };
        let inbuf = InterleavedSlice::new(in_slice, ch, need).expect("input buffer size");
        let mut outbuf = InterleavedSlice::new_mut(&mut self.out_scratch, ch, produce)
            .expect("output buffer size");
        let indexing = partial.map(|n| Indexing::new().partial_len(n));
        let (used, written) = self
            .rs
            .process_into_buffer(&inbuf, &mut outbuf, indexing.as_ref())
            .expect("resampler buffers are sized from input_frames_next/output_frames_next");

        let mut start = 0;
        if self.trim > 0 {
            start = self.trim.min(written);
            self.trim -= start;
        }
        out.extend_from_slice(&self.out_scratch[start * ch..written * ch]);
        self.frames_out += (written - start) as u64;
        used.min(available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, freq: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v = (i as f32 * freq * std::f32::consts::TAU / rate as f32).sin() * 0.5;
                [v, v]
            })
            .collect()
    }

    #[test]
    fn output_length_follows_the_ratio() {
        let mut rs = ResampleStage::new(44_100, 48_000, 2).unwrap();
        let input = sine(44_100, 1000.0, 44_100);
        let mut out = Vec::new();
        for chunk in input.chunks(2 * 777) {
            rs.process(chunk, &mut out);
        }
        rs.finish(&mut out);
        assert_eq!(out.len() / 2, 48_000);
    }

    #[test]
    fn delay_is_trimmed_so_the_tone_starts_in_phase() {
        let mut rs = ResampleStage::new(48_000, 96_000, 2).unwrap();
        let input = sine(48_000, 500.0, 48_000);
        let mut out = Vec::new();
        rs.process(&input, &mut out);
        rs.finish(&mut out);
        // Compare against an ideal 96 kHz rendering of the same tone, away from the edges.
        let ideal = sine(96_000, 500.0, 96_000);
        let err: f32 = (20_000..20_100)
            .map(|i| (out[i * 2] - ideal[i * 2]).abs())
            .fold(0.0, f32::max);
        assert!(err < 0.01, "max error {err}");
    }
}
