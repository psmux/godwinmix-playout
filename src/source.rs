//! The plugin as the core sees it: a source with settings, health and tools.

use crate::catalog::Catalog;
use crate::engine::{Cmd, Engine, Setup};
use crate::settings::Settings;
use crate::state::{self, Mode, SharedSnapshot};
use godwinmix_sdk::media::{self, Streams, VideoFormat};
use godwinmix_sdk::prelude::*;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

struct Running {
    cmds: Sender<Cmd>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

pub struct Playout {
    pub(crate) canvas: Canvas,
    pub(crate) settings: Settings,
    reporter: Option<Reporter>,
    pub(crate) snapshot: SharedSnapshot,
    pub(crate) catalog: Option<Catalog>,
    running: Option<Running>,
    software_only: bool,
}

impl Playout {
    pub fn new() -> Playout {
        Playout {
            canvas: Canvas::default(),
            settings: Settings::default(),
            reporter: None,
            snapshot: Default::default(),
            catalog: None,
            running: None,
            software_only: std::env::var_os("GMX_PLAYOUT_SOFTWARE").is_some(),
        }
    }

    fn take_params(&mut self, params: &Value) -> Result<(), RpcError> {
        self.settings = Settings::from_params(params).map_err(|e| RpcError::new(codes::INVALID_PARAMS, e))?;
        if let Some(c) = self.catalog.as_ref() {
            c.watch(self.settings.items.iter().map(|i| i.path.as_str()));
        }
        Ok(())
    }

    /// Tell the output thread. False when it is not running.
    pub(crate) fn send(&self, cmd: Cmd) -> bool {
        self.running.as_ref().is_some_and(|r| r.cmds.send(cmd).is_ok())
    }
}

impl Default for Playout {
    fn default() -> Self {
        Playout::new()
    }
}

impl Source for Playout {
    fn initialize(&mut self, ready: &Ready, reporter: Reporter) -> Result<InitializeResult, RpcError> {
        self.canvas = ready.canvas;
        self.catalog = Some(Catalog::new((ready.canvas.width, ready.canvas.height)));
        self.take_params(&ready.params)?;
        match crate::gst_runtime::found() {
            Ok(at) => reporter.info(format!("playout '{}' at {}x{}@{}, GStreamer from {at}", ready.instance, ready.canvas.width, ready.canvas.height, ready.canvas.fps)),
            Err(why) => reporter.warn(why),
        }
        self.reporter = Some(reporter);
        Ok(InitializeResult { latency_ms: Some(0) })
    }

    fn start(&mut self, params: &StartParams) -> Result<StartResult, RpcError> {
        self.stop()?;
        let canvas = params.canvas;
        self.canvas = canvas;
        let writer = media::open(params.transport, &params.media, canvas, Streams::video_and_audio(VideoFormat::I420))
            .map_err(|e| RpcError::new(codes::INTERNAL_ERROR, e.to_string()))?;
        let (cmds, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let engine = Engine::new(Setup {
            canvas,
            writer,
            reporter: self.reporter.clone(),
            settings: self.settings.clone(),
            cmds: rx,
            snapshot: self.snapshot.clone(),
            software_only: self.software_only,
        });
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("playout-output".into())
            .spawn(move || engine.run(flag))
            .map_err(|e| RpcError::new(codes::INTERNAL_ERROR, format!("could not start the output thread: {e}")))?;
        self.running = Some(Running { cmds, stop, thread: Some(thread) });
        Ok(StartResult { latency_ms: Some(0) })
    }

    fn stop(&mut self) -> Result<(), RpcError> {
        if let Some(mut r) = self.running.take() {
            r.stop.store(true, Ordering::Relaxed);
            if let Some(t) = r.thread.take() {
                let _ = t.join();
            }
        }
        Ok(())
    }

    fn configure(&mut self, params: Value) -> Result<Configure, RpcError> {
        self.take_params(&params)?;
        self.send(Cmd::Settings(self.settings.clone()));
        Ok(Configure::applied())
    }

    fn health(&mut self) -> Health {
        let s = state::read(&self.snapshot);
        if let Some(why) = s.stopped {
            return Health::failing(why);
        }
        let detail = match (s.mode, s.current.as_deref()) {
            (Mode::Playing, Some(id)) => format!("playing {}", crate::tools::title(&self.settings, id)),
            (Mode::Idle, _) if self.settings.items.is_empty() => "the playlist is empty; add files in the panel".into(),
            (mode, _) => format!("{mode:?}").to_lowercase(),
        };
        if !s.errors.is_empty() && s.errors.len() >= self.settings.items.len() {
            return Health::degraded(format!("nothing in the playlist will play: {}", s.errors.values().next().cloned().unwrap_or_default()));
        }
        Health { state: HealthState::Ok, detail: Some(detail), latency_ms: Some(0) }
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, RpcError> {
        match method {
            "tool.call" => crate::tools::call(self, params),
            other => Err(RpcError::new(codes::METHOD_NOT_FOUND, format!("this plugin has no method '{other}'. Its controls are tools: call tool.call with one of the names in gmx-plugin.toml"))
                .with_data(serde_json::json!({"method": other, "retryable": false}))),
        }
    }
}
