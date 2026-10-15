//! One item of the list, playing or getting ready to.
//!
//! Opening an item never blocks the caller: the file is opened, read and
//! decoded on threads of the item's own, and what they produce waits in two
//! queues, pictures and sound, for the output thread to take at its own pace.
//! Whatever goes wrong is written to `status`, and the output thread skips
//! the item on its next tick.

pub mod audio_queue;
mod convert;
mod feed;
mod sound;
pub mod status;

use crate::media::kind::{sniff, Container};
use crate::media::video_gst::VideoDecoder;
use crate::settings::{Fit, Item, Settings};
use audio_queue::AudioQueue;
use godwinmix_sdk::wire::Canvas;
use status::Status;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};

/// A picture at canvas size, ready to send.
#[derive(Clone)]
pub struct Frame {
    pub pts: u64,
    pub data: Arc<Vec<u8>>,
}

/// Pictures drawn ahead of the output. Four frames at 1080p is 12 MB.
const AHEAD: usize = 4;

/// What the item's threads share.
pub struct Shared {
    pub canvas: Canvas,
    pub fit: Fit,
    pub start_ns: u64,
    pub out_ns: Option<u64>,
    pub software_only: bool,
    pub stop: AtomicBool,
    pub status: Status,
    pub audio: AudioQueue,
    pub frames_tx: SyncSender<Frame>,
    pub decoder: Mutex<Option<Arc<VideoDecoder>>>,
    /// Frames the output has finished with, to draw the next ones into.
    pub spare: Mutex<Vec<Vec<u8>>>,
}

impl Shared {
    pub fn take_buffer(&self) -> Vec<u8> {
        let size = self.canvas.i420_frame_bytes();
        self.spare.lock().unwrap_or_else(|e| e.into_inner()).pop().filter(|b| b.len() == size).unwrap_or_else(|| vec![0; size])
    }
}

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
        spawn(shared.clone(), PathBuf::from(&item.path));
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
            if f.pts > item_ns && (self.shown_any || chosen.is_some()) {
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

    /// Whether the item has run out at `item_ns`.
    pub fn ended(&mut self, item_ns: u64) -> bool {
        if self.out_ns.is_some_and(|o| item_ns >= o) {
            return true;
        }
        match self.media() {
            None => false,
            Some(Media::Picture) => item_ns >= self.start_ns + self.still_ns,
            Some(Media::Sound) => self.shared.audio.drained(),
            Some(Media::Video) => {
                let s = self.status();
                let last = s.last_pts.unwrap_or(0) + s.frame_ns.unwrap_or(0);
                s.video_done && self.peek().is_none() && item_ns + 1 >= last.max(self.start_ns)
            }
        }
    }

    /// How long the item runs from its start, when that is known.
    pub fn length_ns(&self) -> Option<u64> {
        let end = match self.media() {
            Some(Media::Picture) => Some(self.start_ns + self.still_ns),
            _ => self.status().duration_ns,
        };
        let end = match (end, self.out_ns) {
            (Some(e), Some(o)) => Some(e.min(o)),
            (e, o) => e.or(o),
        };
        end.map(|e| e.saturating_sub(self.start_ns))
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

/// Look at the file and read it, on a thread of its own: even opening a
/// file can stall on a network share, and the output thread must not.
fn spawn(shared: Arc<Shared>, path: PathBuf) {
    let _ = std::thread::Builder::new().name("playout-item".into()).spawn(move || {
        let media = match sniff(&path) {
            Ok(Container::Picture) => Media::Picture,
            Ok(Container::Audio) => Media::Sound,
            Ok(_) => Media::Video,
            Err(e) => {
                shared.status.fail(e);
                shared.audio.finish();
                return;
            }
        };
        shared.status.update(|s| s.media = Some(media));
        match media {
            Media::Video => feed::video(shared, path),
            Media::Sound => feed::audio(shared, path),
            Media::Picture => picture(shared, path),
        }
    });
}

fn picture(shared: Arc<Shared>, path: PathBuf) {
    let size = (shared.canvas.width, shared.canvas.height);
    match crate::media::still::load(&path, size, shared.fit) {
        Ok(data) => {
            shared.status.update(|s| s.has_video = Some(true));
            let _ = shared.frames_tx.send(Frame { pts: shared.start_ns, data: Arc::new(data) });
        }
        Err(e) => shared.status.fail(e),
    }
    shared.audio.finish();
}
