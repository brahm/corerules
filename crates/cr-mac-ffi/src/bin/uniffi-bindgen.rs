//! UniFFI's bindings generator, which writes the Swift side of the bridge
//! from the built library (`macos/scripts/make-bindings.sh`).

fn main() {
    uniffi::uniffi_bindgen_main()
}
