//! The order things play in. No media here, only ids, so every rule about
//! what comes next is tested without a file or a clock.

use crate::settings::Repeat;
use std::collections::HashSet;

/// What `after` decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Next {
    pub id: String,
    /// True when this is the top of the list again, which is when a shuffle
    /// is dealt again and failed items get another chance.
    pub wraps: bool,
}

#[derive(Debug)]
pub struct Queue {
    /// Ids in the order this pass plays them: the list's order, or a shuffle.
    order: Vec<String>,
    /// The order for the pass after this one, made when a shuffled list is
    /// first asked what follows its last item, so the answer does not change
    /// between the asking and the playing.
    next_pass: Option<Vec<String>>,
    /// Where the current item is in `order`, kept when it is removed so the
    /// item that slid into its place plays next.
    pos: Option<usize>,
    cued: Option<String>,
    failed: HashSet<String>,
    shuffle: bool,
    rng: u64,
}

impl Queue {
    pub fn new(seed: u64) -> Queue {
        Queue { order: Vec::new(), next_pass: None, pos: None, cued: None, failed: HashSet::new(), shuffle: false, rng: seed | 1 }
    }

    /// What plays after `current`, without moving anything.
    pub fn after(&mut self, current: Option<&str>, repeat: Repeat) -> Option<Next> {
        if repeat == Repeat::One {
            if let Some(c) = current.filter(|c| self.playable(c)) {
                return Some(Next { id: c.to_string(), wraps: false });
            }
        }
        if let Some(c) = self.cued.clone().filter(|c| self.playable(c)) {
            return Some(Next { id: c, wraps: false });
        }
        let start = match (current, self.pos) {
            (Some(c), _) if self.order.iter().any(|id| id == c) => self.order.iter().position(|id| id == c).unwrap() + 1,
            // The current item was removed: whatever slid into its place.
            (Some(_), Some(p)) => p,
            // Nothing playing, after an item that broke: the one after it.
            (None, Some(p)) => p + 1,
            _ => 0,
        };
        if let Some(found) = self.order.iter().skip(start).find(|id| self.playable(id)) {
            return Some(Next { id: found.clone(), wraps: false });
        }
        if repeat == Repeat::Off || self.order.is_empty() {
            return None;
        }
        let next_pass = match self.next_pass.clone() {
            Some(p) => p,
            None => {
                let p = if self.shuffle { self.shuffled(self.order.clone(), None) } else { self.order.clone() };
                self.next_pass = Some(p.clone());
                p
            }
        };
        // Every item failed this pass is not a reason to stop: they are all
        // tried again on the next, once, and a list that fails twice in a row
        // stops being asked by the engine, which counts.
        next_pass.first().map(|id| Next { id: id.clone(), wraps: true })
    }

    /// The item before `current` in this pass, for "previous".
    pub fn before(&self, current: Option<&str>) -> Option<String> {
        let at = current.and_then(|c| self.order.iter().position(|id| id == c)).or(self.pos)?;
        self.order[..at.min(self.order.len())].iter().rev().find(|id| self.playable(id)).cloned()
    }

    /// `next` is now playing. `wraps` is what `after` said.
    pub fn advance(&mut self, id: &str, wraps: bool) {
        if wraps {
            if let Some(pass) = self.next_pass.take() {
                self.order = pass;
            }
            self.failed.clear();
        }
        self.pos = self.order.iter().position(|o| o == id);
        if self.cued.as_deref() == Some(id) {
            self.cued = None;
        }
    }

    /// Back to the top, for play after the list ran out.
    pub fn reset(&mut self) {
        self.pos = None;
        self.failed.clear();
    }

    pub fn cue(&mut self, id: Option<String>) {
        self.cued = id;
    }

    pub fn cued(&self) -> Option<&str> {
        self.cued.as_deref()
    }

    pub fn mark_failed(&mut self, id: &str) {
        self.failed.insert(id.to_string());
    }

    #[cfg(test)]
    pub fn is_failed(&self, id: &str) -> bool {
        self.failed.contains(id)
    }

    /// Ids in the order they play from here, for the panel's "up next".
    pub fn order(&self) -> &[String] {
        &self.order
    }

    fn playable(&self, id: &str) -> bool {
        !self.failed.contains(id) && self.order.iter().any(|o| o == id)
    }
}

#[path = "queue_shuffle.rs"]
mod shuffle;
#[path = "queue_update.rs"]
mod update;

#[cfg(test)]
#[path = "queue_tests.rs"]
mod tests;
