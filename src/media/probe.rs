//! What the panel shows about a file before it plays: its kind, length,
//! size and codec, or why it will not play. Headers only; nothing decoded.

use super::kind::{sniff, Container};
use super::{audio_file::AudioFile, open_demux, video_gst};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Probe {
    /// `video`, `audio` or `picture`.
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    /// The decoder that will be used, for a video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoder: Option<String>,
    /// Why it will not play. The item is skipped, not the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn probe(path: &Path) -> Probe {
    match sniff(path) {
        Err(e) => failed("", e),
        Ok(Container::Picture) => picture(path),
        Ok(Container::Audio) => audio(path),
        Ok(Container::Mp4 | Container::Matroska) => video(path),
        Ok(Container::Unknown) => failed("", format!("{} is not a video, sound or picture file this plugin reads", path.display())),
    }
}

fn failed(kind: &str, error: String) -> Probe {
    Probe { kind: kind.into(), error: Some(error), ..Default::default() }
}

fn picture(path: &Path) -> Probe {
    match image::image_dimensions(path) {
        Ok((w, h)) => Probe { kind: "picture".into(), width: Some(w), height: Some(h), ..Default::default() },
        Err(e) => failed("picture", format!("{} does not read as a picture: {e}", path.display())),
    }
}

fn audio(path: &Path) -> Probe {
    match AudioFile::open(path) {
        Ok(a) => Probe { kind: "audio".into(), duration_ms: a.duration_ns.map(|n| n / 1_000_000), ..Default::default() },
        Err(e) => failed("audio", e),
    }
}

fn video(path: &Path) -> Probe {
    let demux = match open_demux(path) {
        Ok(d) => d,
        Err(e) => return failed("video", e),
    };
    let info = demux.info();
    let duration_ms = info.duration_ns.map(|n| n / 1_000_000);
    let Some(v) = info.video.as_ref() else {
        return Probe { kind: "audio".into(), duration_ms, ..Default::default() };
    };
    let mut p = Probe {
        kind: "video".into(),
        duration_ms,
        width: Some(v.width).filter(|w| *w > 0),
        height: Some(v.height).filter(|h| *h > 0),
        codec: Some(v.codec.name().into()),
        ..Default::default()
    };
    match video_gst::choose(v.codec, false) {
        Ok(d) => p.decoder = Some(d.into()),
        Err(e) => p.error = Some(e),
    }
    p
}
