//! Display backends: the glue between an output, an OpenGL surface, and an mpv
//! player.
//!
//! Two backends are provided:
//!   * [`wayland`] — wlr-layer-shell background surfaces (Sway, Hyprland, river…).
//!   * [`x11`] — desktop-level override-redirect windows (GNOME/KDE/XFCE on X11,
//!     and minimal WMs like i3/bspwm/openbox).
//!
//! Both translate their native outputs into [`crate::monitor::Output`] and drive
//! a per-output [`crate::player::MpvPlayer`].

pub mod wayland;
pub mod x11;

use std::path::Path;

use crate::cli::Backend as BackendChoice;
use crate::config::{Config, Resolved};
use crate::error::{Error, Result};
use crate::monitor::Output;
use crate::source::{self, ResolvedSource};

/// Everything needed to render one output's wallpaper.
pub struct WallpaperPlan {
    pub output: Output,
    pub settings: Resolved,
    pub source: ResolvedSource,
}

/// A display backend able to enumerate outputs and run wallpapers on them.
pub trait Backend {
    /// Human-readable backend name for logs.
    fn name(&self) -> &'static str;

    /// Enumerate the currently connected outputs.
    fn outputs(&mut self) -> Result<Vec<Output>>;

    /// Run the wallpaper engine for the given plans until interrupted.
    fn run(self: Box<Self>, plans: Vec<WallpaperPlan>) -> Result<()>;
}

/// Create a backend according to the user's choice, auto-detecting when asked.
pub fn create(choice: BackendChoice) -> Result<Box<dyn Backend>> {
    let resolved = match choice {
        BackendChoice::Auto => detect(),
        BackendChoice::Wayland => BackendChoice::Wayland,
        BackendChoice::X11 => BackendChoice::X11,
    };

    match resolved {
        BackendChoice::Wayland => {
            log::info!("Using Wayland (wlr-layer-shell) backend");
            Ok(Box::new(wayland::WaylandBackend::connect()?))
        }
        BackendChoice::X11 => {
            log::info!("Using X11 backend");
            Ok(Box::new(x11::X11Backend::connect()?))
        }
        BackendChoice::Auto => unreachable!("detect() never returns Auto"),
    }
}

/// Decide which backend to use from the environment.
fn detect() -> BackendChoice {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        BackendChoice::Wayland
    } else {
        BackendChoice::X11
    }
}

/// Build per-output render plans, filtering to `requested` outputs (empty = all)
/// and resolving each output's settings and media. Outputs that fail to resolve
/// are logged and skipped so a single bad monitor doesn't abort the others.
pub fn build_plans(
    config: &Config,
    outputs: &[Output],
    requested: &[String],
    cli_source: Option<&Path>,
) -> Result<Vec<WallpaperPlan>> {
    let mut plans = Vec::new();
    for output in outputs {
        if !requested.is_empty() && !requested.iter().any(|r| r == &output.name) {
            continue;
        }
        let settings = match config.resolve(&output.name, cli_source) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("skipping output {}: {e}", output.name);
                continue;
            }
        };
        let resolved = match source::resolve(&settings.source) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("skipping output {}: {e}", output.name);
                continue;
            }
        };
        log::info!(
            "output {} -> {} ({} file(s))",
            output.name,
            resolved.primary().display(),
            resolved.files.len()
        );
        plans.push(WallpaperPlan {
            output: output.clone(),
            settings,
            source: resolved,
        });
    }

    if plans.is_empty() {
        return Err(Error::Config(
            "no outputs to render (check your --output filter and config sources)".into(),
        ));
    }
    Ok(plans)
}
