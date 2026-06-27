//! Scanning wallpaper folders into a previewable library.
//!
//! Reuses `desktobian-core`'s source resolver so the same things the engine can
//! play (video/GIF/image files and Wallpaper Engine `project.json` folders) show
//! up here.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// One entry in the wallpaper library, ready to show in the grid.
#[derive(Serialize, Clone)]
pub struct WallpaperItem {
    /// Stable id (hash of the media path).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Absolute path to the primary media file.
    pub path: String,
    /// `"video"` or `"image"`.
    pub kind: String,
    /// A `data:` URL thumbnail, if one could be generated.
    pub thumbnail: Option<String>,
}

/// Still-image extensions (everything else playable is treated as "video",
/// including animated gif/apng/webp).
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "avif", "jxl"];

/// Scan the given folders and return a de-duplicated list of wallpapers.
pub fn scan(folders: &[String], with_thumbnails: bool) -> Vec<WallpaperItem> {
    let mut items: Vec<WallpaperItem> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for folder in folders {
        let dir = Path::new(folder);
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(item) = item_for(&path, with_thumbnails) {
                if seen.insert(item.path.clone()) {
                    items.push(item);
                }
            }
        }
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}

/// Build a library item from a candidate path (a media file, or a Wallpaper
/// Engine project folder). Returns `None` for anything not playable.
fn item_for(path: &Path, with_thumbnails: bool) -> Option<WallpaperItem> {
    // Directories only count if they're Wallpaper Engine project folders; we
    // deliberately don't treat an arbitrary folder as one big playlist item.
    if path.is_dir() && !path.join("project.json").is_file() {
        return None;
    }

    let resolved = desktobian_core::source::resolve(path).ok()?;
    let primary = resolved.primary().to_path_buf();

    let name = if path.is_dir() {
        we_project_title(path).unwrap_or_else(|| dir_name(path))
    } else {
        file_stem(&primary)
    };

    let kind = media_kind(&primary).to_string();
    let path_str = primary.to_string_lossy().into_owned();
    let id = short_hash(&path_str);
    let thumbnail = if with_thumbnails {
        thumbnail_data_url(&primary)
    } else {
        None
    };

    Some(WallpaperItem {
        id,
        name,
        path: path_str,
        kind,
        thumbnail,
    })
}

fn media_kind(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if IMAGE_EXTS.contains(&ext.as_str()) {
        "image"
    } else {
        "video"
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}

/// Read the `title` from a Wallpaper Engine `project.json`, if present.
fn we_project_title(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("project.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("title")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

fn short_hash(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let digest = hasher.finalize();
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// Generate (and cache) a thumbnail for `media`, returning it as a `data:` URL.
/// Returns `None` if no thumbnail tool (ffmpeg) is available or it fails.
fn thumbnail_data_url(media: &Path) -> Option<String> {
    let cache = thumbnail_cache_dir()?;
    let _ = std::fs::create_dir_all(&cache);
    let key = short_hash(&media.to_string_lossy());
    let thumb = cache.join(format!("{key}.jpg"));

    if !thumb.exists() && !generate_thumbnail(media, &thumb) {
        return None;
    }
    let bytes = std::fs::read(&thumb).ok()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/jpeg;base64,{b64}"))
}

fn thumbnail_cache_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "desktobian").map(|d| d.cache_dir().join("thumbnails"))
}

/// Extract a single frame as a JPEG thumbnail using ffmpeg.
fn generate_thumbnail(src: &Path, dst: &Path) -> bool {
    let status = std::process::Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-ss", "1", "-i"])
        .arg(src)
        .args(["-frames:v", "1", "-vf", "scale=480:-1"])
        .arg(dst)
        .status();
    matches!(status, Ok(s) if s.success()) && dst.exists()
}
