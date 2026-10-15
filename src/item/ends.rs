//! When an item has run out, how long it runs, and letting it go.

use super::{ItemPlayer, Media, SLACK_NS};
use std::sync::atomic::Ordering;

impl ItemPlayer {
    /// Whether the item has run out at `item_ns`.
    pub fn ended(&mut self, item_ns: u64) -> bool {
        if self.out_ns.is_some_and(|o| item_ns >= o) {
            return true;
        }
        match self.media() {
            None => false,
            Some(Media::Picture) => item_ns >= self.in_ns + self.still_ns,
            Some(Media::Sound) => self.shared.audio.drained(),
            Some(Media::Video) => {
                let s = self.status();
                let last = s.last_pts.unwrap_or(0) + s.frame_ns.unwrap_or(0);
                s.video_done && item_ns + SLACK_NS >= last.max(self.start_ns)
            }
        }
    }

    /// How long the item runs from its start, when that is known.
    pub fn length_ns(&self) -> Option<u64> {
        let end = match self.media() {
            Some(Media::Picture) => Some(self.in_ns + self.still_ns),
            _ => self.status().duration_ns,
        };
        let end = match (end, self.out_ns) {
            (Some(e), Some(o)) => Some(e.min(o)),
            (e, o) => e.or(o),
        };
        end.map(|e| e.saturating_sub(self.in_ns))
    }
}

impl Drop for ItemPlayer {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        self.shared.audio.close();
        // Setting a pipeline to NULL can take a moment, and this runs on the
        // output thread: hand it to a thread of its own.
        let decoder = self.shared.decoder.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(d) = decoder {
            let _ = std::thread::Builder::new().name("playout-close".into()).spawn(move || d.close());
        }
    }
}
