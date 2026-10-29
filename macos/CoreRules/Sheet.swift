// A character's sheet, or the page of an encounter, table, template or
// treasure, in a window of its own: the PDF `cr-app` draws for printing,
// shown as Preview shows a document, with Print, Save as PDF, the
// exports and Character Sheet Options.

import AppKit
import PDFKit
import SwiftUI
import UniformTypeIdentifiers

@MainActor
final class SheetModel: ObservableObject {
    let session: SheetSession
    private weak var app: AppModel?
    @Published private(set) var title: String
    @Published private(set) var pdf: Data
    let exports: [SheetExport]
    @Published var options: OptionsModel?

    init(_ session: SheetSession, app: AppModel) {
        self.session = session
        self.app = app
        title = session.title()
        pdf = session.pdf(letter: SheetModel.letter)
        exports = session.exports()
    }

    /// Letter where the system measures in inches, A4 elsewhere, as the
    /// other apps choose.
    static var letter: Bool {
        Locale.current.measurementSystem == .us
    }

    private var window: NSWindow? {
        NSApp.keyWindow ?? NSApp.mainWindow
    }

    func print() {
        guard let printer = PDFDocumentPrinter(data: pdf) else {
            app?.notice = Notice(title: tr("Could Not Print"), text: "")
            return
        }
        printer.print(in: window, letter: SheetModel.letter)
    }

    func savePdf() {
        let pdf = self.pdf
        save(fileName(title: title, extension: "pdf"), [.pdf]) { pdf }
    }

    func export(_ export: SheetExport) {
        let ext = exportExtension(export: export)
        let type = UTType(filenameExtension: ext) ?? .plainText
        save(fileName(title: title, extension: ext), [type]) {
            self.session.export(export: export)
        }
    }

    /// Asks where to save a file named `name` and writes what `data` makes.
    private func save(_ name: String, _ types: [UTType], _ data: () -> Data) {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = name
        panel.allowedContentTypes = types
        panel.canCreateDirectories = true
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try data().write(to: url, options: .atomic)
        } catch {
            app?.fail(tr("Not Saved"), error)
        }
    }

    var hasOptions: Bool {
        session.options() != nil
    }

    func editOptions() {
        guard let chosen = session.options() else { return }
        options = OptionsModel(chosen)
    }

    /// Saves the options chosen and draws the sheet again with them.
    func saveOptions(_ chosen: SheetOptions) -> Bool {
        do {
            try session.choose(options: chosen)
        } catch {
            app?.fail(tr("Character Sheet Options"), error)
            return false
        }
        pdf = session.pdf(letter: SheetModel.letter)
        options = nil
        return true
    }
}

/// The sheet's window.
struct SheetWindow: View {
    @EnvironmentObject var app: AppModel
    let id: UUID

    var body: some View {
        if let sheet = app.sheet(id) {
            SheetView(sheet: sheet)
                .onDisappear { app.closeSheet(id) }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "doc")
        }
    }
}

struct SheetView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var sheet: SheetModel

    var body: some View {
        PDFKitView(data: sheet.pdf)
            .frame(minWidth: 640, minHeight: 600)
            .navigationTitle(sheet.title)
            .toolbar { SheetTools(app: app, sheet: sheet) }
            .focusedSceneValue(\.sheet, sheet)
            .sheet(item: $sheet.options) { options in
                SheetOptionsView(options: options, sheet: sheet)
            }
    }
}

struct SheetTools: ToolbarContent {
    @ObservedObject var app: AppModel
    @ObservedObject var sheet: SheetModel

    var body: some ToolbarContent {
        ToolbarItemGroup {
            if sheet.hasOptions {
                Button { sheet.editOptions() } label: {
                    Label(tr("Character Sheet Options"), systemImage: "slider.horizontal.3")
                }
                .help(tr("Character Sheet Options"))
            }
            if !sheet.exports.isEmpty {
                Menu {
                    ForEach(sheet.exports, id: \.self) { (export: SheetExport) in
                        Button(more(plain(exportLabel(export: export)))) {
                            app.perform(Action.export(export), sheet: sheet)
                        }
                    }
                } label: {
                    Label(tr("Export"), systemImage: "square.and.arrow.up")
                }
                .help(tr("Export"))
            }
            Button { app.perform(Action.SHEET_PDF, sheet: sheet) } label: {
                Label(more(tr("Save as _PDF…")), systemImage: "arrow.down.doc")
            }
            .help(more(tr("Save as _PDF…")))
            Button { app.perform(Action.SHEET_PRINT, sheet: sheet) } label: {
                Label(more(tr("_Print…")), systemImage: "printer")
            }
            .help(more(tr("_Print…")))
        }
    }
}

/// The sheet's PDF, as Preview shows it.
struct PDFKitView: NSViewRepresentable {
    let data: Data

