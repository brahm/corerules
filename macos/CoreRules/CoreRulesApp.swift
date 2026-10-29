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
            ToolsCommands(model: model)
            HelpCommands(model: model)
            BookCommands()
        }
        WindowGroup(id: "sheet", for: UUID.self) { $id in
            if let id {
                SheetWindow(id: id)
                    .environmentObject(model)
            }
        }
        WindowGroup(id: "book", for: UUID.self) { $id in
            if let id {
                BookWindow(id: id)
                    .environmentObject(model)
            }
        }
        .defaultSize(width: 1100, height: 760)
        Window(tr("Roll on a Table"), id: "tables") {
            TableWindow()
                .environmentObject(model)
        }
        .defaultSize(width: 760, height: 720)
        Window(tr("Roll Treasure"), id: "treasure") {
            TreasureWindow()
                .environmentObject(model)
        }
        .defaultSize(width: 720, height: 760)
        Window(tr("New Encounter"), id: "encounter") {
            EncounterWindow()
                .environmentObject(model)
        }
        .defaultSize(width: 760, height: 820)
        Window(tr("New Encounter Table"), id: "encounter-table") {
            TableEditWindow()
                .environmentObject(model)
        }
        .defaultSize(width: 680, height: 780)
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
            item(tr("E_nter Character…"), Action.ENTER_CHARACTER)
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

/// The Tools menu: the DM Tool Kit's generators, as the GNOME app's main
/// menu and the Windows app's More menu have them.
struct ToolsCommands: Commands {
    @ObservedObject var model: AppModel

    var body: some Commands {
        CommandMenu(tr("Tools")) {
            item(more(tr("New N_PC…")), Action.NEW_NPC)
                .keyboardShortcut("n", modifiers: [.command, .shift])
            item(more(tr("New NPC _Group…")), Action.NEW_NPC_GROUP)
            Divider()
            item(more(tr("_Roll Treasure…")), Action.ROLL_TREASURE)
            item(more(tr("Roll on a _Table…")), Action.ROLL_TABLE)
            Divider()
            item(more(tr("New _Encounter…")), Action.NEW_ENCOUNTER)
            item(more(tr("New Encounter _Table…")), Action.NEW_TABLE)
        }
    }

    private func item(_ title: String, _ action: Action) -> some View {
        Button(title) { model.perform(action) }
            .disabled(!model.enabled(action))
    }
}

/// The Help menu: the help of the screen in front, as the original's F1
/// opens it, the books, and help on the entry in view.
struct HelpCommands: Commands {
    @ObservedObject var model: AppModel
    @FocusedValue(\.sheet) private var sheet: SheetModel?
    @FocusedValue(\.book) private var book: BookModel?
    @FocusedValue(\.tables) private var tables: TableModel?
    @FocusedValue(\.treasure) private var treasure: TreasureModel?
    @FocusedValue(\.encounter) private var encounter: EncounterModel?
    @FocusedValue(\.tableEdit) private var tableEdit: TableEditModel?

    var body: some Commands {
        CommandGroup(replacing: .help) {
            Button(tr("Core Rules Help")) {
                if book != nil {
                    model.booksHelp()
                } else if let tables {
                    model.openHelp(programHelp(number: tables.session.help()))
                } else if let treasure {
                    model.openHelp(programHelp(number: treasure.session.help()))
                } else if let encounter {
                    model.openHelp(programHelp(number: encounter.session.help()))
                } else if let tableEdit {
                    model.openHelp(programHelp(number: tableEdit.session.help()))
                } else {
                    model.perform(Action.HELP_HERE, sheet: sheet)
                }
            }
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
