//! Control-IPC protocol and client.
//!
//! A running `desktobian` engine daemon listens on a Unix domain socket. This
//! module defines the newline-delimited JSON [`Request`] / [`Response`] protocol
//! and a [`send`] client. Both the CLI and the GUI manager use [`send`] to drive
//! the daemon; the daemon's server side lives in the engine binary.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A request sent by a client to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "lowercase")]
pub enum Request {
    /// Swap the wallpaper to `source` (file / dir / WE project folder).
    Set {
        source: PathBuf,
        #[serde(default)]
        outputs: Vec<String>,
    },
    /// Pause playback.
    Pause {
        #[serde(default)]
        outputs: Vec<String>,
    },
    /// Resume playback.
    Play {
        #[serde(default)]
        outputs: Vec<String>,
    },
    /// Toggle pause.
    Toggle {
        #[serde(default)]
        outputs: Vec<String>,
    },
    /// Mute audio.
    Mute {
        #[serde(default)]
        outputs: Vec<String>,
    },
    /// Unmute audio.
    Unmute {
        #[serde(default)]
        outputs: Vec<String>,
    },
    /// Re-read the config file and re-apply each output's wallpaper & settings.
    Reload {
        #[serde(default)]
        outputs: Vec<String>,
    },
    /// Report the daemon's active outputs.
    Status,
    /// Ask the daemon to shut down.
    Stop,
}

/// The daemon's reply to a [`Request`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
}

impl Response {
    /// A successful reply with a message.
    pub fn ok(message: impl Into<String>) -> Self {
        Response {
            ok: true,
            message: message.into(),
            outputs: Vec::new(),
        }
    }
    /// A failure reply with a message.
    pub fn err(message: impl Into<String>) -> Self {
        Response {
            ok: false,
            message: message.into(),
            outputs: Vec::new(),
        }
    }
    /// Attach the list of affected/active outputs.
    pub fn with_outputs(mut self, outputs: Vec<String>) -> Self {
        self.outputs = outputs;
        self
    }
}

/// The control-socket path. Resolution order:
///   1. `$DESKTOBIAN_SOCKET` (explicit override);
///   2. `$XDG_RUNTIME_DIR/desktobian.sock`;
///   3. `/tmp/desktobian-<uid>.sock`.
pub fn socket_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DESKTOBIAN_SOCKET") {
        return PathBuf::from(path);
    }
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("desktobian.sock");
    }
    // SAFETY: getuid is always safe and never fails.
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/desktobian-{uid}.sock"))
}

/// Connect to the daemon, send `request`, and return its reply.
pub fn send(request: &Request) -> Result<Response> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        Error::Config(format!(
            "cannot reach the desktobian daemon at {} ({e}); is it running? \
             start it with `desktobian run`",
            path.display()
        ))
    })?;

    let mut payload = serde_json::to_string(request).map_err(|e| Error::Config(e.to_string()))?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str(line.trim())
        .map_err(|e| Error::Config(format!("malformed daemon reply: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_through_json() {
        let req = Request::Set {
            source: PathBuf::from("/tmp/a.mp4"),
            outputs: vec!["HDMI-A-1".into()],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"cmd\":\"set\""));
        let back: Request = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Request::Set { .. }));
    }

    #[test]
    fn response_skips_empty_outputs() {
        let json = serde_json::to_string(&Response::ok("hi")).unwrap();
        assert!(!json.contains("outputs"));
        let json =
            serde_json::to_string(&Response::ok("hi").with_outputs(vec!["DP-1".into()])).unwrap();
        assert!(json.contains("DP-1"));
    }
}
