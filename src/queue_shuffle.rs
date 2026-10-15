//! Dealing a shuffle.

use super::Queue;

impl Queue {
    /// A Fisher-Yates shuffle, with `first` kept at the front.
    pub(super) fn shuffled(&mut self, mut ids: Vec<String>, first: Option<&str>) -> Vec<String> {
        for i in (1..ids.len()).rev() {
            let j = (self.next_random() % (i as u64 + 1)) as usize;
            ids.swap(i, j);
        }
        if let Some(f) = first.and_then(|f| ids.iter().position(|id| id == f)) {
            let id = ids.remove(f);
            ids.insert(0, id);
        }
        ids
    }

    pub(super) fn next_random(&mut self) -> u64 {
        // xorshift64: plenty for an order nobody will audit.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }
}
