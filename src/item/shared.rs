//! What an item's threads share with each other and with the output.

use super::audio_queue::AudioQueue;
use super::status::Status;
use crate::media::video_gst::VideoDecoder;
use crate::settings::Fit;
use godwinmix_sdk::wire::Canvas;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

/// A picture at canvas size, ready to send.
#[derive(Clone)]
pub struct Frame {
    pub pts: u64,
    pub data: Arc<Vec<u8>>,
}

/// How early a frame may be and still count as due. Matroska keeps times to
/// the millisecond, so a 30 fps frame due at 66.667 ms is stamped 67 ms; with
/// no slack it would miss its tick, go out a tick late, and the frame after it
/// would be skipped. One millisecond is far below anything a viewer sees and
/// above any container's rounding.
pub const SLACK_NS: u64 = 1_000_000;

/// Pictures drawn ahead of the output. Four frames at 1080p is 12 MB.
pub const AHEAD: usize = 4;

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

