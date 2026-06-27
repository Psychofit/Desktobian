//! Desktop-environment detection and default library locations.

use serde::Serialize;

/// Information about the running desktop environment, surfaced to the UI so it
/// can explain how a wallpaper will be applied.
#[derive(Serialize, Clone)]
pub struct EnvInfo {
    /// `XDG_CURRENT_DESKTOP`, e.g. "KDE".
    pub desktop: String,
    /// `XDG_SESSION_TYPE`, e.g. "x11" or "wayland".
    pub session_type: String,
    /// Whether we'll apply via the KDE Plasma plugin (vs the engine daemon).
    pub is_kde: bool,
    /// How "Apply" will work, for display in the UI.
    pub apply_method: String,
}

/// Detect the current desktop environment.
pub fn detect() -> EnvInfo {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let is_kde = desktop.to_ascii_uppercase().contains("KDE");
    let apply_method = if is_kde {
        "KDE Plasma plugin (org.desktobian.video)".to_string()
    } else {
        "Desktobian engine daemon (control socket)".to_string()
    };
    EnvInfo {
        desktop,
        session_type,
        is_kde,
        apply_method,
    }
}

/// Default folders to scan for wallpapers: the user's Videos directory, a
/// `~/Wallpapers` folder, and the Steam Workshop folder for Wallpaper Engine
/// (so existing video wallpapers show up automatically).
pub fn default_library_folders() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(dirs) = directories::UserDirs::new() {
        if let Some(videos) = dirs.video_dir() {
            push_if_dir(&mut out, videos.to_path_buf());
        }
        let home = dirs.home_dir();
        push_if_dir(&mut out, home.join("Wallpapers"));
        // Steam Workshop content for Wallpaper Engine (appid 431960).
        push_if_dir(
            &mut out,
            home.join(".steam/steam/steamapps/workshop/content/431960"),
        );
        push_if_dir(
            &mut out,
            home.join(".local/share/Steam/steamapps/workshop/content/431960"),
        );
    }
    out
}

fn push_if_dir(out: &mut Vec<String>, path: std::path::PathBuf) {
    if path.is_dir() {
        out.push(path.to_string_lossy().into_owned());
    }
}
