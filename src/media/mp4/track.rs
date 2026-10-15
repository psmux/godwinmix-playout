//! One `trak`: what it carries, its clock, its edit list and its samples.

use super::boxes::{child, children, full, path, Be};
use super::table::{samples, Sample};
use crate::media::{AudioTrack, VideoCodec, VideoTrack};

pub enum Carries {
    Video(VideoTrack),
    Audio(AudioTrack),
}

pub struct Track {
    pub carries: Carries,
    pub timescale: u32,
    pub samples: Vec<Sample>,
    /// The edit list's start, in track ticks: subtracted from every time.
    pub shift: i64,
    /// An empty edit at the start, in nanoseconds: added to every time.
    pub delay_ns: u64,
}

impl Track {
    pub fn ns(&self, ticks: i64) -> u64 {
        self.signed_ns(ticks).max(0) as u64
    }
    /// Before zero for what the edit list cuts off the front.
    pub fn signed_ns(&self, ticks: i64) -> i64 {
        let t = (ticks - self.shift) as i128;
        (t * 1_000_000_000 / self.timescale.max(1) as i128) as i64 + self.delay_ns as i64
    }
    pub fn pts_ns(&self, s: &Sample) -> u64 {
        self.ns(s.dts as i64 + s.cto)
    }
    pub fn dts_ns(&self, s: &Sample) -> u64 {
        self.ns(s.dts as i64)
    }
    pub fn end_ns(&self) -> u64 {
        self.samples.iter().map(|s| self.pts_ns(s)).max().unwrap_or(0)
    }
}

/// Read a track, or `Ok(None)` for one this plugin has no use for (text,
/// timecode, a second video). An error is a track it should play and cannot.
pub fn read(trak: &[u8], movie_timescale: u32) -> Result<Option<Track>, String> {
    let Some(mdia) = child(trak, b"mdia") else { return Ok(None) };
    let handler = child(mdia, b"hdlr").map(|h| full(h).1.get(4..8).unwrap_or(&[]).to_vec()).unwrap_or_default();
    let (timescale, _) = media_header(mdia);
    let stbl = path(mdia, &[b"minf", b"stbl"]).ok_or("a track has no sample table")?;
    let Some(entry) = child(stbl, b"stsd").and_then(|d| children(full(d).1.get(4..)?).into_iter().next()) else {
        return Ok(None);
    };
    let carries = match handler.as_slice() {
        b"vide" => Carries::Video(video_entry(entry.kind, entry.body)?),
        b"soun" => Carries::Audio(super::sound_entry::audio_entry(entry.kind, entry.body)),
        _ => return Ok(None),
    };
    let (shift, delay_ns) = edits(trak, movie_timescale);
    Ok(Some(Track { carries, timescale, samples: samples(stbl)?, shift, delay_ns }))
}

fn media_header(mdia: &[u8]) -> (u32, u64) {
    let Some(mdhd) = child(mdia, b"mdhd") else { return (1000, 0) };
    let (version, body) = full(mdhd);
    let mut r = Be::new(body);
    if version == 1 {
        r.skip(16);
        (r.u32(), r.u64())
    } else {
        r.skip(8);
        (r.u32(), r.u32() as u64)
    }
}

fn video_entry(kind: [u8; 4], body: &[u8]) -> Result<VideoTrack, String> {
    let mut r = Be::new(body);
    r.skip(24);
    let (width, height) = (r.u16() as u32, r.u16() as u32);
    let boxes = body.get(78..).unwrap_or(&[]);
    let (codec, config) = match &kind {
        b"avc1" | b"avc3" => (VideoCodec::H264, child(boxes, b"avcC")),
        b"hvc1" | b"hev1" => (VideoCodec::H265, child(boxes, b"hvcC")),
        b"av01" => (VideoCodec::Av1, child(boxes, b"av1C")),
        b"vp09" => (VideoCodec::Vp9, None),
        b"vp08" => (VideoCodec::Vp8, None),
        other => {
            return Err(format!(
                "its video is '{}', which this plugin does not decode. H.264, HEVC, AV1, VP9 and VP8 play",
                String::from_utf8_lossy(other)
            ))
        }
    };
    Ok(VideoTrack { codec, config: config.map(<[u8]>::to_vec), width, height })
}

/// The edit list's media start and any empty edit before it.
fn edits(trak: &[u8], movie_timescale: u32) -> (i64, u64) {
    let Some(elst) = path(trak, &[b"edts", b"elst"]) else { return (0, 0) };
    let (version, body) = full(elst);
    let mut r = Be::new(body);
    let mut delay = 0u64;
    for _ in 0..r.u32() {
        let (duration, media_time) = if version == 1 { (r.u64(), r.u64() as i64) } else { (r.u32() as u64, r.u32() as i32 as i64) };
        r.skip(4);
        if media_time == -1 {
            delay += duration;
            continue;
        }
        let delay_ns = (delay as u128 * 1_000_000_000 / movie_timescale.max(1) as u128) as u64;
        return (media_time, delay_ns);
    }
    (0, 0)
}
