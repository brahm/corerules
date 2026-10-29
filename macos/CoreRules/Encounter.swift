// Encounters, in a window of their own: New Encounter's templates, then
// the Encounter page of `cr_app::encounter` described as a form through
// the bridge, with the dialogs its edits wait on. The toolbar creates,
// saves, prints and deletes the encounter. The 2-20 table editor has a
// window of its own.

import AppKit
import SwiftUI
import UniformTypeIdentifiers

/// Prints the PDF `data` in the window in front.
@MainActor
private func printPdf(_ data: Data) {
    let window = NSApp.keyWindow
    guard let printer = PDFDocumentPrinter(data: data) else { return }
    printer.print(in: window, letter: SheetModel.letter)
}

/// Asks where to save the PDF `data`, named after `title`.
@MainActor
private func savePdf(_ data: Data, title: String) throws {
    let panel = NSSavePanel()
    panel.nameFieldStringValue = fileName(title: title, extension: "pdf")
    panel.allowedContentTypes = [.pdf]
    panel.canCreateDirectories = true
    guard panel.runModal() == .OK, let url = panel.url else { return }
    try data.write(to: url, options: .atomic)
}

@MainActor
final class EncounterModel: ObservableObject {
    let session: EncounterSession
    @Published private(set) var title = ""
    @Published private(set) var form = FormPage(groups: [], dialog: nil)
    @Published private(set) var choosing = false
    @Published private(set) var canCreate = false
    @Published private(set) var savedName: String?
    @Published private(set) var canSave = true
    @Published var deleting = false
    /// What the window has to say, the first shown.
    @Published private(set) var notices: [EncounterNotice] = []
    /// Called with what a Save came to, when it saved.
    var saved: (SaveResult) -> Void = { _ in }
    /// Called once the saved encounter is deleted, or with why not.
    var deleted: (Error?) -> Void = { _ in }
    /// Called with what a button asks of the app.
    var requested: (EncounterRequest) -> Void = { _ in }
    /// Closes the window; its view sets it.
    var close: () -> Void = {}

    init(_ session: EncounterSession) {
        self.session = session
        refresh()
    }

    func refresh() {
        title = session.title()
        form = session.form()
        choosing = session.choosing()
        canCreate = session.canCreate()
        savedName = session.savedName()
        canSave = session.canSave()
        notices.append(contentsOf: session.takeNotices())
        if let said = session.takeSaved() {
            if case .failed(let error) = said {
                notices.append(EncounterNotice(title: tr("Not Saved"), text: error))
            } else {
                saved(said)
            }
        }
        if let request = session.takeRequest() {
            requested(request)
        }
    }

    func act(_ id: UInt32, _ value: FormValue) {
        session.act(id: id, value: value)
        refresh()
    }

    func create() {
        session.create()
        refresh()
    }

    func save() {
        session.save()
        refresh()
    }

    func perform(_ action: Action) {
        switch action {
        case Action.ENCOUNTER_PRINT:
            printPdf(session.pdf(letter: SheetModel.letter))
        case Action.ENCOUNTER_PDF:
            do {
                try savePdf(session.pdf(letter: SheetModel.letter), title: title)
            } catch {
                notices.append(EncounterNotice(title: tr("Not Saved"), text: message(error)))
            }
        case Action.ENCOUNTER_DELETE:
            deleting = true
        default:
            break
        }
    }

    func delete() {
        do {
            try session.deleteSaved()
            deleted(nil)
        } catch {
            deleted(error)
        }
    }

    /// The trap New Trap saved goes in the encounter.
    func addNewTrap(_ name: String) {
        session.addNewTrap(name: name)
        refresh()
    }

    /// The monster New Monster saved goes in the encounter.
    func addNewMonster(_ name: String) {
        session.addNewMonster(name: name)
        refresh()
    }

    /// Use Treasure: the treasure of `roller` is the encounter's.
    func useTreasure(_ roller: TreasureSession) {
        session.useTreasure(roller: roller)
        refresh()
    }

