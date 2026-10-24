#!/bin/sh
# Builds dist/corerules-VERSION-1.fcNN.ARCH.rpm. Run inside the Fedora
# release to target (the oldest supported, 44), from the repository root.
set -eu
# A pre-release suffix (1.0.0-rc1) is written with a tilde, which both
# dpkg and rpm sort before the release itself (1.0.0~rc1 < 1.0.0).
version=$(printf '%s' "$1" | tr '-' '~')
top=$(mktemp -d)
mkdir -p "$top/SOURCES"
git archive --format=tar.gz --prefix="corerules-$version/" \
    -o "$top/SOURCES/corerules-$version.tar.gz" HEAD
rpmbuild -bb --define "_topdir $top" --define "version_override $version" \
    packaging/rpm/corerules.spec
mkdir -p dist
# The files keep the hyphen, so release asset names have no tilde.
for rpm in $(find "$top/RPMS" -name '*.rpm'); do
    name=$(basename "$rpm")
    cp "$rpm" "dist/$(printf '%s' "$name" | tr '~' '-')"
done
rm -rf "$top"
