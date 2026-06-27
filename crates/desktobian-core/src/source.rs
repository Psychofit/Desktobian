//! Resolving a user-supplied "source" into concrete media for mpv.
//!
//! A source can be:
//!   * a single media file (`forest.mp4`, `loop.gif`, `art.png`);
//!   * a directory of media files, played as a shuffled/ordered playlist;
//!   * a Wallpaper Engine **project folder** containing a `project.json`
//!     (video projects are supported; scene/web projects are recognised and
//!     reported as not-yet-supported rather than failing cryptically).
//!
//! This is the layer that lets Desktobian point straight at a Steam Workshop
//! item directory and "just work" for video wallpapers.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

/// Media file extensions mpv can play as an (animated) wallpaper.
const VIDEO_EXTS: &[&str] = &[
    "mp4", "mkv", "webm", "mov", "avi", "m4v", "flv", "wmv", "mpg", "mpeg", "gif", "apng", "webp",
];

/// Still-image extensions, usable as a static fallback wallpaper.
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "avif", "jxl"];

/// A source resolved into something the player can load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    /// One or more media files, in play order. Always at least one element.
    pub files: Vec<PathBuf>,
}

impl ResolvedSource {
    /// The first (primary) file — handy for single-file sources.
    pub fn primary(&self) -> &Path {
        &self.files[0]
    }

    /// Whether this source represents more than one file (a playlist).
    pub fn is_playlist(&self) -> bool {
        self.files.len() > 1
    }
}

/// Subset of a Wallpaper Engine `project.json` that we care about.
#[derive(Debug, Deserialize)]
struct WeProject {
    /// `video`, `scene`, `web`, `application`, …
    #[serde(default)]
    r#type: String,
    /// The primary asset file, relative to the project folder.
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

/// Resolve `source` into concrete media files.
pub fn resolve(source: &Path) -> Result<ResolvedSource> {
    if !source.exists() {
        return Err(Error::SourceNotFound(source.to_path_buf()));
    }

    if source.is_dir() {
        // A Wallpaper Engine project folder?
        let project_json = source.join("project.json");
        if project_json.is_file() {
            return resolve_we_project(source, &project_json);
        }
        return resolve_directory(source);
    }

    // A single file.
    if is_playable_file(source) {
        Ok(ResolvedSource {
            files: vec![source.to_path_buf()],
        })
    } else {
        Err(Error::UnsupportedSource {
            path: source.to_path_buf(),
            reason: format!(
                "unrecognised media extension (supported: {})",
                supported_extensions_hint()
            ),
        })
    }
}

/// Resolve a Wallpaper Engine project folder via its `project.json`.
fn resolve_we_project(dir: &Path, project_json: &Path) -> Result<ResolvedSource> {
    let text = std::fs::read_to_string(project_json)?;
    let project: WeProject = serde_json::from_str(&text).map_err(|e| Error::UnsupportedSource {
        path: project_json.to_path_buf(),
        reason: format!("invalid project.json: {e}"),
    })?;

    let title = project.title.as_deref().unwrap_or("<untitled>");
    log::info!(
        "Wallpaper Engine project '{title}' (type: {})",
        project.r#type
    );

    match project.r#type.as_str() {
        "video" => {
            let file = project.file.ok_or_else(|| Error::UnsupportedSource {
                path: project_json.to_path_buf(),
                reason: "video project has no `file` entry".to_string(),
            })?;
            let path = dir.join(&file);
            if !path.is_file() {
                return Err(Error::SourceNotFound(path));
            }
            Ok(ResolvedSource { files: vec![path] })
        }
        other => Err(Error::UnsupportedSource {
            path: dir.to_path_buf(),
            reason: format!(
                "Wallpaper Engine '{other}' projects are not supported yet (only 'video'). \
                 Scene and web wallpapers are on the roadmap."
            ),
        }),
    }
}

/// Resolve a plain directory into a sorted playlist of its media files.
fn resolve_directory(dir: &Path) -> Result<ResolvedSource> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_playable_file(p))
        .collect();
    files.sort();

    if files.is_empty() {
        return Err(Error::UnsupportedSource {
            path: dir.to_path_buf(),
            reason: format!(
                "directory contains no playable media (looked for: {})",
                supported_extensions_hint()
            ),
        });
    }
    Ok(ResolvedSource { files })
}

/// Is this file something mpv can play as a wallpaper?
fn is_playable_file(path: &Path) -> bool {
    matches_ext(path, VIDEO_EXTS) || matches_ext(path, IMAGE_EXTS)
}

fn matches_ext(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .map(|e| exts.contains(&e.as_str()))
        .unwrap_or(false)
}

fn supported_extensions_hint() -> String {
    let mut all: Vec<&str> = VIDEO_EXTS
        .iter()
        .chain(IMAGE_EXTS.iter())
        .copied()
        .collect();
    all.sort_unstable();
    all.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmpdir() -> PathBuf {
        let base = std::env::var("CARGO_TARGET_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let dir = base.join(format!("desktobian-src-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn single_video_file_resolves() {
        let dir = tmpdir();
        let file = dir.join("clip.mp4");
        fs::write(&file, b"x").unwrap();
        let r = resolve(&file).unwrap();
        assert_eq!(r.files, vec![file]);
        assert!(!r.is_playlist());
    }

    #[test]
    fn directory_becomes_sorted_playlist() {
        let dir = tmpdir().join("playlist");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("b.mp4"), b"x").unwrap();
        fs::write(dir.join("a.webm"), b"x").unwrap();
        fs::write(dir.join("notes.txt"), b"ignore me").unwrap();
        let r = resolve(&dir).unwrap();
        assert_eq!(r.files.len(), 2);
        assert!(r.files[0].ends_with("a.webm"));
        assert!(r.files[1].ends_with("b.mp4"));
        assert!(r.is_playlist());
    }

    #[test]
    fn we_video_project_resolves_to_file() {
        let dir = tmpdir().join("we-video");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("scene.mp4"), b"x").unwrap();
        fs::write(
            dir.join("project.json"),
            r#"{"type":"video","file":"scene.mp4","title":"Test"}"#,
        )
        .unwrap();
        let r = resolve(&dir).unwrap();
        assert!(r.primary().ends_with("scene.mp4"));
    }

    #[test]
    fn we_scene_project_is_unsupported() {
        let dir = tmpdir().join("we-scene");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("project.json"), r#"{"type":"scene"}"#).unwrap();
        let err = resolve(&dir).unwrap_err();
        assert!(matches!(err, Error::UnsupportedSource { .. }));
    }

    #[test]
    fn missing_source_errors() {
        let err = resolve(Path::new("/definitely/not/here.mp4")).unwrap_err();
        assert!(matches!(err, Error::SourceNotFound(_)));
    }
}
