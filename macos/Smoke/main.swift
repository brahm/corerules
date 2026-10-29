// Calls the Rust backend from Swift through the bridge, as the Mac app
// will: CI builds it with the generated bindings and runs it, so a bridge
// that does not build, link or answer fails before any screen uses it.

import Foundation

func check(_ ok: Bool, _ what: String) {
    if !ok {
        FileHandle.standardError.write("smoke test failed: \(what)\n".data(using: .utf8)!)
        exit(1)
    }
}

check(!version().isEmpty, "version")
check(languages().contains { $0.code == "pt_BR" }, "languages")
check(languageFor(preferred: ["pt-BR"]) == "pt_BR", "languageFor")
check(actions().contains { $0.constant == "NEW_CHARACTER" }, "actions")
check(dataPackageShown().hasPrefix("~/Library/Application Support/"), "dataPackageShown")

setLanguage(code: "pt_BR")
check(tr(msgid: "Characters") == "Personagens", "tr")

let empty = FileManager.default.temporaryDirectory
    .appendingPathComponent("cr-smoke-\(ProcessInfo.processInfo.processIdentifier)")
try? FileManager.default.createDirectory(at: empty, withIntermediateDirectories: true)
do {
    _ = try Library.open(path: empty.path)
    check(false, "an empty folder opened")
} catch CoreError.Failed(let message) {
    check(!message.isEmpty, "the error's message")
}
try? FileManager.default.removeItem(at: empty)

print("Core Rules \(version()): the bridge answers from Swift.")
