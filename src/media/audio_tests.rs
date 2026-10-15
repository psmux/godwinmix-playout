use super::*;
use symphonia::core::audio::Channels;

#[test]
fn xiph_lacing_splits_three_headers() {
    let data = [&[2u8, 3, 2][..], b"abc", b"de", b"fghij"].concat();
    let h = xiph_headers(&data).unwrap();
    assert_eq!(h, [b"abc".to_vec(), b"de".to_vec(), b"fghij".to_vec()]);
}

#[test]
fn mono_is_heard_in_both_ears() {
    let spec = SignalSpec::new(44_100, Channels::FRONT_LEFT);
    let mut out = Vec::new();
    to_stereo(&[0.5, -0.25], &spec, &mut out);
    assert_eq!(out, [0.5, 0.5, -0.25, -0.25]);
}

#[test]
fn a_six_channel_fold_down_does_not_clip() {
    let spec = SignalSpec::new(48_000, Channels::FRONT_LEFT | Channels::FRONT_RIGHT | Channels::FRONT_CENTRE | Channels::LFE1 | Channels::REAR_LEFT | Channels::REAR_RIGHT);
    let mut out = Vec::new();
    to_stereo(&[1.0; 6], &spec, &mut out);
    assert!(out.iter().all(|s| *s <= 1.0001), "{out:?}");
}
