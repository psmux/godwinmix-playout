//! What the output thread says about itself, for the `state` tool and for
//! `health`. Written once a tick under a lock held for a few field copies.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Nothing started yet: an empty list, or autoplay off.
    #[default]
    Idle,
    Playing,
    Paused,
    /// Stopped on an item marked `hold`, on its last frame.
    Holding,
    /// A list that does not loop has run out.
    Ended,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Snapshot {
    pub mode: Mode,
    /// The item on air, if any.
    pub current: Option<String>,
    /// Milliseconds into the current item, from its in point.
    pub position_ms: u64,
    /// How long the current item runs, from its in point, once known.
    pub length_ms: Option<u64>,
    /// What plays next, when it is decided.
    pub next: Option<String>,
    /// Whether `next` has its first picture ready.
    pub next_ready: bool,
    pub cued: Option<String>,
    /// Ids in the order this pass plays them.
    pub order: Vec<String>,
    /// Why an item would not play, by id.
    pub errors: HashMap<String, String>,
    /// The decoder playing the current item.
    pub decoder: Option<String>,
    /// Frames sent, and how many were the previous picture again because the
    /// decoder had nothing new in time.
    pub frames: u64,
    pub repeats: u64,
    /// Counts up by one every time the current item changes, so a panel can
    /// tell an item change from the same item playing on.
    pub changes: u64,
    /// The writer failed: the core went away.
    pub stopped: Option<String>,
}

pub type SharedSnapshot = Arc<Mutex<Snapshot>>;

pub fn read(s: &SharedSnapshot) -> Snapshot {
    s.lock().unwrap_or_else(|e| e.into_inner()).clone()
}
