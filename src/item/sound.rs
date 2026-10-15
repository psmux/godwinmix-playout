//! Sound from inside a video file: decoded, resampled, and queued with its
//! time worked out from the first packet and the count since.

use super::audio_queue::{ns_to_frames, AudioQueue};
use crate::media::audio::AudioDecoder;
use crate::media::resample::Resampler;

/// Sound decoded and resampled, with its time worked out from the first
/// packet and the count since, so the resampler's chunking cannot move it.
pub struct Sound {
    decoder: AudioDecoder,
    resampler: Option<Resampler>,
    rate: u32,
    base_ns: Option<i64>,
    produced: u64,
    raw: Vec<f32>,
    out: Vec<f32>,
}

impl Sound {
    pub fn new(decoder: AudioDecoder) -> Sound {
        Sound { decoder, resampler: None, rate: 0, base_ns: None, produced: 0, raw: Vec::new(), out: Vec::new() }
    }

    pub fn packet(&mut self, pts: i64, data: Vec<u8>, queue: &AudioQueue) {
        self.raw.clear();
        let Some((rate, at)) = self.decoder.decode(data, pts, &mut self.raw) else { return };
        if self.raw.is_empty() {
            return;
        }
        self.resample(at, rate, queue);
    }

    fn resample(&mut self, pts: i64, rate: u32, queue: &AudioQueue) {
        if self.resampler.is_none() || rate != self.rate {
            self.resampler = Some(Resampler::new(rate));
            self.rate = rate;
        }
        let base = *self.base_ns.get_or_insert(pts);
        self.out.clear();
        self.resampler.as_mut().expect("made above").push(&self.raw, &mut self.out);
        self.emit(base, queue);
    }

    fn emit(&mut self, base: i64, queue: &AudioQueue) {
        if self.out.is_empty() {
            return;
        }
        queue.push_at(ns_to_frames(base) + self.produced as i64, &self.out);
        self.produced += (self.out.len() / 2) as u64;
    }

    pub fn finish(&mut self, queue: &AudioQueue) {
        self.raw.clear();
        self.decoder.finish(&mut self.raw);
        let base = self.base_ns.unwrap_or(0);
        self.out.clear();
        if let Some(r) = self.resampler.as_mut() {
            if !self.raw.is_empty() {
                r.push(&self.raw, &mut self.out);
            }
            r.flush(&mut self.out);
        }
        self.emit(base, queue);
    }
}

