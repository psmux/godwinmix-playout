//! Opus, the one sound codec common in video files that symphonia has no
//! decoder for. `opusdec` is libopus, the reference decoder, not FFmpeg.
//!
//! Three elements: `appsrc ! opusdec ! appsink`. The samples come out as
//! 16 bit integers, which is all opusdec gives, and are made stereo f32 here.

use super::{AudioTrack, Res};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::{AppSink, AppSrc};

pub struct GstAudio {
    pipeline: gst::Pipeline,
    src: AppSrc,
    sink: AppSink,
    channels: usize,
}

impl GstAudio {
    pub fn opus(track: &AudioTrack) -> Res<GstAudio> {
        crate::gst_runtime::ready()?;
        let pipeline = gst::parse::launch(
            "appsrc name=src format=time ! opusdec ! appsink name=sink sync=false \
             caps=audio/x-raw,format=S16LE,layout=interleaved,rate=48000",
        )
        .map_err(|e| format!("the Opus decoder (opusdec) is not here: {e}"))?
        .downcast::<gst::Pipeline>()
        .map_err(|_| "the Opus pipeline did not build".to_string())?;
        let src = by_name::<AppSrc>(&pipeline, "src")?;
        let sink = by_name::<AppSink>(&pipeline, "sink")?;
        let channels = track.channels.clamp(1, 2) as i32;
        src.set_caps(Some(&gst::Caps::builder("audio/x-opus").field("channel-mapping-family", 0i32).field("channels", channels).field("rate", 48_000i32).build()));
        pipeline.set_state(gst::State::Playing).map_err(|e| format!("the Opus decoder would not start: {e}"))?;
        // Wait for PLAYING here, on the item's own thread. Until then the
        // sink holds its first buffer and gives nothing, and that was a second
        // and more of silence at the start of every Opus item.
        let _ = pipeline.state(gst::ClockTime::from_seconds(5));
        let me = GstAudio { pipeline, src, sink, channels: channels as usize };
        if let Some(head) = track.config.clone() {
            // In band headers: opusdec reads OpusHead from the first buffer.
            let _ = me.src.push_buffer(gst::Buffer::from_mut_slice(head));
            let _ = me.src.push_buffer(gst::Buffer::from_slice(*b"OpusTags\0\0\0\0\0\0\0\0"));
        }
        Ok(me)
    }

    /// opusdec runs on appsrc's own thread, so what comes out now may be
    /// from packets pushed earlier: the time answered is the first sample's
    /// own, carried through the decoder, not this packet's.
    pub fn decode(&mut self, data: Vec<u8>, pts: i64, out: &mut Vec<f32>) -> Option<(u32, i64)> {
        let mut buffer = gst::Buffer::from_mut_slice(data);
        if let Some(b) = buffer.get_mut() {
            b.set_pts(gst::ClockTime::from_nseconds(pts.max(0) as u64));
        }
        let _ = self.src.push_buffer(buffer);
        let mut first = None;
        // Each packet decodes in well under a millisecond on appsrc's thread;
        // waiting a little for it keeps the sound in step with the demuxer,
        // which may next block on the picture for a while.
        let mut wait = gst::ClockTime::from_mseconds(50);
        while let Some(sample) = self.sink.try_pull_sample(wait) {
            wait = gst::ClockTime::ZERO;
            first = first.or_else(|| sample.buffer().and_then(|b| b.pts()).map(|t| t.nseconds() as i64));
            self.append(&sample, out);
        }
        first.map(|t| (48_000, t))
    }

    pub fn finish(&mut self, out: &mut Vec<f32>) {
        let _ = self.src.end_of_stream();
        while let Some(sample) = self.sink.try_pull_sample(gst::ClockTime::from_mseconds(200)) {
            self.append(&sample, out);
        }
    }

    fn append(&self, sample: &gst::Sample, out: &mut Vec<f32>) {
        let channels = sample
            .caps()
            .and_then(|c| c.structure(0).and_then(|s| s.get::<i32>("channels").ok()))
            .map_or(self.channels, |c| c.max(1) as usize);
        let Some(map) = sample.buffer().and_then(|b| b.map_readable().ok()) else { return };
        let floats = map.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0);
        if channels == 1 {
            out.extend(floats.flat_map(|s| [s, s]));
        } else {
            let all: Vec<f32> = floats.collect();
            out.extend(all.chunks_exact(channels).flat_map(|f| [f[0], f[1]]));
        }
    }
}

impl Drop for GstAudio {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

pub fn by_name<T: IsA<gst::Element>>(pipeline: &gst::Pipeline, name: &str) -> Res<T> {
    pipeline
        .by_name(name)
        .and_then(|e| e.downcast::<T>().ok())
        .ok_or_else(|| format!("the pipeline has no '{name}'"))
}
