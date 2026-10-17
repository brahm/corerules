#!/bin/sh
# Builds the AppImage against the GNOME 51 platform: the app is compiled
# in the org.gnome.Sdk//51 sandbox and build.sh bundles that SDK's GTK 4,
# libadwaita and the rest of the GNOME stack, so the AppImage carries the
# same libraries as the Flatpak. It runs on systems whose glibc is at
# least the SDK's. Needs flatpak and the Flathub remote.
set -eu
version=$1
gnome=${GNOME_VERSION:-51}
flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install -y --noninteractive flathub "org.gnome.Sdk//$gnome"
# The Rust extension follows the SDK's Freedesktop base (24.08, 25.08...).
# The metadata names it as version= or versions= (a list, newest last).
base=$(flatpak info -m "org.gnome.Sdk//$gnome" |
    sed -n '/^\[Extension org.freedesktop.Sdk.Extension\]/,/^\[/s/^versions\{0,1\} *= *//p' |
    head -1 | tr ';' '\n' | grep . | tail -1)
if [ -z "$base" ]; then
    flatpak info -m "org.gnome.Sdk//$gnome" >&2
    echo "$0: cannot tell the SDK's Freedesktop base" >&2
    exit 1
fi
echo "GNOME $gnome SDK on Freedesktop $base"
flatpak install -y --noninteractive flathub "org.freedesktop.Sdk.Extension.rust-stable//$base"
flatpak run --share=network --filesystem=host --cwd="$PWD" --command=sh \
    "org.gnome.Sdk//$gnome" -c '
        set -eu
        export PATH=/usr/lib/sdk/rust-stable/bin:$PATH CARGO_HOME=$PWD/target/cargo-home
        cargo build --release --locked
        packaging/appimage/build.sh "$1"
    ' sh "$version"
