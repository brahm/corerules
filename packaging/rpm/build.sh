#!/bin/sh
# Builds dist/corerules-VERSION-1.fcNN.ARCH.rpm. Run inside the Fedora
# release to target (the oldest supported, 44), from the repository root.
set -eu
version=$1
top=$(mktemp -d)
mkdir -p "$top/SOURCES"
git archive --format=tar.gz --prefix="corerules-$version/" \
    -o "$top/SOURCES/corerules-$version.tar.gz" HEAD
rpmbuild -bb --define "_topdir $top" --define "version_override $version" \
    packaging/rpm/corerules.spec
mkdir -p dist
find "$top/RPMS" -name '*.rpm' -exec cp {} dist/ \;
rm -rf "$top"
