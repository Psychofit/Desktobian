//! High-level application flow: load config, pick a backend, build plans, run.

use std::path::PathBuf;

use crate::backend::{self};
use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::ipc;

/// Run the program according to parsed CLI arguments.
pub fn run(cli: Cli) -> Result<()> {
    let command = cli.command();

    // Client commands talk to an already-running daemon over the control
    // socket; they need neither config nor a display backend.
    if let Some(request) = client_request(&command, &cli.output) {
        return run_client(&request);
    }

    // Daemon-side commands (run / list-outputs) need a display backend.
    let config = load_config(&cli)?;
    let mut backend = backend::create(cli.backend)?;
    let outputs = backend.outputs()?;
    if outputs.is_empty() {
        return Err(Error::NoOutputs);
    }

    match command {
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
            // Start the control socket; `_server` removes it on drop, after the
            // render loop returns.
            let (_server, commands) = ipc::Server::start()?;
            let context = ipc::DaemonContext::new(effective_config_path(&cli), cli.source.clone());
            backend.run(plans, commands, context)
        }
        _ => unreachable!("client commands handled above"),
    }
}

/// Translate a client subcommand into an IPC [`ipc::Request`], or `None` for the
/// daemon-side commands (`run`, `list-outputs`).
fn client_request(command: &Command, outputs: &[String]) -> Option<ipc::Request> {
    let outputs = outputs.to_vec();
    Some(match command {
        Command::Set { source } => ipc::Request::Set {
            source: source.clone(),
            outputs,
        },
        Command::Pause => ipc::Request::Pause { outputs },
        Command::Play => ipc::Request::Play { outputs },
        Command::Toggle => ipc::Request::Toggle { outputs },
        Command::Mute => ipc::Request::Mute { outputs },
        Command::Unmute => ipc::Request::Unmute { outputs },
        Command::Reload => ipc::Request::Reload { outputs },
        Command::Status => ipc::Request::Status,
        Command::Stop => ipc::Request::Stop,
        Command::Run | Command::ListOutputs => return None,
    })
}

/// The config path the daemon should re-read on `reload`: an explicit
/// `--config`, or the default path if a file exists there, else `None`.
fn effective_config_path(cli: &Cli) -> Option<PathBuf> {
    match &cli.config {
        Some(path) => Some(path.clone()),
        None => default_config_path().filter(|p| p.exists()),
    }
}

/// Send a request to the running daemon and report the reply.
fn run_client(request: &ipc::Request) -> Result<()> {
    let response = ipc::send(request)?;
    if response.outputs.is_empty() {
        println!("{}", response.message);
    } else {
        println!("{} [{}]", response.message, response.outputs.join(", "));
    }
    if response.ok {
        Ok(())
    } else {
        Err(Error::Config(response.message))
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
