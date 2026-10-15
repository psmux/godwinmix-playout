//! Video decode: the only job given to GStreamer.
//!
//! Four elements and nothing that searches: `appsrc ! <parser> ! <decoder> !
//! appsink`. The decoder is named, from a list of the platform's own decoders
//! and the permissive software ones (openh264, dav1d, libvpx, libde265). No
//! `decodebin`, so nothing can fall through to FFmpeg's `avdec_*`, and a codec
//! with none of the listed decoders on this machine says so and is skipped.

use super::audio_gst::by_name;
use super::video_codecs::{caps, decoders, parser};
use super::{Res, VideoCodec, VideoTrack};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::{AppSink, AppSrc};

/// The decoder this machine would use for `codec`, or a sentence saying
/// there is none.
pub fn choose(codec: VideoCodec, software_only: bool) -> Res<&'static str> {
    crate::gst_runtime::ready()?;
    let found = decoders(codec, software_only).iter().find(|name| gst::ElementFactory::find(name).is_some());
    // Apple's decoder is `vtdec_hw` when it insists on hardware and `vtdec`
    // when it may fall back; either is VideoToolbox.
    let found = found.copied().or_else(|| (gst::ElementFactory::find("vtdec").is_some() && matches!(codec, VideoCodec::H264 | VideoCodec::H265)).then_some("vtdec"));
    found.ok_or_else(|| {
        format!(
            "nothing on this machine decodes {} without FFmpeg (looked for {}). Install one of them, or convert the file to H.264",
            codec.name(),
            decoders(codec, false).join(", ")
        )
    })
}

pub struct VideoDecoder {
    pipeline: gst::Pipeline,
    src: AppSrc,
    pub sink: AppSink,
    pub element: &'static str,
}

/// What a pull found.
pub enum Pulled {
    Frame(gst::Sample),
    Wait,
    End,
    Failed(String),
}

impl VideoDecoder {
    pub fn new(track: &VideoTrack, software_only: bool) -> Res<VideoDecoder> {
        let element = choose(track.codec, software_only)?;
        let mut chain = vec!["appsrc name=src format=time block=true max-bytes=4194304".to_string()];
        if let Some(p) = parser(track.codec).filter(|p| gst::ElementFactory::find(p).is_some()) {
            chain.push(p.to_string());
        }
        chain.push(element.to_string());
        chain.push("appsink name=sink sync=false max-buffers=3 enable-last-sample=false caps=video/x-raw,format={NV12,I420,P010_10LE,I420_10LE}".into());
        let pipeline = gst::parse::launch(&chain.join(" ! "))
            .map_err(|e| format!("the {element} pipeline did not build: {e}"))?
            .downcast::<gst::Pipeline>()
            .map_err(|_| "the video pipeline did not build".to_string())?;
        let src = by_name::<AppSrc>(&pipeline, "src")?;
        src.set_caps(Some(&caps(track)));
        let sink = by_name::<AppSink>(&pipeline, "sink")?;
        pipeline.set_state(gst::State::Playing).map_err(|e| format!("{element} would not start: {e}"))?;
        Ok(VideoDecoder { pipeline, src, sink, element })
    }

    /// Hand over one compressed frame. Blocks while the decoder is full,
    /// which is what paces the demuxer.
    pub fn push(&self, pts: u64, dts: Option<u64>, key: bool, data: Vec<u8>) -> Res<()> {
        let mut buffer = gst::Buffer::from_mut_slice(data);
        {
            let b = buffer.get_mut().expect("a new buffer is writable");
            b.set_pts(gst::ClockTime::from_nseconds(pts));
            b.set_dts(dts.map(gst::ClockTime::from_nseconds));
            if !key {
                b.set_flags(gst::BufferFlags::DELTA_UNIT);
            }
        }
        self.src.push_buffer(buffer).map(|_| ()).map_err(|e| format!("the decoder took no more: {e:?}"))
    }

    pub fn end(&self) {
        let _ = self.src.end_of_stream();
    }

    pub fn pull(&self, timeout_ms: u64) -> Pulled {
        if let Some(why) = self.error() {
            return Pulled::Failed(why);
        }
        match self.sink.try_pull_sample(gst::ClockTime::from_mseconds(timeout_ms)) {
            Some(sample) => Pulled::Frame(sample),
            None if self.sink.is_eos() => Pulled::End,
            None => Pulled::Wait,
        }
    }

    fn error(&self) -> Option<String> {
        let bus = self.pipeline.bus()?;
        let msg = bus.pop_filtered(&[gst::MessageType::Error])?;
        match msg.view() {
            gst::MessageView::Error(e) => Some(format!("{} could not decode it: {}", self.element, e.error())),
            _ => None,
        }
    }

    /// Stop the pipeline. Unblocks a push waiting on a full queue.
    pub fn close(&self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        self.close();
    }
}
