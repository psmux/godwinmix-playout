use super::*;

fn items(ids: &[&str]) -> Vec<Item> {
    ids.iter()
        .map(|id| Item { id: id.to_string(), path: format!("{id}.mp4"), title: String::new(), in_ms: 0, out_ms: 0, still_ms: 0, hold: false })
        .collect()
}

fn queue(ids: &[&str]) -> Queue {
    let mut q = Queue::new(42);
    q.update(&items(ids), false, None);
    q
}

fn next(q: &mut Queue, current: Option<&str>, repeat: Repeat) -> Option<String> {
    q.after(current, repeat).map(|n| n.id)
}

/// Play the whole thing from the top, `n` times, and say what played.
fn play(q: &mut Queue, repeat: Repeat, n: usize) -> Vec<String> {
    let mut played = Vec::new();
    let mut current: Option<String> = None;
    for _ in 0..n {
        let Some(n) = q.after(current.as_deref(), repeat) else { break };
        q.advance(&n.id, n.wraps);
        played.push(n.id.clone());
        current = Some(n.id);
    }
    played
}

#[test]
fn a_list_plays_in_order_and_comes_back_to_the_top() {
    let mut q = queue(&["a", "b", "c"]);
    assert_eq!(play(&mut q, Repeat::All, 7), ["a", "b", "c", "a", "b", "c", "a"]);
}

#[test]
fn a_list_that_does_not_loop_stops_after_the_last() {
    let mut q = queue(&["a", "b"]);
    assert_eq!(play(&mut q, Repeat::Off, 5), ["a", "b"]);
}

#[test]
fn loop_one_repeats_the_current_item() {
    let mut q = queue(&["a", "b"]);
    q.advance("b", false);
    assert_eq!(next(&mut q, Some("b"), Repeat::One).as_deref(), Some("b"));
}

#[test]
fn a_cued_item_plays_next_once_and_the_list_carries_on_from_it() {
    let mut q = queue(&["a", "b", "c", "d"]);
    q.advance("a", false);
    q.cue(Some("c".into()));
    assert_eq!(next(&mut q, Some("a"), Repeat::All).as_deref(), Some("c"));
    q.advance("c", false);
    assert_eq!(q.cued(), None);
    assert_eq!(next(&mut q, Some("c"), Repeat::All).as_deref(), Some("d"));
}

#[test]
fn a_removed_current_item_is_followed_by_what_slid_into_its_place() {
    let mut q = queue(&["a", "b", "c"]);
    q.advance("b", false);
    q.update(&items(&["a", "c"]), false, Some("b"));
    assert_eq!(next(&mut q, Some("b"), Repeat::All).as_deref(), Some("c"));
}

#[test]
fn moving_items_changes_what_comes_next() {
    let mut q = queue(&["a", "b", "c"]);
    q.advance("a", false);
    q.update(&items(&["a", "c", "b"]), false, Some("a"));
    assert_eq!(next(&mut q, Some("a"), Repeat::All).as_deref(), Some("c"));
}

#[test]
fn failed_items_are_skipped_and_tried_again_next_pass() {
    let mut q = queue(&["a", "b", "c"]);
    q.advance("a", false);
    q.mark_failed("b");
    assert_eq!(next(&mut q, Some("a"), Repeat::All).as_deref(), Some("c"));
    q.advance("c", false);
    let wrap = q.after(Some("c"), Repeat::All).unwrap();
    assert!(wrap.wraps);
    q.advance(&wrap.id, wrap.wraps);
    assert!(!q.is_failed("b"), "a new pass gives a failed item another chance");
}

#[test]
fn a_shuffle_plays_every_item_once_per_pass() {
    let ids = ["a", "b", "c", "d", "e", "f", "g", "h"];
    let mut q = Queue::new(7);
    q.update(&items(&ids), true, None);
    let mut first = play(&mut q, Repeat::All, 8);
    let mut second = play(&mut q, Repeat::All, 8);
    first.sort();
    second.sort();
    assert_eq!(first, ids);
    assert_eq!(second, ids);
}

#[test]
fn turning_shuffle_on_keeps_the_current_item_first() {
    let mut q = queue(&["a", "b", "c", "d"]);
    q.advance("c", false);
    q.update(&items(&["a", "b", "c", "d"]), true, Some("c"));
    assert_eq!(q.order()[0], "c");
}

#[test]
fn an_item_added_to_a_shuffle_plays_this_pass() {
    let mut q = Queue::new(3);
    q.update(&items(&["a", "b", "c"]), true, None);
    let first = q.after(None, Repeat::All).unwrap();
    q.advance(&first.id, false);
    q.update(&items(&["a", "b", "c", "z"]), true, Some(&first.id));
    let at = q.order().iter().position(|id| id == "z").unwrap();
    let current = q.order().iter().position(|id| *id == first.id).unwrap();
    assert!(at > current, "z lands after the current item, not before it");
}

#[test]
fn previous_steps_back_and_stops_at_the_top() {
    let mut q = queue(&["a", "b", "c"]);
    q.advance("c", false);
    assert_eq!(q.before(Some("c")).as_deref(), Some("b"));
    assert_eq!(q.before(Some("a")), None);
}

#[test]
fn an_empty_list_has_nothing_next() {
    let mut q = queue(&[]);
    assert_eq!(q.after(None, Repeat::All), None);
}
