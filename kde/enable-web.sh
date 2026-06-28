#!/usr/bin/env bash
#
# Help QtWebEngine render web wallpapers inside plasmashell.
#
# QtWebEngine usually needs the host application to set up a shared OpenGL
# context before startup; plasmashell doesn't, which makes WebEngineView render
# black. This installs a Plasma environment hook with Chromium flags that work
# around it.
#
# Two modes — pick whichever makes your wallpapers render (re-run to switch):
#   (default)  software / in-process GPU. Most compatible, but heavy WebGL
#              wallpapers can be slow.
#   --gpu      keep hardware acceleration. Faster and fixes some black WebGL
#              wallpapers, but on some drivers it renders black instead.
set -euo pipefail

mode="software"
if [[ "${1:-}" == "--gpu" ]]; then
  mode="gpu"
fi

# Flags that help in plasmashell regardless of GPU mode. --disable-web-security
# and --allow-file-access-from-files let web wallpapers load cross-origin / local
# assets: many fetch a runtime (e.g. Rive/Three.js WASM) from a CDN or read local
# files from a file:// page, which CORS would otherwise block (black screen).
common="--no-sandbox --in-process-gpu --disable-web-security --allow-file-access-from-files"
if [[ "$mode" == "gpu" ]]; then
  flags="$common --ignore-gpu-blocklist --enable-gpu-rasterization"
else
  flags="$common --disable-gpu"
fi

dir="${XDG_CONFIG_HOME:-$HOME/.config}/plasma-workspace/env"
mkdir -p "$dir"
file="$dir/desktobian-webengine.sh"

cat > "$file" <<EOS
# Installed by Desktobian (kde/enable-web.sh, mode: $mode) so QtWebEngine web
# wallpapers render inside plasmashell. Remove this file to undo.
export QTWEBENGINE_CHROMIUM_FLAGS="\${QTWEBENGINE_CHROMIUM_FLAGS:-} $flags"
EOS

echo "Installed $file (mode: $mode)"
echo "Flags: $flags"

# --- Local static server for web wallpapers --------------------------------
# QtWebEngine can't fetch() file:// URLs, so web wallpapers that load local
# assets (e.g. Rive .riv files) need to be served over http. Install a tiny
# localhost-only server, enable it at login (XDG autostart) and start it now.
if command -v python3 >/dev/null 2>&1; then
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/desktobian"
  mkdir -p "$data_dir"
  install -m 0755 "$script_dir/desktobian-webserver.py" "$data_dir/desktobian-webserver.py"

  autostart_dir="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
  mkdir -p "$autostart_dir"
  cat > "$autostart_dir/desktobian-webserver.desktop" <<EOS
[Desktop Entry]
Type=Application
Name=Desktobian Web Wallpaper Server
Comment=Serves web wallpapers over http://127.0.0.1:47821 so they can fetch local assets
Exec=python3 $data_dir/desktobian-webserver.py
X-KDE-autostart-phase=1
NoDisplay=true
EOS

  # A second instance just exits (port in use), so this is safe to re-run.
  nohup python3 "$data_dir/desktobian-webserver.py" >/dev/null 2>&1 &
  disown 2>/dev/null || true
  echo "Web server: http://127.0.0.1:47821 (autostart enabled, started now)"
else
  echo "WARNING: python3 not found — web wallpapers that fetch local assets (e.g. Rive) won't load."
fi
echo
echo "Log out and back in (or reboot) for it to take effect, then re-apply the"
echo "web wallpaper."
echo
echo "If wallpapers render black, re-run in the other mode:"
if [[ "$mode" == "gpu" ]]; then
  echo "  ./enable-web.sh         # software fallback"
else
  echo "  ./enable-web.sh --gpu   # hardware acceleration"
fi
echo
echo "To test right now without logging out, run plasmashell from a terminal:"
echo
echo '  kquitapp5 plasmashell 2>/dev/null || kquitapp6 plasmashell 2>/dev/null'
echo "  QTWEBENGINE_CHROMIUM_FLAGS=\"$flags\" plasmashell &"
