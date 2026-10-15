//! The output thread: one picture and its share of sound per tick of the
//! canvas clock, whatever the items are doing.
//!
//! This thread never waits on a file, a decoder or a lock held by anyone
//! else for long. Each tick it takes what the current item has ready; when
//! the item has nothing new it sends the previous picture again, and when it
//! has no sound it sends silence. Timestamps come from the tick count alone,
//! so they step by exactly one frame from the first tick to the last, across
//! every item change, pause and failure.

mod commands;
mod setup;
mod output;
mod switch;

use crate::item::ItemPlayer;
use crate::queue::Queue;
use crate::settings::Settings;
use crate::state::{Mode, SharedSnapshot};
use godwinmix_sdk::media::MediaWriter;
use godwinmix_sdk::prelude::Reporter;
use godwinmix_sdk::wire::Canvas;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use commands::Cmd;
pub use setup::Setup;

/// The item playing and how many ticks of it have played.
pub struct Current {
    pub player: ItemPlayer,
    pub ticks: u64,
}

/// An item opened ahead of time.
pub struct Prepared {
    pub player: ItemPlayer,
    pub wraps: bool,
}

pub struct Engine {
    canvas: Canvas,
    writer: Box<dyn MediaWriter>,
    reporter: Option<Reporter>,
    settings: Settings,
    queue: Queue,
    cmds: Receiver<Cmd>,
    snapshot: SharedSnapshot,
    software_only: bool,
    current: Option<Current>,
    next: Option<Prepared>,
    /// Something asked to play now (play item, previous, seek). It takes over
    /// from the current item the tick it has a picture.
    pending: Option<Prepared>,
    mode: Mode,
    last: Arc<Vec<u8>>,
    black: Arc<Vec<u8>>,
    frame_no: u64,
    samples_out: u64,
    fifo: std::collections::VecDeque<f32>,
    /// Ticks spent waiting for the next item at the end of the current one.
    waited: u32,
    failures_in_row: usize,
    /// Why items would not play, by id, until the list changes.
    errors: std::collections::HashMap<String, String>,
    changes: u64,
    repeats: u64,
    /// Every item failed in a row; the list is stopped until it changes.
    gave_up: bool,
}

impl Engine {
    /// Run until `stop` is set or the core stops reading.
    pub fn run(mut self, stop: Arc<AtomicBool>) {
        let start = Instant::now();
        let fps = self.canvas.fps.max(1) as u64;
        while !stop.load(Ordering::Relaxed) {
            let due = Duration::from_nanos(self.frame_no * 1_000_000_000 / fps);
            if let Some(wait) = due.checked_sub(start.elapsed()) {
                std::thread::sleep(wait);
            }
            if let Err(e) = self.tick() {
                self.stopped(e);
                return;
            }
        }
        let _ = self.writer.finish();
    }

    fn tick(&mut self) -> std::io::Result<()> {
        self.commands();
        self.settle();
        let picture = self.picture();
        self.write(picture)?;
        if self.mode == Mode::Playing {
            if let Some(c) = self.current.as_mut() {
                c.ticks += 1;
            }
        }
        self.prepare_next();
        self.publish();
        self.frame_no += 1;
        Ok(())
    }

    /// Where the current item is, in nanoseconds from the start of its file.
    fn item_ns(&self) -> u64 {
        let fps = self.canvas.fps.max(1) as u64;
        self.current.as_ref().map_or(0, |c| c.player.start_ns + c.ticks * 1_000_000_000 / fps)
    }

    fn stopped(&mut self, e: std::io::Error) {
        let why = format!("the core stopped reading the media: {e}");
        if let Some(r) = self.reporter.as_ref() {
            r.warn(&why);
        }
        self.snapshot.lock().unwrap_or_else(|e| e.into_inner()).stopped = Some(why);
    }

    fn log(&self, message: String) {
        if let Some(r) = self.reporter.as_ref() {
            r.info(message);
        }
    }

    fn warn(&self, message: String) {
        if let Some(r) = self.reporter.as_ref() {
            r.warn(message);
        }
    }
}
