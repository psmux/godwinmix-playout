//! Any decoded picture onto the canvas, in Rust: scaled with
//! fast_image_resize (SIMD on x86 and ARM), letterboxed or cropped, and
//! written as I420, which is what the mixer takes from a source.
//!
//! A frame already at the canvas size and in I420 is copied row by row and
//! never touches the resizer, which is the common case for a playlist made
//! for the show it plays in.

use crate::media::Res;
use crate::settings::Fit;
use fast_image_resize::images::{CroppedImageMut, Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};

#[path = "place.rs"]
pub mod place;
use place::Place;

/// One decoded plane: its bytes, the stride between rows, and its size.
#[derive(Clone, Copy)]
pub struct Plane<'a> {
    pub data: &'a [u8],
    pub stride: usize,
    pub width: u32,
    pub height: u32,
}

/// A decoded picture, 8 bit, in one of the two layouts decoders give.
pub enum Layout<'a> {
    I420 { y: Plane<'a>, u: Plane<'a>, v: Plane<'a> },
    Nv12 { y: Plane<'a>, uv: Plane<'a> },
}

pub struct Scaler {
    resizer: Resizer,
    canvas: (u32, u32),
    fit: Fit,
    uv: Vec<u8>,
}

impl Scaler {
    pub fn new(width: u32, height: u32, fit: Fit) -> Scaler {
        Scaler { resizer: Resizer::new(), canvas: (width, height), fit, uv: Vec::new() }
    }

    /// Draw `layout`, whose pixels have the aspect `par`, onto `out`, an
    /// I420 frame at the canvas size.
    pub fn draw(&mut self, layout: &Layout, par: (u32, u32), out: &mut [u8]) -> Res<()> {
        let y = match layout {
            Layout::I420 { y, .. } | Layout::Nv12 { y, .. } => *y,
        };
        let place = Place::new((y.width, y.height), par, self.canvas, self.fit);
        let (cw, ch) = self.canvas;
        let (luma, chroma) = out.split_at_mut((cw * ch) as usize);
        let (u_out, v_out) = chroma.split_at_mut(chroma.len() / 2);
        if place.bars() {
            luma.fill(16);
            u_out.fill(128);
            v_out.fill(128);
        }
        self.plane(y, luma, (cw, ch), place.luma(), PixelType::U8)?;
        let half = (cw.div_ceil(2), ch.div_ceil(2));
        match layout {
            Layout::I420 { u, v, .. } => {
                self.plane(*u, u_out, half, place.chroma(), PixelType::U8)?;
                self.plane(*v, v_out, half, place.chroma(), PixelType::U8)?;
            }
            Layout::Nv12 { uv, .. } => self.interleaved(*uv, u_out, v_out, half, place.chroma())?,
        }
        Ok(())
    }

    /// One plane into its rectangle of the canvas plane.
    fn plane(&mut self, src: Plane, dst: &mut [u8], size: (u32, u32), r: place::Rects, px: PixelType) -> Res<()> {
        if r.is_copy(src.width, src.height) && px == PixelType::U8 {
            copy_rows(src, dst, size.0 as usize, r.dst);
            return Ok(());
        }
        let bytes = px.size();
        let view = ImageRef::new((src.stride / bytes) as u32, src.height, src.data, px).map_err(|e| format!("a decoded plane is the wrong size: {e}"))?;
        let mut canvas = Image::from_slice_u8(size.0, size.1, dst, px).map_err(|e| format!("the canvas plane is the wrong size: {e}"))?;
        let mut target = CroppedImageMut::new(&mut canvas, r.dst.0, r.dst.1, r.dst.2, r.dst.3).map_err(|e| format!("the picture does not fit the canvas: {e}"))?;
        let (sx, sy, sw, sh) = r.src;
        let options = ResizeOptions::new()
            .resize_alg(ResizeAlg::Convolution(FilterType::Bilinear))
            .crop(sx as f64, sy as f64, sw as f64, sh as f64);
        self.resizer.resize(&view, &mut target, &options).map_err(|e| format!("could not scale the picture: {e}"))
    }

    /// NV12's interleaved chroma: scaled as pairs, then split.
    fn interleaved(&mut self, uv: Plane, u_out: &mut [u8], v_out: &mut [u8], size: (u32, u32), r: place::Rects) -> Res<()> {
        let (dw, dh) = (r.dst.2 as usize, r.dst.3 as usize);
        if r.is_copy(uv.width, uv.height) {
            // Same size: split the pairs straight into place, no resizer.
            let stride = size.0 as usize;
            for row in 0..dh {
                let line = &uv.data[row * uv.stride..row * uv.stride + dw * 2];
                let at = (r.dst.1 as usize + row) * stride + r.dst.0 as usize;
                for (i, pair) in line.as_chunks::<2>().0.iter().enumerate() {
                    u_out[at + i] = pair[0];
                    v_out[at + i] = pair[1];
                }
            }
            return Ok(());
        }
        self.uv.resize(dw * dh * 2, 128);
        let mut uv_dst = std::mem::take(&mut self.uv);
        let whole = place::Rects { src: r.src, dst: (0, 0, r.dst.2, r.dst.3) };
        let pairs = Plane { width: uv.width, ..uv };
        let result = self.plane(pairs, &mut uv_dst, (r.dst.2, r.dst.3), whole, PixelType::U8x2);
        let stride = size.0 as usize;
        for row in 0..dh {
            let line = &uv_dst[row * dw * 2..(row + 1) * dw * 2];
            let at = (r.dst.1 as usize + row) * stride + r.dst.0 as usize;
            for (i, pair) in line.as_chunks::<2>().0.iter().enumerate() {
                u_out[at + i] = pair[0];
                v_out[at + i] = pair[1];
            }
        }
        self.uv = uv_dst;
        result
    }
}

fn copy_rows(src: Plane, dst: &mut [u8], dst_stride: usize, rect: (u32, u32, u32, u32)) {
    let (x, y, w, h) = (rect.0 as usize, rect.1 as usize, rect.2 as usize, rect.3 as usize);
    for row in 0..h {
        let from = &src.data[row * src.stride..row * src.stride + w];
        let at = (y + row) * dst_stride + x;
        dst[at..at + w].copy_from_slice(from);
    }
}

#[cfg(test)]
#[path = "picture_tests.rs"]
mod tests;
