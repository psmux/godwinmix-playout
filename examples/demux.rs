//! List the first packets a file demuxes to, in Rust, with their times.
//!
//!     cargo run --example demux -- path/to/file.mkv [count]

use godwinmix_playout::media::{open_demux, Packet};

fn main() {
    let path = std::env::args().nth(1).expect("a file");
    let n: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(12);
    let mut d = open_demux(std::path::Path::new(&path)).expect("open");
    println!("{:?}", d.info());
    let (mut v, mut a) = (0, 0);
    for i in 0.. {
        match d.next() {
            Ok(Some(Packet::Video { pts, dts, key, data })) => {
                v += 1;
                if i < n { println!("video pts {pts} dts {dts:?} key {key} {} bytes", data.len()) }
            }
            Ok(Some(Packet::Audio { pts, data })) => {
                a += 1;
                if i < n { println!("audio pts {pts} {} bytes", data.len()) }
            }
            Ok(None) => break,
            Err(e) => { println!("error {e}"); break }
        }
    }
    println!("{v} video packets, {a} audio packets");
}
