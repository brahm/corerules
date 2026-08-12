#!/bin/sh
# Refresh every bare git mirror in a directory.
#
# Ticket 02 decision 1 made the removable card the third copy — machine, private
# remote, card — and step 7 of its checklist is keeping that copy current.
#
# Bare mirrors only. exFAT has no permission bits, no symlinks, and is
# case-insensitive, so a working tree there would be wrong; a bare repo is just an
# object store, whose filenames are lowercase hex, which exFAT holds fine.
#
# Usage:  refresh-mirrors.sh [mirror-dir]
#   Default mirror-dir is the card. Pass a path to refresh mirrors elsewhere.
#
# Adding a mirror is a clone, and this script needs no change — it iterates *.git:
#   git clone --mirror <url> <name>.git

set -e

DIR="${1:-/run/media/brahm/PocketNAS/Backup/git-mirrors}"

if [ ! -d "$DIR" ]; then
    echo "mirror directory not found: $DIR" >&2
    echo "(is the card mounted?)" >&2
    exit 1
fi

cd "$DIR"

found=0
for repo in *.git; do
    [ -d "$repo" ] || continue
    found=$((found + 1))
    printf '%-24s ' "$repo"
    if ( cd "$repo" && git remote update --prune >/dev/null 2>&1 ); then
        ( cd "$repo" && git log --oneline -1 --all )
    else
        echo "FAILED"
    fi
done

[ "$found" -gt 0 ] || echo "no bare mirrors in $DIR"
