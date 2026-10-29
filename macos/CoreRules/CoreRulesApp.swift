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
        .commands {
            MenuCommands(model: model)
            HelpCommands(model: model)
        }
        WindowGroup(id: "sheet", for: UUID.self) { $id in
            if let id {
                SheetWindow(id: id)
                    .environmentObject(model)
            }
        }
        Window(tr("Help"), id: "help") {
            HelpWindow()
                .environmentObject(model)
        }
        .defaultSize(width: 980, height: 680)
        Window(tr("_About Core Rules"), id: "about") {
            AboutView()
                .environmentObject(model)
        }
        .windowResizability(.contentSize)
        .defaultPosition(.center)
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

/// The Help menu: the help of the screen in front, as the original's F1
/// opens it, the books, and help on the entry in view.
struct HelpCommands: Commands {
    @ObservedObject var model: AppModel
    @FocusedValue(\.sheet) private var sheet: SheetModel?

    var body: some Commands {
        CommandGroup(replacing: .help) {
            Button(tr("Core Rules Help")) { model.perform(Action.HELP_HERE, sheet: sheet) }
                .keyboardShortcut("?")
                .disabled(!model.enabled(Action.HELP_HERE))
            Button(entryHelpTitle) { model.perform(Action.HELP_CONTROL) }
                .disabled(!model.enabled(Action.HELP_CONTROL))
            Divider()
            ForEach(helpBooks(), id: \.file) { (book: HelpBook) in
                Button(plain(book.label)) { model.openHelp(book.file) }
                    .disabled(!model.enabled(Action.HELP))
            }
        }
    }

    private var entryHelpTitle: String {
        guard let details = model.details else { return tr("Help") }
        return tr("Help on {name}", ["name": details.title])
    }
}
