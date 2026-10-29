// Roll Treasure, in a window of its own: the roller of `cr_app::treasure`
// described as a form through the bridge, with the dialogs its edits and
// Save wait on. The toolbar rolls, resets, saves, prints and deletes the
// saved treasure.

import AppKit
import SwiftUI
import UniformTypeIdentifiers

@MainActor
final class TreasureModel: ObservableObject {
    let session: TreasureSession
    @Published private(set) var title = ""
    @Published private(set) var form = FormPage(groups: [], dialog: nil)
    @Published private(set) var savedName: String?
    @Published private(set) var canSave = true
    @Published var deleting = false
    /// Called with the name a Save wrote, or why it could not.
    var saved: (TreasureSaved) -> Void = { _ in }
    /// Called once the saved treasure is deleted, or with why not.
    var deleted: (Error?) -> Void = { _ in }
    /// Closes the window; its view sets it.
    var close: () -> Void = {}

    init(_ session: TreasureSession) {
        self.session = session
        refresh()
    }

    func refresh() {
        title = session.title()
        form = session.form()
        savedName = session.savedName()
        canSave = session.canSave()
        if let said = session.takeSaved() {
            saved(said)
        }
    }

    func act(_ id: UInt32, _ value: FormValue) {
        session.act(id: id, value: value)
        refresh()
    }

    func perform(_ action: Action) {
        switch action {
        case Action.TREASURE_SAVE:
            session.save()
            refresh()
        case Action.TREASURE_PRINT:
            let window = NSApp.keyWindow
            guard let printer = PDFDocumentPrinter(data: pdf) else { return }
            printer.print(in: window, letter: SheetModel.letter)
        case Action.TREASURE_PDF:
            savePdf()
        case Action.TREASURE_DELETE:
            deleting = true
        default:
            break
        }
    }

    func roll() {
        session.roll()
        refresh()
    }

    func reset() {
        session.reset()
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

    private var pdf: Data {
        session.pdf(letter: SheetModel.letter)
    }

    private func savePdf() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = fileName(title: title, extension: "pdf")
        panel.allowedContentTypes = [.pdf]
        panel.canCreateDirectories = true
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try pdf.write(to: url)
        } catch {
            deleted(error)
        }
    }

    /// Whether an edit's dialog or Save Treasure is open over the page.
    var dialogShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.form.dialog != nil },
            set: { (shown: Bool) in
                if !shown, let close = self.form.dialog?.close {
                    self.act(close.id, .press)
                }
            })
    }
}

struct TreasureWindow: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        if let treasure = app.treasure {
            TreasureView(treasure: treasure)
                .focusedSceneValue(\.treasure, treasure)
                .onAppear { treasure.close = { dismiss() } }
                .onDisappear { app.closeTreasure() }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "dollarsign.circle")
        }
    }
}

struct TreasureView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var treasure: TreasureModel

    var body: some View {
        FormView(groups: treasure.form.groups) { (id: UInt32, value: FormValue) in
            treasure.act(id, value)
        }
        .frame(minWidth: 560, minHeight: 520)
        .navigationTitle(treasure.title)
        .toolbar { TreasureTools(treasure: treasure) }
        .sheet(isPresented: treasure.dialogShown) {
            if let dialog = treasure.form.dialog {
                FormDialogView(dialog: dialog) { (id: UInt32, value: FormValue) in
                    treasure.act(id, value)
                }
                .environmentObject(app)
            }
        }
        .alert(tr("Delete Saved Treasure?"), isPresented: $treasure.deleting) {
            Button(tr("_Delete"), role: .destructive) { treasure.delete() }
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(
                tr("“{name}” will be removed from Saved Treasure", ["name": treasure.savedName ?? ""]))
        }
    }
}

struct TreasureTools: ToolbarContent {
    @ObservedObject var treasure: TreasureModel

    var body: some ToolbarContent {
        ToolbarItemGroup {
            Button {
                treasure.roll()
            } label: {
                Label(tr("_Roll"), systemImage: "dice")
            }
            .help(tr("_Roll"))
            Button {
                treasure.reset()
            } label: {
                Label(tr("Re_set"), systemImage: "arrow.counterclockwise")
            }
            .help(tr("Re_set"))
            Button {
                treasure.perform(Action.TREASURE_SAVE)
            } label: {
                Label(tr("_Save…"), systemImage: "square.and.arrow.down")
            }
            .disabled(!treasure.canSave)
            .help(tr("_Save…"))
            Menu {
                Button(tr("_Print…")) { treasure.perform(Action.TREASURE_PRINT) }
                Button(tr("Save as P_DF…")) { treasure.perform(Action.TREASURE_PDF) }
                Divider()
                Button(tr("_Delete Saved Treasure"), role: .destructive) {
                    treasure.perform(Action.TREASURE_DELETE)
                }
                .disabled(treasure.savedName == nil)
            } label: {
                Label(tr("More"), systemImage: "ellipsis.circle")
            }
            .help(tr("More"))
        }
    }
}

struct TreasureFocusKey: FocusedValueKey {
    typealias Value = TreasureModel
}

extension FocusedValues {
    /// Roll Treasure, when its window is in front.
    var treasure: TreasureModel? {
        get { self[TreasureFocusKey.self] }
        set { self[TreasureFocusKey.self] = newValue }
    }
}
