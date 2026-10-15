//! godwinmix-playout: a playlist of video, sound and picture files played
//! back to back as one GodwinMix source, with no gap between them.
//!
//! Rust does the playlist, the timing, the containers (MP4, Matroska), the
//! sound (symphonia), the pictures (image), the scaling (fast_image_resize)
//! and the protocol (godwinmix-sdk). GStreamer decodes video, through the
//! platform's own decoders and never FFmpeg. README.md says why.

mod catalog;
mod engine;
mod gst_runtime;
mod item;
mod media;
mod queue;
mod report;
mod settings;
mod source;
mod state;
mod thumbs;
mod tools;

use godwinmix_sdk::prelude::*;

fn main() {
    // First, before any thread: on Windows this may put GStreamer on PATH.
    gst_runtime::locate();
    let env = PluginEnv::from_env();
    let manifest = match Manifest::load(env.root.join("gmx-plugin.toml")) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("playout could not read its manifest: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = runtime::run(&manifest, SourceHandler(source::Playout::new())) {
        eprintln!("playout stopped: {e}");
        std::process::exit(1);
    }
}
