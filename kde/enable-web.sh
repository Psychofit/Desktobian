#!/usr/bin/env bash
#
# Help QtWebEngine render web wallpapers inside plasmashell.
#
# QtWebEngine usually needs the host application to set up a shared OpenGL
# context before startup; plasmashell doesn't, which makes WebEngineView render
# black. Forcing software / in-process GPU sidesteps that. This installs a
# Plasma environment hook with the needed Chromium flags.
set -euo pipefail

dir="${XDG_CONFIG_HOME:-$HOME/.config}/plasma-workspace/env"
mkdir -p "$dir"
file="$dir/desktobian-webengine.sh"

cat > "$file" <<'EOS'
# Installed by Desktobian (kde/enable-web.sh) so QtWebEngine web wallpapers
# render inside plasmashell. Remove this file to undo.
export QTWEBENGINE_CHROMIUM_FLAGS="${QTWEBENGINE_CHROMIUM_FLAGS:-} --disable-gpu --no-sandbox --in-process-gpu"
EOS

echo "Installed $file"
echo
echo "Log out and back in (or reboot) for it to take effect, then re-apply the"
echo "web wallpaper."
echo
echo "To test right now without logging out, run plasmashell from a terminal"
echo "with the flags set:"
echo
echo '  kquitapp5 plasmashell 2>/dev/null || kquitapp6 plasmashell 2>/dev/null'
echo '  QTWEBENGINE_CHROMIUM_FLAGS="--disable-gpu --no-sandbox --in-process-gpu" plasmashell &'
