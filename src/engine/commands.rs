//! What the control side asks of the output thread. Commands are read at the
//! top of a tick, never waited for.

use super::{Engine, Prepared};
use crate::item::ItemPlayer;
use crate::settings::Settings;
use crate::state::Mode;

#[derive(Debug)]
pub enum Cmd {
    /// New params: the list, the loop, the shuffle.
    Settings(Settings),
    Play,
    Pause,
    /// Play the next item now.
    Next,
    Previous,
    /// Play this item now.
    PlayItem(String),
    /// Play this item after the current one, or forget the cue.
    Cue(Option<String>),
    /// Move in the current item, in milliseconds from its in point.
    Seek(u64),
}

impl Engine {
    pub(super) fn commands(&mut self) {
        while let Ok(cmd) = self.cmds.try_recv() {
            self.command(cmd);
        }
    }

    fn command(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Settings(s) => self.new_settings(s),
            Cmd::Play => self.play(),
            Cmd::Pause => {
                if self.mode == Mode::Playing {
                    self.mode = Mode::Paused;
                }
            }
            Cmd::Next => {
                let after = self.queue.after(self.current_id().as_deref(), self.no_repeat_one());
                if let Some(n) = after {
                    self.play_now(&n.id, 0, n.wraps);
                }
            }
            Cmd::Previous => self.previous(),
            Cmd::PlayItem(id) => self.play_now(&id, 0, false),
            Cmd::Cue(id) => {
                self.queue.cue(id);
                self.drop_stale_next();
            }
            Cmd::Seek(ms) => {
                if let Some(id) = self.current_id() {
                    self.play_now(&id, ms, false);
                }
            }
        }
    }

    fn new_settings(&mut self, s: Settings) {
        let current = self.current_id();
        self.queue.update(&s.items, s.shuffle, current.as_deref());
        let was_empty = self.settings.items.is_empty();
        // A changed list gets every item another try.
        self.errors.clear();
        self.failures_in_row = 0;
        self.settings = s;
        self.drop_stale_next();
        if self.gave_up && !self.settings.items.is_empty() {
            self.gave_up = false;
            self.mode = Mode::Playing;
            self.queue.reset();
        }
        if was_empty && self.mode == Mode::Idle && self.settings.autoplay && !self.settings.items.is_empty() {
            self.mode = Mode::Playing;
        }
    }

    fn play(&mut self) {
        match self.mode {
            Mode::Paused => self.mode = Mode::Playing,
            Mode::Holding => {
                self.mode = Mode::Playing;
                self.go_next();
            }
            Mode::Idle | Mode::Ended => {
                self.mode = Mode::Playing;
                self.current = None;
                self.failures_in_row = 0;
                self.queue.reset();
            }
            Mode::Playing => {}
        }
    }

    fn previous(&mut self) {
        // Within the first three seconds, the item before; after that, the
        // start of this one, the way every player does it.
        let into = self.item_ns().saturating_sub(self.current.as_ref().map_or(0, |c| c.player.start_ns));
        let id = match self.current_id() {
            Some(c) if into > 3_000_000_000 => Some(c),
            current => self.queue.before(current.as_deref()).or(current),
        };
        if let Some(id) = id {
            self.play_now(&id, 0, false);
        }
    }

    /// Open `id` at `ms` from its in point, to take over as soon as it has a
    /// picture. The current item plays on until then.
    pub(super) fn play_now(&mut self, id: &str, ms: u64, wraps: bool) {
        let Some(item) = self.settings.items.iter().find(|i| i.id == id).cloned() else {
            self.warn(format!("there is no item '{id}' in this playlist"));
            return;
        };
        if ms == 0 && self.next.as_ref().is_some_and(|n| n.player.id == id) {
            // Already open and drawing: take it as it is, which is instant.
            self.pending = self.next.take().map(|n| Prepared { wraps: wraps || n.wraps, ..n });
            if matches!(self.mode, Mode::Idle | Mode::Ended | Mode::Holding) {
                self.mode = Mode::Playing;
            }
            return;
        }
        let start = item.in_ms * 1_000_000 + ms * 1_000_000;
        let player = ItemPlayer::open(&item, &self.settings, self.canvas, start, self.software_only);
        self.pending = Some(Prepared { player, wraps });
        if matches!(self.mode, Mode::Idle | Mode::Ended | Mode::Holding) {
            self.mode = Mode::Playing;
        }
    }

    pub(super) fn current_id(&self) -> Option<String> {
        self.current.as_ref().map(|c| c.player.id.clone())
    }

    /// "Next" means the next item even when the list repeats one.
    fn no_repeat_one(&self) -> crate::settings::Repeat {
        match self.settings.repeat {
            crate::settings::Repeat::One => crate::settings::Repeat::All,
            r => r,
        }
    }
}
