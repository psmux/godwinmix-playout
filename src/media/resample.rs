//! Any rate to 48 kHz, in Rust with rubato's FFT resampler.
//!
//! The FFT resampler has a fixed ratio, which a file has, and costs a few
//! tenths of a percent of one core for stereo 44.1 to 48 kHz. Sound already
//! at 48 kHz passes straight through.

use rubato::{FftFixedInOut, Resampler as _};

pub const RATE: u32 = 48_000;

pub struct Resampler {
    fft: Option<FftFixedInOut<f32>>,
    /// Input waiting for a whole chunk, one Vec per channel.
    held: [Vec<f32>; 2],
    /// Output frames still to drop: the resampler's own delay, so the first
    /// sample out lines up with the first sample in.
    drop: usize,
    /// Output frames per input frame.
    ratio: f64,
    frames_in: u64,
    frames_out: u64,
}

impl Resampler {
    pub fn new(rate: u32) -> Resampler {
        let fft = (rate != RATE && rate > 0).then(|| FftFixedInOut::new(rate as usize, RATE as usize, 1024, 2).ok()).flatten();
        let drop = fft.as_ref().map_or(0, |f| f.output_delay());
        Resampler { fft, held: [Vec::new(), Vec::new()], drop, ratio: RATE as f64 / rate.max(1) as f64, frames_in: 0, frames_out: 0 }
    }

    /// Stereo interleaved in at the source rate, stereo interleaved out at 48 kHz.
    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        let Some(fft) = self.fft.as_mut() else {
            out.extend_from_slice(input);
            return;
        };
        self.frames_in += (input.len() / 2) as u64;
        for f in input.chunks_exact(2) {
            self.held[0].push(f[0]);
            self.held[1].push(f[1]);
        }
        loop {
            let need = fft.input_frames_next();
            if self.held[0].len() < need {
                break;
            }
            let chunk = [self.held[0].drain(..need).collect::<Vec<_>>(), self.held[1].drain(..need).collect::<Vec<_>>()];
            if let Ok(wave) = fft.process(&chunk, None) {
                let before = out.len();
                interleave(&wave, &mut self.drop, out);
                self.frames_out += ((out.len() - before) / 2) as u64;
            }
        }
    }

    /// The end of the sound: whatever is held and whatever the resampler
    /// holds inside it, so the item comes out exactly as long as it went in.
    pub fn flush(&mut self, out: &mut Vec<f32>) {
        let Some(fft) = self.fft.as_mut() else { return };
        let want = (self.frames_in as f64 * self.ratio).round() as u64;
        let mut tail = Vec::new();
        for _ in 0..4 {
            if self.frames_out + (tail.len() / 2) as u64 >= want {
                break;
            }
            let need = fft.input_frames_next();
            for ch in self.held.iter_mut() {
                ch.resize(need, 0.0);
            }
            if let Ok(wave) = fft.process(&self.held, None) {
                interleave(&wave, &mut self.drop, &mut tail);
            }
            self.held = [Vec::new(), Vec::new()];
        }
        let keep = (want.saturating_sub(self.frames_out) as usize * 2).min(tail.len());
        out.extend_from_slice(&tail[..keep]);
        self.frames_out += (keep / 2) as u64;
    }
}

fn interleave(wave: &[Vec<f32>], drop: &mut usize, out: &mut Vec<f32>) {
    let skip = (*drop).min(wave[0].len());
    *drop -= skip;
    for i in skip..wave[0].len() {
        out.push(wave[0][i]);
        out.push(wave[1][i]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forty_eight_kilohertz_passes_through_untouched() {
        let mut r = Resampler::new(48_000);
        let mut out = Vec::new();
        r.push(&[0.1, 0.2, 0.3, 0.4], &mut out);
        assert_eq!(out, [0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn a_second_at_44_1_comes_out_as_a_second_at_48() {
        let mut r = Resampler::new(44_100);
        let input: Vec<f32> = (0..44_100).flat_map(|i| {
            let s = (i as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin() * 0.5;
            [s, s]
        }).collect();
        let mut out = Vec::new();
        for chunk in input.chunks(2 * 1024) {
            r.push(chunk, &mut out);
        }
        r.flush(&mut out);
        let frames = out.len() / 2;
        assert!((frames as i64 - 48_000).abs() < 64, "{frames} frames");
        let peak = out.iter().fold(0f32, |m, s| m.max(s.abs()));
        assert!((peak - 0.5).abs() < 0.05, "peak {peak}");
    }
}
