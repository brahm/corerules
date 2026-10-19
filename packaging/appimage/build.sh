#!/bin/sh
# Builds dist/CoreRules-VERSION-ARCH.AppImage from target/release with
# linuxdeploy and its GTK plugin, which bundle GTK 4, libadwaita and their
# data from the system it runs on. `build.sh VERSION addons` builds the
# Community Add-ons Collection instead,
# dist/CoreRules-Community-Addons-VERSION-ARCH.AppImage.
# Releases run it inside the GNOME 51 SDK (build-in-sdk.sh), so the
# AppImage carries that platform's libraries and runs on systems with the
# SDK's glibc or newer.
set -eu
version=$1
arch=$(uname -m)
case ${2:-app} in
    app) app=io.github.brahm.CoreRules exe=corerules name=CoreRules ;;
    addons) app=io.github.brahm.CoreRules.Addons exe=corerules-community-addons name=CoreRules-Community-Addons ;;
    *) echo "$0: unknown target $2" >&2; exit 2 ;;
esac
tools=${TOOLS_DIR:-$PWD/target/appimage-tools}
appdir=$PWD/target/AppDir
mkdir -p "$tools"
fetch() {
    [ -x "$tools/$1" ] || { curl -fsSL -o "$tools/$1" "$2" && chmod +x "$tools/$1"; }
}
fetch linuxdeploy "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$arch.AppImage"
fetch linuxdeploy-plugin-gtk.sh "https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh"
fetch appimagetool "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$arch.AppImage"
# Newer GNOME platforms decode images with glycin and ship gdk-pixbuf
# without a loaders directory; the plugin then has nothing to copy there.
# GTK 4 reads PNG, JPEG and TIFF itself.
plugin=$tools/linuxdeploy-plugin-gtk.sh
grep -q 'pixbuf loaders, if any' "$plugin" || sed -i \
    -e 's|^echo "Installing GDK PixBufs"$|if [ -d "$(pkg-config --variable=gdk_pixbuf_binarydir gdk-pixbuf-2.0)" ]; then # pixbuf loaders, if any\n&|' \
    -e 's|^echo "Copying more libraries"$|fi\n&|' "$plugin"

rm -rf "$appdir"
if [ "$exe" = corerules ]; then
    packaging/install.sh "$appdir" /usr
else
    install -Dm755 "target/release/$exe" "$appdir/usr/bin/$exe"
    install -Dm644 "data/addons/$app.desktop" "$appdir/usr/share/applications/$app.desktop"
    for icon in data/addons/icons/hicolor/*/apps/$app.png; do
        size=$(basename "$(dirname "$(dirname "$icon")")")
        install -Dm644 "$icon" "$appdir/usr/share/icons/hicolor/$size/apps/$app.png"
    done
fi
# The GTK plugin reads the version to bundle from DEPLOY_GTK_VERSION.
export DEPLOY_GTK_VERSION=4 APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=1
PATH="$tools:$PATH" linuxdeploy --appdir "$appdir" \
    --executable "$appdir/usr/bin/$exe" \
    --desktop-file "$appdir/usr/share/applications/$app.desktop" \
    --icon-file "$appdir/usr/share/icons/hicolor/256x256/apps/$app.png" \
    --plugin gtk
# libadwaita draws its own light and dark styles; a GTK_THEME set by the
# plugin's hook only makes it warn and breaks the dark style.
sed -i '/export GTK_THEME=/d' "$appdir"/apprun-hooks/*gtk*.sh
# linuxdeploy links the smallest bundled size; file managers show .DirIcon.
ln -sf "usr/share/icons/hicolor/256x256/apps/$app.png" "$appdir/$app.png"
# The app plays no video, and GStreamer is not bundled.
rm -rf "$appdir"/usr/lib/gtk-4.0/4.0.0/media
mkdir -p dist
VERSION=$version ARCH=$arch "$tools/appimagetool" --no-appstream "$appdir" \
    "dist/$name-$version-$arch.AppImage"
