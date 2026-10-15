//! Reading files. Everything here is Rust except `video_gst`, which hands
//! compressed video to the platform's own decoder through GStreamer.
//!
//! ```text
//!   file --> demux (mp4.rs, mkv.rs: Rust) --+--> video_gst (platform decoder)
//!                                           |      --> picture.rs (scale, Rust)
//!                                           +--> audio.rs (symphonia: Rust)
//!                                                  --> resample.rs (rubato: Rust)
//!   audio file --> audio_file.rs (symphonia: Rust)
//!   picture    --> still.rs (image: Rust)
//! ```

pub mod audio;
pub mod audio_file;
pub mod audio_gst;
pub mod frame;
pub mod kind;
pub mod mkv;
pub mod mp4;
pub mod picture;
pub mod probe;
pub mod resample;
pub mod still;
pub mod video_gst;

pub type Res<T> = Result<T, String>;

/// The video codecs a demuxer can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    H265,
    Av1,
    Vp9,
    Vp8,
}

impl VideoCodec {
    pub fn name(self) -> &'static str {
        match self {
            VideoCodec::H264 => "H.264",
            VideoCodec::H265 => "HEVC",
            VideoCodec::Av1 => "AV1",
            VideoCodec::Vp9 => "VP9",
            VideoCodec::Vp8 => "VP8",
        }
    }
}

/// The audio codecs a demuxer can name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCodec {
    Aac,
    Mp3,
    Opus,
    Vorbis,
    Flac,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct VideoTrack {
    pub codec: VideoCodec,
    /// avcC, hvcC or av1C: what the decoder needs before the first frame.
    pub config: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub codec: AudioCodec,
    /// The codec's own setup: an AudioSpecificConfig, Vorbis headers, OpusHead.
    pub config: Option<Vec<u8>>,
    pub rate: u32,
    pub channels: u32,
}

/// What a demuxer found in a file.
#[derive(Debug, Clone, Default)]
pub struct Info {
    pub duration_ns: Option<u64>,
    pub video: Option<VideoTrack>,
    pub audio: Option<AudioTrack>,
}

/// One compressed frame, with times in nanoseconds from the start of the file.
#[derive(Debug)]
pub enum Packet {
    Video { pts: u64, dts: Option<u64>, key: bool, data: Vec<u8> },
    /// Signed: the first AAC packet of a file is often before zero, the
    /// encoder's priming, and is decoded and then cut off.
    Audio { pts: i64, data: Vec<u8> },
}

/// A container read in Rust.
pub trait Demux: Send {
    fn info(&self) -> &Info;
    /// Go to the last video keyframe at or before `ns`, and the audio there.
    fn seek(&mut self, ns: u64) -> Res<()>;
    /// The next packet in time order, or `None` at the end.
    fn next(&mut self) -> Res<Option<Packet>>;
}

/// Open whichever demuxer the file's first bytes call for.
pub fn open_demux(path: &std::path::Path) -> Res<Box<dyn Demux>> {
    match kind::sniff(path)? {
        kind::Container::Mp4 => Ok(Box::new(mp4::Mp4::open(path)?)),
        kind::Container::Matroska => Ok(Box::new(mkv::Mkv::open(path)?)),
        other => Err(format!("{} is not a video container this plugin reads ({other:?})", path.display())),
    }
}
