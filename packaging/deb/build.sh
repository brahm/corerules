#!/bin/sh
# Builds dist/corerules_VERSION_ARCH.deb from target/release.
# Build on the Ubuntu release it targets (26.04) so the package installs there
# and on every newer release.
set -eu
version=$1
arch=$(dpkg --print-architecture)
stage=$(mktemp -d)
pkg=$stage/pkg
packaging/install.sh "$pkg" /usr
# Debian keeps licenses in /usr/share/doc/<package>/copyright.
mkdir -p "$pkg/usr/share/doc/corerules"
mv "$pkg/usr/share/licenses/corerules/LICENSE" "$pkg/usr/share/doc/corerules/copyright"
rm -r "$pkg/usr/share/licenses"

# dpkg-shlibdeps needs a debian/control next to the binaries it reads.
mkdir -p "$stage/debian"
printf 'Source: corerules\n\nPackage: corerules\nArchitecture: any\n' > "$stage/debian/control"
depends=$(cd "$stage" && dpkg-shlibdeps -O "pkg/usr/bin/corerules" "pkg/usr/bin/cr" \
    | sed -n 's/^shlibs:Depends=//p')
# The symbols used would let the package install on older releases too
# (Ubuntu 24.04), which are not supported: ask for the C library of the
# release it is built on.
libc=$(dpkg-query -W -f='${Version}' libc6 | sed 's/-[^-]*$//')
depends="$depends, libc6 (>= $libc)"

mkdir -p "$pkg/DEBIAN"
cat > "$pkg/DEBIAN/control" <<CONTROL
Package: corerules
Version: $version
Architecture: $arch
Maintainer: Brahm <https://github.com/brahm>
Installed-Size: $(du -sk "$pkg/usr" | cut -f1)
Depends: $depends
Section: games
Priority: optional
Homepage: https://github.com/brahm/corerules
Description: Browse and edit AD&D Core Rules 2.0 characters and rules
 A modern GTK replica of AD&D Core Rules 2.0, the 1998 Windows character
 manager for Advanced Dungeons & Dragons 2nd Edition. It reads and writes the
 files of the original program and needs a Core Rules installation or CD.
CONTROL
mkdir -p dist
dpkg-deb --root-owner-group --build "$pkg" "dist/corerules_${version}_${arch}.deb"
rm -rf "$stage"
