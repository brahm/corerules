#!/bin/sh
# Starts the app on a virtual display and checks it is still running after a
# few seconds without failing to load its icons. Usage: packaging/smoke-test.sh COMMAND [ARGS...]
# Needs xvfb-run and dbus-run-session. Leaves a screenshot in
# ${SCREENSHOT:-smoke.png} when xwd and ImageMagick are around.
set -u
exec xvfb-run -a -s '-screen 0 1100x720x24' dbus-run-session -- sh -c '
    log=$(mktemp)
    "$@" 2>"$log" &
    pid=$!
    sleep ${SMOKE_SECONDS:-8}
    if ! kill -0 $pid 2>/dev/null; then
        wait $pid
        status=$?
        cat "$log" >&2
        echo "smoke test: the app exited early (status $status)" >&2
        exit 1
    fi
    if command -v xwd >/dev/null && command -v convert >/dev/null; then
        xwd -root -silent | convert xwd:- "${SCREENSHOT:-smoke.png}"
    fi
    kill $pid
    cat "$log" >&2
    if grep -q "Failed to load icon" "$log"; then
        echo "smoke test: the app started but could not load its icons" >&2
        exit 1
    fi
    echo "smoke test: the app started and kept running"
' smoke "$@"
