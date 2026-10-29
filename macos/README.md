# Core Rules for macOS

The macOS version of Core Rules, in progress ([roadmap](../docs/roadmap.md#step-3-macos-in-progress)).
Its interface will be native SwiftUI, written in Swift; everything it
shows and decides comes from the Rust crates through the bridge
`crates/cr-mac-ffi`, exported to Swift with [UniFFI](https://mozilla.github.io/uniffi-rs/).
It runs on Apple Silicon, macOS 14 or newer.

- `scripts/make-bindings.sh` builds the bridge and writes its Swift side
  into `target/swift`.
- `scripts/smoke-test.sh` builds `Smoke/`, a small Swift program that
  calls the bridge, and runs it.
- `scripts/make-app.sh` builds the app from `CoreRules/` with `swiftc`
  (no Xcode project), as `target/macos/Core Rules.app`, signs it ad hoc
  and packs it with `Package/Read Me First.txt` in
  `target/macos/CoreRules-<version>-arm64.dmg`. CI runs all three on
  every pull request and keeps the `.dmg`.

To build and run it on a Mac with the Xcode command line tools and Rust:

```sh
macos/scripts/make-bindings.sh
macos/scripts/make-app.sh
open "target/macos/Core Rules.app"
```

The interface's texts are wrapped in `tr("…")`, which the translation
extraction of `cr-i18n` finds in the Swift sources as in the Rust ones;
`cargo run --example update-po -p cr-i18n` adds new ones to the
catalogs.

The Mac app refers to each action of `cr_app::action` as `Action.NAME`;
`crates/cr-app/tests/parity.rs` checks its Swift sources against
`NOT_YET_ON_MACOS`, as it checks the Windows app against
`NOT_YET_ON_WINDOWS`.
