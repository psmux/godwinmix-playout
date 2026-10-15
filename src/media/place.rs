//! Where a picture of one shape goes on a canvas of another. Arithmetic
//! only, so every case is a unit test.

use crate::settings::Fit;

/// A rectangle of the source and the rectangle of the canvas it fills, each
/// `(x, y, width, height)` in pixels of the plane being drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rects {
    pub src: (u32, u32, u32, u32),
    pub dst: (u32, u32, u32, u32),
}

impl Rects {
    /// Nothing to scale: the whole source lands at its own size.
    pub fn is_copy(&self, width: u32, height: u32) -> bool {
        self.src == (0, 0, width, height) && self.dst.2 == width && self.dst.3 == height
    }

    fn halved(&self) -> Rects {
        let h = |r: (u32, u32, u32, u32)| (r.0 / 2, r.1 / 2, r.2.div_ceil(2).max(1), r.3.div_ceil(2).max(1));
        Rects { src: h(self.src), dst: h(self.dst) }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Place {
    luma: Rects,
    canvas: (u32, u32),
}

impl Place {
    pub fn new(src: (u32, u32), par: (u32, u32), canvas: (u32, u32), fit: Fit) -> Place {
        let (w, h) = (src.0.max(2), src.1.max(2));
        let (cw, ch) = canvas;
        let shape = w as f64 * par.0.max(1) as f64 / (h as f64 * par.1.max(1) as f64);
        let room = cw as f64 / ch as f64;
        let whole_src = (0, 0, w, h);
        let whole_dst = (0, 0, cw, ch);
        let luma = match fit {
            Fit::Stretch => Rects { src: whole_src, dst: whole_dst },
            Fit::Fit if shape > room => {
                let dh = even(cw as f64 / shape).min(ch);
                Rects { src: whole_src, dst: (0, even_floor((ch - dh) / 2), cw, dh) }
            }
            Fit::Fit => {
                let dw = even(ch as f64 * shape).min(cw);
                Rects { src: whole_src, dst: (even_floor((cw - dw) / 2), 0, dw, ch) }
            }
            Fit::Fill if shape > room => {
                let sw = even(w as f64 * room / shape).min(w);
                Rects { src: (even_floor((w - sw) / 2), 0, sw, h), dst: whole_dst }
            }
            Fit::Fill => {
                let sh = even(h as f64 * shape / room).min(h);
                Rects { src: (0, even_floor((h - sh) / 2), w, sh), dst: whole_dst }
            }
        };
        Place { luma, canvas }
    }

    pub fn luma(&self) -> Rects {
        self.luma
    }

    pub fn chroma(&self) -> Rects {
        self.luma.halved()
    }

    /// Whether part of the canvas is left uncovered and needs painting black.
    pub fn bars(&self) -> bool {
        self.luma.dst != (0, 0, self.canvas.0, self.canvas.1)
    }
}

fn even(v: f64) -> u32 {
    ((v / 2.0).round() as u32 * 2).max(2)
}

fn even_floor(v: u32) -> u32 {
    v & !1
}

#[cfg(test)]
mod tests {
    use super::*;

    const HD: (u32, u32) = (1920, 1080);

    #[test]
    fn the_same_shape_and_size_is_a_copy() {
        let p = Place::new(HD, (1, 1), HD, Fit::Fit);
        assert!(p.luma().is_copy(1920, 1080));
        assert!(!p.bars());
    }

    #[test]
    fn a_portrait_clip_is_pillarboxed_in_the_middle() {
        let p = Place::new((720, 1280), (1, 1), HD, Fit::Fit);
        let (x, y, w, h) = p.luma().dst;
        assert_eq!((y, h), (0, 1080));
        assert_eq!(w, 608);
        assert_eq!(x, (1920 - 608) / 2);
        assert!(p.bars());
    }

    #[test]
    fn four_by_three_filling_sixteen_by_nine_loses_top_and_bottom() {
        let p = Place::new((640, 480), (1, 1), HD, Fit::Fill);
        assert_eq!(p.luma().dst, (0, 0, 1920, 1080));
        assert_eq!(p.luma().src, (0, 60, 640, 360));
    }

    #[test]
    fn anamorphic_pixels_are_honoured() {
        // DV widescreen: 720x576 with 64:45 pixels is 16:9.
        let p = Place::new((720, 576), (64, 45), HD, Fit::Fit);
        assert_eq!(p.luma().dst, (0, 0, 1920, 1080));
    }

    #[test]
    fn chroma_is_half_of_everything_rounded_up() {
        let p = Place::new((720, 1280), (1, 1), (1280, 720), Fit::Fit);
        let c = p.chroma();
        assert_eq!(c.dst.3, 360);
        assert_eq!(c.src, (0, 0, 360, 640));
    }
}
