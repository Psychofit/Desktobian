# Roadmap

Desktobian's north star: be the wallpaper engine Linux users actually want —
covering the common Wallpaper Engine use cases natively, on both X11 and
Wayland.

## v0.1 — Animated wallpapers (done / current)

- [x] libmpv render-API video output into an owned OpenGL surface
- [x] X11 backend (Xrandr enumeration + desktop-layer window per monitor)
- [x] Wayland backend (`wlr-layer-shell` background surface per output)
- [x] Multi-monitor, per-output config
- [x] Video / GIF / APNG / image sources, directory playlists
- [x] Wallpaper Engine **video** project folder support (`project.json`)
- [x] TOML config with global + per-output overrides
- [x] Hardware decoding, mute/loop/fit/fps controls

## v0.2 — Polish & control

- [x] Running-daemon IPC (Unix socket) + client commands:
      `set` / `pause` / `play` / `toggle` / `mute` / `unmute` / `status` / `stop`
- [x] `reload` to re-read the config file and re-apply wallpapers at runtime
- [x] Native **KDE Plasma** wallpaper plugin (Plasma 5 & 6) so desktop icons
      stay visible over the video — see [`kde/`](kde/) (tested on Plasma 5)
- [x] **GUI wallpaper manager** (Tauri) — browse a library & apply with a click,
      driving the KDE plugin or the engine daemon — see [`crates/desktobian-gui/`](crates/desktobian-gui/)
- [x] Refactor into a Cargo workspace with a shared `desktobian-core` crate
- [ ] Same native integration for **GNOME** (Shell extension)
- [ ] Pause/throttle when a window covers the desktop or on battery
      (both the standalone engine and the KDE plugin)
- [ ] Hotplug: react to monitors being connected/disconnected at runtime
- [ ] Proper HiDPI / fractional-scale supersampling on Wayland
- [ ] Per-output render threads so multi-monitor X11 keeps full refresh
- [ ] `systemd` integration polish, packaging (AUR, `.deb`, Flatpak)

## v0.3 — Web wallpapers

- [x] **Web wallpapers on KDE** via QtWebEngine in the Plasma plugin
      (`WebUrl`); GUI imports & applies Wallpaper Engine `web` projects
      (experimental)
- [ ] Web wallpapers for the standalone engine (wlroots/X11) via an embedded
      browser surface (WebKitGTK/CEF)
- [x] Basic Wallpaper Engine JS API shim (`wallpaperRegisterAudioListener`,
      `wallpaperPropertyListener` defaults) injected into web wallpapers
- [x] Serve web wallpapers over a localhost http server so the page can
      `fetch()` local assets (Rive/Three.js runtimes, JSON) that `file://` blocks
- [ ] Feed real desktop audio (PipeWire FFT) to audio-reactive web wallpapers
- [x] Pass each wallpaper's real default properties from project.json
      (the KDE web shim reads `general.properties` and delivers the defaults
      via `applyUserProperties`)
- [x] Let users customise web wallpaper properties (colours, sliders, combos,
      toggles) — from the Plasma config UI *and* directly in the GUI manager;
      overrides apply live and persist
- [ ] Wallpaper Engine **web** project support (`index.html` + assets)
- [x] Mouse-interaction plumbing for web wallpapers — passive mode forwards
      cursor + left/middle-clicks as DOM events (right-click stays the Plasma
      menu); an opt-in native-input mode gives wallpapers full real mouse input
- [ ] Audio-responsive plumbing for web wallpapers

## v0.4 — Scene wallpapers

- [ ] Read Wallpaper Engine `.pkg` scene archives
- [ ] Layered 2D scene renderer (sprites, parallax, particles)
- [ ] Shader/effect support compatible with WE's scene format

## Nice-to-haves / exploring

- [ ] A small GUI/tray for picking wallpapers and browsing a library
- [ ] Steam Workshop browsing/import helper
- [ ] GNOME/KDE-native background integration on X11 and Wayland
- [ ] Playlist scheduling (rotate wallpapers on a timer)

Contributions toward any of these are very welcome — see
[CONTRIBUTING.md](CONTRIBUTING.md).
