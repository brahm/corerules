// The main window: the library's sections, the entries of the section
// chosen, and the details of the entry chosen, in the three columns the
// GNOME and Windows apps have.

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var model: AppModel
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        main
            .frame(minWidth: 820, minHeight: 520)
            .onAppear { model.openSettings = { openSettings() } }
            .sheet(item: $model.modal) { modal in
                ModalView(modal: modal)
            }
            .modifier(NoticeAlert())
            .modifier(DeleteAlert())
    }

    @ViewBuilder
    private var main: some View {
        if model.library == nil {
            NoInstallation()
        } else {
            NavigationSplitView {
                SectionList()
                    .navigationSplitViewColumnWidth(min: 180, ideal: 220)
            } content: {
                EntryList()
                    .navigationSplitViewColumnWidth(min: 220, ideal: 280)
            } detail: {
                DetailView()
            }
        }
    }
}

struct NoInstallation: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        ContentUnavailableView {
            Label(tr("No Core Rules Installation"), systemImage: "externaldrive.badge.questionmark")
        } description: {
            Text(model.noInstallation)
        } actions: {
            Button(more(tr("_Open Folder…"))) { model.perform(Action.OPEN) }
        }
    }
}

struct SectionList: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        List(model.sections, id: \.id, selection: $model.section) { (section: SectionInfo) in
            Label {
                Text(section.title)
            } icon: {
                SectionIconView(section: section.id)
            }
                .badge(Int(section.count))
        }
    }
}

/// The sheet over the main window.
struct ModalView: View {
    @EnvironmentObject var model: AppModel
    let modal: Modal

    var body: some View {
        switch modal {
        case .wizard(let wizard):
            WizardView(model: wizard, close: { model.closeWizard(wizard) })
        case .edit(let edit):
            EditView(model: edit, close: { (saved: Bool) in model.closeEdit(edit, saved: saved) })
        case .importing(let importing):
            ImportView(
                model: importing,
                close: { model.modal = nil },
                imported: { (report: ImportReport) in model.imported(report) },
                failed: { (error: Error) in
                    model.modal = nil
                    model.fail(tr("Import Failed"), error)
                })
        }
    }
}

/// A message the model shows, with OK.
struct NoticeAlert: ViewModifier {
    @EnvironmentObject var model: AppModel

    private var shown: Binding<Bool> {
        Binding<Bool>(
            get: { model.notice != nil },
            set: { (on: Bool) in if !on { model.notice = nil } })
    }

    func body(content: Content) -> some View {
        content.alert(model.notice?.title ?? "", isPresented: shown, presenting: model.notice) {
            (_: Notice) in
            Button(tr("_OK")) { model.notice = nil }
        } message: { (notice: Notice) in
            Text(notice.text)
        }
    }
}

/// Delete's confirmation.
struct DeleteAlert: ViewModifier {
    @EnvironmentObject var model: AppModel

    private var shown: Binding<Bool> {
        Binding<Bool>(
            get: { model.deleting != nil },
            set: { (on: Bool) in if !on { model.deleting = nil } })
    }

    private var title: String {
        tr("Delete {name}?", ["name": model.deleting?.name ?? ""])
    }

    func body(content: Content) -> some View {
        content.alert(title, isPresented: shown) {
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
        List(model.entries, id: \.index, selection: $model.entry) { (row: EntryRow) in
            EntryRowView(row: row)
        }
        .searchable(text: $model.search, placement: .sidebar, prompt: tr("Search"))
        .onDeleteCommand { model.askDelete() }
        .overlay { noResults }
        .navigationTitle(title)
    }

    private var title: String {
        model.sections.first { $0.id == model.section }?.title ?? tr("Core Rules")
    }

    @ViewBuilder
    private var noResults: some View {
        if model.entries.isEmpty && !model.search.isEmpty {
            ContentUnavailableView.search(text: model.search)
        }
    }
}

struct EntryRowView: View {
    @EnvironmentObject var model: AppModel
    let row: EntryRow

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(row.title)
            if !row.subtitle.isEmpty {
                Text(row.subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .contextMenu { menu }
    }

    @ViewBuilder
    private var menu: some View {
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

struct DetailView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        if let details = model.details {
            ScrollView {
                EntryDetailsView(details: details)
            }
            .toolbar {
                if details.character {
                    CharacterTools(model: model)
                }
            }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "doc.text.magnifyingglass")
        }
    }
}

struct EntryDetailsView: View {
    let details: EntryDetails

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            header
            ForEach(Array(details.groups.enumerated()), id: \.offset) { item in
                GroupView(group: item.element)
            }
            if !details.text.isEmpty {
                Text(details.text).textSelection(.enabled)
            }
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var header: some View {
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
    }
}

/// A titled group of an entry's details: its labels and values.
struct GroupView: View {
    let group: Group

    var body: some View {
        GroupBox(group.title) {
            Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 12, verticalSpacing: 6) {
                ForEach(Array(group.rows.enumerated()), id: \.offset) { item in
                    GridRow {
                        Text(item.element.label).foregroundStyle(.secondary)
                        Text(item.element.value).textSelection(.enabled)
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

/// The character's tools in the toolbar.
struct CharacterTools: ToolbarContent {
    @ObservedObject var model: AppModel

    var body: some ToolbarContent {
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
