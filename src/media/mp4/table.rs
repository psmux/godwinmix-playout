//! A track's sample table, expanded into one entry per sample.
//!
//! An hour of 30 fps video is 108,000 entries of 32 bytes, about 3.5 MB,
//! which buys a seek that is a binary search and a read loop with no table
//! arithmetic in it.

use super::boxes::{child, full, Be};

#[derive(Debug, Clone, Copy, Default)]
pub struct Sample {
    pub offset: u64,
    pub size: u32,
    /// Decode time, in the track's timescale.
    pub dts: u64,
    /// Composition offset, in the track's timescale.
    pub cto: i64,
    pub sync: bool,
}

/// Every sample in `stbl`, or a sentence saying which table is missing.
pub fn samples(stbl: &[u8]) -> Result<Vec<Sample>, String> {
    let sizes = sizes(stbl).ok_or("the file has no sample sizes (stsz)")?;
    let offsets = chunk_offsets(stbl).ok_or("the file has no chunk offsets (stco)")?;
    let stsc = child(stbl, b"stsc").ok_or("the file has no sample to chunk table (stsc)")?;
    let mut out: Vec<Sample> = sizes.iter().map(|&size| Sample { size, sync: true, ..Default::default() }).collect();
    place_in_chunks(&mut out, stsc, &offsets);
    times(&mut out, child(stbl, b"stts").ok_or("the file has no sample times (stts)")?);
    if let Some(ctts) = child(stbl, b"ctts") {
        composition(&mut out, ctts);
    }
    if let Some(stss) = child(stbl, b"stss") {
        sync(&mut out, stss);
    }
    Ok(out)
}

fn sizes(stbl: &[u8]) -> Option<Vec<u32>> {
    if let Some(stsz) = child(stbl, b"stsz") {
        let mut r = Be::new(full(stsz).1);
        let fixed = r.u32();
        let count = r.u32() as usize;
        return Some(if fixed != 0 { vec![fixed; count] } else { (0..count).map(|_| r.u32()).collect() });
    }
    let stz2 = child(stbl, b"stz2")?;
    let mut r = Be::new(full(stz2).1);
    r.skip(3);
    let bits = r.u8();
    let count = r.u32() as usize;
    Some(match bits {
        16 => (0..count).map(|_| r.u16() as u32).collect(),
        8 => (0..count).map(|_| r.u8() as u32).collect(),
        _ => (0..count).map(|i| {
            let b = r.data.get(r.at + i / 2).copied().unwrap_or(0);
            if i % 2 == 0 { (b >> 4) as u32 } else { (b & 15) as u32 }
        }).collect(),
    })
}

fn chunk_offsets(stbl: &[u8]) -> Option<Vec<u64>> {
    if let Some(stco) = child(stbl, b"stco") {
        let mut r = Be::new(full(stco).1);
        let n = r.u32() as usize;
        return Some((0..n).map(|_| r.u32() as u64).collect());
    }
    let co64 = child(stbl, b"co64")?;
    let mut r = Be::new(full(co64).1);
    let n = r.u32() as usize;
    Some((0..n).map(|_| r.u64()).collect())
}

fn place_in_chunks(out: &mut [Sample], stsc: &[u8], offsets: &[u64]) {
    let mut r = Be::new(full(stsc).1);
    let runs: Vec<(usize, usize)> = (0..r.u32()).map(|_| {
        let first = r.u32() as usize;
        let per = r.u32() as usize;
        r.skip(4);
        (first.max(1) - 1, per)
    }).collect();
    let mut sample = 0usize;
    for (i, &(first, per)) in runs.iter().enumerate() {
        let end = runs.get(i + 1).map_or(offsets.len(), |r| r.0);
        for &chunk_offset in offsets.iter().take(end).skip(first) {
            let mut at = chunk_offset;
            for s in out.iter_mut().skip(sample).take(per) {
                s.offset = at;
                at += s.size as u64;
            }
            sample += per;
        }
    }
}

fn times(out: &mut [Sample], stts: &[u8]) {
    let mut r = Be::new(full(stts).1);
    let mut i = 0usize;
    let mut t = 0u64;
    for _ in 0..r.u32() {
        let count = r.u32() as usize;
        let delta = r.u32() as u64;
        for s in out.iter_mut().skip(i).take(count) {
            s.dts = t;
            t += delta;
        }
        i += count;
    }
}

fn composition(out: &mut [Sample], ctts: &[u8]) {
    let (version, body) = full(ctts);
    let mut r = Be::new(body);
    let mut i = 0usize;
    for _ in 0..r.u32() {
        let count = r.u32() as usize;
        let raw = r.u32();
        let offset = if version == 0 { raw as i64 } else { raw as i32 as i64 };
        for s in out.iter_mut().skip(i).take(count) {
            s.cto = offset;
        }
        i += count;
    }
}

fn sync(out: &mut [Sample], stss: &[u8]) {
    for s in out.iter_mut() {
        s.sync = false;
    }
    let mut r = Be::new(full(stss).1);
    for _ in 0..r.u32() {
        if let Some(s) = (r.u32() as usize).checked_sub(1).and_then(|i| out.get_mut(i)) {
            s.sync = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(kind);
        v.extend_from_slice(body);
        v
    }

    fn words(w: &[u32]) -> Vec<u8> {
        [&[0u8; 4][..], &w.iter().flat_map(|x| x.to_be_bytes()).collect::<Vec<_>>()].concat()
    }

    #[test]
    fn samples_land_in_their_chunks_with_their_times() {
        // Five samples of 10 bytes: two chunks of two, then one of one.
        let stbl = [
            bx(b"stsz", &words(&[10, 5])),
            bx(b"stco", &words(&[3, 100, 200, 300])),
            bx(b"stsc", &words(&[2, 1, 2, 1, 3, 1, 1])),
            bx(b"stts", &words(&[1, 5, 512])),
            bx(b"stss", &words(&[2, 1, 4])),
        ]
        .concat();
        let s = samples(&stbl).unwrap();
        let offsets: Vec<u64> = s.iter().map(|s| s.offset).collect();
        assert_eq!(offsets, [100, 110, 200, 210, 300]);
        assert_eq!(s[4].dts, 2048);
        assert!(s[0].sync && !s[1].sync && s[3].sync);
    }

    #[test]
    fn a_missing_table_is_named() {
        let e = samples(&bx(b"stco", &words(&[0]))).unwrap_err();
        assert!(e.contains("stsz"), "{e}");
    }
}
