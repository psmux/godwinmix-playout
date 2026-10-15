//! Measure a recording for gaps: every picture's timestamp, black frames,
//! silence, and where the tone changes from one item's pitch to the next.
//!
//!     cargo run --release --example gaps -- programme.mkv
//!
//! Made for a playlist of test files that each carry a steady tone at a pitch
//! of its own, recorded off the programme. An item change then shows up as a
//! change of pitch, and a gap as silence or a repeated timestamp between two
//! pitches. Reads Matroska with the plugin's own demuxer and decoders.

use godwinmix_playout::media::audio::AudioDecoder;
use godwinmix_playout::media::picture::Scaler;
use godwinmix_playout::media::resample::Resampler;
use godwinmix_playout::media::video_gst::{Pulled, VideoDecoder};
use godwinmix_playout::media::{frame, open_demux, Packet};
use godwinmix_playout::settings::Fit;

fn main() {
    godwinmix_playout::gst_runtime::locate();
    let path = std::env::args().nth(1).expect("a recording");
    let mut demux = open_demux(std::path::Path::new(&path)).expect("open the recording");
    let info = demux.info().clone();
    let decoder = std::sync::Arc::new(VideoDecoder::new(info.video.as_ref().expect("a picture"), true).expect("a decoder"));
    let mut sound = AudioDecoder::new(info.audio.as_ref().expect("sound")).expect("a sound decoder");
    let mut resampler: Option<Resampler> = None;
    // Pictures are pulled on a thread of their own, as the plugin does: a
    // push into a full decoder waits for a pull, so one thread doing both
    // can wait on itself.
    let puller = decoder.clone();
    let pictures = std::thread::spawn(move || {
        let mut scaler = Scaler::new(64, 36, Fit::Stretch);
        let mut small = vec![0u8; 64 * 36 * 3 / 2];
        let (mut pts, mut luma) = (Vec::new(), Vec::new());
        loop {
            match puller.pull(2000) {
                Pulled::Frame(s) => {
                    pts.push(frame::pts(&s).unwrap_or(0));
                    frame::draw(&s, &mut scaler, &mut small).expect("draw");
                    luma.push(small[..64 * 36].iter().map(|v| *v as u64).sum::<u64>() / (64 * 36));
                }
                Pulled::End | Pulled::Wait => break,
                Pulled::Failed(e) => panic!("{e}"),
            }
        }
        (pts, luma)
    });
    let (mut pcm, mut raw) = (Vec::new(), Vec::new());
    let mut first_audio: Option<i64> = None;
    while let Some(packet) = demux.next().expect("read") {
        match packet {
            Packet::Video { pts: p, dts, key, data } => decoder.push(p, dts, key, data).expect("push"),
            Packet::Audio { pts: p, data } => {
                first_audio.get_or_insert(p);
                raw.clear();
                if let Some((rate, _)) = sound.decode(data, p, &mut raw) {
                    resampler.get_or_insert_with(|| Resampler::new(rate)).push(&raw, &mut pcm);
                }
            }
        }
    }
    decoder.end();
    raw.clear();
    sound.finish(&mut raw);
    if let Some(r) = resampler.as_mut() {
        r.push(&raw, &mut pcm);
        r.flush(&mut pcm);
    }
    let (pts, luma) = pictures.join().expect("the picture thread");
    report_video(&pts, &luma);
    report_sound(&pcm, first_audio.unwrap_or(0));
}

