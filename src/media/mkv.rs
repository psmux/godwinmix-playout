//! Matroska and WebM, read in Rust with the `matroska-demuxer` crate.

use super::kind::open_error;
use super::{AudioCodec, AudioTrack, Demux, Info, Packet, Res, VideoCodec, VideoTrack};
use matroska_demuxer::{Frame, MatroskaFile, TrackEntry, TrackType};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct Mkv {
    file: MatroskaFile<BufReader<File>>,
    info: Info,
    scale: u64,
    video_track: Option<u64>,
    audio_track: Option<u64>,
    /// After a seek, video frames are dropped until a keyframe, because the
    /// demuxer lands on the frame after the time asked for, not on a keyframe.
    want_key: bool,
    frame: Frame,
}

impl Mkv {
    pub fn open(path: &Path) -> Res<Mkv> {
        let file = File::open(path).map_err(|e| open_error(path, &e))?;
        let file = MatroskaFile::open(BufReader::with_capacity(1 << 20, file))
            .map_err(|e| format!("{} is not a Matroska file this plugin can read: {e}", path.display()))?;
        let scale = file.info().timestamp_scale().get();
        let duration_ns = file.info().duration().map(|d| (d * scale as f64) as u64);
        let video = file.tracks().iter().find(|t| t.track_type() == TrackType::Video);
        let audio = file.tracks().iter().find(|t| t.track_type() == TrackType::Audio);
        let video_info = video.map(video_track).transpose()?;
        let info = Info { duration_ns, video: video_info, audio: audio.map(audio_track) };
        if info.video.is_none() && info.audio.is_none() {
            return Err(format!("{} has no video or sound in it", path.display()));
        }
        let video_track = video.map(|t| t.track_number().get());
        let audio_track = audio.map(|t| t.track_number().get());
        Ok(Mkv { file, info, scale, video_track, audio_track, want_key: false, frame: Frame::default() })
    }
}

impl Demux for Mkv {
    fn info(&self) -> &Info {
        &self.info
    }

    fn seek(&mut self, ns: u64) -> Res<()> {
        // Three seconds before, then forward to a keyframe: the crate has no
        // way to ask for the keyframe at or before a time.
        let back = if self.video_track.is_some() { 3_000_000_000 } else { 0 };
        let ticks = ns.saturating_sub(back) / self.scale.max(1);
        self.file.seek(ticks).map_err(|e| format!("could not seek: {e}"))?;
        self.want_key = self.video_track.is_some() && ns > 0;
        Ok(())
    }

    fn next(&mut self) -> Res<Option<Packet>> {
        loop {
            let more = self.file.next_frame(&mut self.frame).map_err(|e| format!("the file could not be read on: {e}"))?;
            if !more {
                return Ok(None);
            }
            let pts = self.frame.timestamp.saturating_mul(self.scale);
            let data = std::mem::take(&mut self.frame.data);
            if Some(self.frame.track) == self.video_track {
                let key = self.frame.is_keyframe.unwrap_or(true);
                if self.want_key && !key {
                    continue;
                }
                self.want_key = false;
                return Ok(Some(Packet::Video { pts, dts: None, key, data }));
            }
            if Some(self.frame.track) == self.audio_track {
                return Ok(Some(Packet::Audio { pts: pts as i64, data }));
            }
        }
    }
}

fn video_track(t: &TrackEntry) -> Res<VideoTrack> {
    let codec = match t.codec_id() {
        "V_MPEG4/ISO/AVC" => VideoCodec::H264,
        "V_MPEGH/ISO/HEVC" => VideoCodec::H265,
        "V_AV1" => VideoCodec::Av1,
        "V_VP9" => VideoCodec::Vp9,
        "V_VP8" => VideoCodec::Vp8,
        other => return Err(format!("its video is {other}, which this plugin does not decode. H.264, HEVC, AV1, VP9 and VP8 play")),
    };
    let (width, height) = t.video().map_or((0, 0), |v| (v.pixel_width().get() as u32, v.pixel_height().get() as u32));
    Ok(VideoTrack { codec, config: t.codec_private().map(<[u8]>::to_vec), width, height })
}

fn audio_track(t: &TrackEntry) -> AudioTrack {
    let (rate, channels) = t.audio().map_or((48_000, 2), |a| (a.sampling_frequency() as u32, a.channels().get() as u32));
    let private = t.codec_private().map(<[u8]>::to_vec);
    let id = t.codec_id();
    let (codec, config) = match id {
        _ if id.starts_with("A_AAC") => (AudioCodec::Aac, private.or_else(|| Some(aac_config(rate, channels)))),
        "A_OPUS" => (AudioCodec::Opus, private),
        "A_VORBIS" => (AudioCodec::Vorbis, private),
        "A_FLAC" => (AudioCodec::Flac, private),
        "A_MPEG/L3" | "A_MPEG/L2" => (AudioCodec::Mp3, None),
        other => (AudioCodec::Other(other.to_string()), None),
    };
    AudioTrack { codec, config, rate: rate.max(1), channels: channels.max(1) }
}

/// An AAC LC AudioSpecificConfig, for a file that names AAC and gives none.
pub fn aac_config(rate: u32, channels: u32) -> Vec<u8> {
    const RATES: [u32; 13] = [96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350];
    let index = RATES.iter().position(|r| *r == rate).unwrap_or(3) as u16;
    let bits = (2u16 << 11) | (index << 7) | ((channels.min(7) as u16) << 3);
    bits.to_be_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_aac_config_is_made_from_rate_and_channels() {
        // 48 kHz stereo LC is 0x11 0x90, the most common two bytes in video.
        assert_eq!(super::aac_config(48_000, 2), [0x11, 0x90]);
        assert_eq!(super::aac_config(44_100, 2), [0x12, 0x10]);
    }
}
