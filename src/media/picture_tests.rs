use super::*;

fn plane(data: &[u8], w: u32, h: u32, stride: usize) -> Plane<'_> {
    Plane { data, stride, width: w, height: h }
}

#[test]
fn an_i420_frame_at_canvas_size_is_copied_exactly() {
    let (w, h) = (64u32, 36u32);
    let y: Vec<u8> = (0..w * h).map(|i| (i % 200) as u8 + 16).collect();
    let u = vec![90u8; (w * h / 4) as usize];
    let v = vec![170u8; (w * h / 4) as usize];
    let mut out = vec![0u8; (w * h * 3 / 2) as usize];
    let mut s = Scaler::new(w, h, Fit::Fit);
    let layout = Layout::I420 { y: plane(&y, w, h, w as usize), u: plane(&u, w / 2, h / 2, w as usize / 2), v: plane(&v, w / 2, h / 2, w as usize / 2) };
    s.draw(&layout, (1, 1), &mut out).unwrap();
    assert_eq!(&out[..y.len()], &y[..]);
    assert!(out[y.len()..y.len() + u.len()].iter().all(|b| *b == 90));
    assert!(out[y.len() + u.len()..].iter().all(|b| *b == 170));
}

#[test]
fn a_padded_stride_is_read_past() {
    let (w, h) = (8u32, 4u32);
    let mut y = vec![0u8; 16 * 4];
    for row in 0..4 {
        y[row * 16..row * 16 + 8].fill(100);
    }
    let u = vec![128u8; 8 * 2];
    let mut out = vec![0u8; (w * h * 3 / 2) as usize];
    let layout = Layout::I420 { y: plane(&y, w, h, 16), u: plane(&u, 4, 2, 8), v: plane(&u, 4, 2, 8) };
    Scaler::new(w, h, Fit::Fit).draw(&layout, (1, 1), &mut out).unwrap();
    assert!(out[..32].iter().all(|b| *b == 100));
}

#[test]
fn a_small_nv12_portrait_is_scaled_up_with_black_bars() {
    let (w, h) = (36u32, 64u32);
    let y = vec![200u8; (w * h) as usize];
    let uv: Vec<u8> = std::iter::repeat_n([60u8, 190u8], (w * h / 4) as usize).flatten().collect();
    let (cw, ch) = (128u32, 72u32);
    let mut out = vec![0u8; (cw * ch * 3 / 2) as usize];
    let layout = Layout::Nv12 { y: plane(&y, w, h, w as usize), uv: plane(&uv, w / 2, h / 2, w as usize) };
    Scaler::new(cw, ch, Fit::Fit).draw(&layout, (1, 1), &mut out).unwrap();
    let luma = &out[..(cw * ch) as usize];
    assert_eq!(luma[0], 16, "the left bar is black");
    assert_eq!(luma[(36 * cw + 64) as usize], 200, "the middle is the picture");
    let u = &out[(cw * ch) as usize..(cw * ch + cw * ch / 4) as usize];
    let v = &out[(cw * ch + cw * ch / 4) as usize..];
    assert_eq!(u[0], 128);
    assert_eq!(u[(18 * cw / 2 + 32) as usize], 60);
    assert_eq!(v[(18 * cw / 2 + 32) as usize], 190);
}
