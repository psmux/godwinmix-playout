//! Finding GStreamer, which this plugin uses for video decode and nothing else.
//!
//! On Windows the GStreamer DLLs are delay loaded (see build.rs), so the
//! engine runs before they are found. `locate` looks where the mixer and the
//! official installer put them and adds that folder to PATH before anything
//! calls into GStreamer:
//!
//! 1. a folder already on PATH holding `gstreamer-1.0-0.dll`, which is the
//!    case when the desktop app or `godwinmix` started this process;
//! 2. the `bin` folder beside `GST_PLUGIN_SYSTEM_PATH`, which the mixer sets;
//! 3. `GSTREAMER_1_0_ROOT_MSVC_X86_64`, set by GStreamer's own installer;
//! 4. the desktop app's folder and GStreamer's default install folder.
//!
//! On macOS and Linux the `playout` launcher has already pointed the dynamic
//! linker at the right folder before this binary loaded (launch/src/main.rs).

use std::sync::OnceLock;

static FOUND: OnceLock<Result<String, String>> = OnceLock::new();
static INIT: OnceLock<Result<(), String>> = OnceLock::new();

/// Call first thing in `main`, before any thread starts: it may set PATH.
pub fn locate() {
    let _ = FOUND.set(find());
}

/// Where GStreamer was found, for the log.
pub fn found() -> Result<String, String> {
    FOUND.get().cloned().unwrap_or_else(|| Err("locate was never called".into()))
}

/// GStreamer, initialised once, or why it cannot be.
pub fn ready() -> Result<(), String> {
    INIT.get_or_init(|| {
        found()?;
        gstreamer::init().map_err(|e| format!("GStreamer would not start: {e}"))
    })
    .clone()
}

#[cfg(not(windows))]
fn find() -> Result<String, String> {
    Ok("the system's dynamic linker (set up by the playout launcher)".into())
}

#[cfg(windows)]
fn find() -> Result<String, String> {
    use std::path::PathBuf;
    const DLL: &str = "gstreamer-1.0-0.dll";
    let path = std::env::var_os("PATH").unwrap_or_default();
    if let Some(dir) = std::env::split_paths(&path).find(|d| d.join(DLL).is_file()) {
        return Ok(dir.display().to_string());
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(plugins) = std::env::var_os("GST_PLUGIN_SYSTEM_PATH") {
        for p in std::env::split_paths(&plugins) {
            if let Some(root) = p.parent().and_then(|lib| lib.parent()) {
                candidates.push(root.join("bin"));
            }
        }
    }
    for var in ["GSTREAMER_1_0_ROOT_MSVC_X86_64", "GSTREAMER_1_0_ROOT_X86_64"] {
        if let Some(root) = std::env::var_os(var) {
            candidates.push(PathBuf::from(root).join("bin"));
        }
    }
    for base in ["ProgramFiles", "LOCALAPPDATA"] {
        if let Some(b) = std::env::var_os(base) {
            candidates.push(PathBuf::from(&b).join("GodwinMix").join("gstreamer").join("windows").join("bin"));
            candidates.push(PathBuf::from(&b).join("Programs").join("GodwinMix").join("gstreamer").join("windows").join("bin"));
        }
    }
    candidates.push(PathBuf::from(r"C:\Program Files\gstreamer\1.0\msvc_x86_64\bin"));
    let Some(bin) = candidates.into_iter().find(|d| d.join(DLL).is_file()) else {
        return Err("GStreamer was not found, so video files cannot be decoded; sound and pictures still play. \
                    Run this from the GodwinMix desktop app, or install GStreamer from gstreamer.freedesktop.org"
            .into());
    };
    let joined = std::env::join_paths(std::iter::once(bin.clone()).chain(std::env::split_paths(&path)))
        .map_err(|e| format!("could not add {} to PATH: {e}", bin.display()))?;
    // Safe: called from `main` before any other thread exists.
    std::env::set_var("PATH", joined);
    if std::env::var_os("GST_PLUGIN_SYSTEM_PATH").is_none() {
        if let Some(root) = bin.parent() {
            std::env::set_var("GST_PLUGIN_SYSTEM_PATH", root.join("lib").join("gstreamer-1.0"));
        }
    }
    Ok(bin.display().to_string())
}
