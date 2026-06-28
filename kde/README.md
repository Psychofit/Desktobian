# Desktobian Video — KDE Plasma wallpaper plugin

A **native KDE Plasma** wallpaper plugin (Plasma 5 & 6) that plays a looping
video/GIF **or a web (HTML/JS) wallpaper** as your desktop background — using
Plasma's own wallpaper layer, so your **desktop icons and widgets stay visible
on top**.

This is the recommended way to use Desktobian on KDE Plasma. (The standalone
`desktobian` binary targets wlroots-Wayland compositors and minimal X11 WMs,
where it draws an external desktop-layer window; on a full Plasma desktop that
window would cover the icons — hence this plugin.)

It's a pure-QML package built on `QtMultimedia` — no compilation required.
**Both Plasma 5 and Plasma 6 are supported** via two package variants; the
installer picks the right one automatically.

## Requirements

- **KDE Plasma 5 or 6.**
- `QtMultimedia` runtime + GStreamer codecs (for **video** wallpapers). If a
  video doesn't play, install:
  - Plasma 6 / Qt 6: `qml6-module-qtmultimedia`
  - Plasma 5 / Qt 5: `qml-module-qtmultimedia`
  - plus codecs: `gstreamer1.0-libav gstreamer1.0-plugins-good gstreamer1.0-plugins-bad`
- `QtWebEngine` (for **web** wallpapers — optional, only loaded when you use one):
  - Plasma 6 / Qt 6: `qml6-module-qtwebengine`
  - Plasma 5 / Qt 5: `qml-module-qtwebengine`

  If a web wallpaper shows up **black**, QtWebEngine couldn't initialise its GPU
  context inside plasmashell. Run `./enable-web.sh` (installs a Plasma env hook
  with `--disable-gpu --no-sandbox --in-process-gpu`) and log out/in.

## Install

```sh
cd kde
./install.sh        # detects Plasma 5 vs 6 and installs the matching variant
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
kde/
  install.sh                 detects Plasma version, installs the right variant
  plasma6/org.desktobian.video/
    metadata.json            Plasma 6 plugin manifest
    contents/config/main.xml config schema (KConfigXT)
    contents/ui/main.qml     video wallpaper (Qt 6 QtMultimedia)
    contents/ui/config.qml   settings UI (Qt 6)
  plasma5/org.desktobian.video/
    metadata.desktop         Plasma 5 plugin manifest
    contents/config/main.xml config schema (KConfigXT, identical)
    contents/ui/main.qml     video wallpaper (Qt 5 QtMultimedia)
    contents/ui/config.qml   settings UI (Qt 5)
```

The two variants differ only because the QtMultimedia QML API and the plugin
manifest format changed between Qt 5 / Plasma 5 and Qt 6 / Plasma 6.
