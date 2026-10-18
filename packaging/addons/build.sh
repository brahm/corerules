#!/bin/sh
# Builds target/addons/addons-comunidade-expansion.tar.gz, the community
# add-ons package for the Expansion: the consolidated Comunidade.xpx, the
# installer, a static cr that reads it and the data extraction script. The
# Addons package workflow publishes it on GitHub Packages.
#
#   packaging/addons/build.sh [COMUNIDADE.xpx]
#
# Without an argument it unpacks addons/Comunidade.xpx.xz. The static cr is
# built here (x86_64-unknown-linux-musl target).
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
name=addons-comunidade-expansion
stage=$root/target/addons/$name
out=$root/target/addons/$name.tar.gz

cargo build --release --locked -p cr-cli --target x86_64-unknown-linux-musl
rm -rf "$stage"
mkdir -p "$stage/bin"
install -m755 "$root/target/x86_64-unknown-linux-musl/release/cr" "$stage/bin/cr"
strip "$stage/bin/cr"
if [ $# -gt 0 ]; then
    install -m644 "$1" "$stage/Comunidade.xpx"
else
    xz -dc "$root/addons/Comunidade.xpx.xz" > "$stage/Comunidade.xpx"
fi
install -m755 "$root/packaging/addons/instalar-addons.sh" "$stage/instalar-addons.sh"
install -m755 "$root/packaging/extract-data.sh" "$stage/extract-data.sh"
install -m644 "$root/addons/README.md" "$stage/README.md"
for f in relatorio.md compatibilidade.csv conflitos.csv; do
    install -m644 "$root/addons/$f" "$stage/$f"
done
(cd "$stage" && sha256sum Comunidade.xpx bin/cr > SHA256SUMS)
tar -C "$(dirname "$stage")" -czf "$out" "$name"
ls -l "$out"
