# AGENTS.md

For a coding agent working on this plugin. Read GodwinMix's own AGENTS.md for
the rules a plugin is held to; the ones that bite here are below.

## What this is

A GodwinMix source plugin in Rust. One process per playlist source, started by
the core. The list is the source's params; playback is driven by tools. Media
goes to the core as Matroska on stdout, so **nothing in this process may print
to stdout**. Log with the `Reporter`.

## Build and test

```sh
cargo build --release
cargo test --release                     # unit tests and tests/transcript.jsonl
cargo clippy --release --workspace --all-targets -- -D warnings
python dev/capture.py --seconds 20 FILES # the engine with no core
cargo run --release --example gaps -- recording.mkv   # measure for gaps
```

Copy `target/release/playout-engine(.exe)` (and `playout` on macOS and Linux)
into `bin/` before `gmx plugin add .` or `gmx plugin test .`.

## The rules that matter here

1. **The output thread never waits.** `src/engine/` runs one tick per canvas
   frame and only ever `try_recv`s. Opening a file, decoding, scaling and
   tearing down a GStreamer pipeline all happen on an item's own threads
   (`src/item/`). A file on a slow share must not stall the picture.
2. **Timestamps come from the tick count.** Never from a file. That is what
   keeps them stepping by exactly one frame through item changes, pauses and
   failures.
3. **Sound is kept on the item's clock in frames of 48 kHz** (`AudioQueue`).
   Taking always moves time on; late sound is trimmed, not played late.
4. **No FFmpeg.** Decoders are named in `src/media/video_gst.rs`; never use
   `decodebin` or anything that can autoplug `avdec_*`. Prefer Rust for
   anything that is not video decode.
5. **Each crate earns its place.** Say in one sentence why, in Cargo.toml, if
   you add one.
6. **Every error names the next step**, in words an operator can act on.
7. **Files under 150 lines.** Split a module before it grows past that.

## Where things are

| Path | What it is |
|---|---|
| `src/engine/` | the output thread, commands, item changes, the snapshot |
| `src/item/` | one item: open, feed, convert, the audio queue, its status |
| `src/media/` | MP4 and Matroska readers, sound decode, resampling, scaling, pictures |
| `src/queue.rs` | the play order, pure logic, `queue_tests.rs` beside it |
| `src/tools.rs`, `src/report.rs`, `src/source.rs` | the protocol side |
| `src/gst_runtime.rs`, `launch/` | finding GStreamer on each platform |
| `ui/` | the panel; plain ES modules, no build step |
| `tests/transcript.jsonl` | the offline conversation; do not edit it to pass a test |
