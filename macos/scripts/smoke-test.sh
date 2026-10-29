#!/bin/sh
# Builds macos/Smoke against the bridge's Swift bindings and static library
# (make-bindings.sh first) and runs it. macOS only.
set -eu
cd "$(dirname "$0")/../.."
profile=${PROFILE:-release}
swiftc -O -module-name Smoke \
    -I target/swift \
    target/swift/cr_mac_ffi.swift macos/Smoke/main.swift \
    "target/$profile/libcr_mac_ffi.a" \
    -o target/swift/smoke
target/swift/smoke
