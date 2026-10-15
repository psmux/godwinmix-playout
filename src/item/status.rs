//! What an item's threads tell the output thread, without either waiting on
//! the other for longer than a lock held for a few field writes.

use std::sync::Mutex;

#[derive(Debug, Default, Clone)]
pub struct Seen {
    /// Why it stopped. Set once; the item is skipped.
    pub error: Option<String>,
    /// The file's own length, once the demuxer has read it.
    pub duration_ns: Option<u64>,
    /// The decoder has given its last frame.
    pub video_done: bool,
    /// The last frame's time and how long a frame of this file lasts.
    pub last_pts: Option<u64>,
    pub frame_ns: Option<u64>,
    /// Whether the file has a picture at all, once known.
    pub has_video: Option<bool>,
    pub media: Option<super::Media>,
    pub decoder: Option<String>,
}

#[derive(Default)]
pub struct Status(Mutex<Seen>);

impl Status {
    pub fn get(&self) -> Seen {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut Seen)) {
        f(&mut self.0.lock().unwrap_or_else(|e| e.into_inner()));
    }

    pub fn fail(&self, why: impl Into<String>) {
        self.update(|s| {
            if s.error.is_none() {
                s.error = Some(why.into());
            }
        });
    }

    pub fn error(&self) -> Option<String> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).error.clone()
    }
}
