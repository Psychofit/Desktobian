//! Configuration model.
//!
//! Desktobian is configured through a single TOML file (by default
//! `~/.config/desktobian/config.toml`). It has a `[default]` section that
//! applies to every monitor, plus optional `[output.<NAME>]` sections that
//! override individual fields for a specific connector.
//!
//! Example:
//!
//! ```toml
//! [default]
//! source = "~/Wallpapers/forest.mp4"
//! mute = true
//! fit = "cover"
//!
//! [output.HDMI-A-1]
//! source = "~/Wallpapers/city-loop.mp4"
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

/// How the video should be scaled to fit the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// Scale to fill the whole screen, cropping the overflow (keeps aspect).
    /// This is what most people expect from a wallpaper, hence the default.
    #[default]
    Cover,
    /// Scale to fit entirely on screen, letterboxing if needed (keeps aspect).
    Contain,
    /// Stretch to fill the screen, ignoring aspect ratio.
    Fill,
    /// No scaling; show at native size, centered.
    Center,
}

/// The set of knobs that can be set either globally or per output.
///
/// Every field is optional so that an `[output.X]` section only needs to
/// specify what differs from `[default]`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallpaperSettings {
    /// Path to a video, GIF, image, directory (playlist) or a Wallpaper Engine
    /// project folder.
    pub source: Option<PathBuf>,
    /// Mute audio (defaults to `true` — a wallpaper that blasts audio is rarely
    /// what anyone wants).
    pub mute: Option<bool>,
    /// Playback volume 0–100 (only relevant when `mute = false`).
    pub volume: Option<u8>,
    /// Scaling behaviour.
    pub fit: Option<Fit>,
    /// Loop the video forever (default `true`).
    #[serde(rename = "loop")]
    pub loop_playback: Option<bool>,
    /// Cap the render frame rate. `0`/absent means "follow the monitor refresh".
    pub fps: Option<u32>,
    /// Hardware decode mode passed to mpv (`auto`, `no`, `vaapi`, `nvdec`, …).
    pub hwdec: Option<String>,
    /// Extra raw mpv options, e.g. `["--brightness=-10"]`. Power-user escape hatch.
    #[serde(default)]
    pub mpv_options: Vec<String>,
}

/// The full parsed configuration file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Defaults applied to every output.
    #[serde(default)]
    pub default: WallpaperSettings,
    /// Per-connector overrides, keyed by output name (e.g. `eDP-1`).
    #[serde(default)]
    pub output: HashMap<String, WallpaperSettings>,
}

/// Settings with all defaults filled in, ready to drive a player.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub source: PathBuf,
    pub mute: bool,
    pub volume: u8,
    pub fit: Fit,
    pub loop_playback: bool,
    pub fps: u32,
    pub hwdec: String,
    pub mpv_options: Vec<String>,
}

impl Config {
    /// Load and parse the config from `path`.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
        let config: Config = toml::from_str(&text).map_err(|source| Error::ConfigParse {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(config)
    }

    /// Resolve the effective settings for a given output name, merging the
    /// per-output override on top of the global default, with `cli_source` (a
    /// `--source` flag) taking ultimate precedence if provided.
    pub fn resolve(&self, output_name: &str, cli_source: Option<&Path>) -> Result<Resolved> {
        let global = &self.default;
        let specific = self.output.get(output_name);

        // Pick a field from the per-output override, then the global default.
        macro_rules! pick {
            ($field:ident) => {
                specific
                    .and_then(|s| s.$field.clone())
                    .or_else(|| global.$field.clone())
            };
        }

        let source = cli_source
            .map(|p| p.to_path_buf())
            .or_else(|| pick!(source))
            .ok_or_else(|| {
                Error::Config(format!(
                    "no `source` configured for output `{output_name}` (set one in [default] \
                     or [output.{output_name}], or pass --source)"
                ))
            })?;

        let source = expand_tilde(&source);

        // Per-output `mpv_options` extend the global ones rather than replacing.
        let mut mpv_options = global.mpv_options.clone();
        if let Some(s) = specific {
            mpv_options.extend(s.mpv_options.iter().cloned());
        }

        Ok(Resolved {
            source,
            mute: pick!(mute).unwrap_or(true),
            volume: pick!(volume).unwrap_or(100).min(100),
            fit: pick!(fit).unwrap_or_default(),
            loop_playback: pick!(loop_playback).unwrap_or(true),
            fps: pick!(fps).unwrap_or(0),
            hwdec: pick!(hwdec).unwrap_or_else(|| "auto-safe".to_string()),
            mpv_options,
        })
    }
}

/// Expand a leading `~` to the user's home directory.
fn expand_tilde(path: &Path) -> PathBuf {
    let Ok(stripped) = path.strip_prefix("~") else {
        return path.to_path_buf();
    };
    match directories::BaseDirs::new() {
        Some(dirs) => dirs.home_dir().join(stripped),
        None => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_wins_over_default() {
        let toml = r#"
            [default]
            source = "/a/global.mp4"
            mute = true
            fit = "cover"

            [output.HDMI-A-1]
            source = "/a/specific.mp4"
            mute = false
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();

        let edp = cfg.resolve("eDP-1", None).unwrap();
        assert_eq!(edp.source, PathBuf::from("/a/global.mp4"));
        assert!(edp.mute);
        assert_eq!(edp.fit, Fit::Cover);

        let hdmi = cfg.resolve("HDMI-A-1", None).unwrap();
        assert_eq!(hdmi.source, PathBuf::from("/a/specific.mp4"));
        assert!(!hdmi.mute);
        // Inherited from [default].
        assert_eq!(hdmi.fit, Fit::Cover);
    }

    #[test]
    fn cli_source_overrides_everything() {
        let cfg: Config = toml::from_str(
            r#"[default]
            source = "/a/global.mp4""#,
        )
        .unwrap();
        let r = cfg
            .resolve("eDP-1", Some(Path::new("/cli/override.mp4")))
            .unwrap();
        assert_eq!(r.source, PathBuf::from("/cli/override.mp4"));
    }

    #[test]
    fn missing_source_is_an_error() {
        let cfg = Config::default();
        assert!(cfg.resolve("eDP-1", None).is_err());
    }

    #[test]
    fn mpv_options_are_merged() {
        let toml = r#"
            [default]
            source = "/a.mp4"
            mpv_options = ["--brightness=-5"]

            [output.DP-1]
            mpv_options = ["--contrast=10"]
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let r = cfg.resolve("DP-1", None).unwrap();
        assert_eq!(r.mpv_options, vec!["--brightness=-5", "--contrast=10"]);
    }
}