    /// Whether an edit's dialog or a picker is open over the page.
    var dialogShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.form.dialog != nil },
            set: { (shown: Bool) in
                if !shown, let close = self.form.dialog?.close {
                    self.act(close.id, .press)
                }
            })
    }

    var noticeShown: Binding<Bool> {
        Binding<Bool>(
            get: { !self.notices.isEmpty },
            set: { (shown: Bool) in
                if !shown && !self.notices.isEmpty { self.notices.removeFirst() }
            })
    }
}

struct EncounterWindow: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        if let encounter = app.encounter {
            EncounterView(encounter: encounter)
                .focusedSceneValue(\.encounter, encounter)
                // A new one may take the window's place.
                .task(id: ObjectIdentifier(encounter)) { encounter.close = { dismiss() } }
                .onDisappear { app.closeEncounter() }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "map")
        }
    }
}

struct EncounterView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var encounter: EncounterModel

    var body: some View {
        FormView(groups: encounter.form.groups) { (id: UInt32, value: FormValue) in
            encounter.act(id, value)
        }
        .frame(minWidth: 600, minHeight: 560)
        .navigationTitle(encounter.title)
        .toolbar { EncounterTools(encounter: encounter) }
        .sheet(isPresented: encounter.dialogShown) {
            if let dialog = encounter.form.dialog {
                FormDialogView(dialog: dialog) { (id: UInt32, value: FormValue) in
                    encounter.act(id, value)
                }
                .environmentObject(app)
            }
        }
        .alert(tr("Delete Encounter?"), isPresented: $encounter.deleting) {
            Button(tr("_Delete"), role: .destructive) { encounter.delete() }
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(
                tr(
                    "“{name}” will be removed from the saved encounters",
                    ["name": encounter.savedName ?? ""]))
        }
        .alert(
            encounter.notices.first?.title ?? "", isPresented: encounter.noticeShown,
            presenting: encounter.notices.first
        ) { (_: EncounterNotice) in
            Button(tr("_OK")) {}
        } message: { (notice: EncounterNotice) in
            Text(notice.text)
        }
    }
}

struct EncounterTools: ToolbarContent {
    @ObservedObject var encounter: EncounterModel

    var body: some ToolbarContent {
        ToolbarItemGroup {
            if encounter.choosing {
                Button {
                    encounter.create()
                } label: {
                    Label(tr("Create Encounter"), systemImage: "plus")
                }
                .disabled(!encounter.canCreate)
                .help(tr("Create Encounter"))
            } else {
                Button {
                    encounter.save()
                } label: {
                    Label(tr("_Save"), systemImage: "square.and.arrow.down")
                }
                .keyboardShortcut("s")
                .disabled(!encounter.canSave)
                .help(tr("_Save"))
                Menu {
                    Button(tr("_Print…")) { encounter.perform(Action.ENCOUNTER_PRINT) }
                    Button(tr("Save as P_DF…")) { encounter.perform(Action.ENCOUNTER_PDF) }
                    Divider()
                    Button(tr("_Delete Encounter"), role: .destructive) {
                        encounter.perform(Action.ENCOUNTER_DELETE)
                    }
                    .disabled(encounter.savedName == nil)
                } label: {
                    Label(tr("More"), systemImage: "ellipsis.circle")
                }
                .help(tr("More"))
            }
        }
    }
}

struct EncounterFocusKey: FocusedValueKey {
    typealias Value = EncounterModel
}

/// The 2-20 table editor: the table's name, the filter Fill picks by,
/// and the template of each roll.
@MainActor
final class TableEditModel: ObservableObject {
    let session: TableEditSession
    @Published private(set) var title = ""
    @Published private(set) var form = FormPage(groups: [], dialog: nil)
    @Published private(set) var savedName: String?
    @Published private(set) var canSave = true
    @Published var deleting = false
    @Published private(set) var notices: [String] = []
    /// Called with what a Save came to.
    var saved: (SaveResult) -> Void = { _ in }
    /// Called once the saved table is deleted, or with why not.
    var deleted: (Error?) -> Void = { _ in }
    /// Called with the encounter Roll generates.
    var generate: (TableGenerate) -> Void = { _ in }
    /// Closes the window; its view sets it.
    var close: () -> Void = {}

