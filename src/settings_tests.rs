use super::*;
use serde_json::json;

#[test]
fn an_empty_object_is_an_empty_list_that_loops() {
    let s = Settings::from_params(&json!({})).unwrap();
    assert!(s.items.is_empty());
    assert_eq!(s.repeat, Repeat::All);
    assert!(s.autoplay);
}

#[test]
fn items_without_ids_get_stable_distinct_ones() {
    let p = json!({"items": [{"path": "a.mp4"}, {"path": "a.mp4"}, {"path": "b.mp4", "id": "b"}]});
    let one = Settings::from_params(&p).unwrap();
    let two = Settings::from_params(&p).unwrap();
    assert_eq!(one.items, two.items);
    assert_ne!(one.items[0].id, one.items[1].id);
    assert_eq!(one.items[2].id, "b");
}

#[test]
fn duplicate_ids_are_made_unique() {
    let p = json!({"items": [{"path": "a.mp4", "id": "x"}, {"path": "b.mp4", "id": "x"}]});
    let s = Settings::from_params(&p).unwrap();
    assert_ne!(s.items[0].id, s.items[1].id);
}

#[test]
fn loop_and_fit_read_their_words() {
    let s = Settings::from_params(&json!({"loop": "one", "fit": "fill", "at_end": "hold"})).unwrap();
    assert_eq!(s.repeat, Repeat::One);
    assert_eq!(s.fit, Fit::Fill);
    assert_eq!(s.at_end, AtEnd::Hold);
}

#[test]
fn a_still_takes_its_own_time_or_the_lists() {
    let s = Settings::from_params(&json!({"still_seconds": 4})).unwrap();
    let mut item = Item { id: "a".into(), path: "a.png".into(), title: String::new(), in_ms: 0, out_ms: 0, still_ms: 0, hold: false };
    assert_eq!(s.still_ns(&item), 4_000_000_000);
    item.still_ms = 1500;
    assert_eq!(s.still_ns(&item), 1_500_000_000);
}
