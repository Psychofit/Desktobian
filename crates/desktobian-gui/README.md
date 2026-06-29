# Desktobian GUI — wallpaper manager

A lightweight desktop application (built with [Tauri](https://tauri.app)) to
**browse a library of wallpapers and apply one** with a click. It works across
desktop environments because it doesn't render wallpapers itself — it drives the
native renderer:

- on **KDE Plasma**, it sets the [Plasma plugin](../../kde/) via plasmashell
  D-Bus (so your desktop icons stay visible);
- **elsewhere** (wlroots/X11), it tells the standalone `desktobian` engine
  daemon to switch wallpaper over its control socket.

## What it does

- Scans your **Videos** folder, `~/Wallpapers`, and the Steam **Workshop**
  folder for Wallpaper Engine (appid 431960) — so existing video wallpapers and
  Workshop items show up automatically.
- Shows a grid of **thumbnails** (generated with `ffmpeg`, cached).
- **Apply** a wallpaper, choosing mute and fit (crop/fit/stretch).
- **Configure web wallpaper properties in-app** (KDE): selecting a web
  wallpaper shows a settings panel built from its `project.json` — colours,
  sliders, drop-downs, toggles and text fields. Changes apply live (no need to
  open Plasma's separate *Configure Desktop and Wallpaper…* dialog) and are
  remembered per wallpaper; **Reset to defaults** clears them.
- Add an extra folder, or pick a single video, with native file dialogs.

## Requirements

- Rust toolchain.
- Tauri's Linux WebView/GTK deps:
  ```sh
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
  ```
- `ffmpeg` (optional, for thumbnails): `sudo apt install ffmpeg`
- For applying on KDE: the [Plasma plugin](../../kde/) installed (`kde/install.sh`),
  and `qdbus`. For applying elsewhere: a running `desktobian run` daemon.

## Run / build

```sh
# from the repo root
cargo run -p desktobian-gui                 # dev run
cargo build -p desktobian-gui --release     # release binary at target/release/desktobian-gui
```

The frontend is plain static HTML/CSS/JS (in `frontend/`) — no Node/bundler step
is required.

## Window & tray behaviour

- Closing the window **minimises it to the system tray** — the wallpaper keeps
  playing.
- Use the tray's **Quit** entry to actually exit; on quit it restores a default
  wallpaper (on KDE, the standard image wallpaper plugin).

## How "Apply" works under the hood

| Content        | KDE Plasma                                                              | Other (wlroots/X11)            |
| -------------- | ---------------------------------------------------------------------- | ------------------------------ |
| Video / GIF    | `org.desktobian.video` plugin via plasmashell `evaluateScript`         | `ipc::send(Request::Set { … })` to the daemon |
| Still image    | `org.kde.image` (KDE's built-in image wallpaper)                       | engine displays it via mpv     |
| Web (HTML/JS)  | `org.desktobian.video` plugin's `WebUrl` (QtWebEngine)                 | not supported yet              |

> Web wallpapers need the QtWebEngine QML module (`qml-module-qtwebengine` on
> Plasma 5, `qml6-module-qtwebengine` on Plasma 6).
