//! The thread an item starts on: what the file is, then reading it.

use super::{feed, Frame, Media, Shared};
use crate::media::kind::{sniff, Container};
use std::path::PathBuf;
use std::sync::Arc;

/// Look at the file and read it, on a thread of its own: even opening a
/// file can stall on a network share, and the output thread must not.
pub fn spawn(shared: Arc<Shared>, path: PathBuf) {
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
