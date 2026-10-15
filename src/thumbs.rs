//! Small JPEG thumbnails for the panel's list, made only when the panel asks.

use crate::media::kind::{sniff, Container};
use crate::media::picture::Scaler;
use crate::media::video_gst::{Pulled, VideoDecoder};
use crate::media::{frame, open_demux, Packet, Res};
use crate::settings::Fit;
use std::path::Path;
use std::time::{Duration, Instant};

/// A `data:image/jpeg` URI, or an empty string for a file with no picture.
pub fn make(path: &Path, width: u32, canvas: (u32, u32)) -> Res<String> {
    let width = width.clamp(32, 640) & !1;
    let height = ((width as u64 * canvas.1 as u64 / canvas.0.max(1) as u64) as u32).max(2) & !1;
    let i420 = match sniff(path)? {
        Container::Picture => crate::media::still::load(path, (width, height), Fit::Fit)?,
        Container::Mp4 | Container::Matroska => video_frame(path, (width, height))?,
        _ => return Ok(String::new()),
    };
    Ok(format!("data:image/jpeg;base64,{}", base64(&jpeg(&i420, width, height)?)))
}

/// One frame from about a second in, or a third of the way into a short clip.
fn video_frame(path: &Path, size: (u32, u32)) -> Res<Vec<u8>> {
    let mut demux = open_demux(path)?;
    let info = demux.info().clone();
    let track = info.video.as_ref().ok_or("it has no picture")?;
    let at = info.duration_ns.map_or(1_000_000_000, |d| (d / 3).min(1_000_000_000));
    demux.seek(at)?;
    let decoder = VideoDecoder::new(track, false)?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut scaler = Scaler::new(size.0, size.1, Fit::Fit);
    let mut out = vec![0u8; (size.0 * size.1 * 3 / 2) as usize];
    let mut ended = false;
    while Instant::now() < deadline {
        if !ended {
            match demux.next()? {
                Some(Packet::Video { pts, dts, key, data }) => decoder.push(pts, dts, key, data)?,
                Some(Packet::Audio { .. }) => continue,
                None => {
                    decoder.end();
                    ended = true;
                }
            }
        }
        match decoder.pull(if ended { 200 } else { 0 }) {
            Pulled::Frame(s) if frame::pts(&s).is_some_and(|p| p + 50_000_000 >= at) || ended => {
                frame::draw(&s, &mut scaler, &mut out)?;
                return Ok(out);
            }
            Pulled::Failed(e) => return Err(e),
            Pulled::End => break,
            _ => {}
        }
    }
    Err("no frame came out of the decoder in time".into())
}

/// I420 to RGB to JPEG, at quality 80.
fn jpeg(i420: &[u8], w: u32, h: u32) -> Res<Vec<u8>> {
    let (wu, hu) = (w as usize, h as usize);
    let (cw, ch) = (wu.div_ceil(2), hu.div_ceil(2));
    let (y, rest) = i420.split_at(wu * hu);
    let (u, v) = rest.split_at(cw * ch);
    let mut rgb = Vec::with_capacity(wu * hu * 3);
    for row in 0..hu {
        for col in 0..wu {
            let yy = (y[row * wu + col] as f32 - 16.0) * 1.164;
            let uu = u[(row / 2) * cw + col / 2] as f32 - 128.0;
            let vv = v[(row / 2) * cw + col / 2] as f32 - 128.0;
            rgb.push((yy + 1.793 * vv).clamp(0.0, 255.0) as u8);
            rgb.push((yy - 0.213 * uu - 0.533 * vv).clamp(0.0, 255.0) as u8);
            rgb.push((yy + 2.112 * uu).clamp(0.0, 255.0) as u8);
        }
    }
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80)
        .encode(&rgb, w, h, image::ExtendedColorType::Rgb8)
        .map_err(|e| format!("could not make the thumbnail: {e}"))?;
    Ok(out)
}

fn base64(data: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_the_rfc_examples() {
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
