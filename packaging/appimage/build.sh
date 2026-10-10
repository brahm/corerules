#!/bin/sh
# Builds dist/CoreRules-VERSION-ARCH.AppImage from target/release with
# linuxdeploy and its GTK plugin, which bundle GTK 4, libadwaita and their
# data. Build on the oldest distribution supported (Ubuntu 24.04): the
# AppImage runs on systems with the same glibc or newer.
set -eu
version=$1
arch=$(uname -m)
tools=${TOOLS_DIR:-$PWD/target/appimage-tools}
appdir=$PWD/target/AppDir
mkdir -p "$tools"
fetch() {
    [ -x "$tools/$1" ] || { curl -fsSL -o "$tools/$1" "$2" && chmod +x "$tools/$1"; }
}
fetch linuxdeploy "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$arch.AppImage"
fetch linuxdeploy-plugin-gtk.sh "https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh"
fetch appimagetool "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$arch.AppImage"

rm -rf "$appdir"
packaging/install.sh "$appdir" /usr
# The GTK plugin reads the version to bundle from DEPLOY_GTK_VERSION.
export DEPLOY_GTK_VERSION=4 APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=1
PATH="$tools:$PATH" linuxdeploy --appdir "$appdir" \
    --executable "$appdir/usr/bin/corerules" \
    --desktop-file "$appdir/usr/share/applications/io.github.brahm.CoreRules.desktop" \
    --icon-file "$appdir/usr/share/icons/hicolor/scalable/apps/io.github.brahm.CoreRules.svg" \
    --plugin gtk
# libadwaita draws its own light and dark styles; a GTK_THEME set by the
# plugin's hook only makes it warn and breaks the dark style.
sed -i '/export GTK_THEME=/d' "$appdir"/apprun-hooks/*gtk*.sh
# The app plays no video, and GStreamer is not bundled.
rm -rf "$appdir"/usr/lib/gtk-4.0/4.0.0/media
mkdir -p dist
VERSION=$version ARCH=$arch "$tools/appimagetool" --no-appstream "$appdir" \
    "dist/CoreRules-$version-$arch.AppImage"
