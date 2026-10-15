//! A picture as a canvas frame, in Rust: decoded by the `image` crate,
//! scaled by fast_image_resize, and turned into BT.709 I420 here. Done once
//! per item, off the output thread.

use super::picture::place::Place;
use super::Res;
use crate::settings::Fit;
use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use std::path::Path;

/// The picture at `path`, drawn on a black canvas of `canvas` size.
pub fn load(path: &Path, canvas: (u32, u32), fit: Fit) -> Res<Vec<u8>> {
    let picture = image::ImageReader::open(path)
        .map_err(|e| super::kind::open_error(path, &e))?
        .with_guessed_format()
        .map_err(|e| format!("could not read {}: {e}", path.display()))?
        .decode()
        .map_err(|e| format!("{} is not a picture this plugin reads: {e}", path.display()))?
        .into_rgba8();
    let (w, h) = picture.dimensions();
    draw_rgba(picture.as_raw(), (w, h), canvas, fit)
}

/// RGBA pixels onto a black I420 canvas.
pub fn draw_rgba(rgba: &[u8], size: (u32, u32), canvas: (u32, u32), fit: Fit) -> Res<Vec<u8>> {
    let (cw, ch) = canvas;
    let place = Place::new(size, (1, 1), canvas, fit).luma();
    let (dx, dy, dw, dh) = place.dst;
    let (sx, sy, sw, sh) = place.src;
    let src = ImageRef::new(size.0, size.1, rgba, PixelType::U8x4).map_err(|e| format!("the picture is the wrong size: {e}"))?;
    let mut scaled = Image::new(dw, dh, PixelType::U8x4);
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::CatmullRom)).crop(sx as f64, sy as f64, sw as f64, sh as f64);
    Resizer::new().resize(&src, &mut scaled, &options).map_err(|e| format!("could not scale the picture: {e}"))?;
    let mut out = black(cw, ch);
    to_i420(scaled.buffer(), (dw, dh), (dx, dy), (cw, ch), &mut out);
    Ok(out)
}

/// A black I420 frame.
pub fn black(width: u32, height: u32) -> Vec<u8> {
    let luma = (width * height) as usize;
    let chroma = (width.div_ceil(2) * height.div_ceil(2)) as usize;
    let mut f = vec![16u8; luma + 2 * chroma];
    f[luma..].fill(128);
    f
}

/// RGBA (alpha over black) into the canvas at `at`, BT.709 limited range.
fn to_i420(rgba: &[u8], size: (u32, u32), at: (u32, u32), canvas: (u32, u32), out: &mut [u8]) {
    let (w, h) = (size.0 as usize, size.1 as usize);
    let (cw, ch) = (canvas.0 as usize, canvas.1 as usize);
    let (x0, y0) = (at.0 as usize, at.1 as usize);
    let half = cw.div_ceil(2);
    let (luma, chroma) = out.split_at_mut(cw * ch);
    let (u, v) = chroma.split_at_mut(chroma.len() / 2);
    let rgb = |x: usize, y: usize| -> [f32; 3] {
        let p = &rgba[(y * w + x) * 4..(y * w + x) * 4 + 4];
        let a = p[3] as f32 / 255.0 / 255.0;
        [p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a]
    };
    for y in 0..h {
        for x in 0..w {
            let [r, g, b] = rgb(x, y);
            luma[(y0 + y) * cw + x0 + x] = (16.0 + 46.559 * r + 156.629 * g + 15.812 * b).round() as u8;
        }
    }
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let mut sum = [0f32; 3];
            let mut n = 0.0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                if x + dx < w && y + dy < h {
                    let p = rgb(x + dx, y + dy);
                    sum = [sum[0] + p[0], sum[1] + p[1], sum[2] + p[2]];
                    n += 1.0;
                }
            }
            let [r, g, b] = sum.map(|s| s / n);
            let at = (y0 + y) / 2 * half + (x0 + x) / 2;
            u[at] = (128.0 - 25.664 * r - 86.336 * g + 112.0 * b).round().clamp(16.0, 240.0) as u8;
            v[at] = (128.0 + 112.0 * r - 101.730 * g - 10.270 * b).round().clamp(16.0, 240.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_red_and_transparent_land_where_bt709_puts_them() {
        // Two by two: white, red, then two transparent pixels.
        let rgba = [255, 255, 255, 255, 255, 0, 0, 255, 0, 0, 0, 0, 9, 9, 9, 0];
        let f = draw_rgba(&rgba, (2, 2), (2, 2), Fit::Stretch).unwrap();
        assert_eq!(f[0], 235, "white");
        assert_eq!(f[1], 63, "red's luma");
        assert_eq!(f[2], 16, "transparent is black");
    }

    #[test]
    fn a_square_picture_on_a_wide_canvas_has_black_bars() {
        let rgba = vec![255u8; 16 * 16 * 4];
        let f = draw_rgba(&rgba, (16, 16), (64, 36), Fit::Fit).unwrap();
        assert_eq!(f[0], 16);
        assert_eq!(f[18 * 64 + 32], 235);
    }
}
