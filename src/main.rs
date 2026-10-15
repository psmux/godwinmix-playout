//! The engine binary: the protocol loop around `godwinmix_playout::source`.

use godwinmix_playout::{gst_runtime, source};
use godwinmix_sdk::prelude::*;
fn main() {
    // First, before any thread: on Windows this may put GStreamer on PATH.
    gst_runtime::locate();
    let env = PluginEnv::from_env();
    let manifest = match Manifest::load(env.root.join("gmx-plugin.toml")) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("playout could not read its manifest: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = runtime::run(&manifest, SourceHandler(source::Playout::new())) {
        eprintln!("playout stopped: {e}");
        std::process::exit(1);
    }
}
