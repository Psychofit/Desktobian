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

/// Wallpaper Engine's Steam appid.
const WE_APPID: &str = "431960";

/// Default folders to scan for wallpapers: the user's Videos directory, a
/// `~/Wallpapers` folder, and the Steam Workshop folder for Wallpaper Engine
/// across every detected Steam library (so existing wallpapers show up
/// automatically — even on a second drive).
pub fn default_library_folders() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(dirs) = directories::UserDirs::new() {
        if let Some(videos) = dirs.video_dir() {
            push_if_dir(&mut out, videos.to_path_buf());
        }
        push_if_dir(&mut out, dirs.home_dir().join("Wallpapers"));
    }
    for lib in steam_library_paths() {
        push_if_dir(
            &mut out,
            lib.join("steamapps/workshop/content").join(WE_APPID),
        );
    }
    out
}

/// All Steam library roots: the standard install locations plus any extra
/// libraries registered in `libraryfolders.vdf` (e.g. on another drive).
fn steam_library_paths() -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(dirs) = directories::UserDirs::new() {
        let home = dirs.home_dir();
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".local/share/Steam"));
        // Flatpak Steam.
        roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }

    // Parse libraryfolders.vdf from the known roots to find extra libraries.
    let mut extra = Vec::new();
    for root in &roots {
        for vdf in [
            root.join("steamapps/libraryfolders.vdf"),
            root.join("config/libraryfolders.vdf"),
        ] {
            if let Ok(text) = std::fs::read_to_string(&vdf) {
                extra.extend(parse_vdf_library_paths(&text));
            }
        }
    }
    roots.extend(extra);

    roots.sort();
    roots.dedup();
    roots
}

/// Pull the `"path"  "<dir>"` values out of a Steam `libraryfolders.vdf`.
fn parse_vdf_library_paths(text: &str) -> Vec<std::path::PathBuf> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with("\"path\"") {
                return None;
            }
            // `"path"   "/mnt/games/SteamLibrary"`
            let value = line.split('"').nth(3)?;
            Some(std::path::PathBuf::from(value.replace("\\\\", "/")))
        })
        .collect()
}

fn push_if_dir(out: &mut Vec<String>, path: std::path::PathBuf) {
    if path.is_dir() {
        let s = path.to_string_lossy().into_owned();
        if !out.contains(&s) {
            out.push(s);
        }
    }
}
