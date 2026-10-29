// The main window: the library's sections, the entries of the section
// chosen, and the details of the entry chosen, in the three columns the
// GNOME and Windows apps have.

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var model: AppModel
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        Group {
            if model.library == nil {
                ContentUnavailableView {
                    Label(tr("No Core Rules Installation"), systemImage: "externaldrive.badge.questionmark")
                } description: {
                    Text(model.noInstallation)
                } actions: {
                    Button(more(tr("_Open Folder…"))) { model.perform(Action.OPEN) }
                }
            } else {
                NavigationSplitView {
                    List(model.sections, id: \.id, selection: $model.section) { section in
                        Label(section.title, systemImage: SectionIcon.name(section.id))
                            .badge(Int(section.count))
                    }
                    .navigationSplitViewColumnWidth(min: 180, ideal: 220)
                } content: {
                    EntryList()
                        .navigationSplitViewColumnWidth(min: 220, ideal: 280)
                } detail: {
                    DetailView()
                }
            }
        }
        .frame(minWidth: 820, minHeight: 520)
        .onAppear { model.openSettings = { openSettings() } }
        .sheet(item: $model.modal) { modal in
            switch modal {
            case .wizard(let wizard):
                WizardView(model: wizard) { model.closeWizard(wizard) }
            case .edit(let edit):
                EditView(model: edit) { saved in model.closeEdit(edit, saved: saved) }
            case .importing(let importing):
                ImportView(
                    model: importing,
                    close: { model.modal = nil },
                    imported: { model.imported($0) },
                    failed: { error in
                        model.modal = nil
                        model.fail(tr("Import Failed"), error)
                    })
            }
        }
        .alert(
            model.notice?.title ?? "",
            isPresented: Binding(get: { model.notice != nil }, set: { if !$0 { model.notice = nil } }),
            presenting: model.notice
        ) { _ in
            Button(tr("_OK")) { model.notice = nil }
        } message: { notice in
            Text(notice.text)
        }
        .alert(
            tr("Delete {name}?", ["name": model.deleting?.name ?? ""]),
            isPresented: Binding(get: { model.deleting != nil }, set: { if !$0 { model.deleting = nil } })
        ) {
            Button(tr("_Delete"), role: .destructive) {
                if let file = model.deleting?.file { model.delete(file) }
            }
            Button(tr("_Cancel"), role: .cancel) { model.deleting = nil }
        } message: {
            Text(tr("The character leaves the character list and its file is removed."))
        }
    }
}

struct EntryList: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        List(model.entries, id: \.index, selection: $model.entry) { row in
            VStack(alignment: .leading, spacing: 2) {
                Text(row.title)
                if !row.subtitle.isEmpty {
                    Text(row.subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .contextMenu {
                if row.character && model.library?.isWritable() == true {
                    Button(more(tr("Edit Character"))) {
                        model.entry = row.index
                        model.editCharacter()
                    }
                    Button(more(tr("_Delete"))) {
                        model.entry = row.index
                        model.askDelete()
                    }
                }
            }
        }
        .searchable(text: $model.search, placement: .sidebar, prompt: tr("Search"))
        .onDeleteCommand { model.askDelete() }
        .overlay {
            if model.entries.isEmpty && !model.search.isEmpty {
                ContentUnavailableView.search(text: model.search)
            }
        }
        .navigationTitle(model.sections.first { $0.id == model.section }?.title ?? tr("Core Rules"))
    }
}

struct DetailView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        if let details = model.details {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    HStack(alignment: .top, spacing: 16) {
                        if let path = details.picture, let image = NSImage(contentsOfFile: path) {
                            Image(nsImage: image)
                                .resizable()
                                .scaledToFit()
                                .frame(maxWidth: 120, maxHeight: 160)
                        }
                        VStack(alignment: .leading, spacing: 4) {
                            Text(details.title).font(.title.bold()).textSelection(.enabled)
                            if !details.subtitle.isEmpty {
                                Text(details.subtitle).foregroundStyle(.secondary)
                            }
                        }
                    }
                    ForEach(Array(details.groups.enumerated()), id: \.offset) { _, group in
                        GroupBox(group.title) {
                            Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 12, verticalSpacing: 6) {
                                ForEach(Array(group.rows.enumerated()), id: \.offset) { _, row in
                                    GridRow {
                                        Text(row.label).foregroundStyle(.secondary)
                                        Text(row.value).textSelection(.enabled)
                                    }
                                }
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                    if !details.text.isEmpty {
                        Text(details.text).textSelection(.enabled)
                    }
                }
                .padding(20)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .toolbar {
                if details.character {
                    ToolbarItemGroup {
                        Button { model.editCharacter() } label: {
                            Label(tr("Edit Character"), systemImage: "pencil")
                        }
                        .disabled(model.library?.isWritable() != true)
                        Button { model.perform(Action.SHEET_PRINT) } label: {
                            Label(tr("_Print…"), systemImage: "printer")
                        }
                        Button { model.askDelete() } label: {
                            Label(tr("_Delete"), systemImage: "trash")
                        }
                        .disabled(model.library?.isWritable() != true)
                    }
                }
            }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "doc.text.magnifyingglass")
        }
    }
}
