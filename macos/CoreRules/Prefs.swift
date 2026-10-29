// What the app keeps between runs, in its user defaults
// (`io.github.brahm.CoreRules`), as the GNOME app keeps it in GSettings
// and the Windows app in its settings file.

import Foundation

enum Prefs {
    private static var defaults: UserDefaults { .standard }

    /// The interface language chosen in Settings, by its catalog code;
    /// nil follows the system's.
    static var language: String? {
        get { defaults.string(forKey: "language") }
        set { defaults.set(newValue, forKey: "language") }
    }

    /// The folder opened with Open Folder…, opened again at start.
    static var folder: String? {
        get { defaults.string(forKey: "folder") }
        set { defaults.set(newValue, forKey: "folder") }
    }

    /// Whether New Character still shows the Expansion's Core Rules Vs
    /// Player's Option message.
    static var coreVsOptionShown: Bool {
        get { defaults.object(forKey: "core-vs-option-shown") as? Bool ?? true }
        set { defaults.set(newValue, forKey: "core-vs-option-shown") }
    }

    /// The file the books' reading positions and zoom are kept in, as
    /// the other apps keep them: in Application Support.
    static var readerState: String {
        let support =
            FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        let folder = support.appendingPathComponent("Core Rules", isDirectory: true)
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        return folder.appendingPathComponent("reader-state").path
    }

    /// The language the interface starts in: the one chosen, else the
    /// first of the system's that has a catalog.
    static func startLanguage() -> String {
        language ?? languageFor(preferred: Locale.preferredLanguages)
    }
}
