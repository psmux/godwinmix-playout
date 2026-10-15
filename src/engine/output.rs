//! One tick's picture and sound, written to the core, and the snapshot.

use super::Engine;
use crate::item::Media;
use crate::media::resample::RATE;
use crate::settings::AtEnd;
use crate::state::Mode;
use std::sync::Arc;

/// Ten milliseconds of stereo: the buffer size the media contract asks for.
const BUFFER: usize = (RATE as usize / 100) * 2;

impl Engine {
    /// The picture for this tick. Moves to the next item when the current
    /// one has run out.
    pub(super) fn picture(&mut self) -> Arc<Vec<u8>> {
        if let Some(why) = self.current.as_ref().and_then(|c| c.player.error()) {
            // A file that breaks halfway: its last good picture stays up
            // while the next item takes over.
            let id = self.current_id().unwrap_or_default();
            self.failed(&id, why);
            self.current = None;
        }
        match self.mode {
            Mode::Idle => return self.black.clone(),
            Mode::Ended if self.settings.at_end == AtEnd::Black => return self.black.clone(),
            Mode::Playing => {
                let t = self.item_ns();
                if self.current.as_mut().is_some_and(|c| c.player.media() != Some(Media::Sound) && c.player.ended(t)) {
                    self.finish_current();
                }
            }
            _ => {}
        }
        if self.mode == Mode::Playing {
            let t = self.item_ns();
            let fresh = self.current.as_mut().and_then(|c| c.player.frame_at(t));
            match fresh {
                Some(f) => {
                    let old = std::mem::replace(&mut self.last, f.data);
                    if let Some(c) = self.current.as_ref() {
                        c.player.recycle(crate::item::Frame { pts: 0, data: old });
                    }
                }
                None if self.current.as_mut().is_some_and(|c| c.player.media() == Some(Media::Video)) => self.repeats += 1,
                None => {}
            }
            if self.current.as_mut().is_some_and(|c| c.player.media() == Some(Media::Sound)) {
                self.last = self.black.clone();
            }
        }
        self.last.clone()
    }

    /// This tick's share of sound, from the current item, with silence
    /// where it has none. A sound file that ends inside the tick hands over
    /// to the next item inside the tick, so back to back songs meet exactly.
    fn sound(&mut self) -> Vec<f32> {
        let fps = self.canvas.fps.max(1) as u64;
        let k = self.frame_no;
        let n = ((k + 1) * RATE as u64 / fps - k * RATE as u64 / fps) as usize;
        let mut out = vec![0.0f32; n * 2];
        if self.mode != Mode::Playing {
            return out;
        }
        let mut got = 0;
        for _ in 0..2 {
            let Some(c) = self.current.as_ref() else { break };
            got += c.player.audio().take(&mut out[got * 2..]);
            let sound_only = c.player.media() == Some(Media::Sound);
            if got >= n || !sound_only || !c.player.audio().drained() {
                break;
            }
            self.finish_current();
            if self.mode != Mode::Playing {
                break;
            }
        }
        out
    }

    pub(super) fn write(&mut self, picture: Arc<Vec<u8>>) -> std::io::Result<()> {
        let fps = self.canvas.fps.max(1) as u64;
        let pts = self.frame_no * 1_000_000_000 / fps;
        self.writer.write_video(pts, &picture, true)?;
        let sound = self.sound();
        self.fifo.extend(sound);
        let mut bytes = Vec::with_capacity(BUFFER * 4);
        while self.fifo.len() >= BUFFER {
            bytes.clear();
            bytes.extend(self.fifo.drain(..BUFFER).flat_map(f32::to_le_bytes));
            let pts = self.samples_out * 1_000_000_000 / RATE as u64;
            self.writer.write_audio(pts, &bytes)?;
            self.samples_out += (BUFFER / 2) as u64;
        }
        self.writer.flush()
    }

    pub(super) fn publish(&mut self) {
        let position = self.item_ns().saturating_sub(self.current.as_ref().map_or(0, |c| c.player.start_ns));
        let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        s.mode = self.mode;
        s.current = self.current.as_ref().map(|c| c.player.id.clone());
        s.position_ms = position / 1_000_000;
        s.length_ms = self.current.as_ref().and_then(|c| c.player.length_ns()).map(|n| n / 1_000_000);
        s.decoder = self.current.as_ref().and_then(|c| c.player.status().decoder);
        s.next = self.next.as_ref().map(|n| n.player.id.clone());
        s.next_ready = self.next.as_mut().is_some_and(|n| n.player.ready());
        s.cued = self.queue.cued().map(str::to_string);
        if s.order.len() != self.queue.order().len() || s.order.iter().zip(self.queue.order()).any(|(a, b)| a != b) {
            s.order = self.queue.order().to_vec();
        }
        if s.errors != self.errors {
            s.errors = self.errors.clone();
        }
        s.frames = self.frame_no + 1;
        s.repeats = self.repeats;
        s.changes = self.changes;
    }
}
