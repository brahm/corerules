// Export…, New Package… and Edit Package…, in sheets over the main
// window: the user's records and characters to take into a package, and
// the package editor with its items by category, Add Items and Save. The
// choices and the files come from `cr_app::packages` through the bridge.

import AppKit
import SwiftUI
import UniformTypeIdentifiers

/// Where a new package goes, asked with the Save panel; nil when the
/// user cancels.
@MainActor
func askPackagePath(extension ext: String, name: String) -> String? {
    let panel = NSSavePanel()
    if let type = UTType(filenameExtension: ext) {
        panel.allowedContentTypes = [type]
    }
    panel.nameFieldStringValue = "\(name).\(ext)"
    guard panel.runModal() == .OK, let url = panel.url else { return nil }
    return url.path
}

@MainActor
final class ExportModel: ObservableObject, Identifiable {
    let session: ExportSession
    /// The package editor whose Add Items this is; nil for Export….
    let adding: PackageModel?
    @Published private(set) var categories: [ImportCategory] = []
    @Published private(set) var all = false
    @Published private(set) var any = false

    init(_ session: ExportSession, adding: PackageModel? = nil) {
        self.session = session
        self.adding = adding
        refresh()
    }

    func refresh() {
        categories = session.categories()
        all = session.all()
        any = session.any()
    }

    func set(_ item: ImportItem, _ on: Bool) {
        session.set(index: item.index, on: on)
        refresh()
    }

    var allShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.all },
            set: { (on: Bool) in
                self.session.setAll(on: on)
                self.refresh()
            })
    }
}

struct ExportView: View {
    @ObservedObject var model: ExportModel
    let close: () -> Void
    /// Called with what Export… did.
    let exported: (PackageDone) -> Void
    let failed: (Error) -> Void

    private var title: String {
        model.adding == nil ? tr("Export") : tr("Add Items")
    }

    var body: some View {
        VStack(spacing: 0) {
            Text(title)
                .font(.title2.bold())
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 20)
                .padding(.top, 16)
                .padding(.bottom, 8)
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            Divider()
            HStack {
                Spacer()
                Button(tr("_Cancel"), role: .cancel, action: close)
                    .keyboardShortcut(.cancelAction)
                Button(model.adding == nil ? tr("_Export…") : tr("_Add"), action: act)
                    .keyboardShortcut(.defaultAction)
                    .disabled(!model.any)
            }
            .padding()
        }
        .frame(minWidth: 480, idealWidth: 540, minHeight: 480, idealHeight: 620)
    }

    @ViewBuilder
    private var content: some View {
        if model.categories.isEmpty {
            ContentUnavailableView(
                tr("No Records"), systemImage: "shippingbox",
                description: Text(tr("Packages take the records you made and your characters")))
        } else {
            List {
                Toggle(tr("Select _All"), isOn: model.allShown)
                ForEach(model.categories, id: \.label) { (category: ImportCategory) in
                    Section(category.label) {
                        ForEach(category.items, id: \.index) { (item: ImportItem) in
                            Toggle(item.name, isOn: checked(item))
                        }
                    }
                }
            }
        }
    }

    private func checked(_ item: ImportItem) -> Binding<Bool> {
        Binding<Bool>(get: { item.checked }, set: { (on: Bool) in model.set(item, on) })
    }

    private func act() {
        if let package = model.adding {
            package.add(model.session)
            close()
            return
        }
        guard let path = askPackagePath(extension: model.session.extension(), name: tr("Export"))
        else { return }
        do {
            exported(try model.session.export(path: path))
        } catch {
            failed(error)
        }
    }
}

@MainActor
final class PackageModel: ObservableObject {
    let session: PackageSession
    let library: Library
    @Published private(set) var title = ""
    @Published private(set) var categories: [PackageCategory] = []
    @Published private(set) var changed = false
    @Published var error: String?
    /// Add Items, open over the editor.
    @Published var adding: ExportModel?

    init(_ session: PackageSession, library: Library) {
        self.session = session
        self.library = library
        refresh()
    }

