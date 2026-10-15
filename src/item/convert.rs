//! The thread that takes decoded frames and draws them at canvas size.
//!
//! It looks one frame ahead, so it knows how long each frame is on screen
//! for, and only draws the frames the output will show. A 60 fps file in a
//! 30 fps show has every other frame dropped here, before any scaling is
//! paid for.

use super::{Frame, Shared};
use crate::media::frame;
use crate::media::picture::Scaler;
use crate::media::video_gst::{Pulled, VideoDecoder};
use gstreamer as gst;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Converter {
    shared: Arc<Shared>,
    decoder: Arc<VideoDecoder>,
    scaler: Scaler,
    held: Option<(u64, gst::Sample)>,
    last: Option<u64>,
    step: Option<u64>,
}

impl Converter {
    pub fn spawn(shared: Arc<Shared>, decoder: Arc<VideoDecoder>) {
        let scaler = Scaler::new(shared.canvas.width, shared.canvas.height, shared.fit);
        let mut me = Converter { shared, decoder, scaler, held: None, last: None, step: None };
        let _ = std::thread::Builder::new().name("playout-convert".into()).spawn(move || me.run());
    }

    fn run(&mut self) {
        while !self.shared.stop.load(Ordering::Relaxed) {
            match self.decoder.pull(100) {
                Pulled::Wait => {}
                Pulled::Frame(sample) => {
                    let Some(pts) = frame::pts(&sample) else { continue };
                    if let Some((held_pts, held)) = self.held.take() {
                        if self.shown(held_pts, pts) && !self.send(held_pts, &held) {
                            return;
                        }
                    }
                    if let Some(last) = self.last.filter(|l| pts > *l) {
                        self.step = Some(pts - last);
                    }
                    self.last = Some(pts);
                    self.held = Some((pts, sample));
                }
                Pulled::End => break,
                Pulled::Failed(why) => {
                    self.shared.status.fail(why);
                    return;
                }
            }
        }
        // The last frame lasts as long as the one before it did.
        if let Some((pts, held)) = self.held.take() {
            if self.shown(pts, pts + self.step.unwrap_or(1_000_000_000)) {
                self.send(pts, &held);
            }
        }
        if self.last.is_none() && !self.shared.stop.load(Ordering::Relaxed) {
            self.shared.status.fail(format!(
                "{} gave no pictures for this file. Its profile is probably one that decoder does not take (4:4:4 or 10 bit H.264 are the usual ones); convert it to ordinary 8 bit 4:2:0",
                self.decoder.element
            ));
            return;
        }
        let (last, step) = (self.last, self.step);
        self.shared.status.update(|s| {
            s.video_done = true;
            s.last_pts = last;
            s.frame_ns = step;
        });
    }

    /// Whether a frame at `pts`, replaced at `next`, is ever on screen: some
    /// output tick lands in between. Ticks are the item's start plus whole
    /// frames of the canvas rate.
    fn shown(&self, pts: u64, next: u64) -> bool {
        let start = self.shared.start_ns;
        let (pts, next) = (pts.saturating_sub(super::SLACK_NS), next.saturating_sub(super::SLACK_NS));
        if next <= start {
            return false;
        }
        let fps = self.shared.canvas.fps.max(1) as u64;
        let from = pts.max(start) - start;
        let k = (from * fps).div_ceil(1_000_000_000);
        let tick = start + k * 1_000_000_000 / fps;
        tick < next
    }

    /// Draw and hand over. False when the output has gone.
    fn send(&mut self, pts: u64, sample: &gst::Sample) -> bool {
        let mut data = self.shared.take_buffer();
        if let Err(e) = frame::draw(sample, &mut self.scaler, &mut data) {
            self.shared.status.fail(e);
            return false;
        }
        self.shared.frames_tx.send(Frame { pts, data: Arc::new(data) }).is_ok()
    }
}
