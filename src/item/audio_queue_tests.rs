use super::*;

fn frames(n: usize, v: f32) -> Vec<f32> {
    vec![v; n * 2]
}

#[test]
fn priming_before_the_start_is_cut_off() {
    let q = AudioQueue::new(0);
    // 1024 frames that start 21.33 ms before zero, then real sound.
    q.push(-21_333_334, &frames(1024, 9.0));
    q.push(0, &frames(480, 1.0));
    let mut out = vec![0.0; 960];
    assert_eq!(q.take(&mut out), 480);
    assert!(out.iter().all(|s| *s == 1.0), "nothing of the priming is heard");
}

#[test]
fn a_hole_in_the_sound_is_filled_with_silence() {
    let q = AudioQueue::new(0);
    q.push(0, &frames(480, 1.0));
    q.push(20_000_000, &frames(480, 1.0));
    assert_eq!(q.frames(), 1440);
    let mut out = vec![5.0; 2880];
    q.take(&mut out);
    assert_eq!(out[960], 0.0);
    assert_eq!(out[1920], 1.0);
}

#[test]
fn a_queue_seeked_to_two_seconds_drops_what_is_before() {
    let q = AudioQueue::new(2_000_000_000);
    q.push(1_990_000_000, &frames(960, 1.0));
    assert_eq!(q.frames(), 480);
}

#[test]
fn sound_that_arrives_late_is_trimmed_not_played_late() {
    let q = AudioQueue::new(0);
    let mut out = vec![0.0; 960];
    assert_eq!(q.take(&mut out), 0, "nothing there yet: silence went out");
    q.push(0, &frames(960, 1.0));
    assert_eq!(q.frames(), 480, "the first 10 ms already passed and are dropped");
}

#[test]
fn drained_means_done_and_empty() {
    let q = AudioQueue::new(0);
    q.push(0, &frames(10, 1.0));
    q.finish();
    assert!(!q.drained());
    q.take(&mut [0.0; 20]);
    assert!(q.drained());
}
