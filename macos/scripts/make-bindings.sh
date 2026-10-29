#!/bin/sh
# Builds the Mac app's bridge (crates/cr-mac-ffi) and writes its Swift side
# with UniFFI into target/swift: cr_mac_ffi.swift, the C header and the
# module map. The static library the app links is
# target/<profile>/libcr_mac_ffi.a. Run from anywhere; PROFILE=debug for a
# debug build.
set -eu
cd "$(dirname "$0")/../.."
profile=${PROFILE:-release}
flag=--release
[ "$profile" = release ] || flag=
cargo build $flag -p cr-mac-ffi --lib
case "$(uname -s)" in
    Darwin) library=target/$profile/libcr_mac_ffi.dylib ;;
    *) library=target/$profile/libcr_mac_ffi.so ;;
esac
rm -rf target/swift
cargo run -q $flag -p cr-mac-ffi --features bindgen --bin uniffi-bindgen -- \
    generate --library "$library" --language swift --out-dir target/swift
# Swift finds the C side by this name.
mv target/swift/cr_mac_ffiFFI.modulemap target/swift/module.modulemap
ls target/swift
