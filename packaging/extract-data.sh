#!/bin/sh
# Copies the files Core Rules for Linux needs out of an installation of the
# original Windows program into a data package, so the app can run without
# Wine. The package holds TSR/Wizards of the Coast content: keep it for your
# own use and do not redistribute it.
#
#   extract-data.sh SOURCE [OUTPUT.tar.gz]   write a package
#   extract-data.sh --install SOURCE         copy into the app's data folder
#   extract-data.sh --install PACKAGE.tar.gz unpack a package there
#
# SOURCE is the program folder (the one holding Database/), or a Wine prefix
# where Core Rules is installed in the default place.
set -eu

usage() {
    sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
}

data_home="${XDG_DATA_HOME:-$HOME/.local/share}/corerules/data"

install=0
if [ "${1:-}" = "--install" ]; then
    install=1
    shift
fi
[ $# -ge 1 ] || usage
source=$1
output=${2:-}

# Unpacking an existing package.
if [ -f "$source" ]; then
    [ "$install" = 1 ] || usage
    if [ -e "$data_home" ]; then
        echo "$data_home already exists; move it away first" >&2
        exit 1
    fi
    mkdir -p "$data_home"
    tar -xzf "$source" -C "$data_home" --strip-components=1
    echo "Unpacked into $data_home"
    exit 0
fi

# Case-insensitive lookup of one entry in a folder (Windows names vary).
entry() {
    find "$1" -maxdepth 1 -mindepth 1 -iname "$2" -print -quit 2>/dev/null
}

root=$source
if [ -z "$(entry "$root" Database)" ]; then
    root="$source/drive_c/Program Files/AD&D Core Rules 2.0"
fi
database=$(entry "$root" Database)
if [ -z "$database" ]; then
    echo "$source is not a Core Rules installation (no Database folder)" >&2
    exit 1
fi

program=$(entry "$root" Program)
flavor=core20
if [ -n "$program" ] && [ -n "$(entry "$program" Tsrp3.exe)" ]; then
    flavor=expansion
fi

stage=$(mktemp -d)
trap 'rm -rf "${stage:?}"' EXIT
dest="$stage/corerules-data-$flavor"
mkdir -p "$dest"

# Rules databases, user databases and characters.
for dir in Database UserDbas Chars; do
    found=$(entry "$root" "$dir")
    if [ -n "$found" ]; then
        cp -R "$found" "$dest/$dir"
    else
        mkdir -p "$dest/$dir"
    fi
done
# The book reader and Table Rolls.
for dir in Books Tables; do
    found=$(entry "$root" "$dir")
    [ -z "$found" ] || cp -R "$found" "$dest/$dir"
done
# Texts the original keeps inside its program (Table Rolls, NPC names).
if [ -n "$program" ]; then
    mkdir -p "$dest/Program"
    for exe in Tsrp3.exe Evermore.exe; do
        found=$(entry "$program" "$exe")
        [ -z "$found" ] || cp "$found" "$dest/Program/"
    done
fi
# Contents files of the help books (book outlines).
help=$(entry "$root" Help)
if [ -n "$help" ]; then
    mkdir -p "$dest/Help"
    find "$help" -maxdepth 1 -type f -iname '*.cnt' -exec cp {} "$dest/Help/" \;
fi
# Leftover backups of an earlier edit are not needed.
find "$dest" -type f -name '*.bak' -delete

cat > "$dest/README.txt" <<EOF
Core Rules data package ($flavor), made by extract-data.sh from
$root
on $(date -u +%Y-%m-%d).

It holds TSR/Wizards of the Coast content from your own copy of AD&D Core
Rules 2.0. Keep it for your own use; do not redistribute it.

The app finds it in ~/.local/share/corerules/data (or open this folder with
"Open Folder…"). Characters and user databases you edit are saved here.
EOF

if [ "$install" = 1 ]; then
    if [ -e "$data_home" ]; then
        echo "$data_home already exists; move it away first" >&2
        exit 1
    fi
    mkdir -p "$(dirname "$data_home")"
    cp -R "$dest" "$data_home"
    echo "Installed into $data_home"
else
    [ -n "$output" ] || output="$PWD/corerules-data-$flavor.tar.gz"
    tar -czf "$output" -C "$stage" "corerules-data-$flavor"
    echo "Wrote $output"
fi
