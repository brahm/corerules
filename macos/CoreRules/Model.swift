// The main window's state: the installation opened, the section and entry
// in view with the search and filters, and the sheets and alerts over it.
// Everything it shows comes from the bridge; the views only draw it.

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

/// A question asked before a change that cannot be undone.
enum Confirm {
    case delete(file: String, name: String)
    case toNpc(file: String)
}

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var library: Library?
    @Published private(set) var sections: [SectionInfo] = []
    @Published var section: String? {
        didSet {
            if section != oldValue {
                entry = nil
                filter.origins = 0
                filter.partType = nil
                filter.magicOnly = false
                refreshFilters()
            }
        }
    }
    /// The search and the filters of the list.
    @Published var filter: ListFilter = AppModel.noFilter {
        didSet {
            if filter != oldValue { refreshEntries() }
        }
    }
    @Published private(set) var filters: SectionFilters = AppModel.noFilters
    @Published private(set) var entries: [EntryRow] = []
    @Published var entry: UInt32? {
        didSet {
            if entry != oldValue { refreshDetails() }
        }
    }
    @Published private(set) var details: EntryDetails?
    /// The picture shown, which Next Image changes, and its frame.
    @Published private(set) var picture: Data?
    private var frame: UInt32 = 0
    @Published var fullScreen = false
    @Published var modal: Modal?
    @Published var notice: Notice?
    @Published var confirm: Confirm?

    static let noFilter = ListFilter(
        search: "", origins: 0, party: nil, partType: nil, magicOnly: false)
    static let noFilters = SectionFilters(origins: false, parties: [], partTypes: [], magic: false)

    /// The sheets open in their windows, by the id their window has.
    private(set) var sheets: [UUID: SheetModel] = [:]
    /// The books open in their windows, by the id their window has.
    private(set) var books: [UUID: BookModel] = [:]
    /// The help of the installation opened, once it is read.
    @Published private(set) var help: HelpModel?

    /// Opens Settings, a sheet's window, the help's and About; the main
    /// window's view sets them, as only a view can.
    var openSettings: () -> Void = {}
    var openSheet: (UUID) -> Void = { _ in }
    var openHelpWindow: () -> Void = {}
    var openBookWindow: (UUID) -> Void = { _ in }
    var openAbout: () -> Void = {}

    init() {
        if let folder = Prefs.folder, let library = try? Library.open(path: folder) {
            show(library)
        } else {
            show(discover())
        }
    }

    // MARK: The library

    private func show(_ library: Library?) {
        if library !== self.library {
            help = nil
        }
        self.library = library
        sections = library?.sections() ?? []
        if section == nil || !sections.contains(where: { $0.id == section }) {
            section = sections.first?.id
        }
        refreshFilters()
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
        refreshDetails()
    }

    private func refreshFilters() {
        guard let library, let section else {
            filters = AppModel.noFilters
            refreshEntries()
            return
        }
        filters = library.filters(section: section)
        if !filters.parties.contains(where: { $0.value == filter.party }) {
            filter.party = nil
        }
        refreshEntries()
    }

    private func refreshEntries() {
        guard let library, let section else {
            entries = []
            return
        }
        entries = library.entries(section: section, filter: filter)
        if let entry, !entries.contains(where: { $0.index == entry }) {
            self.entry = nil
        }
    }

    private func refreshDetails() {
        fullScreen = false
        frame = 0
        guard let library, let section, let entry else {
            details = nil
            picture = nil
            return
        }
        details = library.details(section: section, index: entry)
        picture = details?.picture
    }

    var currentSection: SectionInfo? {
        sections.first { $0.id == section }
    }

    private var selectedRow: EntryRow? {
        entries.first { $0.index == entry }
    }

    /// The file of the character in view, when a character is.
    var character: String? {
        guard details?.character == true else { return nil }
        return selectedRow?.key
    }

    var writable: Bool {
        library?.isWritable() == true
    }

    /// Next Image: the monster type's next frame.
    func nextImage() {
        guard let library, let section, let entry,
            let next = library.nextFrame(section: section, index: entry, at: frame)
        else { return }
        frame = next.at
        picture = next.png
    }

    // MARK: Actions

    /// Whether `action` can be done now; `sheet` is the sheet's window in
    /// front, when one is.
    func enabled(_ action: Action, sheet: SheetModel? = nil) -> Bool {
        switch action {
        case Action.NEW_CHARACTER, Action.ENTER_CHARACTER, Action.IMPORT:
            return writable && modal == nil
        case Action.SHEET_PRINT:
            if sheet != nil { return true }
            guard let commands = details?.commands else { return false }
            return commands.sheet || commands.print
        case Action.HELP, Action.HELP_HERE:
            return library != nil
        case Action.HELP_CONTROL:
            return library != nil && details != nil
        case Action.OPEN_BOOK:
            return details?.commands.read == true
        case Action.SHEET_PDF:
            return sheet != nil || details?.commands.sheet == true
        case Action.SHEET_HTML, Action.SHEET_TXT, Action.SHEET_MD:
            let exports = sheet?.exports ?? (character != nil ? sheetExports : [])
            return exports.contains { Action.export($0) == action }
        default:
            return true
        }
    }

    /// The files the sheets of this installation are exported to.
    var sheetExports: [SheetExport] {
        library?.sheetExports() ?? []
    }

    func perform(_ action: Action, sheet: SheetModel? = nil) {
        guard enabled(action, sheet: sheet) else { return }
        switch action {
        case Action.PREFERENCES: openSettings()
        case Action.ABOUT: openAbout()
        case Action.OPEN: chooseFolder()
        case Action.NEW_CHARACTER: newCharacter(entering: false)
        case Action.ENTER_CHARACTER: newCharacter(entering: true)
        case Action.IMPORT: chooseImport()
        case Action.SHEET_PRINT:
            if sheet == nil && details?.commands.monster == true {
                monsterHelp(print: true)
            } else {
                (sheet ?? selectedSheet())?.print()
            }
        case Action.SHEET_PDF: (sheet ?? selectedSheet())?.savePdf()
        case Action.SHEET_HTML: (sheet ?? selectedSheet())?.export(.html)
        case Action.SHEET_TXT: (sheet ?? selectedSheet())?.export(.text)
        case Action.SHEET_MD: (sheet ?? selectedSheet())?.export(.markdown)
        case Action.HELP: openHelp(helpBooks().first?.file ?? "")
        case Action.HELP_HERE: openHelp(programHelp(number: here(sheet: sheet)))
        case Action.HELP_CONTROL: entryHelp()
        case Action.OPEN_BOOK: readBook()
        default: break
        }
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

    /// New Character, or Enter Character with `entering`.
    private func newCharacter(entering: Bool) {
        guard let library else { return }
        do {
            let session =
                entering
                ? try library.enterCharacter()
                : try library.newCharacter(coreVsOption: Prefs.coreVsOptionShown)
            let wizard = WizardModel(session)
            wizard.created = { [weak self, weak wizard] (file: String) in
                guard let self, let wizard else { return }
                self.created(file, by: wizard)
            }
            modal = .wizard(wizard)
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

    /// Create wrote the character to `file`: the wizard closes, and the
    /// character shows in the library.
    private func created(_ file: String, by wizard: WizardModel) {
        guard case .wizard(let open) = modal, open === wizard else { return }
        closeWizard(wizard)
        if let characters = sections.first(where: { $0.id == characterSection() }) {
            section = characters.id
        }
        reload(select: file)
        // Enter Character's "Character Saved".
        notice = Notice(
            title: wizard.session.entering() ? tr("Character saved") : tr("Character created"),
            text: "")
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
        confirm = .delete(file: file, name: details.title)
    }

    func askToNpc() {
        guard let file = character else { return }
        confirm = .toNpc(file: file)
    }

    /// The question in view, answered.
    func answer(_ yes: Bool) {
        guard let asked = confirm else { return }
        confirm = nil
        guard yes, let library else { return }
        do {
            switch asked {
            case .delete(let file, _):
                _ = try library.deleteCharacter(file: file)
                entry = nil
                reload()
            case .toNpc(let file):
                let npc = try library.convertToNpc(file: file)
                reload(select: npc)
            }
        } catch {
            fail(tr("Not Saved"), error)
        }
    }

    func toCharacter() {
        guard let library, let file = character else { return }
        do {
            _ = try library.convertToCharacter(file: file)
            reload(select: file)
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

    // MARK: The sheets

    /// The sheet of the character in view, or the page of the entry in
    /// view, made to print or save.
    private func selectedSheet() -> SheetModel? {
        guard let library, let row = selectedRow, let details else { return nil }
        do {
            let session =
                details.commands.sheet
                ? try library.characterSheet(file: row.key)
                : try library.page(key: row.key)
            return SheetModel(session, app: self)
        } catch {
            fail(tr("Could Not Read"), error)
            return nil
        }
    }

    /// Character Sheet, or View on a page: the sheet in a window of its
    /// own.
    func showSheet() {
        guard let sheet = selectedSheet() else { return }
        present(sheet)
    }

    private func present(_ sheet: SheetModel) {
        let id = UUID()
        sheets[id] = sheet
        openSheet(id)
    }

    func sheet(_ id: UUID) -> SheetModel? {
        sheets[id]
    }

    func closeSheet(_ id: UUID) {
        sheets[id] = nil
    }

    // MARK: The books

    /// Read Book, on the book in view.
    private func readBook() {
        guard let library, let section, let entry,
            let name = library.entryBook(section: section, index: entry)
        else { return }
        openBook(name)
    }

    /// Opens the book `name` in a window of its own; `find` starts Find
    /// with a text.
    func openBook(_ name: String, find: String? = nil) {
        guard let library else { return }
        let id = UUID()
        books[id] = BookModel(name: name, find: find, library: library)
        openBookWindow(id)
    }

    /// Help on the reader, as F1 in a book opens it.
    func booksHelp() {
        openHelp(programHelp(number: screenHelp(screen: .books)))
    }

    func book(_ id: UUID) -> BookModel? {
        books[id]
    }

    func closeBook(_ id: UUID) {
        books[id] = nil
    }

    // MARK: Help

    /// Opens the help at `spec` in its window; `print` prints the topic
    /// once it is shown.
    func openHelp(_ spec: String, print: Bool = false) {
        guard let library else {
            notice = Notice(title: tr("Help"), text: tr("Open a Core Rules folder to read its help"))
            return
        }
        do {
            let help = try self.help ?? HelpModel(library.help(), app: self)
            let before = help.topic?.html
            help.printNext = print
            try help.open(spec)
            self.help = help
            openHelpWindow()
            // The same topic is not drawn again.
            if print && help.topic?.html == before && help.webView != nil {
                help.printNext = false
                help.print()
            }
        } catch {
            fail(tr("Help"), error)
        }
    }

    /// The program help's topic of the screen in front.
    private func here(sheet: SheetModel?) -> UInt32 {
        if sheet != nil {
            return screenHelp(screen: .characterSheet)
        }
        switch modal {
        case .wizard(let wizard): return wizard.session.help()
        case .edit(let edit): return edit.session.help()
        case .importing: return screenHelp(screen: .`import`)
        case nil: return section.map { sectionHelp(section: $0) } ?? screenHelp(screen: .mainMenu)
        }
    }

    /// Help on the entry in view, as a right-click on its row shows it in
    /// the original.
    func entryHelp() {
        guard let library, let section, let entry else { return }
        showHelp(library.entryHelp(section: section, index: entry))
    }

    /// View or Print on a monster: its help topic, or its page.
    func monsterHelp(print: Bool) {
        guard let library, let section, let entry else { return }
        showHelp(library.monsterHelp(section: section, index: entry), print: print)
    }

    private func showHelp(_ found: EntryHelp, print: Bool = false) {
        switch found {
        case .open(let spec):
            openHelp(spec, print: print)
        case .info(let title, let text):
            notice = Notice(title: title, text: text)
        case .page(let session):
            let sheet = SheetModel(session, app: self)
            if print {
                sheet.print()
            } else {
                present(sheet)
            }
        case .nothing(let message):
            if !message.isEmpty {
                notice = Notice(title: tr("Help"), text: message)
            }
        }
    }

    // MARK: Messages

    var noInstallation: String {
        tr(
            "Put the data package made by Extract Data from CD… in {folder}, or open the folder where AD&D Core Rules 2.0 is installed.",
            ["folder": dataPackageShown()])
    }

    func fail(_ title: String, _ error: Error) {
        notice = Notice(title: title, text: message(error))
    }
}

/// What went wrong, as a sentence for the user.
func message(_ error: Error) -> String {
    if let error = error as? CoreError, case .Failed(let message) = error {
        return message
    }
    return error.localizedDescription
}
