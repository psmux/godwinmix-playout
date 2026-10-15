//! The playlist's controls, as tools. Every one is reachable from the panel,
//! the CLI, MCP and any client of the mixer's protocol with
//! `tool.call {"name": "playout/<tool>", "source": "<source id>", ...}`.
//!
//! The list itself is not edited here: it is the source's params, changed
//! with `source.set`, which is what the project file saves.

use crate::engine::Cmd;
use crate::settings::Settings;
use crate::source::Playout;
use crate::state;
use godwinmix_sdk::prelude::*;
use serde_json::{json, Value};

pub fn title(settings: &Settings, id: &str) -> String {
    settings.items.iter().find(|i| i.id == id).map_or_else(|| id.to_string(), |i| {
        if !i.title.is_empty() {
            return i.title.clone();
        }
        std::path::Path::new(&i.path).file_name().map_or(i.path.clone(), |n| n.to_string_lossy().into_owned())
    })
}

pub fn call(p: &mut Playout, params: Value) -> Result<Value, RpcError> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let structured = match name.as_str() {
        "state" => state_of(p, args.get("items_since").and_then(Value::as_u64)),
        "thumbnail" => thumbnail(p, &args)?,
        "play" => command(p, Cmd::Play, "playing")?,
        "pause" => command(p, Cmd::Pause, "paused")?,
        "next" => command(p, Cmd::Next, "moving to the next item")?,
        "previous" => command(p, Cmd::Previous, "moving back")?,
        "play_item" => {
            let id = item_id(p, &args)?;
            command(p, Cmd::PlayItem(id.clone()), &format!("playing {} now", title(&p.settings, &id)))?
        }
        "cue" => {
            let id = match args.get("id").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                Some(_) => Some(item_id(p, &args)?),
                None => None,
            };
            let said = id.as_ref().map_or("cue cleared".to_string(), |i| format!("{} plays next", title(&p.settings, i)));
            command(p, Cmd::Cue(id), &said)?
        }
        "seek" => {
            let ms = args.get("position_ms").and_then(Value::as_u64).ok_or_else(|| {
                RpcError::new(codes::INVALID_PARAMS, "seek takes position_ms, milliseconds from the item's in point. Example: {\"position_ms\": 30000}")
            })?;
            command(p, Cmd::Seek(ms), &format!("moving to {}", clock(ms)))?
        }
        other => {
            return Err(RpcError::new(
                codes::METHOD_NOT_FOUND,
                format!("there is no tool '{other}'. The tools are state, play, pause, next, previous, play_item, cue, seek and thumbnail"),
            )
            .with_data(json!({"tool": other, "retryable": false})))
        }
    };
    let text = structured.get("said").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| summary(&structured));
    Ok(json!({"content": [{"type": "text", "text": text}], "structuredContent": structured}))
}

fn state_of(p: &Playout, items_since: Option<u64>) -> Value {
    let snap = state::read(&p.snapshot);
    match p.catalog.as_ref() {
        Some(c) => crate::report::state(&p.settings, &snap, c, items_since),
        None => json!({"mode": "idle", "items": []}),
    }
}

fn command(p: &mut Playout, cmd: Cmd, said: &str) -> Result<Value, RpcError> {
    if !p.send(cmd) {
        return Err(RpcError::new(
            -32001,
            "the playlist is not running, so there is nothing to control. Put the source on a scene, or call source.restart",
        )
        .with_data(json!({"state": "stopped", "retryable": true})));
    }
    Ok(json!({"said": said}))
}

fn item_id(p: &Playout, args: &Value) -> Result<String, RpcError> {
    let id = args.get("id").and_then(Value::as_str).unwrap_or_default();
    if p.settings.items.iter().any(|i| i.id == id) {
        return Ok(id.to_string());
    }
    let known: Vec<&str> = p.settings.items.iter().map(|i| i.id.as_str()).take(20).collect();
    Err(RpcError::new(codes::INVALID_PARAMS, format!("there is no item '{id}' in this playlist. Its items are: {}. Call state for their titles", known.join(", ")))
        .with_data(json!({"id": id, "items": known})))
}

fn thumbnail(p: &Playout, args: &Value) -> Result<Value, RpcError> {
    let id = item_id(p, args)?;
    let width = args.get("width").and_then(Value::as_u64).unwrap_or(160) as u32;
    let path = p.settings.items.iter().find(|i| i.id == id).map(|i| i.path.clone()).unwrap_or_default();
    let thumb = p.catalog.as_ref().and_then(|c| c.thumb(&path, width));
    Ok(match thumb {
        Some(uri) => json!({"id": id, "ready": true, "uri": uri, "said": "thumbnail ready"}),
        None => json!({"id": id, "ready": false, "said": "being made; ask again once thumbs_made in state goes up"}),
    })
}

fn summary(state: &Value) -> String {
    let mode = state.get("mode").and_then(Value::as_str).unwrap_or("idle");
    let count = state.pointer("/list/count").and_then(Value::as_u64).unwrap_or(0);
    let left = state.pointer("/list/remaining_ms").and_then(Value::as_u64).map(clock);
    match left {
        Some(l) => format!("{mode}, {count} items, {l} to the end of the list"),
        None => format!("{mode}, {count} items"),
    }
}

pub fn clock(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}
