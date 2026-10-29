// The main window: the library's sections, the entries of the section
// chosen with the search and filters, and the details of the entry
// chosen, in the three columns the GNOME and Windows apps have.

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var model: AppModel
    @Environment(\.openSettings) private var openSettings
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        main
            .frame(minWidth: 900, minHeight: 560)
            .onAppear {
                model.openSettings = { openSettings() }
                model.openSheet = { (id: UUID) in openWindow(id: "sheet", value: id) }
                model.openHelpWindow = { openWindow(id: "help") }
                model.openBookWindow = { (id: UUID) in openWindow(id: "book", value: id) }
                model.openAbout = { openWindow(id: "about") }
                model.openTablesWindow = { openWindow(id: "tables") }
                model.openTreasureWindow = { openWindow(id: "treasure") }
            }
            .sheet(item: $model.modal) { modal in
                ModalView(modal: modal)
                    .environmentObject(model)
            }
            .sheet(isPresented: $model.fullScreen) {
                FullScreenPicture()
                    .environmentObject(model)
            }
            .modifier(NoticeAlert())
            .modifier(ConfirmAlert())
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
                    .navigationSplitViewColumnWidth(min: 240, ideal: 300)
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
        case .npc(let npc):
            NpcView(model: npc, close: { model.modal = nil })
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

/// The question asked before Delete or Convert to NPC.
struct ConfirmAlert: ViewModifier {
    @EnvironmentObject var model: AppModel

    private var shown: Binding<Bool> {
        Binding<Bool>(
            get: { model.confirm != nil },
            set: { (on: Bool) in if !on { model.confirm = nil } })
    }

    private var title: String {
        switch model.confirm {
        case .delete(_, let name): return tr("Delete {name}?", ["name": name])
        case .toNpc: return tr("Convert to NPC?")
        case nil: return ""
        }
    }

    private var text: String {
        switch model.confirm {
        case .delete: return tr("The character leaves the character list and its file is removed.")
        case .toNpc:
            return tr("This will move the selected Player Character to the NPC List in the DM Toolkit.")
        case nil: return ""
        }
    }

    private var yes: String {
        if case .toNpc = model.confirm { return tr("_Convert") }
        return tr("_Delete")
    }

    private var destructive: Bool {
        if case .delete = model.confirm { return true }
        return false
    }

    func body(content: Content) -> some View {
        content.alert(title, isPresented: shown) {
            Button(yes, role: destructive ? .destructive : nil) { model.answer(true) }
            Button(tr("_Cancel"), role: .cancel) { model.answer(false) }
        } message: {
            Text(text)
        }
    }
}

struct EntryList: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            if model.library?.isCore20() == true {
                ReadOnlyBanner()
            }
            FilterBar()
            List(model.entries, id: \.index, selection: $model.entry) { (row: EntryRow) in
                EntryRowView(row: row)
            }
            .overlay { empty }
        }
        .searchable(text: $model.filter.search, placement: .sidebar, prompt: tr("Search"))
        .onDeleteCommand { model.askDelete() }
        .navigationTitle(model.currentSection?.title ?? tr("Core Rules"))
    }

    @ViewBuilder
    private var empty: some View {
        if let section = model.currentSection, model.entries.isEmpty {
            if let error = section.error {
                ContentUnavailableView(
                    tr("Could Not Read"), systemImage: "exclamationmark.triangle",
                    description: Text(error))
            } else if section.count == 0 {
                ContentUnavailableView(
                    tr("No {section}", ["section": section.title]),
                    systemImage: SectionIcon.name(section.id))
            } else if !model.filter.search.isEmpty {
                ContentUnavailableView.search(text: model.filter.search)
            } else {
                ContentUnavailableView(tr("No Results"), systemImage: "line.3.horizontal.decrease.circle")
            }
        }
    }
}

/// Core Rules 2.0 is only read, which the list says above it.
struct ReadOnlyBanner: View {
    var body: some View {
        Label {
            VStack(alignment: .leading, spacing: 2) {
                Text(tr("Core Rules 2.0 is read-only")).bold()
                Text(
                    tr(
                        "Open an Expansion installation and import this content with Import Core Rules 2.0…"
                    )
                )
                .font(.caption)
                .foregroundStyle(.secondary)
            }
        } icon: {
            Image(systemName: "lock")
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(10)
        .background(.quaternary.opacity(0.5))
    }
}

/// The filters the section offers: standard or custom records, a party,
/// a part type, Magic Only.
struct FilterBar: View {
    @EnvironmentObject var model: AppModel

    private var shown: Bool {
        let f = model.filters
        return f.origins || !f.parties.isEmpty || !f.partTypes.isEmpty || f.magic
    }

    var body: some View {
        if shown {
            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    if model.filters.origins {
                        origins
                    }
                    if !model.filters.parties.isEmpty {
                        choices(tr("Show a Party"), model.filters.parties, $model.filter.party)
                    }
                    if !model.filters.partTypes.isEmpty {
                        choices(tr("Show a Part Type"), model.filters.partTypes, $model.filter.partType)
                    }
                }
                if model.filters.magic {
                    Toggle(tr("Magic Only"), isOn: $model.filter.magicOnly)
                        .toggleStyle(.checkbox)
                }
            }
            .controlSize(.small)
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
            Divider()
        }
    }

    private var origins: some View {
        Picker(tr("Show Standard or Custom Records"), selection: $model.filter.origins) {
            ForEach(0..<originNames().count, id: \.self) { (index: Int) in
                Text(originNames()[index]).tag(UInt32(index))
            }
        }
        .labelsHidden()
        .fixedSize()
        .help(tr("Show Standard or Custom Records"))
    }

    private func choices(_ help: String, _ choices: [FilterChoice], _ value: Binding<String?>)
        -> some View
    {
        Picker(help, selection: value) {
            ForEach(choices, id: \.label) { (choice: FilterChoice) in
                Text(choice.label).tag(choice.value)
            }
        }
        .labelsHidden()
        .fixedSize()
        .help(help)
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
        if row.character && model.writable {
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
                EntryTools(model: model, commands: details.commands)
            }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "doc.text.magnifyingglass")
        }
    }
}

