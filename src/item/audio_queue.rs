//! Decoded sound for one item, at 48 kHz stereo, kept on the item's clock.
//!
//! The queue knows the time of its first sample (`head`) and of the sample
//! after its last (`end`), in frames of 48 kHz. Taking always moves `head`
//! on by what was asked for, whether or not the sound was there: if the
//! decoder was late, silence went out, and the late sound is trimmed when it
//! arrives rather than played late. A chunk that starts after `end` is
//! padded up to with silence. So the sound cannot drift from the picture.

use crate::media::resample::RATE;
use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

/// Frames held before the producer waits: four seconds, comfortably more
/// than the video pipeline holds, so the sound never runs dry waiting for a
/// picture to be decoded.
const CAP: usize = 4 * RATE as usize;

pub struct AudioQueue {
    inner: Mutex<Inner>,
    room: Condvar,
}

struct Inner {
    samples: VecDeque<f32>,
    head: i64,
    done: bool,
    closed: bool,
}

impl Inner {
    fn end(&self) -> i64 {
        self.head + (self.samples.len() / 2) as i64
    }
}

pub fn frames_to_ns(frames: u64) -> u64 {
    frames * 1_000_000_000 / RATE as u64
}

fn ns_to_frames(ns: i64) -> i64 {
    (ns as i128 * RATE as i128).div_euclid(1_000_000_000) as i64
}

impl AudioQueue {
    /// A queue whose first sample is at `start_ns` in the item.
    pub fn new(start_ns: u64) -> AudioQueue {
        let inner = Inner { samples: VecDeque::new(), head: ns_to_frames(start_ns as i64), done: false, closed: false };
        AudioQueue { inner: Mutex::new(inner), room: Condvar::new() }
    }

    /// Queue stereo `samples` starting at `pts_ns`. Waits while the queue is
    /// full, until there is room or the item is closed.
    pub fn push(&self, pts_ns: i64, samples: &[f32]) {
        let mut q = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        while q.samples.len() / 2 >= CAP && !q.closed {
            q = self.room.wait(q).unwrap_or_else(|e| e.into_inner());
        }
        if q.closed {
            return;
        }
        let at = ns_to_frames(pts_ns);
        let gap = at - q.end();
        let mut from = 0usize;
        if gap > 0 {
            // Silence up to where this chunk starts, at most two seconds of it.
            let pad = gap.min(2 * RATE as i64) as usize;
            q.samples.extend(std::iter::repeat_n(0.0, pad * 2));
        } else if gap < 0 {
            from = ((-gap) as usize * 2).min(samples.len());
        }
        q.samples.extend(&samples[from..]);
    }

    /// No more is coming.
    pub fn finish(&self) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).done = true;
    }

    /// Stop the producer waiting: the item is going away.
    pub fn close(&self) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        self.room.notify_all();
    }

    /// The next `out.len() / 2` frames of time. Answers how many had sound;
    /// the rest of `out` is left as it was, which the caller made silence.
    pub fn take(&self, out: &mut [f32]) -> usize {
        let mut q = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let want = out.len() / 2;
        let n = want.min(q.samples.len() / 2) * 2;
        for (o, s) in out[..n].iter_mut().zip(q.samples.drain(..n)) {
            *o = s;
        }
        q.head += want as i64;
        drop(q);
        self.room.notify_one();
        n / 2
    }

    /// Frames queued now.
    pub fn frames(&self) -> usize {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).samples.len() / 2
    }

    /// Everything has come and been taken.
    pub fn drained(&self) -> bool {
        let q = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        q.done && q.samples.is_empty()
    }

    pub fn is_done(&self) -> bool {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).done
    }
}

#[cfg(test)]
#[path = "audio_queue_tests.rs"]
mod tests;
