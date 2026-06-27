# Desktobian

**An open-source [Wallpaper Engine](https://www.wallpaperengine.io/) alternative for Linux.**

Desktobian brings animated video & GIF wallpapers to the Linux desktop on **both
X11 and Wayland**. It renders your videos straight onto the desktop background
using [libmpv](https://mpv.io/) and OpenGL — hardware-accelerated, looping,
multi-monitor, and light on resources.

It was born out of a simple frustration: Wallpaper Engine is the one thing many
people genuinely miss after switching to Linux, and the upstream developers have
no plans to port it. So this is the community doing it itself.

> **Status: early but functional (v0.1).** Video/GIF/image wallpapers work on
> X11 and wlroots-based Wayland compositors. Scene (`.pkg`) and web wallpapers
> are on the [roadmap](ROADMAP.md).

---

## Features

- 🎞️ **Animated wallpapers** — play any video, GIF or APNG mpv can decode (mp4,
  mkv, webm, mov, …) as your desktop background.
- 🖥️ **X11 _and_ Wayland** — a desktop-layer window on X11, a `wlr-layer-shell`
  background surface on Wayland. One tool, both worlds.
- 🪟 **Multi-monitor** — a different (or the same) wallpaper per output, matched
  by connector name (`eDP-1`, `HDMI-A-1`, …).
- ⚡ **Hardware decoding** — VAAPI/NVDEC via mpv's `hwdec`, so a looping 4K video
  doesn't cook your CPU.
- 🔇 **Sensible wallpaper defaults** — muted, looping, "cover" scaling out of the
  box; everything overridable.
- 🧩 **Wallpaper Engine compatibility** — point it at a Steam Workshop **video**
  project folder (with a `project.json`) and it just works.
- 🛠️ **One small static-ish binary**, configured by a single TOML file.

## How it works

```
            ┌─────────────┐     OpenGL/EGL     ┌──────────────────────┐
  video ──▶ │   libmpv    │ ─────render API──▶ │  desktop background   │
            │  (decode)   │                    │  X11 window / layer   │
            └─────────────┘                    └──────────────────────┘
```

Desktobian owns the background surface (an override-redirect desktop window on
X11, or a `wlr-layer-shell` surface on the *background* layer on Wayland),
creates an EGL/OpenGL context on it, and hands that context to libmpv's
[render API](https://github.com/mpv-player/mpv/blob/master/libmpv/render.h).
mpv decodes and draws each frame into our framebuffer; we present it.

## Requirements

Runtime + build dependencies:

| Need            | Debian/Ubuntu                                  | Fedora                         | Arch                |
| --------------- | ---------------------------------------------- | ------------------------------ | ------------------- |
| libmpv          | `libmpv-dev` (build) / `libmpv2` (run)         | `mpv-libs-devel`               | `mpv`               |
| EGL + OpenGL    | `libegl1-mesa-dev libgl1-mesa-dev`             | `mesa-libEGL-devel mesa-libGL-devel` | `mesa`        |
| Wayland         | `libwayland-dev`                               | `wayland-devel`                | `wayland`           |
| X11             | `libx11-dev libxrandr-dev`                     | `libX11-devel libXrandr-devel` | `libx11 libxrandr`  |
| Rust toolchain  | `rustup` (≥ 1.74)                              | `rustup`                       | `rustup`            |

On Wayland you also need a compositor that implements `wlr-layer-shell`
(Sway, Hyprland, river, Wayfire, Niri, …). **GNOME and KDE on Wayland do not**
implement it — use the X11 backend there for now.

## Build & install

```sh
git clone https://github.com/psychofit/desktobian
cd desktobian
cargo build --release
install -Dm755 target/release/desktobian ~/.local/bin/desktobian
```

## Usage

Quickest possible start — just point it at a file:

```sh
desktobian --source ~/Wallpapers/forest-loop.mp4
```

Other handy commands:

```sh
desktobian list-outputs                 # see your monitors as Desktobian sees them
desktobian --source ~/wp.mp4 -o HDMI-A-1   # only one monitor
desktobian -b x11 --source ~/wp.mp4        # force a backend
desktobian -v                               # more logging (-vv for trace)
```

Run it from your compositor's autostart (Sway `exec`, Hyprland `exec-once`, an
X11 `.desktop` autostart entry, or the provided systemd user service in
[`packaging/`](packaging/)).

## Configuration

Desktobian reads `~/.config/desktobian/config.toml`. A `[default]` section
applies to every monitor; `[output.<NAME>]` sections override per connector.
See [`examples/config.toml`](examples/config.toml) for a fully-commented sample.

```toml
[default]
source = "~/Wallpapers/forest-loop.mp4"
mute   = true
fit    = "cover"          # cover | contain | fill | center
hwdec  = "auto-safe"

# A different wallpaper on the external monitor:
[output.HDMI-A-1]
source = "~/Wallpapers/city-rain.mp4"
```

| Key           | Default     | Meaning                                                |
| ------------- | ----------- | ------------------------------------------------------ |
| `source`      | —           | File, directory (playlist) or WE project folder        |
| `mute`        | `true`      | Mute audio                                             |
| `volume`      | `100`       | 0–100 (when not muted)                                 |
| `fit`         | `cover`     | Scaling mode                                           |
| `loop`        | `true`      | Loop forever                                           |
| `fps`         | `0`         | Frame-rate cap; `0` follows the monitor refresh        |
| `hwdec`       | `auto-safe` | mpv hardware-decode mode (`no`, `vaapi`, `nvdec`, …)   |
| `mpv_options` | `[]`        | Raw mpv options passthrough, e.g. `["--brightness=-5"]`|

## Limitations (v0.1)

- X11 desktop integration uses an override-redirect window — perfect on minimal
  WMs (i3, bspwm, openbox, …); on full GNOME/KDE X11 sessions the result depends
  on how the DE paints its own desktop.
- Only **video/GIF/image** wallpapers so far. Scene & web are planned.
- HiDPI on Wayland renders at the compositor-suggested size (no per-output
  fractional supersampling yet).

See [ROADMAP.md](ROADMAP.md) for what's next, and
[CONTRIBUTING.md](CONTRIBUTING.md) if you'd like to help build it.

## License

Apache-2.0. See [LICENSE](LICENSE).

Desktobian is an independent project and is not affiliated with or endorsed by
Wallpaper Engine or Valve. "Wallpaper Engine" is a trademark of its respective
owner; it is referenced here only to describe the kind of tool this is.
