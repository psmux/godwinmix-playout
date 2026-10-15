//! Play one file through an item player with no core, and say what came out.
//!
//!     cargo run --example item -- path/to/file.mp4 [seconds]
//!
//! Prints the decoder, the first frame's time and how many frames and how
//! much sound arrived. For finding out why a file will not play.

use godwinmix_playout::item::ItemPlayer;
use godwinmix_playout::settings::{Item, Settings};
use godwinmix_sdk::wire::Canvas;
use std::time::{Duration, Instant};

fn main() {
    godwinmix_playout::gst_runtime::locate();
    let path = std::env::args().nth(1).expect("a file to play");
    let secs: f64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3.0);
    let item = Item { id: "x".into(), path, title: String::new(), in_ms: 0, out_ms: 0, still_ms: 0, hold: false };
    let canvas = Canvas::new(1280, 720, 30);
    let mut p = ItemPlayer::open(&item, &Settings::default(), canvas, 0, false);
    let start = Instant::now();
    let (mut frames, mut first) = (0, None);
    let mut sound = vec![0f32; 3200];
    let mut heard = 0usize;
    let mut tick = 0u64;
    while start.elapsed().as_secs_f64() < secs {
        let t = tick * 1_000_000_000 / 30;
        if p.ready() {
            if let Some(f) = p.frame_at(t) {
                frames += 1;
                first.get_or_insert((start.elapsed(), f.pts));
            }
            heard += p.audio().take(&mut sound);
            tick += 1;
        }
        if let Some(e) = p.error() {
            println!("error: {e}");
            break;
        }
        if p.ended(t) {
            println!("ended at {} ms", t / 1_000_000);
            break;
        }
        std::thread::sleep(Duration::from_millis(33));
    }
    println!("{:?} {:?}", p.media(), p.status());
    println!("first frame {:?}, {frames} frames shown, {} ms of sound", first, heard * 1000 / 48_000);
}
