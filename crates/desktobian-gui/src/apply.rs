//! Applying a chosen wallpaper.
//!
//! On KDE we drive the Plasma plugin (`org.desktobian.video`) through
//! plasmashell's D-Bus `evaluateScript`. Elsewhere we drive the standalone
//! engine daemon over its control socket.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A request from the UI to apply a wallpaper.
#[derive(Deserialize)]
pub struct ApplyRequest {
    pub path: String,
    #[serde(default = "default_true")]
    pub muted: bool,
    /// 0 = stretch, 1 = fit (letterbox), 2 = crop (fill).
    #[serde(default = "default_fill")]
    pub fill_mode: u8,
}

fn default_true() -> bool {
    true
}
fn default_fill() -> u8 {
    2
}

/// The outcome of applying a wallpaper, surfaced to the UI.
#[derive(Serialize)]
pub struct ApplyResult {
    pub ok: bool,
    pub message: String,
    /// Which path was used: `kde-plasma` or `engine-daemon`.
    pub method: String,
}

/// Apply the wallpaper using whichever mechanism suits the current desktop.
pub fn apply(req: ApplyRequest) -> ApplyResult {
    if crate::env::detect().is_kde {
        apply_kde(&req)
    } else {
        apply_engine(&req)
    }
}

/// Set the KDE Plasma wallpaper via plasmashell D-Bus.
///
/// Videos/GIFs use our `org.desktobian.video` plugin; still images use KDE's
/// built-in `org.kde.image` wallpaper (our video plugin can't display them).
fn apply_kde(req: &ApplyRequest) -> ApplyResult {
    let url = to_file_url(&req.path);
    // The path is percent-encoded (no quotes), so embedding it in single-quoted
    // JS strings is safe.
    let script = if is_image(&req.path) {
        format!(
            "var ds = desktops(); for (var i = 0; i < ds.length; i++) {{ \
               var d = ds[i]; \
               d.wallpaperPlugin = 'org.kde.image'; \
               d.currentConfigGroup = ['Wallpaper', 'org.kde.image', 'General']; \
               d.writeConfig('Image', '{url}'); \
             }}"
        )
    } else {
        let muted = if req.muted { "true" } else { "false" };
        let fill = req.fill_mode;
        format!(
            "var ds = desktops(); for (var i = 0; i < ds.length; i++) {{ \
               var d = ds[i]; \
               d.wallpaperPlugin = 'org.desktobian.video'; \
               d.currentConfigGroup = ['Wallpaper', 'org.desktobian.video', 'General']; \
               d.writeConfig('VideoUrl', '{url}'); \
               d.writeConfig('Muted', {muted}); \
               d.writeConfig('FillMode', {fill}); \
             }}"
        )
    };

    if run_plasma_script(&script) {
        ApplyResult {
            ok: true,
            message: "Wallpaper applied via KDE Plasma.".into(),
            method: "kde-plasma".into(),
        }
    } else {
        ApplyResult {
            ok: false,
            message: "Could not reach plasmashell (is qdbus installed?). For videos, make \
                      sure the Desktobian Video plugin is installed (kde/install.sh)."
                .into(),
            method: "kde-plasma".into(),
        }
    }
}

/// Revert the desktop to a plain default wallpaper. Used when the GUI quits.
pub fn revert_to_default() {
    if crate::env::detect().is_kde {
        // Switch back to KDE's standard image wallpaper plugin.
        let script = "var ds = desktops(); for (var i = 0; i < ds.length; i++) { \
                        ds[i].wallpaperPlugin = 'org.kde.image'; }";
        let _ = run_plasma_script(script);
    } else {
        // Ask the engine daemon to shut down (which clears the wallpaper).
        use desktobian_core::ipc::{send, Request};
        let _ = send(&Request::Stop);
    }
}

/// Run a Plasma scripting snippet through plasmashell, trying the available
/// `qdbus` variants. Returns whether it succeeded.
fn run_plasma_script(script: &str) -> bool {
    for tool in ["qdbus6", "qdbus", "qdbus-qt5"] {
        if let Ok(status) = std::process::Command::new(tool)
            .args([
                "org.kde.plasmashell",
                "/PlasmaShell",
                "org.kde.PlasmaShell.evaluateScript",
                script,
            ])
            .status()
        {
            if status.success() {
                return true;
            }
        }
    }
    false
}

/// Still-image extensions (these are applied via KDE's image wallpaper).
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "avif", "jxl"];

fn is_image(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Tell the standalone engine daemon to switch wallpaper over the control socket.
fn apply_engine(req: &ApplyRequest) -> ApplyResult {
    use desktobian_core::ipc::{send, Request};
    let request = Request::Set {
        source: PathBuf::from(&req.path),
        outputs: Vec::new(),
    };
    match send(&request) {
        Ok(resp) => ApplyResult {
            ok: resp.ok,
            message: resp.message,
            method: "engine-daemon".into(),
        },
        Err(e) => ApplyResult {
            ok: false,
            message: format!("{e}"),
            method: "engine-daemon".into(),
        },
    }
}

/// Build a `file://` URL, percent-encoding the path while keeping `/` separators.
fn to_file_url(path: &str) -> String {
    let mut encoded = String::from("file://");
    for &b in path.as_bytes() {
        match b {
            b'/' | b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char)
            }
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_url_encodes_spaces_and_quotes() {
        let url = to_file_url("/home/me/my video's.mp4");
        assert!(url.starts_with("file:///home/me/"));
        assert!(url.contains("%20")); // space
        assert!(url.contains("%27")); // apostrophe
        assert!(!url.contains('\'')); // no raw quotes -> safe in JS
    }
}
