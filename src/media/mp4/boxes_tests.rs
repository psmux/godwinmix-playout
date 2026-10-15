use super::*;

fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    v.extend_from_slice(kind);
    v.extend_from_slice(body);
    v
}

#[test]
fn nested_boxes_are_found_by_path() {
    let inner = bx(b"mdhd", &[1, 2, 3]);
    let outer = bx(b"mdia", &inner);
    let top = [bx(b"free", &[]), bx(b"trak", &outer)].concat();
    assert_eq!(path(&top, &[b"trak", b"mdia", b"mdhd"]), Some(&[1u8, 2, 3][..]));
    assert_eq!(path(&top, &[b"trak", b"minf"]), None);
}

#[test]
fn a_box_running_past_the_end_stops_the_walk() {
    let mut data = bx(b"free", &[0; 4]);
    data.extend_from_slice(&[0, 0, 0, 99, b'm', b'o', b'o', b'v']);
    assert_eq!(children(&data).len(), 1);
}

#[test]
fn esds_gives_the_aac_config() {
    let dsi = [0x11, 0x90];
    let dcd = [&[4u8, 15, 0x40, 0x15][..], &[0; 11], &[5, 2], &dsi].concat();
    let es = [&[3u8, (3 + dcd.len()) as u8, 0, 1, 0][..], &dcd].concat();
    let body = [&[0u8, 0, 0, 0][..], &es].concat();
    assert_eq!(esds(&body), Some((0x40, dsi.to_vec())));
}
