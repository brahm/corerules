// Core Rules for macOS: the app, its main window, its menus and its
// Settings window.

import AppKit
import SwiftUI

@main
struct CoreRulesApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var model: AppModel

    init() {
        // Before the first translated text: the catalogs pick their
        // language once.
        setLanguage(code: Prefs.startLanguage())
        let model = AppModel()
        _model = StateObject(wrappedValue: model)
        AppDelegate.model = model
    }

    var body: some Scene {
        Window(tr("Core Rules"), id: "main") {
            ContentView()
                .environmentObject(model)
        }
        .commands { MenuCommands(model: model) }
        WindowGroup(id: "sheet", for: UUID.self) { $id in
            if let id {
                SheetWindow(id: id)
                    .environmentObject(model)
            }
        }
        Settings {
            SettingsView()
                .environmentObject(model)
        }
    }
}

/// Takes the packages opened from the Finder.
final class AppDelegate: NSObject, NSApplicationDelegate {
    @MainActor static var model: AppModel?

    func application(_ application: NSApplication, open urls: [URL]) {
        Task { @MainActor in
            for url in urls {
                AppDelegate.model?.openPackage(url)
            }
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}

/// The menu bar's items for the app's actions.
struct MenuCommands: Commands {
    @ObservedObject var model: AppModel
    /// The sheet's window in front, which Print and Save act on.
    @FocusedValue(\.sheet) private var sheet: SheetModel?

    var body: some Commands {
        CommandGroup(replacing: .appInfo) {
            item(tr("_About Core Rules"), Action.ABOUT)
        }
        CommandGroup(replacing: .appSettings) {
            item(more(tr("Settings")), Action.PREFERENCES)
                .keyboardShortcut(",")
        }
        CommandGroup(replacing: .newItem) {
            item(more(tr("New Character")), Action.NEW_CHARACTER)
                .keyboardShortcut("n")
            item(more(tr("_Open Folder…")), Action.OPEN)
                .keyboardShortcut("o")
            Divider()
            item(more(tr("_Import…")), Action.IMPORT)
                .keyboardShortcut("i", modifiers: [.command, .shift])
        }
        CommandGroup(replacing: .printItem) {
            item(more(tr("_Print…")), Action.SHEET_PRINT)
                .keyboardShortcut("p")
            item(more(tr("Save as _PDF…")), Action.SHEET_PDF)
            ForEach(sheet?.exports ?? model.sheetExports, id: \.self) { (export: SheetExport) in
                item(more(plain(exportLabel(export: export))), Action.export(export))
            }
        }
    }

    private func item(_ title: String, _ action: Action) -> some View {
        Button(title) { model.perform(action, sheet: sheet) }
            .disabled(!model.enabled(action, sheet: sheet))
    }
}