struct EntryDetailsView: View {
    @EnvironmentObject var model: AppModel
    let details: EntryDetails

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            header
            if let picture = model.picture, let image = NSImage(data: picture) {
                PictureView(image: image, monster: details.monsterPicture, frames: details.frames)
            }
            ForEach(Array(details.groups.enumerated()), id: \.offset) { item in
                if !item.element.rows.isEmpty {
                    GroupView(group: item.element)
                }
            }
            if !details.text.isEmpty {
                GroupBox(tr("Description")) {
                    Text(details.text)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var header: some View {
        HStack(alignment: .center, spacing: 16) {
            if let icon = details.icon, let image = NSImage(data: icon) {
                Image(nsImage: image)
                    .interpolation(.none)
                    .accessibilityLabel(tr("Icon"))
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

/// An entry's picture; a monster's with Next Image and Full Screen.
struct PictureView: View {
    @EnvironmentObject var model: AppModel
    let image: NSImage
    let monster: Bool
    let frames: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if monster {
                HStack {
                    if frames {
                        Button { model.nextImage() } label: {
                            Label(tr("Next Image"), systemImage: "arrow.right")
                        }
                    }
                    Button { model.fullScreen = true } label: {
                        Label(tr("Full Screen"), systemImage: "arrow.up.left.and.arrow.down.right")
                    }
                }
                .controlSize(.small)
            }
            Image(nsImage: image)
                .resizable()
                .interpolation(.none)
                .scaledToFit()
                .frame(maxWidth: 360, maxHeight: 240, alignment: .leading)
        }
    }
}

/// Full Screen: a monster's picture as large as the window allows.
struct FullScreenPicture: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        VStack {
            if let picture = model.picture, let image = NSImage(data: picture) {
                Image(nsImage: image)
                    .resizable()
                    .interpolation(.none)
                    .scaledToFit()
            }
        }
        .frame(minWidth: 640, minHeight: 480)
        .background(.black)
        .onTapGesture { model.fullScreen = false }
        .onExitCommand { model.fullScreen = false }
    }
}

/// A titled group of an entry's details: its labels and values; a row
/// without a value is a list item.
struct GroupView: View {
    let group: Group

    var body: some View {
        GroupBox(group.title) {
            Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 12, verticalSpacing: 6) {
                ForEach(Array(group.rows.enumerated()), id: \.offset) { item in
                    if item.element.value.isEmpty {
                        GridRow {
                            Text(item.element.label)
                                .textSelection(.enabled)
                                .gridCellColumns(2)
                        }
                    } else {
                        GridRow {
                            Text(item.element.label).foregroundStyle(.secondary)
                            Text(item.element.value).textSelection(.enabled)
                        }
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

/// The commands of the entry in view, in the toolbar, as
/// `cr_app::library` gives them.
struct EntryTools: ToolbarContent {
    @ObservedObject var model: AppModel
    let commands: EntryCommands

    var body: some ToolbarContent {
        ToolbarItemGroup {
            if commands.sheet || (commands.view && !commands.monster) {
                Button { model.showSheet() } label: {
                    Label(
                        commands.sheet ? tr("Character Sheet") : tr("View"),
                        systemImage: "doc.richtext")
                }
                .help(commands.sheet ? tr("Character Sheet") : tr("View"))
            }
            if commands.view && commands.monster {
                Button { model.monsterHelp(print: false) } label: {
                    Label(tr("View"), systemImage: "doc.richtext")
                }
                .help(tr("View"))
            }
            if commands.read {
                Button { model.perform(Action.OPEN_BOOK) } label: {
                    Label(tr("_Read Book"), systemImage: "book")
                }
                .help(tr("_Read Book"))
            }
            if commands.edit && commands.sheet {
                Button { model.editCharacter() } label: {
                    Label(tr("Edit Character"), systemImage: "pencil")
                }
                .help(tr("Edit Character"))
            }
            if commands.edit, let name = model.treasureName {
                Button { model.rollTreasure(saved: name) } label: {
                    Label(tr("Edit"), systemImage: "pencil")
                }
                .help(tr("Edit"))
                if commands.copy {
                    Button { model.rollTreasure(saved: name, copy: true) } label: {
                        Label(tr("Copy"), systemImage: "doc.on.doc")
                    }
                    .help(tr("Copy"))
                }
            }
            if commands.sheet || commands.print {
                Button { model.perform(Action.SHEET_PRINT) } label: {
                    Label(more(tr("_Print…")), systemImage: "printer")
                }
                .help(more(tr("_Print…")))
            }
            if commands.toCharacter || commands.toNpc || commands.delete {
                Menu {
                    if commands.toCharacter {
                        Button(tr("Convert to Character")) { model.toCharacter() }
                    }
                    if commands.toNpc {
                        Button(more(tr("Convert to NPC"))) { model.askToNpc() }
                    }
                    if commands.delete {
                        Divider()
                        Button(more(tr("Delete Character")), role: .destructive) { model.askDelete() }
                    }
                } label: {
                    Label(tr("More"), systemImage: "ellipsis.circle")
                }
                .help(tr("More"))
            }
            Button { model.perform(Action.HELP_CONTROL) } label: {
                Label(tr("Help"), systemImage: "questionmark.circle")
            }
            .help(tr("Help"))
        }
    }
}
