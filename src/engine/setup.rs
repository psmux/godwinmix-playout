//! Making the output thread's state from what `start` was given.

use super::{Cmd, Engine};
use crate::queue::Queue;
use crate::settings::Settings;
use crate::state::{Mode, SharedSnapshot};
use godwinmix_sdk::media::MediaWriter;
use godwinmix_sdk::prelude::Reporter;
use godwinmix_sdk::wire::Canvas;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

pub struct Setup {
    pub canvas: Canvas,
    pub writer: Box<dyn MediaWriter>,
    pub reporter: Option<Reporter>,
    pub settings: Settings,
    pub cmds: Receiver<Cmd>,
    pub snapshot: SharedSnapshot,
    pub software_only: bool,
}

impl Engine {
    pub fn new(s: Setup) -> Engine {
        let black = Arc::new(crate::media::still::black(s.canvas.width, s.canvas.height));
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
        let mut queue = Queue::new(seed);
        queue.update(&s.settings.items, s.settings.shuffle, None);
        let mode = if s.settings.autoplay && !s.settings.items.is_empty() { Mode::Playing } else { Mode::Idle };
        Engine {
            canvas: s.canvas,
            writer: s.writer,
            reporter: s.reporter,
            settings: s.settings,
            queue,
            cmds: s.cmds,
            snapshot: s.snapshot,
            software_only: s.software_only,
            current: None,
            next: None,
            pending: None,
            mode,
            last: black.clone(),
            black,
            frame_no: 0,
            samples_out: 0,
            fifo: Default::default(),
            waited: 0,
            failures_in_row: 0,
            errors: Default::default(),
            changes: 0,
            repeats: 0,
            gave_up: false,
        }
    }
}
