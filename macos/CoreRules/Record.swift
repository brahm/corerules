// A record of the user databases, edited in a sheet over the main window:
// the editor of `cr_app::records::form` through the bridge, its pages
// chosen above them, each described as a form, with the dialogs its
// fields open, its questions, Choose Picture, and Save.

import AppKit
import SwiftUI
import UniformTypeIdentifiers

@MainActor
final class RecordModel: ObservableObject {
    let session: RecordSession
    let title: String
    @Published private(set) var tabs: [String] = []
    @Published private(set) var tab = 0
    @Published private(set) var form = FormPage(groups: [], dialog: nil)
    @Published private(set) var prompt: PromptInfo?
    @Published private(set) var error: String?
    @Published private(set) var canSave = false
    @Published private(set) var saving = false
    /// The field Choose… asked a picture for.
    @Published var picking: UInt32?
    /// Called once the record is saved.
    var done: (RecordDone) -> Void = { _ in }

    init(_ session: RecordSession) {
        self.session = session
        title = session.title()
        refresh()
    }

    func refresh() {
        tabs = session.tabs()
        tab = Int(session.tab())
        form = session.formPage()
        prompt = session.prompt()
        error = session.error()
        canSave = session.canSave()
        saving = session.saving()
        if let field = session.takePicking() {
            picking = field
        }
        if let saved = session.takeDone() {
            done(saved)
        }
    }

    func act(_ id: UInt32, _ value: FormValue) {
        session.act(id: id, value: value)
        refresh()
    }

    func answer(_ yes: Bool) {
        session.answer(yes: yes)
        refresh()
    }

    func save() {
        session.save()
        refresh()
    }

    var tabShown: Binding<Int> {
        Binding<Int>(
            get: { self.tab },
            set: { (index: Int) in
                self.session.setTab(index: UInt32(index))
                self.refresh()
            })
    }

    /// Whether a field's dialog, or the items', is open over the page.
    var dialogShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.form.dialog != nil },
            set: { (shown: Bool) in
                if !shown, let close = self.form.dialog?.close {
                    self.act(close.id, .press)
                }
            })
    }

    var pickingShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.picking != nil },
            set: { (shown: Bool) in if !shown { self.picking = nil } })
    }

    /// Keeps the picture chosen for the field Choose… asked it for.
    func picked(_ stored: String) {
        guard let field = picking else { return }
        picking = nil
        session.setPicture(field: field, stored: stored)
        refresh()
    }

    /// Makes a picture from the user's file at `url`.
    func pickedFile(_ url: URL) throws {
        guard let field = picking else { return }
        picking = nil
        try session.addPicture(field: field, path: url.path)
        refresh()
    }
}

struct RecordView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var model: RecordModel
    let close: () -> Void
    @State private var discarding = false

    var body: some View {
        VStack(spacing: 0) {
            RecordHeading(model: model)
                .sheet(isPresented: model.pickingShown) {
                    if let field = model.picking {
                        PictureChooser(session: model.session, field: field, model: model)
                    }
                }
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            Divider()
            buttons
        }
        .frame(minWidth: 680, idealWidth: 760, minHeight: 600, idealHeight: 720)
        .sheet(isPresented: model.dialogShown) {
            if let dialog = model.form.dialog {
                FormDialogView(dialog: dialog) { (id: UInt32, value: FormValue) in
                    model.act(id, value)
                }
                .environmentObject(app)
            }
        }
        .alert(tr("Discard Changes?"), isPresented: $discarding) {
            Button(tr("_Discard"), role: .destructive, action: close)
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(tr("The record has changes that are not saved."))
        }
    }

    @ViewBuilder
    private var content: some View {
        if let prompt = model.prompt {
            PromptView(
                prompt: prompt, promptOff: { (_: Bool) in },
                answer: { (yes: Bool) in model.answer(yes) })
        } else {
            FormView(groups: model.form.groups) { (id: UInt32, value: FormValue) in
                model.act(id, value)
            }
        }
    }

    private func cancel() {
        if model.session.changed() {
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
                .disabled(model.saving)
            Button(tr("_Save")) { model.save() }
                .keyboardShortcut(.defaultAction)
                .disabled(!model.canSave)
        }
        .padding()
    }
}

/// The editor's title and its pages, as a segmented control when they
/// are few and a menu when they are many.
struct RecordHeading: View {
    @ObservedObject var model: RecordModel

