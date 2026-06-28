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
