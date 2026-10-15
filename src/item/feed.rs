//! The thread that reads one file: demux, video to the decoder, sound
//! decoded and resampled into the item's queue. It runs ahead of the output
//! until the decoder or the queue is full, and then waits on them, which is
//! all the pacing it needs.

use super::audio_queue::frames_to_ns;
use super::convert::Converter;
use super::sound::Sound;
use super::Shared;
use crate::media::audio::AudioDecoder;
use crate::media::audio_file::AudioFile;
use crate::media::resample::Resampler;
use crate::media::video_gst::VideoDecoder;
use crate::media::{open_demux, Demux, Packet, Res};
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// A video file: open, start the converter, then read to the end.
pub fn video(shared: Arc<Shared>, path: std::path::PathBuf) {
    if let Err(e) = video_inner(&shared, &path) {
        shared.status.fail(e);
    }
    shared.audio.finish();
}

fn video_inner(shared: &Arc<Shared>, path: &std::path::Path) -> Res<()> {
    let mut demux = open_demux(path)?;
    let info = demux.info().clone();
    shared.status.update(|s| {
        s.duration_ns = info.duration_ns;
        s.has_video = Some(info.video.is_some());
    });
    let decoder = match info.video.as_ref() {
        Some(track) => {
            let d = Arc::new(VideoDecoder::new(track, shared.software_only)?);
            shared.status.update(|s| s.decoder = Some(d.element.to_string()));
            *shared.decoder.lock().unwrap_or_else(|e| e.into_inner()) = Some(d.clone());
            Converter::spawn(shared.clone(), d.clone());
            Some(d)
        }
        None => None,
    };
    let mut sound = match info.audio.as_ref().map(AudioDecoder::new) {
        Some(Ok(d)) => Some(Sound::new(d)),
        Some(Err(e)) => {
            shared.status.update(|s| s.decoder.get_or_insert_with(String::new).push_str(&format!(" (silent: {e})")));
            None
        }
        None => None,
    };
    if shared.start_ns > 0 {
        demux.seek(shared.start_ns)?;
    }
    let result = pump(shared, &mut *demux, decoder.as_deref(), &mut sound);
    if let Some(d) = decoder.as_ref() {
        d.end();
    }
    if let Some(s) = sound.as_mut() {
        s.finish(&shared.audio);
    }
    result
}

fn pump(shared: &Shared, demux: &mut dyn Demux, video: Option<&VideoDecoder>, sound: &mut Option<Sound>) -> Res<()> {
    let stop_at = shared.out_ns.map(|o| o + 1_000_000_000);
    while !shared.stop.load(Ordering::Relaxed) {
        match demux.next()? {
            None => break,
            Some(Packet::Video { pts, dts, key, data }) => {
                if let Some(v) = video {
                    if stop_at.is_some_and(|s| dts.unwrap_or(pts) > s) {
                        break;
                    }
                    v.push(pts, dts, key, data)?;
                }
            }
            Some(Packet::Audio { pts, data }) => {
                if let Some(s) = sound.as_mut() {
                    s.packet(pts, data, &shared.audio);
                }
            }
        }
    }
    Ok(())
}

/// A sound file on its own.
pub fn audio(shared: Arc<Shared>, path: std::path::PathBuf) {
    if let Err(e) = audio_inner(&shared, &path) {
        shared.status.fail(e);
    }
    shared.audio.finish();
}

fn audio_inner(shared: &Shared, path: &std::path::Path) -> Res<()> {
    let mut file = AudioFile::open(path)?;
    shared.status.update(|s| {
        s.duration_ns = file.duration_ns;
        s.has_video = Some(false);
    });
    if shared.start_ns > 0 {
        file.seek(shared.start_ns)?;
    }
    let mut resampler = Resampler::new(file.rate);
    let (mut raw, mut out) = (Vec::new(), Vec::new());
    let mut base: Option<i64> = None;
    let mut produced = 0u64;
    while !shared.stop.load(Ordering::Relaxed) {
        raw.clear();
        out.clear();
        let Some(pts) = file.next(&mut raw)? else { break };
        resampler.push(&raw, &mut out);
        let b = *base.get_or_insert(pts as i64);
        if !out.is_empty() {
            shared.audio.push(b + frames_to_ns(produced) as i64, &out);
            produced += (out.len() / 2) as u64;
        }
    }
    out.clear();
    resampler.flush(&mut out);
    shared.audio.push(base.unwrap_or(0) + frames_to_ns(produced) as i64, &out);
    Ok(())
}
