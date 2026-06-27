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

/// Set the KDE Plasma wallpaper to our video plugin via plasmashell D-Bus.
fn apply_kde(req: &ApplyRequest) -> ApplyResult {
    let url = to_file_url(&req.path);
    let muted = if req.muted { "true" } else { "false" };
    let fill = req.fill_mode;
    // The path is percent-encoded (no quotes), so embedding it in a single-quoted
    // JS string is safe.
    let script = format!(
        "var ds = desktops(); for (var i = 0; i < ds.length; i++) {{ \
           var d = ds[i]; \
           d.wallpaperPlugin = 'org.desktobian.video'; \
           d.currentConfigGroup = ['Wallpaper', 'org.desktobian.video', 'General']; \
           d.writeConfig('VideoUrl', '{url}'); \
           d.writeConfig('Muted', {muted}); \
           d.writeConfig('FillMode', {fill}); \
         }}"
    );

    for tool in ["qdbus6", "qdbus", "qdbus-qt5"] {
        match std::process::Command::new(tool)
            .args([
                "org.kde.plasmashell",
                "/PlasmaShell",
                "org.kde.PlasmaShell.evaluateScript",
                &script,
            ])
            .status()
        {
            Ok(s) if s.success() => {
                return ApplyResult {
                    ok: true,
                    message: "Wallpaper applied via KDE Plasma.".into(),
                    method: "kde-plasma".into(),
                };
            }
            // Tool ran but reported failure, or wasn't found — try the next.
            _ => continue,
        }
    }
    ApplyResult {
        ok: false,
        message: "Could not reach plasmashell (is qdbus installed?). Make sure the \
                  Desktobian Video plugin is installed (kde/install.sh)."
            .into(),
        method: "kde-plasma".into(),
    }
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
