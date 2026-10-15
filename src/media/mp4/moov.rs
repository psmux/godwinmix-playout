//! The movie header: finding `moov` in the file and reading its tracks.

use super::boxes;
use super::track::{self, Carries};
use super::Cursor;
use crate::media::Res;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Walk the top level boxes for `moov`, reading only it. Also says whether
/// the file has fragments, which `moov` alone cannot play.
pub fn find_moov(file: &mut File) -> std::io::Result<(Option<Vec<u8>>, bool)> {
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

pub type Tracks = (Option<Cursor>, Option<Cursor>, Option<u64>);

pub fn tracks(moov: &[u8]) -> Res<Tracks> {
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
