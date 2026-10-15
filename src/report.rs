//! The `state` tool's answer: the list as the panel draws it, with lengths,
//! what is playing and how long is left of it and of the whole list.

use crate::catalog::Catalog;
use crate::settings::{Item, Settings};
use crate::state::{Mode, Snapshot};
use serde_json::{json, Value};

/// How long `item` plays for, once its file has been looked at.
pub fn item_ms(item: &Item, settings: &Settings, catalog: &Catalog) -> Option<u64> {
    let probe = catalog.probe(&item.path)?;
    if probe.error.is_some() {
        return None;
    }
    if probe.kind == "picture" {
        return Some(settings.still_ns(item) / 1_000_000);
    }
    let end = match (probe.duration_ms, item.out_ms) {
        (Some(d), 0) => d,
        (Some(d), o) => d.min(o),
        (None, 0) => return None,
        (None, o) => o,
    };
    Some(end.saturating_sub(item.in_ms))
}

pub fn state(settings: &Settings, snap: &Snapshot, catalog: &Catalog) -> Value {
    let items: Vec<Value> = settings.items.iter().map(|i| item(i, settings, snap, catalog)).collect();
    let total: u64 = settings.items.iter().filter_map(|i| item_ms(i, settings, catalog)).sum();
    let unknown = settings.items.iter().filter(|i| catalog.probe(&i.path).is_none()).count();
    let remaining_item = snap.length_ms.map(|l| l.saturating_sub(snap.position_ms));
    json!({
        "mode": snap.mode,
        "current": snap.current.as_ref().map(|id| json!({
            "id": id,
            "position_ms": snap.position_ms,
            "length_ms": snap.length_ms,
            "remaining_ms": remaining_item,
            "decoder": snap.decoder,
        })),
        "next": snap.next.clone(),
        "next_ready": snap.next_ready,
        "cued": snap.cued.clone(),
        "order": snap.order.clone(),
        "items": items,
        "list": {
            "count": settings.items.len(),
            "total_ms": total,
            "unknown_lengths": unknown,
            "remaining_ms": list_remaining(settings, snap, catalog, remaining_item),
        },
        "loop": settings.repeat,
        "shuffle": settings.shuffle,
        "at_end": settings.at_end,
        "fit": settings.fit,
        "frames": snap.frames,
        "repeated_frames": snap.repeats,
        "changes": snap.changes,
        "thumbs_made": catalog.thumbs_made(),
        "stopped": snap.stopped.clone(),
    })
}

fn item(i: &Item, settings: &Settings, snap: &Snapshot, catalog: &Catalog) -> Value {
    let probe = catalog.probe(&i.path);
    let error = snap.errors.get(&i.id).cloned().or_else(|| probe.as_ref().and_then(|p| p.error.clone()));
    let title = if i.title.is_empty() {
        std::path::Path::new(&i.path).file_name().map_or(i.path.clone(), |n| n.to_string_lossy().into_owned())
    } else {
        i.title.clone()
    };
    json!({
        "id": i.id,
        "title": title,
        "path": i.path,
        "kind": probe.as_ref().map(|p| p.kind.clone()),
        "length_ms": item_ms(i, settings, catalog),
        "file_ms": probe.as_ref().and_then(|p| p.duration_ms),
        "width": probe.as_ref().and_then(|p| p.width),
        "height": probe.as_ref().and_then(|p| p.height),
        "codec": probe.as_ref().and_then(|p| p.codec.clone()),
        "in_ms": i.in_ms,
        "out_ms": i.out_ms,
        "still_ms": i.still_ms,
        "hold": i.hold,
        "error": error,
        "probed": probe.is_some(),
        "playing": snap.current.as_deref() == Some(i.id.as_str()) && snap.mode != Mode::Idle,
    })
}

/// From now to the end of this pass of the list: what is left of the
/// current item and the length of every item after it in the play order.
/// Items that will not play count nothing; items not looked at yet count
/// nothing either, and `unknown_lengths` says how many there are.
fn list_remaining(settings: &Settings, snap: &Snapshot, catalog: &Catalog, current_left: Option<u64>) -> Option<u64> {
    let at = snap.current.as_ref().and_then(|c| snap.order.iter().position(|o| o == c))?;
    let mut left = current_left?;
    for id in &snap.order[at + 1..] {
        if snap.errors.contains_key(id) {
            continue;
        }
        if let Some(item) = settings.items.iter().find(|i| &i.id == id) {
            left += item_ms(item, settings, catalog).unwrap_or(0);
        }
    }
    Some(left)
}
