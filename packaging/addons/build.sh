#!/bin/sh
# Builds addons/addons-comunidade-expansion.tar.gz, the community add-ons
# package for the Expansion: the consolidated Comunidade.xpx, the installer,
# a static cr that reads it, the AppImage and the data extraction script.
#
#   packaging/addons/build.sh COMUNIDADE.xpx CoreRules-*.AppImage
#
# The static cr is built here (x86_64-unknown-linux-musl target).
set -eu
xpx=$1
appimage=$2
root=$(cd "$(dirname "$0")/../.." && pwd)
name=addons-comunidade-expansion
stage=$root/target/addons/$name

cargo build --release --locked -p cr-cli --target x86_64-unknown-linux-musl
rm -rf "$stage"
mkdir -p "$stage/bin"
install -m755 "$root/target/x86_64-unknown-linux-musl/release/cr" "$stage/bin/cr"
strip "$stage/bin/cr"
install -m644 "$xpx" "$stage/Comunidade.xpx"
install -m755 "$appimage" "$stage/$(basename "$appimage")"
install -m755 "$root/packaging/addons/instalar-addons.sh" "$stage/instalar-addons.sh"
install -m755 "$root/packaging/extract-data.sh" "$stage/extract-data.sh"
install -m644 "$root/addons/README.md" "$stage/README.md"
for f in relatorio.md compatibilidade.csv conflitos.csv; do
    install -m644 "$root/addons/$f" "$stage/$f"
done
(cd "$stage" && sha256sum Comunidade.xpx bin/cr ./*.AppImage > SHA256SUMS)
tar -C "$(dirname "$stage")" -czf "$root/addons/$name.tar.gz" "$name"
ls -l "$root/addons/$name.tar.gz"
