//! Video playback via libmpv's OpenGL render API.

mod ffi;
mod mpv;

pub use mpv::{MpvPlayer, NativeDisplay};
