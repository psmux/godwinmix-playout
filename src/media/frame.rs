//! A decoded GStreamer sample, read as planes and drawn on the canvas.

use super::picture::{Layout, Plane, Scaler};
use super::Res;
use gstreamer as gst;
use gstreamer_video::prelude::*;
use gstreamer_video::{VideoFormat, VideoFrameRef, VideoInfo};

/// The sample's presentation time in nanoseconds.
pub fn pts(sample: &gst::Sample) -> Option<u64> {
    sample.buffer()?.pts().map(gst::ClockTime::nseconds)
}

/// Draw `sample` into `out`, an I420 canvas frame.
pub fn draw(sample: &gst::Sample, scaler: &mut Scaler, out: &mut [u8]) -> Res<()> {
    let caps = sample.caps().ok_or("a decoded frame came with no caps")?;
    let info = VideoInfo::from_caps(caps).map_err(|e| format!("a decoded frame's caps did not read: {e}"))?;
    let buffer = sample.buffer().ok_or("a decoded frame came with no buffer")?;
    let frame = VideoFrameRef::from_buffer_ref_readable(buffer, &info).map_err(|_| "a decoded frame could not be mapped".to_string())?;
    let par = (info.par().numer().max(1) as u32, info.par().denom().max(1) as u32);
    let (w, h) = (info.width(), info.height());
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let plane = |i: u32, pw: u32, ph: u32| -> Res<Plane> {
        let data = frame.plane_data(i).map_err(|_| format!("plane {i} of a decoded frame is missing"))?;
        Ok(Plane { data, stride: frame.plane_stride()[i as usize] as usize, width: pw, height: ph })
    };
    match info.format() {
        VideoFormat::I420 => {
            let layout = Layout::I420 { y: plane(0, w, h)?, u: plane(1, cw, ch)?, v: plane(2, cw, ch)? };
            scaler.draw(&layout, par, out)
        }
        VideoFormat::Nv12 => scaler.draw(&Layout::Nv12 { y: plane(0, w, h)?, uv: plane(1, cw, ch)? }, par, out),
        VideoFormat::P01010le => {
            // Ten bits in the top of sixteen: the high byte is the 8 bit value.
            let y = narrow(plane(0, w, h)?, w as usize, 8);
            let uv = narrow(plane(1, cw * 2, ch)?, cw as usize * 2, 8);
            let layout = Layout::Nv12 { y: owned(&y, w, h), uv: owned(&uv, cw, ch) };
            scaler.draw(&layout, par, out)
        }
        VideoFormat::I42010le => {
            // Ten bits in the bottom of sixteen.
            let y = narrow(plane(0, w, h)?, w as usize, 2);
            let u = narrow(plane(1, cw, ch)?, cw as usize, 2);
            let v = narrow(plane(2, cw, ch)?, cw as usize, 2);
            let layout = Layout::I420 { y: owned(&y, w, h), u: owned(&u, cw, ch), v: owned(&v, cw, ch) };
            scaler.draw(&layout, par, out)
        }
        other => Err(format!("the decoder gave {other:?} frames, which this plugin does not convert")),
    }
}

/// Sixteen bit little endian samples down to eight, `shift` bits off.
fn narrow(p: Plane, samples_per_row: usize, shift: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples_per_row * p.height as usize);
    for row in 0..p.height as usize {
        let line = &p.data[row * p.stride..row * p.stride + samples_per_row * 2];
        out.extend(line.chunks_exact(2).map(|b| (u16::from_le_bytes([b[0], b[1]]) >> shift).min(255) as u8));
    }
    out
}

fn owned(data: &[u8], width: u32, height: u32) -> Plane<'_> {
    let stride = data.len() / height.max(1) as usize;
    Plane { data, stride, width, height }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_bit_samples_narrow_to_their_top_eight_bits() {
        let data = [0x00u8, 0xff, 0x40, 0x80, 0xff, 0x03, 0x00, 0x01];
        let p = Plane { data: &data, stride: 4, width: 2, height: 2 };
        assert_eq!(narrow(p, 2, 8), [0xff, 0x80, 0x03, 0x01]);
        assert_eq!(narrow(p, 2, 2), [255, 255, 255, 64]);
    }
}
