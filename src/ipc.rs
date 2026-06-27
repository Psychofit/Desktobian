//! Server side of the control IPC: the daemon's socket listener and the logic
//! that applies incoming [`Request`]s to the running players.
//!
//! The wire protocol ([`Request`] / [`Response`]) and the client [`send`] live
//! in `desktobian-core` so the GUI can reuse them; here we add the daemon-only
//! pieces (socket server, command application).
//!
//! All mutation of mpv happens on the daemon's render thread: the accept thread
//! merely forwards parsed requests over an mpsc channel and waits for the reply,
//! so we never touch a player from two threads at once.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{sync_channel, Receiver, Sender, SyncSender};
use std::thread;

use crate::config::{Config, Resolved};
use crate::error::{Error, Result};
use crate::player::MpvPlayer;
use crate::source::{self, ResolvedSource};
use crate::util;

// The wire protocol and client are shared with the GUI via desktobian-core.
pub use desktobian_core::ipc::{send, socket_path, Request, Response};

/// Context the daemon needs to re-resolve wallpapers on `reload`: where the
/// config lives and any `--source` override that was passed at launch.
#[derive(Debug, Clone, Default)]
pub struct DaemonContext {
    config_path: Option<PathBuf>,
    cli_source: Option<PathBuf>,
}

impl DaemonContext {
    pub fn new(config_path: Option<PathBuf>, cli_source: Option<PathBuf>) -> Self {
        DaemonContext {
            config_path,
            cli_source,
        }
    }

    /// Freshly resolve the effective settings + media for one output, reading
    /// the config file from disk so edits are picked up.
    pub fn resolve(&self, output_name: &str) -> Result<(Resolved, ResolvedSource)> {
        let config = match &self.config_path {
            Some(path) => Config::load(path)?,
            None => Config::default(),
        };
        let settings = config.resolve(output_name, self.cli_source.as_deref())?;
        let source = source::resolve(&settings.source)?;
        Ok((settings, source))
    }
}

/// A parsed request plus a one-shot channel for its reply, handed from the
/// accept thread to the daemon's render loop.
pub struct DaemonCommand {
    pub request: Request,
    pub reply: SyncSender<Response>,
}

/// Something the daemon can drive in response to a command — i.e. one output's
/// player. The player is optional because a Wayland surface's GL/mpv state is
/// created lazily on first configure.
pub trait Controllable {
    fn output_name(&self) -> &str;
    fn player(&self) -> Option<&MpvPlayer>;
}

/// Owns the listening socket; removes the socket file on drop.
pub struct Server {
    path: PathBuf,
}

impl Server {
    /// Bind the control socket and spawn the accept thread. Returns the server
    /// handle plus the receiver the render loop should drain.
    pub fn start() -> Result<(Server, Receiver<DaemonCommand>)> {
        let path = socket_path();

        // Refuse to start if another daemon is already listening; otherwise
        // clean up a stale socket file left by a previous crash.
        if path.exists() {
            if UnixStream::connect(&path).is_ok() {
                return Err(Error::Config(format!(
                    "another desktobian daemon is already running (socket {})",
                    path.display()
                )));
            }
            let _ = std::fs::remove_file(&path);
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let listener = UnixListener::bind(&path)?;
        let (tx, rx) = std::sync::mpsc::channel();
        thread::Builder::new()
            .name("desktobian-ipc".into())
            .spawn(move || accept_loop(listener, tx))
            .map_err(Error::Io)?;

        log::info!("Control socket listening at {}", path.display());
        Ok((Server { path }, rx))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Accept connections forever, forwarding each request to the render loop.
fn accept_loop(listener: UnixListener, tx: Sender<DaemonCommand>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream, &tx) {
                    log::debug!("ipc connection error: {e}");
                }
            }
            Err(e) => log::debug!("ipc accept error: {e}"),
        }
    }
}

/// Read one request, forward it, and write the reply back.
fn handle_connection(stream: UnixStream, tx: &Sender<DaemonCommand>) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let response = match serde_json::from_str::<Request>(line.trim()) {
        Ok(request) => {
            let (reply_tx, reply_rx) = sync_channel(1);
            match tx.send(DaemonCommand {
                request,
                reply: reply_tx,
            }) {
                Ok(()) => reply_rx
                    .recv()
                    .unwrap_or_else(|_| Response::err("daemon did not reply")),
                Err(_) => Response::err("daemon is shutting down"),
            }
        }
        Err(e) => Response::err(format!("invalid request: {e}")),
    };

    let mut payload = serde_json::to_string(&response).unwrap_or_else(|_| {
        "{\"ok\":false,\"message\":\"failed to serialise response\"}".to_string()
    });
    payload.push('\n');
    writer.write_all(payload.as_bytes())?;
    Ok(())
}

