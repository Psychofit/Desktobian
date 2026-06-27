//! OpenGL/EGL rendering primitives shared by the display backends.

mod egl;

pub use egl::{mpv_get_proc_address, EglDisplay, GlSurface};
