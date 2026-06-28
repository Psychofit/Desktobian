//! Scanning wallpaper folders into a previewable library.
//!
//! Understands plain media files, plain folders of media, and — importantly for
//! importing from Steam Workshop — **Wallpaper Engine project folders** (a
//! `project.json` next to the asset and a `preview` image). Video projects are
//! applyable; scene/web/application projects are listed (with their preview) but
//! marked unsupported until those renderers land.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One entry in the wallpaper library, ready to show in the grid.
#[derive(Serialize, Clone)]
pub struct WallpaperItem {
    /// Stable id (hash of the path).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Path to apply: the media file for video/image, or the project folder for
    /// unsupported types (which can't be applied yet).
    pub path: String,
    /// `video` | `image` | `scene` | `web` | `application`.
    pub kind: String,
    /// Whether this item can actually be applied today.
    pub supported: bool,
    /// A `data:` URL thumbnail, if one could be generated.
    pub thumbnail: Option<String>,
}

/// Still-image extensions (everything else playable is treated as "video",
/// including animated gif/apng/webp).
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "avif", "jxl"];

/// Subset of a Wallpaper Engine `project.json`.
#[derive(Deserialize, Default)]
struct WeProjectMeta {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    preview: Option<String>,
}

/// Scan the given folders and return a de-duplicated list of wallpapers.
pub fn scan(folders: &[String], with_thumbnails: bool) -> Vec<WallpaperItem> {
    let mut items: Vec<WallpaperItem> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut push = |maybe: Option<WallpaperItem>| {
        if let Some(item) = maybe {
            if seen.insert(item.path.clone()) {
                items.push(item);
            }
        }
    };

    for folder in folders {
        let dir = Path::new(folder);
        // The folder might itself be a Wallpaper Engine project…
        if dir.join("project.json").is_file() {
            push(we_project_item(dir, with_thumbnails));
            continue;
        }
        // …otherwise scan its entries (files, and subfolders-that-are-projects,
        // e.g. a Steam Workshop content/431960 directory).
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.join("project.json").is_file() {
                    push(we_project_item(&path, with_thumbnails));
                }
            } else if path.is_file() {
                push(file_item(&path, with_thumbnails));
            }
        }
    }

    // Supported items first, then by name.
    items.sort_by(|a, b| {
        b.supported
            .cmp(&a.supported)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    items
}

/// Build an item from a Wallpaper Engine project folder.
fn we_project_item(dir: &Path, with_thumbnails: bool) -> Option<WallpaperItem> {
    let text = std::fs::read_to_string(dir.join("project.json")).ok()?;
    let meta: WeProjectMeta = serde_json::from_str(&text).unwrap_or_default();

    let kind = meta.kind.to_ascii_lowercase();
    let name = meta.title.clone().unwrap_or_else(|| dir_name(dir));
    let preview = meta
        .preview
        .as_ref()
        .map(|p| dir.join(p))
        .filter(|p| p.is_file());

    if kind == "video" {
        let file = meta.file.as_ref()?;
        let media = dir.join(file);
        if !media.is_file() {
            return None;
        }
        let thumb_src = preview.unwrap_or_else(|| media.clone());
        return Some(WallpaperItem {
            id: short_hash(&media.to_string_lossy()),
            name,
            path: media.to_string_lossy().into_owned(),
            kind: "video".into(),
            supported: true,
            thumbnail: maybe_thumb(with_thumbnails, &thumb_src),
        });
    }

    if kind == "web" {
        // The applyable path is the web wallpaper's entry HTML.
        let file = meta
            .file
            .clone()
            .unwrap_or_else(|| "index.html".to_string());
        let entry = dir.join(&file);
        if !entry.is_file() {
            return None;
        }
        return Some(WallpaperItem {
            id: short_hash(&entry.to_string_lossy()),
            name,
            path: entry.to_string_lossy().into_owned(),
            kind: "web".into(),
            supported: true,
            // ffmpeg can't thumbnail an HTML page; use the project preview only.
            thumbnail: preview.and_then(|p| maybe_thumb(with_thumbnails, &p)),
        });
    }

    // scene / application: list it (with its preview) but not applyable.
    Some(WallpaperItem {
        id: short_hash(&dir.to_string_lossy()),
        name,
        path: dir.to_string_lossy().into_owned(),
        kind: if kind.is_empty() {
            "unknown".into()
        } else {
            kind
        },
        supported: false,
        thumbnail: preview.and_then(|p| maybe_thumb(with_thumbnails, &p)),
    })
}

/// Build an item from a plain media file.
fn file_item(path: &Path, with_thumbnails: bool) -> Option<WallpaperItem> {
    let resolved = desktobian_core::source::resolve(path).ok()?;
    let primary = resolved.primary().to_path_buf();
    let path_str = primary.to_string_lossy().into_owned();
    Some(WallpaperItem {
        id: short_hash(&path_str),
        name: file_stem(&primary),
        path: path_str,
        kind: media_kind(&primary).into(),
        supported: true,
        thumbnail: maybe_thumb(with_thumbnails, &primary),
    })
}

fn maybe_thumb(enabled: bool, src: &Path) -> Option<String> {
    if enabled {
        thumbnail_data_url(src)
    } else {
        None
    }
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

/// Extract a single frame as a JPEG thumbnail using ffmpeg. Tries to seek ~1s in
/// for videos (to skip black intros), falling back to the first frame for
/// images and very short clips (e.g. preview gifs).
fn generate_thumbnail(src: &Path, dst: &Path) -> bool {
    if media_kind(src) == "video" && run_ffmpeg(src, dst, true) {
        return true;
    }
    run_ffmpeg(src, dst, false)
}

fn run_ffmpeg(src: &Path, dst: &Path, seek: bool) -> bool {
    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.args(["-y", "-loglevel", "error"]);
    if seek {
        cmd.args(["-ss", "1"]);
    }
    cmd.arg("-i")
        .arg(src)
        .args(["-frames:v", "1", "-vf", "scale=480:-1"])
        .arg(dst);
    matches!(cmd.status(), Ok(s) if s.success()) && dst.exists()
}