    var body: some View {
        VStack(spacing: 12) {
            Text(model.title)
                .font(.title2.bold())
                .frame(maxWidth: .infinity, alignment: .leading)
            if model.tabs.count > 1 {
                if model.tabs.count <= 5 {
                    pages.pickerStyle(.segmented)
                } else {
                    pages.pickerStyle(.menu)
                        .frame(maxWidth: 320, alignment: .leading)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .padding(.horizontal, 20)
        .padding(.top, 16)
        .padding(.bottom, 8)
    }

    private var pages: some View {
        Picker(tr("Page"), selection: model.tabShown) {
            ForEach(0..<model.tabs.count, id: \.self) { (index: Int) in
                Text(model.tabs[index]).tag(index)
            }
        }
        .labelsHidden()
        .disabled(model.prompt != nil)
    }
}

/// Choose Picture: the installation's pictures, or the items' icons with
/// a search, or one made from a picture of the user's.
struct PictureChooser: View {
    @Environment(\.dismiss) private var dismiss
    let session: RecordSession
    let field: UInt32
    @ObservedObject var model: RecordModel
    @State private var choices: [PictureChoice]?
    @State private var error: String?
    @State private var search = ""

    var body: some View {
        VStack(spacing: 0) {
            TextField(tr("Search"), text: $search)
                .textFieldStyle(.roundedBorder)
                .padding()
            content
                .frame(minWidth: 600, minHeight: 440)
            Divider()
            HStack {
                Button(tr("Choose _File…")) { chooseFile() }
                Spacer()
                Button(tr("_Cancel"), role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
            }
            .padding()
        }
        .task { await load() }
    }

    @ViewBuilder
    private var content: some View {
        if let error {
            ContentUnavailableView(
                tr("Could Not Read"), systemImage: "photo", description: Text(error))
        } else if let choices {
            PictureGrid(choices: shown(choices)) { (stored: String) in
                model.picked(stored)
                dismiss()
            }
        } else {
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private func shown(_ choices: [PictureChoice]) -> [PictureChoice] {
        let needle = search.trimmingCharacters(in: .whitespaces).lowercased()
        if needle.isEmpty { return choices }
        return choices.filter { $0.searched.contains(needle) }
    }

    private func load() async {
        let session = self.session
        let field = self.field
        let result = await Task.detached {
            Result { try session.pictureChoices(field: field) }
        }.value
        switch result {
        case .success(let found): choices = found
        case .failure(let failure): error = message(failure)
        }
    }

    private func chooseFile() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.bmp, .jpeg, .gif, .png]
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try model.pickedFile(url)
            dismiss()
        } catch {
            self.error = message(error)
        }
    }
}

/// The pictures in their groups, each a button that chooses it.
struct PictureGrid: View {
    let choices: [PictureChoice]
    let pick: (String) -> Void

    private var groups: [String] {
        var seen: [String] = []
        for choice in choices where !seen.contains(choice.group) {
            seen.append(choice.group)
        }
        return seen
    }

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 12) {
                ForEach(groups, id: \.self) { (group: String) in
                    if !group.isEmpty {
                        Text(group).font(.headline)
                    }
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 96))], spacing: 12) {
                        ForEach(choices.filter { $0.group == group }, id: \.stored) {
                            (choice: PictureChoice) in
                            PictureTile(choice: choice, pick: pick)
                        }
                    }
                }
            }
            .padding()
        }
    }
}

struct PictureTile: View {
    let choice: PictureChoice
    let pick: (String) -> Void

    private var image: NSImage? {
        if let png = choice.png { return NSImage(data: png) }
        if let path = choice.path { return NSImage(contentsOfFile: path) }
        return nil
    }

    var body: some View {
        Button {
            pick(choice.stored)
        } label: {
            VStack(spacing: 4) {
                if let image {
                    Image(nsImage: image)
                        .resizable()
                        .interpolation(.none)
                        .aspectRatio(contentMode: .fit)
                        .frame(width: 84, height: 64)
                } else {
                    Image(systemName: "photo")
                        .frame(width: 84, height: 64)
                        .foregroundStyle(.tertiary)
                }
                Text(choice.name)
                    .font(.caption)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
        .buttonStyle(.plain)
        .help(choice.name)
    }
}

/// New in the other parts' section: the part type to make first.
@MainActor
final class PartTypeModel: ObservableObject {
    let types: [PartType]
    @Published var chosen: Int

    init(types: [PartType], chosen: UInt32) {
        self.types = types
        self.chosen = Int(chosen)
    }
}

struct PartTypeView: View {
    @ObservedObject var model: PartTypeModel
    let close: () -> Void
    let create: (PartType) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(tr("New Part")).font(.title2.bold())
            Text(tr("Select the type of the new part."))
            Picker(tr("Part Type"), selection: $model.chosen) {
                ForEach(0..<model.types.count, id: \.self) { (index: Int) in
                    Text(model.types[index].name).tag(index)
                }
            }
            HStack {
                Spacer()
                Button(tr("_Cancel"), role: .cancel, action: close)
                    .keyboardShortcut(.cancelAction)
                Button(tr("_OK")) {
                    if model.chosen < model.types.count {
                        create(model.types[model.chosen])
                    }
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(minWidth: 380)
    }
}
