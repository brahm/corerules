// The interface's texts, translated by the Rust catalogs as the other
// apps' are. The extraction of texts to translate finds each one in
// `tr("…")`, so every text the app shows goes through here.

import Foundation

/// The translation of `msgid`, without the mnemonic underscores of the
/// GNOME and Windows menus: macOS has none.
func tr(_ msgid: String) -> String {
    plain(tr(msgid: msgid))
}

/// The translation of `msgid` with its `{name}` placeholders filled.
func tr(_ msgid: String, _ values: [String: String]) -> String {
    var text = tr(msgid)
    for (name, value) in values {
        text = text.replacingOccurrences(of: "{\(name)}", with: value)
    }
    return text
}

/// `text` without mnemonic underscores; a doubled one stays, single.
func plain(_ text: String) -> String {
    var out = ""
    var underscore = false
    for c in text {
        if c == "_" && !underscore {
            underscore = true
            continue
        }
        underscore = false
        out.append(c)
    }
    return out
}

/// `text` with the ellipsis a menu item that asks for more ends with.
func more(_ text: String) -> String {
    text.hasSuffix("…") ? text : text + "…"
}
