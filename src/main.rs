//! Desktobian — an open-source Wallpaper Engine alternative for Linux.
//!
//! Renders animated video & GIF wallpapers onto the desktop background on both
//! X11 (root-window) and Wayland (wlr-layer-shell) using libmpv + OpenGL.

mod app;
mod backend;
mod cli;
mod ipc;
mod logging;
mod player;
mod render;
mod util;

// Shared, platform-independent layers live in `desktobian-core`. Re-export them
// under their familiar `crate::` paths so the rest of the engine is unchanged.
pub use desktobian_core::{config, error, monitor, source};

use clap::Parser;

use cli::Cli;

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    logging::init(cli.verbosity());

    match app::run(cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}
