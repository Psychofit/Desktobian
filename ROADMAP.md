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
- [ ] `reload` to re-read the config file at runtime
- [ ] Pause/throttle when a fullscreen app is focused or on battery
- [ ] Hotplug: react to monitors being connected/disconnected at runtime
- [ ] Proper HiDPI / fractional-scale supersampling on Wayland
- [ ] Per-output render threads so multi-monitor X11 keeps full refresh
- [ ] `systemd` integration polish, packaging (AUR, `.deb`, Flatpak)

## v0.3 — Web wallpapers

- [ ] Embedded browser surface (WebKitGTK or CEF) on the background layer
- [ ] Wallpaper Engine **web** project support (`index.html` + assets)
- [ ] Audio-responsive + mouse-interaction plumbing for web wallpapers

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
