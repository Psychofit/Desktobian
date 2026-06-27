//! High-level application flow: load config, pick a backend, build plans, run.

use std::path::PathBuf;

use crate::backend::{self};
use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::error::Result;

/// Run the program according to parsed CLI arguments.
pub fn run(cli: Cli) -> Result<()> {
    let config = load_config(&cli)?;

    let mut backend = backend::create(cli.backend)?;
    let outputs = backend.outputs()?;

    if outputs.is_empty() {
        return Err(crate::error::Error::NoOutputs);
    }

    match cli.command() {
        Command::ListOutputs => {
            println!(
                "Detected {} output(s) via {}:",
                outputs.len(),
                backend.name()
            );
            for o in &outputs {
                println!("  {}", o.summary());
            }
            Ok(())
        }
        Command::Run => {
            let plans =
                backend::build_plans(&config, &outputs, &cli.output, cli.source.as_deref())?;
            log::info!(
                "Starting wallpaper engine on {} output(s) via {}",
                plans.len(),
                backend.name()
            );
            backend.run(plans)
        }
    }
}

/// Load the config file. A missing file at the default path is fine (we fall
/// back to an empty config and rely on `--source`); a missing file at an
/// explicitly-requested path is an error.
fn load_config(cli: &Cli) -> Result<Config> {
    match &cli.config {
        Some(path) => Config::load(path),
        None => {
            let path = default_config_path();
            match path {
                Some(p) if p.exists() => {
                    log::debug!("Loading config from {}", p.display());
                    Config::load(&p)
                }
                _ => {
                    log::debug!("No config file found; using defaults");
                    Ok(Config::default())
                }
            }
        }
    }
}

/// `~/.config/desktobian/config.toml` (respecting `XDG_CONFIG_HOME`).
pub fn default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "desktobian")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}