    func refresh() {
        title = session.title()
        categories = session.categories()
        changed = session.changed()
    }

    func remove(_ item: PackageItem) {
        session.remove(id: item.id)
        refresh()
    }

    /// Add Items: the user's records and characters, to choose from.
    func offer() {
        do {
            adding = ExportModel(try library.exportable(), adding: self)
        } catch {
            self.error = message(error)
        }
    }

    /// Add Items' Add.
    func add(_ chosen: ExportSession) {
        do {
            try session.add(chosen: chosen)
            error = nil
        } catch {
            self.error = message(error)
        }
        refresh()
    }

    /// Save: where the package is, or where the Save panel says; nil when
    /// the user cancels or it fails.
    func save() -> PackageDone? {
        var path: String?
        if !session.hasPath() {
            path = askPackagePath(extension: session.extension(), name: title)
            if path == nil { return nil }
        }
        do {
            let done = try session.save(path: path)
            error = nil
            refresh()
            return done
        } catch {
            self.error = message(error)
            return nil
        }
    }
}

struct PackageView: View {
    @ObservedObject var model: PackageModel
    let close: () -> Void
    let saved: (PackageDone) -> Void
    @State private var discarding = false

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(model.title).font(.title2.bold())
                Spacer()
                Button {
                    model.offer()
                } label: {
                    Label(tr("Add Items"), systemImage: "plus")
                }
                .disabled(!model.session.canAdd())
            }
            .padding(.horizontal, 20)
            .padding(.top, 16)
            .padding(.bottom, 8)
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            Divider()
            buttons
        }
        .frame(minWidth: 520, idealWidth: 600, minHeight: 480, idealHeight: 640)
        .sheet(item: $model.adding) { (adding: ExportModel) in
            ExportView(
                model: adding,
                close: { model.adding = nil },
                exported: { (_: PackageDone) in model.adding = nil },
                failed: { (error: Error) in
                    model.adding = nil
                    model.error = message(error)
                })
        }
        .alert(tr("Discard Changes?"), isPresented: $discarding) {
            Button(tr("_Discard"), role: .destructive, action: close)
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(tr("The package has changes that are not saved."))
        }
    }

    @ViewBuilder
    private var content: some View {
        if model.categories.isEmpty {
            ContentUnavailableView(
                tr("Empty Package"), systemImage: "shippingbox",
                description: Text(tr("Add records and characters with the + button")))
        } else {
            List {
                if let note = model.session.layoutNote() {
                    Label(note, systemImage: "info.circle")
                        .foregroundStyle(.secondary)
                }
                ForEach(model.categories, id: \.label) { (category: PackageCategory) in
                    Section(category.label) {
                        ForEach(category.items, id: \.id) { (item: PackageItem) in
                            PackageItemRow(item: item) { model.remove(item) }
                        }
                    }
                }
                if let note = model.session.mapsNote() {
                    Text(note).font(.caption).foregroundStyle(.secondary)
                }
            }
        }
    }

    private func cancel() {
        if model.changed && model.session.hasPath() {
            discarding = true
        } else {
            close()
        }
    }

    private var buttons: some View {
        HStack {
            if let error = model.error {
                Label(error, systemImage: "exclamationmark.octagon.fill")
                    .foregroundStyle(Color.red)
                    .lineLimit(3)
                    .textSelection(.enabled)
            }
            Spacer()
            Button(tr("_Cancel"), role: .cancel, action: cancel)
                .keyboardShortcut(.cancelAction)
            Button(tr("_Save")) {
                if let done = model.save() {
                    saved(done)
                }
            }
            .keyboardShortcut(.defaultAction)
        }
        .padding()
    }
}

struct PackageItemRow: View {
    let item: PackageItem
    let remove: () -> Void

    var body: some View {
        LabeledContent {
            Button(role: .destructive, action: remove) {
                Label(tr("Remove"), systemImage: "minus.circle")
                    .labelStyle(.iconOnly)
            }
            .buttonStyle(.borderless)
            .help(tr("Remove"))
        } label: {
            Text(item.name)
        }
    }
}
