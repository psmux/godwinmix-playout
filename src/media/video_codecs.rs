//! For each video codec: the decoders to look for, the parser in front of
//! them, and the caps that describe the compressed stream.

use super::{VideoCodec, VideoTrack};
use gstreamer as gst;

/// Hardware first, then software. Each name exists on at most a platform or
/// two; whatever this machine has is used in this order.
pub fn decoders(codec: VideoCodec, software_only: bool) -> &'static [&'static str] {
    let all: &'static [&'static str] = match codec {
        VideoCodec::H264 => &["d3d11h264dec", "d3d12h264dec", "vtdec_hw", "vah264dec", "v4l2slh264dec", "v4l2h264dec", "openh264dec"],
        VideoCodec::H265 => &["d3d11h265dec", "d3d12h265dec", "vtdec_hw", "vah265dec", "v4l2slh265dec", "v4l2h265dec", "libde265dec"],
        VideoCodec::Av1 => &["d3d11av1dec", "d3d12av1dec", "vaav1dec", "dav1ddec"],
        VideoCodec::Vp9 => &["d3d11vp9dec", "d3d12vp9dec", "vavp9dec", "vp9dec"],
        VideoCodec::Vp8 => &["d3d11vp8dec", "vavp8dec", "vp8dec"],
    };
    if software_only {
        &all[all.len() - 1..]
    } else {
        all
    }
}

pub fn parser(codec: VideoCodec) -> Option<&'static str> {
    match codec {
        VideoCodec::H264 => Some("h264parse"),
        VideoCodec::H265 => Some("h265parse"),
        VideoCodec::Av1 => Some("av1parse"),
        VideoCodec::Vp9 => Some("vp9parse"),
        VideoCodec::Vp8 => None,
    }
}

pub fn caps(track: &VideoTrack) -> gst::Caps {
    let mut b = match track.codec {
        VideoCodec::H264 => {
            let format = if track.config.is_some() { "avc" } else { "avc3" };
            gst::Caps::builder("video/x-h264").field("stream-format", format).field("alignment", "au")
        }
        VideoCodec::H265 => gst::Caps::builder("video/x-h265").field("stream-format", if track.config.is_some() { "hvc1" } else { "hev1" }).field("alignment", "au"),
        VideoCodec::Av1 => gst::Caps::builder("video/x-av1").field("stream-format", "obu-stream").field("alignment", "tu"),
        VideoCodec::Vp9 => gst::Caps::builder("video/x-vp9"),
        VideoCodec::Vp8 => gst::Caps::builder("video/x-vp8"),
    };
    if track.width > 0 && track.height > 0 {
        b = b.field("width", track.width as i32).field("height", track.height as i32);
    }
    if let Some(config) = track.config.clone() {
        b = b.field("codec_data", gst::Buffer::from_mut_slice(config));
    }
    b.build()
}
