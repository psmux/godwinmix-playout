//! What an operator can set on a playlist, read from the source's params.
//!
//! The params are the playlist. They are what `source.set` changes, what the
//! project file keeps, and what `configure` hands over in full every time, so
//! there is no second copy of the list anywhere that could disagree with them.

use serde::Deserialize;
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

pub use crate::choices::{AtEnd, Fit, Repeat, SoundPicture};

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
    pub sound_picture: SoundPicture,
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
            sound_picture: SoundPicture::Black,
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
#[path = "settings_tests.rs"]
mod tests;