    init(_ session: TableEditSession) {
        self.session = session
        refresh()
    }

    func refresh() {
        title = session.title()
        form = session.form()
        savedName = session.savedName()
        canSave = session.canSave()
        notices.append(contentsOf: session.takeNotices())
        if let said = session.takeSaved() {
            saved(said)
        }
        if let made = session.takeGenerate() {
            generate(made)
        }
    }

    func act(_ id: UInt32, _ value: FormValue) {
        session.act(id: id, value: value)
        refresh()
    }

    func save() {
        session.save()
        refresh()
    }

    func fill() {
        session.fill()
        refresh()
    }

    func roll() {
        session.roll()
        refresh()
    }

    func delete() {
        do {
            try session.deleteSaved()
            deleted(nil)
        } catch {
            deleted(error)
        }
    }

    /// Whether the dialog that picks a roll's template is open.
    var dialogShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.form.dialog != nil },
            set: { (shown: Bool) in
                if !shown, let close = self.form.dialog?.close {
                    self.act(close.id, .press)
                }
            })
    }

    var noticeShown: Binding<Bool> {
        Binding<Bool>(
            get: { !self.notices.isEmpty },
            set: { (shown: Bool) in
                if !shown && !self.notices.isEmpty { self.notices.removeFirst() }
            })
    }
}

struct TableEditWindow: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        if let table = app.tableEdit {
            TableEditView(table: table)
                .focusedSceneValue(\.tableEdit, table)
                // A new one may take the window's place.
                .task(id: ObjectIdentifier(table)) { table.close = { dismiss() } }
                .onDisappear { app.closeTableEdit() }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "tablecells")
        }
    }
}

struct TableEditView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var table: TableEditModel

    var body: some View {
        FormView(groups: table.form.groups) { (id: UInt32, value: FormValue) in
            table.act(id, value)
        }
        .frame(minWidth: 560, minHeight: 560)
        .navigationTitle(table.title)
        .toolbar { TableEditTools(table: table) }
        .sheet(isPresented: table.dialogShown) {
            if let dialog = table.form.dialog {
                FormDialogView(dialog: dialog) { (id: UInt32, value: FormValue) in
                    table.act(id, value)
                }
                .environmentObject(app)
            }
        }
        .alert(tr("Delete Encounter Table?"), isPresented: $table.deleting) {
            Button(tr("_Delete"), role: .destructive) { table.delete() }
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(tr("“{name}” will be removed", ["name": table.savedName ?? ""]))
        }
        .alert(table.notices.first ?? "", isPresented: table.noticeShown) {
            Button(tr("_OK")) {}
        }
    }
}

struct TableEditTools: ToolbarContent {
    @ObservedObject var table: TableEditModel

    var body: some ToolbarContent {
        ToolbarItemGroup {
            Button {
                table.save()
            } label: {
                Label(tr("_Save"), systemImage: "square.and.arrow.down")
            }
            .keyboardShortcut("s")
            .disabled(!table.canSave)
            .help(tr("_Save"))
            Button {
                table.fill()
            } label: {
                Label(tr("_Fill"), systemImage: "list.bullet")
            }
            .help(tr("_Fill"))
            Button {
                table.roll()
            } label: {
                Label(tr("_Roll"), systemImage: "dice")
            }
            .help(tr("_Roll"))
            if table.savedName != nil {
                Menu {
                    Button(tr("Delete Table"), role: .destructive) { table.deleting = true }
                } label: {
                    Label(tr("More"), systemImage: "ellipsis.circle")
                }
                .help(tr("More"))
            }
        }
    }
}

struct TableEditFocusKey: FocusedValueKey {
    typealias Value = TableEditModel
}

extension FocusedValues {
    /// An encounter, when its window is in front.
    var encounter: EncounterModel? {
        get { self[EncounterFocusKey.self] }
        set { self[EncounterFocusKey.self] = newValue }
    }

    /// The 2-20 table editor, when its window is in front.
    var tableEdit: TableEditModel? {
        get { self[TableEditFocusKey.self] }
        set { self[TableEditFocusKey.self] = newValue }
    }
}
