//! Shared core for Desktobian.
//!
//! This crate holds the parts that are useful both to the `desktobian` engine
//! binary and to the `desktobian-gui` manager application:
//!
//!   * [`config`] — the TOML configuration model;
//!   * [`source`] — resolving a path into playable media (incl. Wallpaper
//!     Engine project folders);
//!   * [`webprops`] — parsing a Wallpaper Engine web project's editable
//!     properties (`general.properties`) for the manager's property editor;
//!   * [`monitor`] — a backend-agnostic display-output descriptor;
//!   * [`ipc`] — the control-socket **protocol** ([`ipc::Request`] /
//!     [`ipc::Response`]) and a [`ipc::send`] client used to talk to a running
//!     engine daemon;
//!   * [`error`] — the shared error type.

pub mod config;
pub mod error;
pub mod ipc;
pub mod monitor;
pub mod source;
pub mod webprops;
