// The main window's state: the installation opened, the section and entry
// in view, and the sheets and alerts over it. Everything it shows comes
// from the bridge; the views only draw it.

import AppKit
import SwiftUI
import UniformTypeIdentifiers

/// A sheet over the main window.
enum Modal: Identifiable {
    case wizard(WizardModel)
    case edit(EditModel)
    case importing(ImportModel)

    var id: ObjectIdentifier {
        switch self {
        case .wizard(let m): return ObjectIdentifier(m)
        case .edit(let m): return ObjectIdentifier(m)
        case .importing(let m): return ObjectIdentifier(m)
        }
    }
}

/// A message in an alert.
struct Notice: Identifiable {
    let id = UUID()
    let title: String
    let text: String
}

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var library: Library?
    @Published private(set) var sections: [SectionInfo] = []
    @Published var section: String? {
        didSet {
            if section != oldValue {
                entry = nil
                refreshEntries()
            }
        }
    }
    @Published var search = "" {
        didSet { refreshEntries() }
    }
    @Published private(set) var entries: [EntryRow] = []
    @Published var entry: UInt32? {
        didSet { refreshDetails() }
    }
    @Published private(set) var details: EntryDetails?
    @Published var modal: Modal?
    @Published var notice: Notice?
    /// The character Delete asks about: its file and name.
    @Published var deleting: (file: String, name: String)?

    /// Opens Settings; the main window's view sets it, as only a view can.
    var openSettings: () -> Void = {}

    init() {
        if let folder = Prefs.folder, let library = try? Library.open(path: folder) {
            show(library)
        } else {
            show(discover())
        }
    }

    // MARK: The library

    private func show(_ library: Library?) {
        self.library = library
        sections = library?.sections() ?? []
        if section == nil || !sections.contains(where: { $0.id == section }) {
            section = sections.first?.id
        }
        refreshEntries()
    }

    /// Reads the installation again after a change, keeping the entry in
    /// view by its key when it is still there.
    private func reload(select key: String? = nil) {
        guard let library else { return }
        let key = key ?? selectedRow?.key
        do {
            show(try library.reload())
        } catch {
            fail(tr("Could Not Read"), error)
            return
        }
        entry = entries.first { $0.key == key }?.index
    }

    private func refreshEntries() {
        guard let library, let section else {
            entries = []
            return
        }
        entries = library.entries(section: section, search: search)
        if let entry, !entries.contains(where: { $0.index == entry }) {
            self.entry = nil
        }
    }

    private func refreshDetails() {
        guard let library, let section, let entry else {
            details = nil
            return
        }
        details = library.details(section: section, index: entry)
    }

    private var selectedRow: EntryRow? {
        entries.first { $0.index == entry }
    }

    /// The file of the character in view, when a character is.
    var character: String? {
        guard details?.character == true else { return nil }
        return selectedRow?.key
    }

    var sheetExports: [SheetExport] {
        library?.sheetExports() ?? []
    }

    // MARK: Actions

    /// Whether `action` can be done now.
    func enabled(_ action: Action) -> Bool {
        switch action {
        case Action.NEW_CHARACTER, Action.IMPORT:
            return library?.isWritable() == true && modal == nil
        case Action.SHEET_PRINT, Action.SHEET_PDF:
            return character != nil
        case Action.SHEET_HTML, Action.SHEET_TXT, Action.SHEET_MD:
            return character != nil && sheetExports.contains { Action.export($0) == action }
        default:
            return true
        }
    }

    func perform(_ action: Action) {
        guard enabled(action) else { return }
        switch action {
        case Action.PREFERENCES: openSettings()
        case Action.ABOUT: about()
        case Action.OPEN: chooseFolder()
        case Action.NEW_CHARACTER: newCharacter()
        case Action.IMPORT: chooseImport()
        case Action.SHEET_PRINT: printSheet()
        case Action.SHEET_PDF: saveSheetPdf()
        case Action.SHEET_HTML: exportSheet(.html)
        case Action.SHEET_TXT: exportSheet(.text)
        case Action.SHEET_MD: exportSheet(.markdown)
        default: break
        }
    }

    private func about() {
        NSApp.orderFrontStandardAboutPanel(options: [
            .applicationName: tr("Core Rules"),
            .applicationVersion: version(),
        ])
    }

    func chooseFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            let library = try Library.open(path: url.path)
            Prefs.folder = url.path
            section = nil
            show(library)
        } catch {
            fail(tr("No Core Rules Installation"), error)
        }
    }

    private func newCharacter() {
        guard let library else { return }
        do {
            let session = try library.newCharacter(coreVsOption: Prefs.coreVsOptionShown)
            modal = .wizard(WizardModel(session))
        } catch {
            fail(tr("Could Not Read"), error)
        }
    }

    /// Closes the wizard, keeping what it asked to keep.
    func closeWizard(_ wizard: WizardModel) {
        if wizard.session.coreVsOptionOff() {
            Prefs.coreVsOptionShown = false
        }
        modal = nil
    }

    func editCharacter() {
        guard let library, let file = character else { return }
        do {
            modal = .edit(EditModel(try library.editCharacter(file: file), file: file))
        } catch {
            fail(tr("Could Not Read"), error)
        }
    }

    /// Edit Character closed; `saved` when it wrote the character.
    func closeEdit(_ edit: EditModel, saved: Bool) {
        modal = nil
        if saved {
            reload(select: edit.file)
        }
    }

    func askDelete() {
        guard let file = character, let details else { return }
        deleting = (file, details.title)
    }

    func delete(_ file: String) {
        guard let library else { return }
        deleting = nil
        do {
            _ = try library.deleteCharacter(file: file)
            entry = nil
            reload()
        } catch {
            fail(tr("Not Saved"), error)
        }
    }

    private func chooseImport() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = ["xpx", "xpt"].compactMap { UTType(filenameExtension: $0) }
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        openPackage(url)
    }

    /// A package opened from the Finder or from Import….
    func openPackage(_ url: URL) {
        guard let library else {
            notice = Notice(title: tr("No Core Rules Installation"), text: noInstallation)
            return
        }
        do {
            modal = .importing(ImportModel(try library.importPackage(path: url.path)))
        } catch {
            fail(tr("Import Failed"), error)
        }
    }

    /// Import… finished: shows what it did and reads the library again.
    func imported(_ report: ImportReport) {
        modal = nil
        notice = Notice(title: report.heading, text: report.lines.joined(separator: "\n"))
        reload()
    }

    // MARK: The sheet

    private var window: NSWindow? {
        NSApp.keyWindow ?? NSApp.mainWindow
    }

    /// Letter where the system measures in inches, A4 elsewhere, as the
    /// other apps choose.
    private var letter: Bool {
        Locale.current.measurementSystem == .us
    }

    private func printSheet() {
        guard let library, let file = character else { return }
        do {
            let data = try library.sheetPdf(file: file, letter: letter)
            guard let document = PDFDocumentPrinter(data: data) else {
                notice = Notice(title: tr("Could Not Print"), text: "")
                return
            }
            document.print(in: window, letter: letter)
        } catch {
            fail(tr("Could Not Print"), error)
        }
    }

    private func saveSheetPdf() {
        guard let library, let file = character, let details else { return }
        save(fileName(title: details.title, extension: "pdf"), [.pdf]) {
            try library.sheetPdf(file: file, letter: self.letter)
        }
    }

    private func exportSheet(_ export: SheetExport) {
        guard let library, let file = character, let details else { return }
        let ext = exportExtension(export: export)
        let type = UTType(filenameExtension: ext) ?? .plainText
        save(fileName(title: details.title, extension: ext), [type]) {
            try library.sheetExport(file: file, export: export)
        }
    }

    /// Asks where to save a file named `name` and writes what `data` makes.
    private func save(_ name: String, _ types: [UTType], _ data: () throws -> Data) {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = name
        panel.allowedContentTypes = types
        panel.canCreateDirectories = true
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try data().write(to: url, options: .atomic)
        } catch {
            fail(tr("Not Saved"), error)
        }
    }

    // MARK: Messages

    var noInstallation: String {
        tr(
            "Put the data package made by Extract Data from CD… in {folder}, or open the folder where AD&D Core Rules 2.0 is installed.",
            ["folder": dataPackageShown()])
    }

    func fail(_ title: String, _ error: Error) {
        let text: String
        if case CoreError.Failed(let message) = error {
            text = message
        } else {
            text = error.localizedDescription
        }
        notice = Notice(title: title, text: text)
    }
}
