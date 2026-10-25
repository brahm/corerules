#!/bin/sh
# Turns a release's .flatpak into the website's Flatpak repository, so the
# app can be installed and updated with flatpak. It writes, inside the built
# website:
#   flatpak/                          the signed repository (OSTree)
#   corerules.flatpakrepo             adds the repository to flatpak
#   io.github.brahm.CoreRules.flatpakref  installs the app from it, with
#                                     the GNOME runtime from Flathub
# The repository is made anew from the bundle each time, so it holds only
# the latest release and keeps no state between publications.
# Usage: packaging/flatpak/repo.sh BUNDLE SITE URL KEY
#   BUNDLE  the .flatpak of the latest release
#   SITE    the built website (python3 site/build.py SITE)
#   URL     where SITE is served, such as https://brahm.github.io/corerules
#   KEY     fingerprint of the signing key in GnuPG's keyring ($GNUPGHOME)
set -eu
bundle=$1
site=$2
url=${3%/}
key=$4
app=io.github.brahm.CoreRules
repo=$site/flatpak

rm -rf "$repo"
ostree init --mode=archive-z2 --repo="$repo"
flatpak build-import-bundle --gpg-sign="$key" "$repo" "$bundle"
flatpak build-update-repo --gpg-sign="$key" --generate-static-deltas \
    --title="Core Rules" "$repo"

ref=$(ostree refs --repo="$repo" | grep "^app/$app/" | head -n 1)
[ -n "$ref" ] || { echo "No app/$app ref in $bundle" >&2; exit 1; }
branch=${ref##*/}
gpgkey=$(gpg --export "$key" | base64 -w 0)

cat > "$site/corerules.flatpakrepo" <<EOF
[Flatpak Repo]
Title=Core Rules
Url=$url/flatpak/
Homepage=$url/
Comment=The Core Rules app for AD&D Core Rules 2.0
Icon=$url/icons/corerules.png
GPGKey=$gpgkey
EOF

cat > "$site/$app.flatpakref" <<EOF
[Flatpak Ref]
Name=$app
Branch=$branch
Title=Core Rules
Url=$url/flatpak/
SuggestRemoteName=corerules
Homepage=$url/
Icon=$url/icons/corerules.png
RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo
IsRuntime=false
GPGKey=$gpgkey
EOF

commit=$(ostree rev-parse --repo="$repo" "$ref")
# Leftovers of the tools that the website must not serve.
rm -rf "$repo/tmp" "$repo/.lock"
echo "Published $ref at $commit in $repo"
