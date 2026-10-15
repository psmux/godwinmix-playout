//! Moving from one item to the next: opening it early, taking it over the
//! tick it has a picture, and skipping it when it fails.

use super::{Current, Engine, Prepared};
use crate::item::ItemPlayer;
use crate::settings::AtEnd;
use crate::state::Mode;

/// Open the next item this long before the current one ends. Long enough for
/// any decoder to have its first frames out; short enough that two decoders
/// run together for only a few seconds.
const PREROLL_NS: u64 = 5_000_000_000;

/// How long the end of an item waits for the next one before giving up on it.
const PATIENCE_TICKS: u32 = 150;

impl Engine {
    /// Before the picture is chosen: take over from a pending item that is
    /// ready, and start the list if it should be playing and is not.
    pub(super) fn settle(&mut self) {
        if let Some(mut p) = self.pending.take() {
            if let Some(why) = p.player.error() {
                self.failed(&p.player.id, why);
            } else if p.player.ready() {
                self.become_current(p);
            } else {
                self.pending = Some(p);
            }
        }
        if self.mode == Mode::Playing && self.current.is_none() && self.pending.is_none() {
            match self.queue.after(None, self.settings.repeat) {
                Some(n) => self.play_now(&n.id, 0, n.wraps),
                None => self.mode = if self.settings.items.is_empty() { Mode::Idle } else { Mode::Ended },
            }
        }
    }

    /// The current item has run out. Hold on it, or move on.
    pub(super) fn finish_current(&mut self) {
        if self.current.as_ref().is_some_and(|c| c.player.hold) && self.mode == Mode::Playing {
            self.mode = Mode::Holding;
            return;
        }
        self.go_next();
    }

    pub(super) fn go_next(&mut self) {
        if self.next.is_none() {
            self.prepare();
        }
        let Some(mut n) = self.next.take() else {
            // Nothing follows: the list is over.
            self.mode = Mode::Ended;
            if self.settings.at_end == AtEnd::Black {
                self.last = self.black.clone();
            }
            self.current = None;
            return;
        };
        if let Some(why) = n.player.error() {
            self.failed(&n.player.id, why);
            return;
        }
        if n.player.ready() || self.waited >= PATIENCE_TICKS {
            self.waited = 0;
            self.become_current(n);
        } else {
            self.waited += 1;
            self.next = Some(n);
        }
    }

    fn become_current(&mut self, p: Prepared) {
        self.queue.advance(&p.player.id, p.wraps);
        let queued_ms = p.player.audio().frames() as u64 * 1000 / 48_000;
        let left = self.current.as_ref().map(|c| (c.ticks, c.player.audio().frames(), c.player.status().video_done));
        self.log(format!("now playing {} ({queued_ms} ms of sound ready; the item before ended after {left:?} ticks, frames of sound left, done)", self.title_of(&p.player.id)));
        self.current = Some(Current { player: p.player, ticks: 0 });
        self.failures_in_row = 0;
        self.changes += 1;
        self.drop_stale_next();
    }

    /// An item would not play: say why once, mark it, and carry on.
    pub(super) fn failed(&mut self, id: &str, why: String) {
        self.warn(format!("skipped {}: {why}", self.title_of(id)));
        self.queue.mark_failed(id);
        self.errors.insert(id.to_string(), why);
        self.failures_in_row += 1;
        if self.failures_in_row > self.settings.items.len().max(1) {
            self.warn("nothing in this playlist will play; it is stopped until the list changes".into());
            self.mode = Mode::Ended;
            self.current = None;
            self.gave_up = true;
        }
    }

    /// Open the next item when the current one is close to its end.
    pub(super) fn prepare_next(&mut self) {
        if let Some(why) = self.next.as_ref().and_then(|n| n.player.error()) {
            let id = self.next.take().map(|n| n.player.id.clone()).unwrap_or_default();
            self.failed(&id, why);
        }
        if self.next.is_some() || self.current.is_none() || !matches!(self.mode, Mode::Playing | Mode::Paused) {
            return;
        }
        let near_end = self.current.as_ref().is_some_and(|c| {
            let played = self.item_ns().saturating_sub(c.player.in_ns);
            c.player.length_ns().is_none_or(|len| len.saturating_sub(played) <= PREROLL_NS)
        });
        if near_end {
            self.prepare();
        }
    }

    fn prepare(&mut self) {
        let current = self.current_id();
        let Some(n) = self.queue.after(current.as_deref(), self.settings.repeat) else { return };
        let Some(item) = self.settings.items.iter().find(|i| i.id == n.id).cloned() else { return };
        let player = ItemPlayer::open(&item, &self.settings, self.canvas, item.in_ms * 1_000_000, self.software_only);
        self.next = Some(Prepared { player, wraps: n.wraps });
    }

    /// The list, the cue or the order changed: an item opened as next that is
    /// no longer next is closed.
    pub(super) fn drop_stale_next(&mut self) {
        let Some(n) = self.next.as_ref() else { return };
        let current = self.current_id();
        let wanted = self.queue.after(current.as_deref(), self.settings.repeat).map(|n| n.id);
        let item_changed = !self.settings.items.iter().any(|i| i.id == n.player.id);
        if wanted.as_deref() != Some(n.player.id.as_str()) || item_changed {
            self.next = None;
        }
    }

    pub(super) fn title_of(&self, id: &str) -> String {
        crate::tools::title(&self.settings, id)
    }
}