    func makeNSView(context: Context) -> PDFView {
        let view = PDFView()
        view.autoScales = true
        view.displayMode = .singlePageContinuous
        view.displaysPageBreaks = true
        view.document = PDFDocument(data: data)
        return view
    }

    func updateNSView(_ view: PDFView, context: Context) {
        if view.document?.dataRepresentation() != data {
            view.document = PDFDocument(data: data)
        }
    }
}

/// The sheet's window in front, for the menu bar's Print and Save.
struct SheetFocusKey: FocusedValueKey {
    typealias Value = SheetModel
}

extension FocusedValues {
    var sheet: SheetModel? {
        get { self[SheetFocusKey.self] }
        set { self[SheetFocusKey.self] = newValue }
    }
}

// MARK: Character Sheet Options

/// A section in the options' list; the same section may be there twice.
struct SectionChoice: Identifiable, Equatable {
    let id = UUID()
    let section: UInt32
}

@MainActor
final class OptionsModel: ObservableObject, Identifiable {
    @Published var sections: [SectionChoice]
    @Published var border: Int
    private let start: SheetOptions

    init(_ options: SheetOptions) {
        start = options
        sections = options.sections.map { SectionChoice(section: $0) }
        border = Int(options.border)
    }

    var chosen: SheetOptions {
        SheetOptions(sections: sections.map(\.section), border: UInt32(border))
    }

    func add(_ section: UInt32) {
        sections.append(SectionChoice(section: section))
    }

    func move(_ choice: SectionChoice, by: Int) {
        guard let at = sections.firstIndex(of: choice), sections.indices.contains(at + by)
        else { return }
        sections.swapAt(at, at + by)
    }

    func remove(_ choice: SectionChoice) {
        sections.removeAll { $0.id == choice.id }
    }

    var changed: Bool {
        chosen != start
    }
}

struct SheetOptionsView: View {
    @ObservedObject var options: OptionsModel
    let sheet: SheetModel
    @State private var askingEmpty = false
    private let offered = sheetSectionsOffered()

    var body: some View {
        VStack(spacing: 0) {
            Form {
                Picker(selection: $options.border) {
                    ForEach(0..<borderLabels().count, id: \.self) { (index: Int) in
                        Text(borderLabels()[index]).tag(index)
                    }
                } label: {
                    Text(tr("Border Style"))
                    Text(tr("How the printed sheet frames its boxes"))
                }
                Section {
                    SectionRows(options: options)
                } header: {
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(tr("Sections"))
                            Text(tr("Printed in this order. A section can be added more than once."))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        addMenu
                    }
                }
            }
            .formStyle(.grouped)
            Divider()
            HStack {
                Spacer()
                Button(tr("_Cancel"), role: .cancel) { sheet.options = nil }
                    .keyboardShortcut(.cancelAction)
                Button(tr("_Save"), action: save)
                    .keyboardShortcut(.defaultAction)
            }
            .padding()
        }
        .frame(minWidth: 480, minHeight: 520)
        .alert(tr("No Sections"), isPresented: $askingEmpty) {
            Button(tr("_Save")) { _ = sheet.saveOptions(options.chosen) }
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(tr("No sections: the sheet will be empty"))
        }
    }

    private var addMenu: some View {
        Menu(tr("Add Section")) {
            ForEach(offered, id: \.id) { (section: SheetSectionInfo) in
                Button(section.name) { options.add(section.id) }
            }
        }
        .fixedSize()
    }

    private func save() {
        if options.sections.isEmpty {
            askingEmpty = true
        } else {
            _ = sheet.saveOptions(options.chosen)
        }
    }
}

struct SectionRows: View {
    @ObservedObject var options: OptionsModel

    var body: some View {
        if options.sections.isEmpty {
            Text(tr("No sections: the sheet will be empty")).foregroundStyle(.secondary)
        } else {
            ForEach(options.sections) { (choice: SectionChoice) in
                SectionRow(
                    options: options, choice: choice,
                    first: choice == options.sections.first,
                    last: choice == options.sections.last)
            }
        }
    }
}

struct SectionRow: View {
    @ObservedObject var options: OptionsModel
    let choice: SectionChoice
    let first: Bool
    let last: Bool

    var body: some View {
        HStack {
            Text(sheetSectionName(id: choice.section))
            Spacer()
            Button { options.move(choice, by: -1) } label: {
                Label(tr("Move Up"), systemImage: "chevron.up")
            }
            .disabled(first)
            .help(tr("Move Up"))
            Button { options.move(choice, by: 1) } label: {
                Label(tr("Move Down"), systemImage: "chevron.down")
            }
            .disabled(last)
            .help(tr("Move Down"))
            Button { options.remove(choice) } label: {
                Label(tr("Remove"), systemImage: "minus.circle")
            }
            .help(tr("Remove"))
        }
        .labelStyle(.iconOnly)
        .buttonStyle(.borderless)
    }
}
