//! `playout`: finds GStreamer for `playout-engine`, then becomes it.
//!
//! The engine links GStreamer, and on macOS the dynamic linker looks for it
//! only where the engine was built, which on a GodwinMix desktop app user's
//! Mac is not where it is: the app carries its own copy inside the bundle.
//! This binary links nothing, so it always starts. It works out where the
//! GStreamer libraries are, points the dynamic linker there, and replaces
//! itself with the engine (exec), so the core is talking to the engine
//! directly on the same stdin, stdout and stderr, with the same process id.
//!
//! Where it looks, first match wins:
//! 1. `GMX_PLAYOUT_GST_LIB`, for anyone with GStreamer somewhere unusual;
//! 2. the `lib` folder above `GST_PLUGIN_SYSTEM_PATH`, which the mixer sets
//!    to its bundled plugins, so this finds the bundle wherever the app is;
//! 3. the desktop app in /Applications, GStreamer.framework, Homebrew;
//! 4. on Linux, nothing: the system's GStreamer is where the linker looks.

#[cfg(unix)]
fn main() {
    use std::os::unix::process::CommandExt;
    let engine = match std::env::current_exe() {
        Ok(me) => me.with_file_name("playout-engine"),
        Err(e) => fail(&format!("cannot find myself: {e}")),
    };
    let mut cmd = std::process::Command::new(&engine);
    cmd.args(std::env::args_os().skip(1));
    if let Some(lib) = find() {
        let var = if cfg!(target_os = "macos") { "DYLD_LIBRARY_PATH" } else { "LD_LIBRARY_PATH" };
        let joined = match std::env::var_os(var) {
            Some(old) if !old.is_empty() => {
                let mut v = lib.into_os_string();
                v.push(":");
                v.push(old);
                v
            }
            _ => lib.into_os_string(),
        };
        cmd.env(var, joined);
    }
    let e = cmd.exec();
    fail(&format!("could not start {}: {e}", engine.display()));
}

#[cfg(unix)]
fn find() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let lib_name = if cfg!(target_os = "macos") { "libgstreamer-1.0.0.dylib" } else { "libgstreamer-1.0.so.0" };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::var_os("GMX_PLAYOUT_GST_LIB") {
        candidates.push(dir.into());
    }
    if let Some(plugins) = std::env::var_os("GST_PLUGIN_SYSTEM_PATH") {
        for p in std::env::split_paths(&plugins) {
            if let Some(lib) = p.parent() {
                candidates.push(lib.to_path_buf());
            }
        }
    }
    if cfg!(target_os = "macos") {
        for dir in [
            "/Applications/GodwinMix.app/Contents/Resources/gstreamer/macos/lib",
            "/Library/Frameworks/GStreamer.framework/Versions/1.0/lib",
            "/opt/homebrew/lib",
            "/usr/local/lib",
        ] {
            candidates.push(dir.into());
        }
    }
    candidates.into_iter().find(|d| d.join(lib_name).is_file())
}

#[cfg(unix)]
fn fail(why: &str) -> ! {
    eprintln!("playout: {why}");
    std::process::exit(2);
}

/// On Windows the engine finds GStreamer by itself (its DLLs are delay
/// loaded) and the manifest starts it directly. This only says so.
#[cfg(not(unix))]
fn main() {
    eprintln!("playout: on Windows the manifest starts playout-engine.exe directly; this launcher is for macOS and Linux");
    std::process::exit(2);
}
