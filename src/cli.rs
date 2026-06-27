//! Command-line interface.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Desktobian — animated video & GIF wallpapers for Linux (X11 & Wayland).
///
/// Run with no subcommand to start the wallpaper engine using your config file.
#[derive(Debug, Parser)]
#[command(name = "desktobian", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to the config file (default: ~/.config/desktobian/config.toml).
    #[arg(short, long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Wallpaper source (file, directory or Wallpaper Engine project folder).
    /// Overrides whatever is in the config for every output.
    #[arg(short, long, global = true, value_name = "PATH")]
    pub source: Option<PathBuf>,

    /// Restrict to specific output(s) by name, e.g. `-o HDMI-A-1`. Repeatable.
    #[arg(short, long, global = true, value_name = "NAME")]
    pub output: Vec<String>,

    /// Force a display backend instead of auto-detecting.
    #[arg(short, long, global = true, value_enum, default_value_t = Backend::Auto)]
    pub backend: Backend,

    /// Increase verbosity (-v debug, -vv trace). Repeatable.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Decrease verbosity (-q warnings only, -qq errors only). Repeatable.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub quiet: u8,
}

impl Cli {
    /// Net verbosity: positive = more verbose, negative = quieter.
    pub fn verbosity(&self) -> i8 {
        self.verbose as i8 - self.quiet as i8
    }

    /// The subcommand, defaulting to `Run` when none was given.
    pub fn command(&self) -> Command {
        self.command.clone().unwrap_or(Command::Run)
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Start the wallpaper engine (default).
    Run,
    /// List detected monitors/outputs and exit.
    ListOutputs,
    /// Swap the wallpaper on a running daemon (use -o to target outputs).
    Set {
        /// Wallpaper source: file, directory, or Wallpaper Engine project folder.
        #[arg(value_name = "PATH")]
        source: PathBuf,
    },
    /// Pause playback on a running daemon.
    Pause,
    /// Resume playback on a running daemon.
    Play,
    /// Toggle pause on a running daemon.
    Toggle,
    /// Mute audio on a running daemon.
    Mute,
    /// Unmute audio on a running daemon.
    Unmute,
    /// Ask a running daemon to shut down.
    Stop,
    /// Query a running daemon's status.
    Status,
}

/// Which display backend to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
#[value(rename_all = "lower")]
pub enum Backend {
    /// Pick Wayland if `WAYLAND_DISPLAY` is set, otherwise X11.
    #[default]
    Auto,
    /// Force the Wayland (wlr-layer-shell) backend.
    Wayland,
    /// Force the X11 (root-window) backend.
    X11,
}
