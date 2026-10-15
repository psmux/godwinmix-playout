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
