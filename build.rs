//! Build script for Desktobian.
//!
//! We link the platform multimedia / GL libraries that the renderer depends on:
//!   * `mpv`  — video decoding & the OpenGL render API (resolved via pkg-config).
//!   * `EGL`  — context/surface creation (the `khronos-egl` crate calls into it).
//!
//! X11 (Xlib/Xrandr) and Wayland are loaded at runtime by their respective
//! crates (`x11-dl` via `dlopen`, `wayland-client` via its system backend), so
//! they do not need explicit link directives here.

fn main() {
    // libmpv: prefer pkg-config so we pick up the right include/lib paths, but
    // fall back to a plain `-lmpv` if pkg-config is unavailable on the host.
    if pkg_config::Config::new()
        .atleast_version("0.36")
        .probe("mpv")
        .is_err()
    {
        println!("cargo:rustc-link-lib=dylib=mpv");
    }

    // EGL is dynamically linked; the `khronos-egl` "static" feature expects the
    // symbol to be resolvable at link time.
    println!("cargo:rustc-link-lib=dylib=EGL");

    println!("cargo:rerun-if-changed=build.rs");
}
