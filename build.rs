//! On Windows, load GStreamer's DLLs on first use instead of at start.
//!
//! Then the engine starts on a machine where GStreamer is somewhere Windows
//! does not look, finds it (src/gst_runtime.rs), puts it on PATH, and only
//! then calls into it. Without GStreamer at all it still starts, and plays
//! sound and pictures; video items say what is missing. The mixer's own
//! binary does the same (crates/godwinmix/build.rs in GodwinMix).

const DELAYED: &[&str] = &[
    "glib-2.0-0.dll",
    "gobject-2.0-0.dll",
    "gstreamer-1.0-0.dll",
    "gstapp-1.0-0.dll",
    "gstvideo-1.0-0.dll",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("TARGET").unwrap_or_default().ends_with("windows-msvc") {
        for dll in DELAYED {
            println!("cargo:rustc-link-arg-bins=/DELAYLOAD:{dll}");
        }
        println!("cargo:rustc-link-arg-bins=delayimp.lib");
    }
}
