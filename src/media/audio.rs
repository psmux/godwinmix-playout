//! Sound from inside a video file, decoded in Rust with symphonia: AAC, MP3,
//! FLAC and Vorbis. Opus, which symphonia does not decode, goes to
//! `audio_gst` (libopus through GStreamer, not FFmpeg).
//!
//! Out of here comes interleaved stereo f32 at the file's own rate. The
//! resampler takes it to 48 kHz.

use super::{audio_gst::GstAudio, AudioCodec, AudioTrack, Res};
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{CodecParameters, Decoder, DecoderOptions, CODEC_TYPE_AAC, CODEC_TYPE_FLAC, CODEC_TYPE_MP3, CODEC_TYPE_VORBIS};
use symphonia::core::formats::Packet;

pub enum AudioDecoder {
    Rust { decoder: Box<dyn Decoder>, buffer: Option<SampleBuffer<f32>> },
    Gst(GstAudio),
}

impl AudioDecoder {
    pub fn new(track: &AudioTrack) -> Res<AudioDecoder> {
        let codec = match track.codec {
            AudioCodec::Aac => CODEC_TYPE_AAC,
            AudioCodec::Mp3 => CODEC_TYPE_MP3,
            AudioCodec::Flac => CODEC_TYPE_FLAC,
            AudioCodec::Vorbis => CODEC_TYPE_VORBIS,
            AudioCodec::Opus => return Ok(AudioDecoder::Gst(GstAudio::opus(track)?)),
            AudioCodec::Other(ref what) => return Err(format!("its sound is {what}, which this plugin does not decode, so it plays silent")),
        };
        let mut params = CodecParameters::new();
        params.for_codec(codec).with_sample_rate(track.rate);
        if let Some(extra) = extra_data(track) {
            params.with_extra_data(extra.into_boxed_slice());
        }
        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(|e| format!("the sound decoder would not start: {e}"))?;
        Ok(AudioDecoder::Rust { decoder, buffer: None })
    }

    /// Decode one packet, appending stereo samples to `out`. Answers the
    /// rate they are at. A packet that does not decode is skipped: one bad
    /// packet is a 20 ms click, not a reason to stop.
    pub fn decode(&mut self, data: Vec<u8>, out: &mut Vec<f32>) -> Option<u32> {
        match self {
            AudioDecoder::Gst(g) => g.decode(data, out),
            AudioDecoder::Rust { decoder, buffer } => {
                let packet = Packet::new_from_boxed_slice(0, 0, 0, data.into_boxed_slice());
                let decoded = decoder.decode(&packet).ok()?;
                let spec = *decoded.spec();
                let frames = decoded.capacity() as u64;
                let buf = match buffer {
                    Some(b) if b.capacity() >= decoded.capacity() * spec.channels.count() => b,
                    _ => buffer.insert(SampleBuffer::new(frames, spec)),
                };
                buf.copy_interleaved_ref(decoded);
                to_stereo(buf.samples(), &spec, out);
                Some(spec.rate)
            }
        }
    }

    /// What a decoder with latency still holds, at the end of the file.
    pub fn finish(&mut self, out: &mut Vec<f32>) {
        if let AudioDecoder::Gst(g) = self {
            g.finish(out);
        }
    }
}

/// The setup a decoder wants, in the shape symphonia reads it.
fn extra_data(track: &AudioTrack) -> Option<Vec<u8>> {
    let config = track.config.clone()?;
    match track.codec {
        // Matroska gives the FLAC file header; symphonia wants STREAMINFO.
        AudioCodec::Flac if config.starts_with(b"fLaC") => config.get(8..42).map(<[u8]>::to_vec),
        // Matroska laces the three Vorbis headers together; symphonia wants
        // the first and the third.
        AudioCodec::Vorbis => xiph_headers(&config).map(|h| [h[0].as_slice(), h[2].as_slice()].concat()),
        _ => Some(config),
    }
}

/// Split Xiph laced headers: a count, the sizes of all but the last, the data.
pub fn xiph_headers(data: &[u8]) -> Option<Vec<Vec<u8>>> {
    let count = *data.first()? as usize + 1;
    let mut at = 1;
    let mut sizes = Vec::new();
    for _ in 0..count - 1 {
        let mut size = 0usize;
        loop {
            let b = *data.get(at)? as usize;
            at += 1;
            size += b;
            if b != 255 {
                break;
            }
        }
        sizes.push(size);
    }
    let mut out = Vec::new();
    for size in sizes {
        out.push(data.get(at..at + size)?.to_vec());
        at += size;
    }
    out.push(data.get(at..)?.to_vec());
    (out.len() == 3).then_some(out)
}

/// Any channel count to stereo. Five point one folds down the usual way,
/// with the centre and the surrounds at -3 dB, scaled so it cannot clip.
pub fn to_stereo(samples: &[f32], spec: &SignalSpec, out: &mut Vec<f32>) {
    let n = spec.channels.count().max(1);
    match n {
        1 => out.extend(samples.iter().flat_map(|s| [*s, *s])),
        2 => out.extend_from_slice(samples),
        _ => {
            const K: f32 = std::f32::consts::FRAC_1_SQRT_2;
            let norm = 1.0 / (1.0 + 2.0 * K);
            for f in samples.chunks_exact(n) {
                // symphonia's order: FL, FR, FC, LFE, then the rest in pairs.
                let c = f.get(2).copied().unwrap_or(0.0) * K;
                let (sl, sr) = (f.get(4).copied().unwrap_or(0.0) * K, f.get(5).copied().unwrap_or(0.0) * K);
                out.push((f[0] + c + sl) * norm);
                out.push((f[1] + c + sr) * norm);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symphonia::core::audio::Channels;

    #[test]
    fn xiph_lacing_splits_three_headers() {
        let data = [&[2u8, 3, 2][..], b"abc", b"de", b"fghij"].concat();
        let h = xiph_headers(&data).unwrap();
        assert_eq!(h, [b"abc".to_vec(), b"de".to_vec(), b"fghij".to_vec()]);
    }

    #[test]
    fn mono_is_heard_in_both_ears() {
        let spec = SignalSpec::new(44_100, Channels::FRONT_LEFT);
        let mut out = Vec::new();
        to_stereo(&[0.5, -0.25], &spec, &mut out);
        assert_eq!(out, [0.5, 0.5, -0.25, -0.25]);
    }

    #[test]
    fn a_six_channel_fold_down_does_not_clip() {
        let spec = SignalSpec::new(48_000, Channels::FRONT_LEFT | Channels::FRONT_RIGHT | Channels::FRONT_CENTRE | Channels::LFE1 | Channels::REAR_LEFT | Channels::REAR_RIGHT);
        let mut out = Vec::new();
        to_stereo(&[1.0; 6], &spec, &mut out);
        assert!(out.iter().all(|s| *s <= 1.0001), "{out:?}");
    }
}
