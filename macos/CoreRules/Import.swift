// Import…: a package's records and characters, to take or leave, in a
// sheet over the main window. The choice and the import come from
// `cr_app::packages` through the bridge.

import SwiftUI

@MainActor
final class ImportModel: ObservableObject {
    let session: ImportSession
    @Published private(set) var categories: [ImportCategory] = []
    @Published private(set) var canImport = false

    init(_ session: ImportSession) {
        self.session = session
        refresh()
    }

    func refresh() {
        categories = session.categories()
        canImport = session.canImport()
    }

    func set(_ item: ImportItem, _ on: Bool) {
        session.set(index: item.index, on: on)
        refresh()
    }
}

struct ImportView: View {
    @ObservedObject var model: ImportModel
    let close: () -> Void
    let imported: (ImportReport) -> Void
    let failed: (Error) -> Void

    var body: some View {
        VStack(spacing: 0) {
            content
            Divider()
            buttons
        }
        .frame(minWidth: 480, minHeight: 480)
    }

    @ViewBuilder
    private var content: some View {
        if let refused = model.session.refused() {
            ContentUnavailableView {
                Label(tr("Import Failed"), systemImage: "exclamationmark.triangle")
            } description: {
                Text(refused)
            }
        } else {
            List {
                ForEach(model.categories, id: \.label) { (category: ImportCategory) in
                    ImportCategoryView(model: model, category: category)
                }
                if let note = model.session.mapsNote() {
                    Text(note).font(.caption).foregroundStyle(.secondary)
                }
            }
        }
    }

    private func importChosen() {
        do {
            imported(try model.session.`import`())
        } catch {
            failed(error)
        }
    }

    private var buttons: some View {
        HStack {
            Text(model.session.fileName()).foregroundStyle(.secondary)
            Spacer()
            Button(tr("_Cancel"), role: .cancel, action: close)
                .keyboardShortcut(.cancelAction)
            Button(tr("_Import"), action: importChosen)
                .keyboardShortcut(.defaultAction)
                .disabled(!model.canImport || model.session.refused() != nil)
        }
        .padding()
    }
}

struct ImportCategoryView: View {
    @ObservedObject var model: ImportModel
    let category: ImportCategory

    var body: some View {
        Section(category.label) {
            ForEach(category.items, id: \.index) { (item: ImportItem) in
                Toggle(isOn: checked(item)) {
                    ImportItemLabel(item: item)
                }
            }
        }
    }

    private func checked(_ item: ImportItem) -> Binding<Bool> {
        Binding<Bool>(get: { item.checked }, set: { (on: Bool) in model.set(item, on) })
    }
}

struct ImportItemLabel: View {
    let item: ImportItem

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(item.name)
            if !item.note.isEmpty {
                Text(item.note)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }
}
