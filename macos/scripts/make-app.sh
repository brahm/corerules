#!/bin/sh
# Builds the Mac app, Core Rules.app, from the bridge (make-bindings.sh
# first, with the same PROFILE) and the SwiftUI sources in CoreRules/, and
# packs it in target/macos/CoreRules-<version>-arm64.dmg. The app is
# signed ad hoc, with no Apple developer account: macOS asks once, in
# System Settings, before opening it (Package/Read Me First.txt).
# macOS on Apple Silicon only.
set -eu
cd "$(dirname "$0")/../.."
profile=${PROFILE:-release}
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
out=target/macos
app="$out/Core Rules.app"
rm -rf "$out"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

optimize=-O
[ "$profile" = release ] || optimize=-Onone
swiftc $optimize -parse-as-library -module-name CoreRules \
    -target arm64-apple-macos14 \
    -I target/swift \
    target/swift/cr_mac_ffi.swift macos/CoreRules/*.swift \
    "target/$profile/libcr_mac_ffi.a" \
    -o "$app/Contents/MacOS/CoreRules"

sed "s/@VERSION@/$version/g" macos/Package/Info.plist > "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"
# The languages the menus macOS draws follow, as the interface's catalogs.
for language in en pt-BR es it fr de; do
    mkdir -p "$app/Contents/Resources/$language.lproj"
done
# The license of the Material Design Icons sword the sidebar draws,
# which About shows.
cp crates/cr-win/package/MaterialDesignIcons-LICENSE.txt "$app/Contents/Resources/"

# The icon, from the GNOME app's.
iconset="$out/CoreRules.iconset"
mkdir -p "$iconset"
icon=data/icons/hicolor/512x512/apps/io.github.brahm.CoreRules.png
for size in 16 32 128 256 512; do
    sips -z $size $size "$icon" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    [ $double -le 512 ] && sips -z $double $double "$icon" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
cp "$icon" "$iconset/icon_256x256@2x.png"
iconutil -c icns "$iconset" -o "$app/Contents/Resources/CoreRules.icns"
rm -rf "$iconset"

# Signed ad hoc: Apple Silicon runs no unsigned code, and there is no
# developer certificate to sign with.
codesign --force --deep --sign - "$app"
codesign --verify --strict "$app"

dmg="$out/CoreRules-$version-arm64.dmg"
stage="$out/dmg"
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
cp "macos/Package/Read Me First.txt" "$stage/"
hdiutil create -volname "Core Rules" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$dmg" >/dev/null
rm -rf "$stage"
echo "$dmg"
