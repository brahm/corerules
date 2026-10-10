#!/bin/sh
# Starts the app on a virtual display and checks it is still running after a
# few seconds. Usage: packaging/smoke-test.sh COMMAND [ARGS...]
# Needs xvfb-run and dbus-run-session. Leaves a screenshot in
# ${SCREENSHOT:-smoke.png} when xwd and ImageMagick are around.
set -u
exec xvfb-run -a -s '-screen 0 1100x720x24' dbus-run-session -- sh -c '
    "$@" &
    pid=$!
    sleep ${SMOKE_SECONDS:-8}
    if ! kill -0 $pid 2>/dev/null; then
        wait $pid
        echo "smoke test: the app exited early (status $?)" >&2
        exit 1
    fi
    if command -v xwd >/dev/null && command -v convert >/dev/null; then
        xwd -root -silent | convert xwd:- "${SCREENSHOT:-smoke.png}"
    fi
    kill $pid
    echo "smoke test: the app started and kept running"
' smoke "$@"
