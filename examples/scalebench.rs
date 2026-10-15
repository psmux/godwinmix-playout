//! How long drawing one decoded frame onto the canvas takes.
//!
//!     cargo run --release --example scalebench
use godwinmix_playout::media::picture::{Layout, Plane, Scaler};
use godwinmix_playout::settings::Fit;
use std::time::Instant;

fn main() {
    for (sw, sh) in [(1280u32, 720u32), (1920, 1080), (3840, 2160), (720, 1280)] {
        let y = vec![100u8; (sw * sh) as usize];
        let uv = vec![128u8; (sw * sh / 2) as usize];
        let layout = Layout::Nv12 { y: Plane { data: &y, stride: sw as usize, width: sw, height: sh }, uv: Plane { data: &uv, stride: sw as usize, width: sw / 2, height: sh / 2 } };
        let mut s = Scaler::new(1920, 1080, Fit::Fit);
        let mut out = vec![0u8; 1920 * 1080 * 3 / 2];
        s.draw(&layout, (1, 1), &mut out).unwrap();
        let t = Instant::now();
        for _ in 0..60 {
            s.draw(&layout, (1, 1), &mut out).unwrap();
        }
        println!("{sw}x{sh} NV12 to 1920x1080: {:.2} ms a frame", t.elapsed().as_secs_f64() * 1000.0 / 60.0);
    }
}
