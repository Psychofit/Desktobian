# Contributing to Desktobian

Thanks for wanting to help build a wallpaper engine for Linux! This is an
early-stage project, so there's lots of high-impact work available.

## Getting set up

1. Install the dependencies listed in the [README](README.md#requirements).
2. Build and run the tests:
   ```sh
   cargo build
   cargo test
   cargo clippy --all-targets
   cargo fmt --check
   ```
3. Run it against a real session:
   ```sh
   cargo run -- --source /path/to/a/video.mp4 -vv
   ```

CI runs `fmt`, `clippy` (warnings denied) and `test` on every PR, so running
those four commands locally before pushing keeps things green.

## Project layout

```
src/
  main.rs          entry point + CLI dispatch
  cli.rs           argument parsing (clap)
  config.rs        TOML config model + merge logic (unit-tested)
  source.rs        resolve a path → playable media / WE project (unit-tested)
  monitor.rs       backend-agnostic Output descriptor
  app.rs           load config, pick backend, build plans, run
  player/          libmpv wrapper
    ffi.rs         hand-written libmpv FFI (from the system headers)
    mpv.rs         safe-ish MpvPlayer over the render API
  render/
    egl.rs         EGL/OpenGL context + surface helpers
  backend/
    mod.rs         Backend trait, auto-detection, plan building
    x11.rs         Xlib + Xrandr + EGL desktop window
    wayland.rs     wlr-layer-shell + EGL background surface
  util.rs          signal handling
```

The architecture deliberately keeps **platform-independent logic pure and
testable** (`config`, `source`, `monitor`) and isolates the `unsafe` FFI in the
`player`, `render`, and `backend` modules.

## Guidelines

- **Match the surrounding style.** Run `cargo fmt`; keep comments explaining
  *why*, not *what*.
- **Justify `unsafe`.** Every `unsafe` block should have a `// SAFETY:` comment
  stating the invariant that makes it sound.
- **Keep it buildable on both backends.** A change to one backend shouldn't
  break the other; the shared abstractions (`Output`, `MpvPlayer`, EGL helpers)
  exist to keep them symmetric.
- **Add tests** for anything in the pure modules (`config`, `source`, …).
- **Small, focused PRs** are much easier to review than large ones.

## Good first issues

- Test the X11/Wayland backends on a compositor you use and report what happens.
- A `desktobian set <path>` command + a tiny IPC socket to a running daemon.
- "Pause when a fullscreen window is focused" for the X11 backend.
- Packaging (AUR PKGBUILD, `.deb`, Flatpak manifest).

By contributing you agree your work is licensed under the project's Apache-2.0
license.
