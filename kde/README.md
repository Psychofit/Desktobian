# Desktobian Video — KDE Plasma wallpaper plugin

A **native KDE Plasma 6** wallpaper plugin that plays a looping video/GIF as your
desktop background — using Plasma's own wallpaper layer, so your **desktop icons
and widgets stay visible on top** of the video.

This is the recommended way to use Desktobian on KDE Plasma. (The standalone
`desktobian` binary targets wlroots-Wayland compositors and minimal X11 WMs,
where it draws an external desktop-layer window; on a full Plasma desktop that
window would cover the icons — hence this plugin.)

It's a pure-QML package built on `QtMultimedia` — no compilation required.

## Requirements

- **KDE Plasma 6** (Qt 6).
- `QtMultimedia` runtime + GStreamer codecs (usually already present on Kubuntu;
  if a video doesn't play, install `qml6-module-qtmultimedia` and the
  `gstreamer1.0-libav` / `gstreamer1.0-plugins-{good,bad}` packages).

> On **Plasma 5** (Qt 5) the QML differs (QtMultimedia and metadata changed
> between Qt 5 and 6). If you're on Plasma 5, open an issue / let us know and we
> can ship a 5.x variant.

## Install

```sh
cd kde
./install.sh
```

Then: **right-click the desktop → Configure Desktop and Wallpaper…**, set
**Wallpaper type** to **"Desktobian Video"**, pick a video, and **Apply**.

If it doesn't appear right away, restart the shell:

```sh
kquitapp6 plasmashell && (kstart plasmashell >/dev/null 2>&1 &)
```

## Uninstall

```sh
kpackagetool6 --type Plasma/Wallpaper --remove org.desktobian.video
# or, if installed by copy:
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/plasma/wallpapers/org.desktobian.video"
```

## Settings

| Setting    | Meaning                                              |
| ---------- | ---------------------------------------------------- |
| Video      | The video / GIF file to loop                         |
| Muted      | Mute audio (on by default)                           |
| Volume     | 0–100 (when not muted)                               |
| Fill mode  | Stretch / Fit (letterbox) / Crop (fill, default)     |
| Loop       | Loop the video forever                               |

## Package layout

```
org.desktobian.video/
  metadata.json              Plasma/Wallpaper plugin manifest
  contents/
    config/main.xml          config schema (KConfigXT)
    ui/main.qml              the video wallpaper itself (QtMultimedia)
    ui/config.qml            the settings UI
```