/// Apply a command to the daemon's instances, returning the reply.
pub fn process<C: Controllable>(
    request: &Request,
    instances: &[C],
    ctx: &DaemonContext,
) -> Response {
    match request {
        Request::Set { source, outputs } => {
            let resolved = match source::resolve(source) {
                Ok(r) => r,
                Err(e) => return Response::err(e.to_string()),
            };
            apply(instances, outputs, "set wallpaper on", |p| {
                p.load_source(&resolved)
            })
        }
        Request::Reload { outputs } => reload(instances, outputs, ctx),
        Request::Pause { outputs } => apply(instances, outputs, "paused", |p| p.set_paused(true)),
        Request::Play { outputs } => apply(instances, outputs, "resumed", |p| p.set_paused(false)),
        Request::Toggle { outputs } => apply(instances, outputs, "toggled", |p| p.toggle_paused()),
        Request::Mute { outputs } => apply(instances, outputs, "muted", |p| p.set_muted(true)),
        Request::Unmute { outputs } => apply(instances, outputs, "unmuted", |p| p.set_muted(false)),
        Request::Status => {
            let names: Vec<String> = instances
                .iter()
                .map(|i| i.output_name().to_string())
                .collect();
            Response::ok(format!("daemon running on {} output(s)", names.len())).with_outputs(names)
        }
        Request::Stop => {
            util::request_shutdown();
            Response::ok("shutting down")
        }
    }
}

/// Run `action` on each instance whose output matches `outputs` (empty = all).
fn apply<C: Controllable>(
    instances: &[C],
    outputs: &[String],
    verb: &str,
    action: impl Fn(&MpvPlayer) -> Result<()>,
) -> Response {
    let mut affected = Vec::new();
    for inst in instances {
        if !outputs.is_empty() && !outputs.iter().any(|o| o == inst.output_name()) {
            continue;
        }
        match inst.player() {
            Some(player) => {
                if let Err(e) = action(player) {
                    return Response::err(format!("{}: {e}", inst.output_name()));
                }
                affected.push(inst.output_name().to_string());
            }
            // Surface not initialised yet (Wayland pre-configure) — skip quietly.
            None => continue,
        }
    }
    if affected.is_empty() {
        return Response::err("no matching, initialised outputs");
    }
    Response::ok(format!("{verb} {} output(s)", affected.len())).with_outputs(affected)
}

/// Re-read the config from disk and re-apply each matching output's wallpaper
/// and live settings (mute/volume/scaling).
fn reload<C: Controllable>(instances: &[C], outputs: &[String], ctx: &DaemonContext) -> Response {
    let mut affected = Vec::new();
    for inst in instances {
        if !outputs.is_empty() && !outputs.iter().any(|o| o == inst.output_name()) {
            continue;
        }
        let Some(player) = inst.player() else {
            continue;
        };
        let (settings, source) = match ctx.resolve(inst.output_name()) {
            Ok(v) => v,
            Err(e) => return Response::err(format!("{}: {e}", inst.output_name())),
        };
        if let Err(e) = player
            .load_source(&source)
            .and_then(|_| player.apply_live_settings(&settings))
        {
            return Response::err(format!("{}: {e}", inst.output_name()));
        }
        affected.push(inst.output_name().to_string());
    }
    if affected.is_empty() {
        return Response::err("no matching, initialised outputs");
    }
    Response::ok(format!("reloaded {} output(s)", affected.len())).with_outputs(affected)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake controllable with no real player, for protocol tests.
    struct FakeInstance(&'static str);
    impl Controllable for FakeInstance {
        fn output_name(&self) -> &str {
            self.0
        }
        fn player(&self) -> Option<&MpvPlayer> {
            None
        }
    }

    #[test]
    fn status_lists_outputs() {
        let instances = [FakeInstance("eDP-1"), FakeInstance("HDMI-A-1")];
        let resp = process(&Request::Status, &instances, &DaemonContext::default());
        assert!(resp.ok);
        assert_eq!(resp.outputs, vec!["eDP-1", "HDMI-A-1"]);
    }

    #[test]
    fn stop_requests_shutdown_ok() {
        let instances: [FakeInstance; 0] = [];
        let resp = process(&Request::Stop, &instances, &DaemonContext::default());
        assert!(resp.ok);
        // (sets the global shutdown flag; other backends observe it.)
    }

    #[test]
    fn server_client_round_trip_over_socket() {
        // Isolate this test's socket from any real daemon / parallel tests.
        let dir = std::env::temp_dir().join(format!("desktobian-ipc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("DESKTOBIAN_SOCKET", dir.join("test.sock"));

        let (server, rx) = Server::start().expect("server start");

        // A stand-in daemon: process one command against fake instances.
        let daemon = std::thread::spawn(move || {
            if let Ok(cmd) = rx.recv() {
                let instances = [FakeInstance("DP-1")];
                let resp = process(&cmd.request, &instances, &DaemonContext::default());
                let _ = cmd.reply.try_send(resp);
            }
        });

        let resp = send(&Request::Status).expect("client send");
        assert!(resp.ok);
        assert_eq!(resp.outputs, vec!["DP-1"]);

        daemon.join().unwrap();
        drop(server);
        std::env::remove_var("DESKTOBIAN_SOCKET");
    }
}
