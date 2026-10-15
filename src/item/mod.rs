//! One item of the list, playing or getting ready to.
//!
//! Opening an item never blocks the caller: the file is opened, read and
//! decoded on threads of the item's own, and what they produce waits in two
//! queues, pictures and sound, for the output thread to take at its own pace.
//! Whatever goes wrong is written to `status`, and the output thread skips
//! the item on its next tick.

pub mod audio_queue;
mod convert;
mod ends;
mod feed;
mod open;
mod shared;
mod sound;
pub mod status;

use crate::settings::{Item, Settings};
use audio_queue::AudioQueue;
use godwinmix_sdk::wire::Canvas;
use status::Status;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{sync_channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

pub use shared::{Frame, Shared, AHEAD, SLACK_NS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Media {
    Video,
    Sound,
    Picture,
}

pub struct ItemPlayer {
    pub id: String,
    pub start_ns: u64,
    pub out_ns: Option<u64>,
    /// How long a picture stays up.
    pub still_ns: u64,
    pub hold: bool,
    shared: Arc<Shared>,
    frames: Receiver<Frame>,
    ahead: Option<Frame>,
    shown_any: bool,
}

impl ItemPlayer {
    /// Start getting `item` ready, from `start_ns` into it.
    pub fn open(item: &Item, settings: &Settings, canvas: Canvas, start_ns: u64, software_only: bool) -> ItemPlayer {
        let (frames_tx, frames) = sync_channel(AHEAD);
        let out_ns = (item.out_ms > 0).then_some(item.out_ms * 1_000_000);
        let shared = Arc::new(Shared {
            canvas,
            fit: settings.fit,
            start_ns,
            out_ns,
            software_only,
            stop: AtomicBool::new(false),
            status: Status::default(),
            audio: AudioQueue::new(start_ns),
            frames_tx,
            decoder: Mutex::new(None),
            spare: Mutex::new(Vec::new()),
        });
        open::spawn(shared.clone(), PathBuf::from(&item.path));
        ItemPlayer {
            id: item.id.clone(),
            start_ns,
            out_ns,
            still_ns: settings.still_ns(item),
            hold: item.hold,
            shared,
            frames,
            ahead: None,
            shown_any: false,
        }
    }

    pub fn error(&self) -> Option<String> {
        self.shared.status.error()
    }

    pub fn status(&self) -> status::Seen {
        self.shared.status.get()
    }

    pub fn audio(&self) -> &AudioQueue {
        &self.shared.audio
    }

    /// What the file turned out to be, once its thread has looked.
    pub fn media(&self) -> Option<Media> {
        let s = self.status();
        match (s.media, s.has_video) {
            (Some(Media::Video), Some(false)) => Some(Media::Sound),
            (m, _) => m,
        }
    }

    /// Something to show, or a reason to give up, has arrived.
    pub fn ready(&mut self) -> bool {
        if self.error().is_some() {
            return true;
        }
        match self.media() {
            None => false,
            Some(Media::Sound) => self.shared.audio.frames() > 0 || self.shared.audio.is_done(),
            Some(_) => self.peek().is_some(),
        }
    }

    fn peek(&mut self) -> Option<&Frame> {
        if self.ahead.is_none() {
            match self.frames.try_recv() {
                Ok(f) => self.ahead = Some(f),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
            }
        }
        self.ahead.as_ref()
    }

    /// The newest frame due at `item_ns`, when it differs from the last one
    /// given. The first frame of an item is given whatever its time, so an
    /// item never opens on the previous item's last picture.
    pub fn frame_at(&mut self, item_ns: u64) -> Option<Frame> {
        let mut chosen = None;
        while let Some(f) = self.peek() {
            if f.pts > item_ns + SLACK_NS && (self.shown_any || chosen.is_some()) {
                break;
            }
            if let Some(skipped) = std::mem::replace(&mut chosen, self.ahead.take()) {
                self.recycle(skipped);
            }
            self.shown_any = true;
        }
        chosen
    }

    /// Give a frame's memory back for the next one.
    pub fn recycle(&self, frame: Frame) {
        if let Ok(data) = Arc::try_unwrap(frame.data) {
            let mut spare = self.shared.spare.lock().unwrap_or_else(|e| e.into_inner());
            if spare.len() < 2 {
                spare.push(data);
            }
        }
    }
}
