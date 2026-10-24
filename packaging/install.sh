#!/bin/sh
# Installs the built binaries and desktop data into a staging tree.
# Usage: packaging/install.sh DESTDIR [PREFIX] [TARGET_DIR]
# Every package format (deb, rpm, pacman, Flatpak, AppImage) uses this layout.
set -eu
destdir=$1
prefix=${2:-/usr}
target=${3:-target/release}
root=$(dirname "$0")/..
app=io.github.brahm.CoreRules

install -Dm755 "$target/corerules" "$destdir$prefix/bin/corerules"
install -Dm755 "$target/cr" "$destdir$prefix/bin/cr"
install -Dm644 "$root/data/$app.desktop" "$destdir$prefix/share/applications/$app.desktop"
install -Dm644 "$root/data/$app.metainfo.xml" "$destdir$prefix/share/metainfo/$app.metainfo.xml"
for icon in "$root"/data/icons/hicolor/*/apps/$app.png; do
    size=$(basename "$(dirname "$(dirname "$icon")")")
    install -Dm644 "$icon" "$destdir$prefix/share/icons/hicolor/$size/apps/$app.png"
done
install -Dm644 "$root/LICENSE" "$destdir$prefix/share/licenses/corerules/LICENSE"
