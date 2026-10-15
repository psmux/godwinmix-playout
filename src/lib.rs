//! godwinmix-playout: a playlist of video, sound and picture files played
//! back to back as one GodwinMix source, with no gap between them.
//!
//! Rust does the playlist, the timing, the containers (MP4, Matroska), the
//! sound (symphonia), the pictures (image), the scaling (fast_image_resize)
//! and the protocol (godwinmix-sdk). GStreamer decodes video, through the
//! platform's own decoders and never FFmpeg. README.md says why.

pub mod catalog;
pub mod choices;
pub mod engine;
pub mod gst_runtime;
pub mod item;
pub mod media;
pub mod queue;
pub mod report;
pub mod settings;
pub mod source;
pub mod state;
pub mod thumbs;
pub mod tools;

