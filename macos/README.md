# Core Rules for macOS

The macOS version of Core Rules, in progress ([roadmap](../docs/roadmap.md#step-3-macos-in-progress)).
Its interface will be native SwiftUI, written in Swift; everything it
shows and decides comes from the Rust crates through the bridge
`crates/cr-mac-ffi`, exported to Swift with [UniFFI](https://mozilla.github.io/uniffi-rs/).
It runs on Apple Silicon, macOS 14 or newer.

- `scripts/make-bindings.sh` builds the bridge and writes its Swift side
  into `target/swift`.
- `scripts/smoke-test.sh` builds `Smoke/`, a small Swift program that
  calls the bridge, and runs it. CI runs both on every pull request.

The Mac app refers to each action of `cr_app::action` as `Action.NAME`;
`crates/cr-app/tests/parity.rs` checks its Swift sources against
`NOT_YET_ON_MACOS`, as it checks the Windows app against
`NOT_YET_ON_WINDOWS`.
