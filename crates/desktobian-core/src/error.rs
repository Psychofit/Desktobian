//! Error types shared across Desktobian.

use std::path::PathBuf;

/// Top-level result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur while resolving sources, talking to mpv, or driving a
/// display backend.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("could not read config file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not parse config file {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("wallpaper source not found: {0}")]
    SourceNotFound(PathBuf),

    #[error("unsupported wallpaper source {path}: {reason}")]
    UnsupportedSource { path: PathBuf, reason: String },

    #[error("mpv error: {0}")]
    Mpv(String),

    #[error("EGL error: {0}")]
    Egl(String),

    #[error("X11 backend error: {0}")]
    X11(String),

    #[error("Wayland backend error: {0}")]
    Wayland(String),

    #[error("no outputs/monitors detected")]
    NoOutputs,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Convenience constructor for ad-hoc mpv errors.
    pub fn mpv(msg: impl Into<String>) -> Self {
        Error::Mpv(msg.into())
    }

    /// Convenience constructor for ad-hoc EGL errors.
    pub fn egl(msg: impl Into<String>) -> Self {
        Error::Egl(msg.into())
    }
}
