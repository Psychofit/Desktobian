#!/usr/bin/env bash
#
# Install the Desktobian Video wallpaper plugin for KDE Plasma.
#
# Auto-detects Plasma 5 vs 6 and installs the matching package variant
# (kde/plasma5/ or kde/plasma6/). Prefers kpackagetool; falls back to copying
# into the user's Plasma wallpapers directory.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"

# --- Detect Plasma major version -------------------------------------------
major=""
if command -v plasmashell >/dev/null 2>&1; then
    major="$(plasmashell --version 2>/dev/null | grep -oE '[0-9]+' | head -1 || true)"
fi
if [ -z "$major" ]; then
    if command -v kpackagetool6 >/dev/null 2>&1; then
        major=6
    else
        major=5
    fi
fi

case "$major" in
    6) variant=plasma6; tool=kpackagetool6 ;;
    5) variant=plasma5; tool=kpackagetool5 ;;
    *) echo "Unsupported Plasma version: $major" >&2; exit 1 ;;
esac

pkg="$here/$variant/org.desktobian.video"
echo "Detected Plasma $major; installing $variant package."

# --- Install ----------------------------------------------------------------
if command -v "$tool" >/dev/null 2>&1; then
    if "$tool" --type Plasma/Wallpaper --list 2>/dev/null | grep -q org.desktobian.video; then
        "$tool" --type Plasma/Wallpaper --upgrade "$pkg"
    else
        "$tool" --type Plasma/Wallpaper --install "$pkg"
    fi
else
    dest="${XDG_DATA_HOME:-$HOME/.local/share}/plasma/wallpapers/org.desktobian.video"
    echo "$tool not found; copying to $dest"
    mkdir -p "$(dirname "$dest")"
    rm -rf "$dest"
    cp -r "$pkg" "$dest"
fi

# --- Done -------------------------------------------------------------------
echo
echo "Installed. To use it:"
echo "  Right-click the desktop -> Configure Desktop and Wallpaper…"
echo "  -> set 'Wallpaper type' to 'Desktobian Video' -> pick a video -> Apply."
echo
if [ "$major" = "6" ]; then
    echo "If it doesn't show up, restart the shell:"
    echo "  kquitapp6 plasmashell && (kstart plasmashell >/dev/null 2>&1 &)"
else
    echo "If it doesn't show up, restart the shell:"
    echo "  kquitapp5 plasmashell && (kstart5 plasmashell >/dev/null 2>&1 &)"
fi
echo "(or just log out and back in)."
