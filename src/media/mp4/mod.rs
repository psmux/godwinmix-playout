//! MP4 and QuickTime, read in Rust: `.mp4`, `.mov`, `.m4v`, `.m4a`.
//!
//! Only the `moov` box is read into memory. Samples are read from the file
//! one at a time, video and audio interleaved by decode time, so the two
//! never get more than a frame apart however the file was muxed.

pub mod boxes;
mod moov;
mod sound_entry;
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
    pub(super) track: Track,
    pub(super) next: usize,
}

impl Cursor {
    fn due_ns(&self) -> Option<u64> {
        self.track.samples.get(self.next).map(|s| self.track.dts_ns(s))
    }
}

impl Mp4 {
    pub fn open(path: &Path) -> Res<Mp4> {
        let mut file = File::open(path).map_err(|e| open_error(path, &e))?;
        let (moov, fragmented) = moov::find_moov(&mut file).map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let moov = moov.ok_or_else(|| format!("{} has no movie header (moov); it may be unfinished", path.display()))?;
        let (video, audio, duration_ns) = moov::tracks(&moov)?;
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
        file.seek(SeekFrom::Start(0)).map_err(|e| format!("could not read {}: {e}", path.display()))?;
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
        let cursor_dts = cursor.track.signed_ns(s.dts as i64);
        cursor.next += 1;
        let data = self.read_at(s.offset, s.size)?;
        Ok(Some(if take_video {
            // A decode time the edit list puts before zero has no GStreamer
            // spelling; the decoder works it out from the order instead.
            Packet::Video { pts, dts: (cursor_dts >= 0).then_some(dts), key: s.sync, data }
        } else {
            Packet::Audio { pts: cursor_signed, data }
        }))
    }
}
