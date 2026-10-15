//! What is known about each file in the list: its probe, and its thumbnail
//! once somebody has asked for one. Worked out on one background thread,
//! never on the output thread or the protocol thread.

use crate::media::probe::{probe, Probe};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// A file as it was when it was looked at. A file changed since is looked
/// at again.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Stamp {
    path: String,
    modified: Option<SystemTime>,
    size: u64,
}

fn stamp(path: &str) -> Stamp {
    let meta = std::fs::metadata(path).ok();
    Stamp { path: path.into(), modified: meta.as_ref().and_then(|m| m.modified().ok()), size: meta.map_or(0, |m| m.len()) }
}

enum Job {
    Probe(String),
    Thumb(String, u32),
}

#[derive(Default)]
struct Known {
    probes: HashMap<String, (Stamp, Probe)>,
    thumbs: HashMap<(String, u32), String>,
    asked: std::collections::HashSet<(String, u32)>,
    /// Counts thumbnails made, so a panel knows when to ask again.
    made: u64,
}

#[derive(Clone)]
pub struct Catalog {
    known: Arc<Mutex<Known>>,
    jobs: Sender<Job>,
}

impl Catalog {
    pub fn new(canvas: (u32, u32)) -> Catalog {
        let known = Arc::new(Mutex::new(Known::default()));
        let (jobs, rx) = channel::<Job>();
        let k = known.clone();
        let _ = std::thread::Builder::new().name("playout-catalog".into()).spawn(move || {
            while let Ok(job) = rx.recv() {
                match job {
                    Job::Probe(path) => {
                        let s = stamp(&path);
                        let p = probe(&PathBuf::from(&path));
                        k.lock().unwrap_or_else(|e| e.into_inner()).probes.insert(path, (s, p));
                    }
                    Job::Thumb(path, width) => {
                        let t = crate::thumbs::make(&PathBuf::from(&path), width, canvas).unwrap_or_default();
                        let mut known = k.lock().unwrap_or_else(|e| e.into_inner());
                        known.thumbs.insert((path, width), t);
                        known.made += 1;
                    }
                }
            }
        });
        Catalog { known, jobs }
    }

    /// Look at every path not looked at yet, or changed since.
    pub fn watch<'a>(&self, paths: impl Iterator<Item = &'a str>) {
        let known = self.known.lock().unwrap_or_else(|e| e.into_inner());
        let mut queued = std::collections::HashSet::new();
        for path in paths {
            let fresh = known.probes.get(path).is_some_and(|(s, _)| *s == stamp(path));
            if !fresh && queued.insert(path.to_string()) {
                let _ = self.jobs.send(Job::Probe(path.to_string()));
            }
        }
    }

    pub fn probe(&self, path: &str) -> Option<Probe> {
        self.known.lock().unwrap_or_else(|e| e.into_inner()).probes.get(path).map(|(_, p)| p.clone())
    }

    /// A thumbnail as a data URI, or `None` while it is being made. An empty
    /// string means there is no picture to make one from.
    pub fn thumb(&self, path: &str, width: u32) -> Option<String> {
        let mut known = self.known.lock().unwrap_or_else(|e| e.into_inner());
        let key = (path.to_string(), width);
        if let Some(t) = known.thumbs.get(&key) {
            return Some(t.clone());
        }
        if known.asked.insert(key) {
            let _ = self.jobs.send(Job::Thumb(path.to_string(), width));
        }
        None
    }

    pub fn thumbs_made(&self) -> u64 {
        self.known.lock().unwrap_or_else(|e| e.into_inner()).made
    }
}
