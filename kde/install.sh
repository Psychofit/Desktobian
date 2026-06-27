#!/usr/bin/env bash
#
# Install the Desktobian Video wallpaper plugin for KDE Plasma 6.
#
# Prefers kpackagetool6 (proper registration); falls back to a plain copy into
# the user's Plasma wallpapers directory.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
pkg="$here/org.desktobian.video"

if command -v kpackagetool6 >/dev/null 2>&1; then
    if kpackagetool6 --type Plasma/Wallpaper --list 2>/dev/null | grep -q org.desktobian.video; then
        echo "Upgrading existing package…"
        kpackagetool6 --type Plasma/Wallpaper --upgrade "$pkg"
    else
        kpackagetool6 --type Plasma/Wallpaper --install "$pkg"
    fi
else
    dest="${XDG_DATA_HOME:-$HOME/.local/share}/plasma/wallpapers/org.desktobian.video"
    echo "kpackagetool6 not found; copying to $dest"
    mkdir -p "$(dirname "$dest")"
    rm -rf "$dest"
    cp -r "$pkg" "$dest"
fi

cat <<'EOF'

Installed. To use it:
  Right-click the desktop -> Configure Desktop and Wallpaper…
  -> set "Wallpaper type" to "Desktobian Video" -> pick a video -> Apply.

If it doesn't show up immediately, restart plasmashell:
  kquitapp6 plasmashell && (kstart plasmashell >/dev/null 2>&1 &)
(or just log out and back in).
EOF
