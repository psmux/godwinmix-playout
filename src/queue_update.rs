//! Taking a changed list or a changed shuffle without losing the place.

use super::Queue;
use crate::settings::Item;

impl Queue {
    /// The list or the shuffle changed. The current item keeps its place.
    pub fn update(&mut self, items: &[Item], shuffle: bool, current: Option<&str>) {
        let ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
        let was_shuffled = self.shuffle;
        self.shuffle = shuffle;
        self.next_pass = None;
        self.failed.retain(|id| ids.contains(id));
        if self.cued.as_ref().is_some_and(|c| !ids.contains(c)) {
            self.cued = None;
        }
        self.order = if !shuffle {
            ids
        } else if was_shuffled {
            // Keep the shuffle that is playing, drop what went, and put what
            // arrived somewhere after the current item.
            let mut kept: Vec<String> = self.order.iter().filter(|id| ids.contains(id)).cloned().collect();
            let arrived: Vec<String> = ids.iter().filter(|id| !kept.contains(id)).cloned().collect();
            for id in arrived {
                let after = current.and_then(|c| kept.iter().position(|k| k == c)).map_or(0, |p| p + 1);
                let at = after + (self.next_random() as usize) % (kept.len() - after + 1);
                kept.insert(at, id);
            }
            kept
        } else {
            self.shuffled(ids, current)
        };
        if let Some(c) = current {
            if let Some(p) = self.order.iter().position(|id| id == c) {
                self.pos = Some(p);
            }
        }
        if let Some(p) = self.pos {
            self.pos = Some(p.min(self.order.len()));
        }
    }
}
