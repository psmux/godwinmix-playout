//! A sound sample entry: which codec, its setup, its rate and channels.

use super::boxes::{child, esds, full, Be};
use crate::media::{AudioCodec, AudioTrack};

pub fn audio_entry(kind: [u8; 4], body: &[u8]) -> AudioTrack {
    let mut r = Be::new(body);
    r.skip(8);
    let version = r.u16();
    r.skip(6);
    let channels = r.u16() as u32;
    r.skip(6);
    let rate = r.u32() >> 16;
    let skip = match version {
        1 => 28 + 16,
        2 => 28 + 36,
        _ => 28,
    };
    let boxes = body.get(skip..).unwrap_or(&[]);
    let (codec, config) = match &kind {
        b"mp4a" => match child(boxes, b"esds").and_then(esds) {
            Some((0x40 | 0x66 | 0x67 | 0x68, dsi)) => (AudioCodec::Aac, Some(dsi)),
            Some((0x69 | 0x6b, _)) => (AudioCodec::Mp3, None),
            Some((other, _)) => (AudioCodec::Other(format!("MPEG-4 audio type {other:#x}")), None),
            None => (AudioCodec::Other("mp4a with no esds".into()), None),
        },
        b".mp3" => (AudioCodec::Mp3, None),
        b"Opus" => (AudioCodec::Opus, child(boxes, b"dOps").map(opus_head)),
        b"fLaC" => (AudioCodec::Flac, child(boxes, b"dfLa").and_then(|d| full(d).1.get(4..).map(<[u8]>::to_vec))),
        other => (AudioCodec::Other(String::from_utf8_lossy(other).into_owned()), None),
    };
    AudioTrack { codec, config, rate: rate.max(1), channels: channels.max(1) }
}

/// An Opus `dOps` box, big endian, as the little endian `OpusHead` packet
/// a decoder reads.
fn opus_head(dops: &[u8]) -> Vec<u8> {
    let mut r = Be::new(dops);
    r.skip(1);
    let channels = r.u8();
    let pre_skip = r.u16();
    let rate = r.u32();
    let gain = r.u16();
    let family = r.u8();
    let mut head = b"OpusHead".to_vec();
    head.extend_from_slice(&[1, channels]);
    head.extend_from_slice(&pre_skip.to_le_bytes());
    head.extend_from_slice(&rate.to_le_bytes());
    head.extend_from_slice(&gain.to_le_bytes());
    head.push(family);
    head.extend_from_slice(r.bytes(r.left()));
    head
}
