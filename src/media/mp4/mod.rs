//! MP4 and QuickTime, read in Rust: `.mp4`, `.mov`, `.m4v`, `.m4a`.
//!
//! Only the `moov` box is read into memory. Samples are read from the file
//! one at a time, video and audio interleaved by decode time, so the two
//! never get more than a frame apart however the file was muxed.

pub mod boxes;
pub mod table;
pub mod track;

use super::kind::open_error;
use super::{Demux, Info, Packet, Res};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use track::{Carries, Track};

pub struct Mp4 {
    file: BufReader<File>,
    pos: u64,
    info: Info,
    video: Option<Cursor>,
    audio: Option<Cursor>,
}

struct Cursor {
    track: Track,
    next: usize,
}

impl Cursor {
    fn due_ns(&self) -> Option<u64> {
        self.track.samples.get(self.next).map(|s| self.track.dts_ns(s))
    }
}

impl Mp4 {
    pub fn open(path: &Path) -> Res<Mp4> {
        let mut file = File::open(path).map_err(|e| open_error(path, &e))?;
        let (moov, fragmented) = find_moov(&mut file).map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let moov = moov.ok_or_else(|| format!("{} has no movie header (moov); it may be unfinished", path.display()))?;
        let (video, audio, duration_ns) = tracks(&moov)?;
        let has_samples = video.as_ref().is_some_and(|c| !c.track.samples.is_empty())
            || audio.as_ref().is_some_and(|c| !c.track.samples.is_empty());
        if fragmented && !has_samples {
            return Err(format!("{} is a fragmented MP4, which this plugin does not read yet. Remux it to a plain MP4", path.display()));
        }
        let info = Info {
            duration_ns,
            video: video.as_ref().and_then(|c| match &c.track.carries { Carries::Video(v) => Some(v.clone()), _ => None }),
            audio: audio.as_ref().and_then(|c| match &c.track.carries { Carries::Audio(a) => Some(a.clone()), _ => None }),
        };
        Ok(Mp4 { file: BufReader::with_capacity(1 << 20, file), pos: 0, info, video, audio })
    }

    fn read_at(&mut self, offset: u64, size: u32) -> Res<Vec<u8>> {
        if offset != self.pos {
            let delta = offset as i64 - self.pos as i64;
            self.file.seek_relative(delta).map_err(|e| format!("seek failed: {e}"))?;
        }
        let mut data = vec![0u8; size as usize];
        self.file.read_exact(&mut data).map_err(|e| format!("the file ends early: {e}"))?;
        self.pos = offset + size as u64;
        Ok(data)
    }
}

impl Demux for Mp4 {
    fn info(&self) -> &Info {
        &self.info
    }

    fn seek(&mut self, ns: u64) -> Res<()> {
        // Decode times only ever go up, which presentation times with B
        // frames do not, so the search is on those and the keyframe walk
        // goes back from there.
        if let Some(v) = self.video.as_mut() {
            let t = &v.track;
            let at = t.samples.partition_point(|s| t.dts_ns(s) <= ns).saturating_sub(1);
            v.next = (0..=at).rev().find(|&i| t.samples.get(i).is_some_and(|s| s.sync)).unwrap_or(0);
        }
        if let Some(a) = self.audio.as_mut() {
            let t = &a.track;
            a.next = t.samples.partition_point(|s| t.dts_ns(s) <= ns).saturating_sub(1);
        }
        Ok(())
    }

    fn next(&mut self) -> Res<Option<Packet>> {
        let v = self.video.as_ref().and_then(Cursor::due_ns);
        let a = self.audio.as_ref().and_then(Cursor::due_ns);
        let take_video = match (v, a) {
            (Some(v), Some(a)) => v <= a,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return Ok(None),
        };
        let cursor = if take_video { self.video.as_mut() } else { self.audio.as_mut() }.expect("chosen above");
        let s = cursor.track.samples[cursor.next];
        let (pts, dts) = (cursor.track.pts_ns(&s), cursor.track.dts_ns(&s));
        let cursor_signed = cursor.track.signed_ns(s.dts as i64 + s.cto);
        cursor.next += 1;
        let data = self.read_at(s.offset, s.size)?;
        Ok(Some(if take_video {
            Packet::Video { pts, dts: Some(dts), key: s.sync, data }
        } else {
            Packet::Audio { pts: cursor_signed, data }
        }))
    }
}

/// Walk the top level boxes for `moov`, reading only it. Also says whether
/// the file has fragments, which `moov` alone cannot play.
fn find_moov(file: &mut File) -> std::io::Result<(Option<Vec<u8>>, bool)> {
    let len = file.metadata()?.len();
    let (mut at, mut moov, mut fragmented) = (0u64, None, false);
    while at + 8 <= len {
        file.seek(SeekFrom::Start(at))?;
        let mut head = [0u8; 16];
        file.read_exact(&mut head[..8])?;
        let size32 = u32::from_be_bytes(head[..4].try_into().unwrap()) as u64;
        let kind: [u8; 4] = head[4..8].try_into().unwrap();
        let (header, size) = match size32 {
            1 => {
                file.read_exact(&mut head[8..16])?;
                (16, u64::from_be_bytes(head[8..16].try_into().unwrap()))
            }
            0 => (8, len - at),
            n => (8, n),
        };
        if size < header {
            break;
        }
        if &kind == b"moof" {
            fragmented = true;
        }
        if &kind == b"moov" {
            let mut body = vec![0u8; (size - header) as usize];
            file.read_exact(&mut body)?;
            moov = Some(body);
        }
        at += size;
    }
    Ok((moov, fragmented))
}

type Tracks = (Option<Cursor>, Option<Cursor>, Option<u64>);

fn tracks(moov: &[u8]) -> Res<Tracks> {
    let (movie_scale, movie_duration) = movie_header(moov);
    let (mut video, mut audio, mut refusal) = (None, None, None);
    for trak in boxes::children(moov).into_iter().filter(|b| &b.kind == b"trak") {
        match track::read(trak.body, movie_scale) {
            Ok(Some(t)) => match t.carries {
                Carries::Video(_) if video.is_none() => video = Some(Cursor { track: t, next: 0 }),
                Carries::Audio(_) if audio.is_none() => audio = Some(Cursor { track: t, next: 0 }),
                _ => {}
            },
            Ok(None) => {}
            Err(e) => refusal = Some(e),
        }
    }
    if video.is_none() && audio.is_none() {
        return Err(refusal.unwrap_or_else(|| "the file has no video or sound in it".into()));
    }
    if let (None, Some(e)) = (&video, refusal) {
        return Err(e);
    }
    let end = [&video, &audio].iter().filter_map(|c| c.as_ref().map(|c| c.track.end_ns())).max();
    let duration = movie_duration.filter(|d| *d > 0).or(end);
    Ok((video, audio, duration))
}

fn movie_header(moov: &[u8]) -> (u32, Option<u64>) {
    let Some(mvhd) = boxes::child(moov, b"mvhd") else { return (1000, None) };
    let (version, body) = boxes::full(mvhd);
    let mut r = boxes::Be::new(body);
    let (scale, duration) = if version == 1 {
        r.skip(16);
        (r.u32(), r.u64())
    } else {
        r.skip(8);
        (r.u32(), r.u32() as u64)
    };
    let ns = (duration as u128 * 1_000_000_000 / scale.max(1) as u128) as u64;
    (scale, Some(ns))
}
