//! A sound file on its own, read and decoded in Rust by symphonia: MP3,
//! WAV, FLAC, Ogg Vorbis, AIFF and AAC (ADTS).

use super::audio::to_stereo;
use super::kind::open_error;
use super::Res;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

pub struct AudioFile {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: u32,
    pub rate: u32,
    pub duration_ns: Option<u64>,
    time_base: Option<symphonia::core::units::TimeBase>,
    buffer: Option<SampleBuffer<f32>>,
}

impl AudioFile {
    pub fn open(path: &Path) -> Res<AudioFile> {
        let file = std::fs::File::open(path).map_err(|e| open_error(path, &e))?;
        let stream = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(&hint, stream, &FormatOptions { enable_gapless: true, ..Default::default() }, &MetadataOptions::default())
            .map_err(|e| format!("{} is not a sound file this plugin reads: {e}", path.display()))?;
        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| format!("{} has no sound in it", path.display()))?;
        let params = track.codec_params.clone();
        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(|e| format!("the sound in {} is in a codec this plugin does not decode: {e}", path.display()))?;
        let rate = params.sample_rate.unwrap_or(48_000);
        let duration_ns = params.n_frames.map(|n| n * 1_000_000_000 / rate.max(1) as u64);
        Ok(AudioFile { track: track.id, format, decoder, rate, duration_ns, time_base: params.time_base, buffer: None })
    }

    pub fn seek(&mut self, ns: u64) -> Res<()> {
        let time = Time::new(ns / 1_000_000_000, (ns % 1_000_000_000) as f64 / 1e9);
        self.format
            .seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(self.track) })
            .map(|_| self.decoder.reset())
            .map_err(|e| format!("could not seek: {e}"))
    }

    /// Decode the next packet onto `out` as stereo. Answers its time, or
    /// `None` at the end.
    pub fn next(&mut self, out: &mut Vec<f32>) -> Res<Option<u64>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(symphonia::core::errors::Error::ResetRequired) => return Ok(None),
                Err(e) => return Err(format!("the sound could not be read on: {e}")),
            };
            if packet.track_id() != self.track {
                continue;
            }
            let ns = self.time_base.map_or(0, |tb| {
                let t = tb.calc_time(packet.ts());
                t.seconds * 1_000_000_000 + (t.frac * 1e9) as u64
            });
            let Ok(decoded) = self.decoder.decode(&packet) else { continue };
            let spec = *decoded.spec();
            let need = decoded.capacity() * spec.channels.count();
            let buf = match self.buffer.as_mut() {
                Some(b) if b.capacity() >= need => b,
                _ => self.buffer.insert(SampleBuffer::new(decoded.capacity() as u64, spec)),
            };
            buf.copy_interleaved_ref(decoded);
            to_stereo(buf.samples(), &spec, out);
            return Ok(Some(ns));
        }
    }
}
