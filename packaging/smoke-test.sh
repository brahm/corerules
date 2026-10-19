#!/bin/sh
# Starts the app on a virtual display once for each interface language and
# checks that it keeps running, loads its icons and shows its texts in that
# language. Usage: packaging/smoke-test.sh COMMAND [ARGS...]
# Needs xvfb-run, dbus-run-session and awk, and the catalogs in
# crates/cr-i18n/po next to this script (SMOKE_PO overrides the folder).
# SMOKE_LANGUAGES picks the languages. Leaves a screenshot of each language
# in ${SCREENSHOT:-smoke}-LANGUAGE.png when xwd and ImageMagick are around.
set -u
script=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
if [ -z "${SMOKE_INSIDE:-}" ]; then
    SMOKE_INSIDE=1 exec xvfb-run -a -s '-screen 0 1100x720x24' dbus-run-session -- "$script" "$@"
fi

po=${SMOKE_PO:-$(dirname "$script")/../crates/cr-i18n/po}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# A catalog's entries as "msgid<TAB>msgstr" lines, the strings as they are
# written in the catalog (escapes such as \n kept, \" unescaped). Plural
# entries are left out; the header has an empty msgid.
entries() {
    awk '
        function unquote(s) { sub(/^[^"]*"/, "", s); sub(/"[ \t]*$/, "", s); gsub(/\\"/, "\"", s); return s }
        function flush() { if (id != "" && !plural) print id "\t" str; id = ""; str = ""; plural = 0; field = "" }
        /^msgctxt / { flush(); field = "ctxt"; next }
        /^msgid / { if (field != "ctxt") flush(); id = unquote($0); field = "id"; next }
        /^msgid_plural / { plural = 1; field = "plural"; next }
        /^msgstr/ { str = unquote($0); field = "str"; next }
        /^"/ { if (field == "id") id = id unquote($0); else if (field == "str") str = str unquote($0); next }
        /^[ \t]*$/ { flush() }
        END { flush() }
    ' "$1"
}

status=0
for code in ${SMOKE_LANGUAGES:-en_US pt_BR es it fr de}; do
    log=$tmp/$code.log
    texts=$tmp/$code.txt
    LANG=$code.UTF-8 LC_ALL= LC_MESSAGES= LANGUAGE= CORERULES_PRINT_TEXTS=1 \
        "$@" >"$texts" 2>"$log" &
    pid=$!
    sleep "${SMOKE_SECONDS:-8}"
    if ! kill -0 $pid 2>/dev/null; then
        wait $pid
        exited=$?
        cat "$log" >&2
        echo "smoke test ($code): the app exited early (status $exited)" >&2
        exit 1
    fi
    if command -v xwd >/dev/null && command -v convert >/dev/null; then
        xwd -root -silent | convert xwd:- "${SCREENSHOT:-smoke}-$code.png"
    fi
    kill $pid
    wait $pid 2>/dev/null
    cat "$log" >&2
    if grep -q "Failed to load icon" "$log"; then
        echo "smoke test ($code): the app started but could not load its icons" >&2
        exit 1
    fi

    # English shows the catalogs' msgids; any other language shows their
    # translations and no msgid that has a different translation.
    if [ "$code" = en_US ]; then
        entries "$po/pt_BR.po" | awk -F '\t' '{ print $1 "\t" $1 }' >"$tmp/catalog"
    else
        entries "$po/$code.po" >"$tmp/catalog"
    fi
    result=$(awk -F '\t' '
        FNR == NR { if ($1 != "") { tr[$2] = 1; if ($2 != $1) en[$1] = 1 }; next }
        { shown++ }
        $0 in tr { translated++ }
        $0 in en { print "  untranslated: " $0 }
        END { printf "%d of %d texts on screen are in the catalog\n", translated, shown }
    ' "$tmp/catalog" "$texts")
    echo "$result" | sed "s/^/smoke test ($code): /"
    if echo "$result" | grep -q "untranslated" || echo "$result" | grep -q "^0 of"; then
        echo "smoke test ($code): the interface is not in this language" >&2
        status=1
    else
        echo "smoke test ($code): the app started and speaks this language"
    fi
done
exit $status
