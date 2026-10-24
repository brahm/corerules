#!/bin/sh
# Builds dist/corerules-VERSION-1-ARCH.pkg.tar.zst and dist/PKGBUILD from
# the checkout. Run on an up-to-date Arch Linux (or a container of it) from
# the repository root, as a regular user: makepkg refuses to run as root.
# The build dependencies (base-devel, git, cargo, desktop-file-utils,
# appstream-glib) have to be installed already.
set -eu
tag=v$1
# A pre-release suffix (1.0.0-rc1) loses its hyphen, which pacman does not
# allow in a version and without which it sorts before the release
# (1.0.0rc1 < 1.0.0).
pkgver=$(printf '%s' "$1" | tr -d '-')
stage=$(mktemp -d)
sed -e "s/^pkgver=.*/pkgver=$pkgver/" -e "s/^_tag=.*/_tag=$tag/" \
    packaging/arch/PKGBUILD > "$stage/PKGBUILD"
# The tarball makepkg would download for the tag, made from the checkout
# so pull requests and untagged commits build too.
git archive --format=tar.gz --prefix="corerules-$1/" \
    -o "$stage/corerules-$1.tar.gz" HEAD
(cd "$stage" && makepkg --noconfirm --cleanbuild)
mkdir -p dist
cp "$stage"/corerules-*.pkg.tar.zst "$stage/PKGBUILD" dist/
rm -rf "$stage"