fn report_video(pts: &[u64], luma: &[u64]) {
    let steps: Vec<i64> = pts.windows(2).map(|w| w[1] as i64 - w[0] as i64).collect();
    let nominal = median(&steps);
    let odd: Vec<(usize, i64)> = steps.iter().copied().enumerate().filter(|(_, s)| (s - nominal).abs() > 1_000_000).collect();
    println!("video: {} frames over {:.3} s, step {:.3} ms", pts.len(), (pts.last().unwrap_or(&0) - pts[0]) as f64 / 1e9, nominal as f64 / 1e6);
    println!("video: {} steps off the nominal by more than 1 ms", odd.len());
    for (i, s) in odd.iter().take(10) {
        println!("    at frame {i} ({:.3} s): {:.3} ms", pts[*i] as f64 / 1e9, *s as f64 / 1e6);
    }
    let black: Vec<usize> = luma.iter().enumerate().filter(|(_, l)| **l < 20).map(|(i, _)| i).collect();
    println!("video: {} black frames (mean luma under 20)", black.len());
    for i in black.iter().take(10) {
        println!("    black at frame {i} ({:.3} s)", pts[*i] as f64 / 1e9);
    }
}

fn report_sound(pcm: &[f32], start_ns: i64) {
    // 5 ms windows of the left channel: loudness, and pitch from zero crossings.
    let win = 240usize;
    let left: Vec<f32> = pcm.chunks_exact(2).map(|f| f[0]).collect();
    // Pitch is read over 20 ms from the window's start: 5 ms of a 330 Hz tone
    // has too few zero crossings to tell it from 300.
    let windows: Vec<(f32, f32)> = (0..left.len() / win).map(|i| {
        let w = &left[i * win..(i + 1) * win];
        let rms = (w.iter().map(|s| s * s).sum::<f32>() / w.len() as f32).sqrt();
        let span = &left[i * win..(i * win + 4 * win).min(left.len())];
        let crossings = span.windows(2).filter(|p| (p[0] < 0.0) != (p[1] < 0.0)).count();
        (rms, crossings as f32 * 48_000.0 / 2.0 / span.len() as f32)
    }).collect();
    let t = |i: usize| start_ns as f64 / 1e9 + i as f64 * 0.005;
    println!("sound: {:.3} s", left.len() as f64 / 48_000.0);
    let silent: Vec<usize> = windows.iter().enumerate().filter(|(_, w)| w.0 < 0.005).map(|(i, _)| i).collect();
    println!("sound: {} silent 5 ms windows", silent.len());
    for run in runs(&silent).iter().take(12) {
        println!("    silence {:.3} s to {:.3} s ({} ms)", t(run.0), t(run.1 + 1), (run.1 + 1 - run.0) * 5);
    }
    // Runs of one pitch: a window joins the run while it is within 60 Hz of
    // the run's mean, and silence is a run of its own. Each item of a test
    // list has a tone of its own, so each run is an item.
    let mut segs: Vec<(usize, usize, f32)> = Vec::new();
    for (i, w) in windows.iter().enumerate() {
        let p = if w.0 < 0.005 { 0.0 } else { w.1 };
        match segs.last_mut() {
            Some((_, end, mean)) if (p - *mean).abs() < 60.0 && (p == 0.0) == (*mean == 0.0) => {
                let n = (*end + 1 - (i - (*end + 1 - i.min(*end + 1)))) as f32;
                *mean += (p - *mean) / n.max(1.0);
                *end = i;
            }
            _ => segs.push((i, i, p)),
        }
    }
    for (a, b, mean) in segs.iter().filter(|s| s.1 + 1 - s.0 >= 4) {
        println!("    {:8.3} s to {:8.3} s  {:5} ms  {}", t(*a), t(*b + 1), (b + 1 - a) * 5,
                 if *mean == 0.0 { "silence".to_string() } else { format!("{:.0} Hz", mean) });
    }
    let short: Vec<_> = segs.iter().filter(|s| s.1 + 1 - s.0 < 4).collect();
    println!("sound: {} runs shorter than 20 ms (the edges between tones)", short.len());
}

fn runs(idx: &[usize]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for &i in idx {
        match out.last_mut() {
            Some(r) if r.1 + 1 == i => r.1 = i,
            _ => out.push((i, i)),
        }
    }
    out
}

fn median(v: &[i64]) -> i64 {
    let mut s = v.to_vec();
    s.sort();
    s.get(s.len() / 2).copied().unwrap_or(0)
}
