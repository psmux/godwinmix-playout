//! What an operator can set on a playlist, read from the source's params.
//!
//! The params are the playlist. They are what `source.set` changes, what the
//! project file keeps, and what `configure` hands over in full every time, so
//! there is no second copy of the list anywhere that could disagree with them.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One entry in the list.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Item {
    /// A short id, unique in this list. The panel makes one; an entry without
    /// one gets one made from its path and position.
    #[serde(default)]
    pub id: String,
    /// The file, on the machine the mixer runs on.
    pub path: String,
    /// What the panel shows. The file name when empty.
    #[serde(default)]
    pub title: String,
    /// Where to start, in milliseconds from the start of the file.
    #[serde(default)]
    pub in_ms: u64,
    /// Where to stop. Zero or absent means the end of the file.
    #[serde(default)]
    pub out_ms: u64,
    /// How long a picture stays up. Zero means the playlist's `still_seconds`.
    #[serde(default)]
    pub still_ms: u64,
    /// Stop on this item's last frame and wait for play or next.
    #[serde(default)]
    pub hold: bool,
}

/// How the list repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    /// Back to the top after the last item.
    #[default]
    All,
    /// The current item, again and again.
    One,
    /// Stop after the last item.
    Off,
}

/// What the source shows once a list that does not repeat has run out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AtEnd {
    #[default]
    Black,
    Hold,
}

/// How a picture of another shape is put on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// All of the picture, with bars where the shapes differ.
    #[default]
    Fit,
    /// The canvas filled, with the picture's edges cut off.
    Fill,
    /// The canvas filled, with the picture squashed or stretched.
    Stretch,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub items: Vec<Item>,
    #[serde(rename = "loop")]
    pub repeat: Repeat,
    pub shuffle: bool,
    pub at_end: AtEnd,
    pub still_seconds: f64,
    pub fit: Fit,
    /// Start playing as soon as the source starts.
    pub autoplay: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            items: Vec::new(),
            repeat: Repeat::All,
            shuffle: false,
            at_end: AtEnd::Black,
            still_seconds: 10.0,
            fit: Fit::Fit,
            autoplay: true,
        }
    }
}

impl Settings {
    /// Read the params. Anything that does not parse falls back to its
    /// default rather than failing the source: the core has already checked
    /// the params against the schema, so this only meets what the schema let
    /// through.
    pub fn from_params(params: &Value) -> Result<Settings, String> {
        let mut settings: Settings = serde_json::from_value(params.clone())
            .map_err(|e| format!("the playlist settings did not read: {e}. Check them against schemas/playlist.json"))?;
        settings.still_seconds = settings.still_seconds.clamp(0.5, 86_400.0);
        give_ids(&mut settings.items);
        Ok(settings)
    }

    /// How long a picture stays up, for this item.
    pub fn still_ns(&self, item: &Item) -> u64 {
        if item.still_ms > 0 {
            item.still_ms * 1_000_000
        } else {
            (self.still_seconds * 1e9) as u64
        }
    }
}

/// Make sure every item has an id and no two share one. An id made here
/// comes from the path and how many times that path has appeared so far, so
/// the same list read twice gets the same ids.
fn give_ids(items: &mut [Item]) {
    let mut seen = std::collections::HashSet::new();
    let mut count: std::collections::HashMap<String, u32> = Default::default();
    for item in items.iter_mut() {
        let n = count.entry(item.path.clone()).or_default();
        *n += 1;
        if item.id.is_empty() || seen.contains(&item.id) {
            item.id = format!("i{:08x}{}", hash(&item.path), n);
        }
        while seen.contains(&item.id) {
            item.id.push('x');
        }
        seen.insert(item.id.clone());
    }
}

fn hash(s: &str) -> u32 {
    // FNV-1a: stable across runs and platforms, which the standard hasher is not.
    s.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

#[cfg(test)]
mod tests {
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
}
