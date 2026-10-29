// Edit Character, in a sheet over the main window. Its pages, chosen
// above them, come from `cr_app::editor` through the bridge, each
// described as a form, with its questions and Save's checks.

import SwiftUI

@MainActor
final class EditModel: ObservableObject {
    let session: EditSession
    /// The character's file, to select it again after saving.
    let file: String
    let tabs: [String]
    @Published private(set) var tab = 0
    @Published private(set) var form = FormPage(groups: [], dialog: nil)
    @Published private(set) var problem: String?
    @Published private(set) var error: String?
    @Published private(set) var canSave = false
    @Published private(set) var saving = false
    @Published private(set) var prompt: PromptInfo?

    init(_ session: EditSession, file: String) {
        self.session = session
        self.file = file
        tabs = session.tabs()
        refresh()
    }

    /// Takes the session's state after an input.
    func refresh() {
        tab = Int(session.tab())
        form = session.form()
        problem = session.problem()
        error = session.error()
        canSave = session.canSave()
        saving = session.saving()
        prompt = session.prompt()
    }

    func act(_ id: UInt32, _ value: FormValue) {
        session.act(id: id, value: value)
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

    /// Whether Levels or Add Experience is open over the page.
    var dialogShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.form.dialog != nil },
            set: { (shown: Bool) in
                if !shown, let close = self.form.dialog?.close {
                    self.act(close.id, .press)
                }
            })
    }

    /// Save, or an answer to its question: true once the character is
    /// written.
    func settle(_ state: SaveState) -> Bool {
        refresh()
        if case .saved = state { return true }
        return false
    }
}

struct EditView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var model: EditModel
    /// Closes the sheet; true when the character was saved.
    let close: (Bool) -> Void
    @State private var discarding = false

    var body: some View {
        VStack(spacing: 0) {
            EditHeading(model: model)
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
            Button(tr("_Discard"), role: .destructive) { close(false) }
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(tr("The character has changes that are not saved."))
        }
    }

    @ViewBuilder
    private var content: some View {
        if let prompt = model.prompt {
            PromptView(prompt: prompt, promptOff: { (_: Bool) in }, answer: answer)
        } else {
            FormView(groups: model.form.groups) { (id: UInt32, value: FormValue) in
                model.act(id, value)
            }
        }
    }

    private func answer(_ yes: Bool) {
        if model.settle(model.session.answer(yes: yes)) { close(true) }
    }

    private func save() {
        if model.settle(model.session.save()) { close(true) }
    }

    private func cancel() {
        if model.session.changed() {
            discarding = true
        } else {
            close(false)
        }
    }

    private var buttons: some View {
        HStack {
            if let message = model.error ?? model.problem {
                Label(
                    message,
                    systemImage: model.error != nil
                        ? "exclamationmark.octagon.fill" : "exclamationmark.triangle.fill"
                )
                .foregroundStyle(model.error != nil ? Color.red : Color.secondary)
                .lineLimit(2)
                .textSelection(.enabled)
            }
            Spacer()
            Button(tr("_Cancel"), role: .cancel, action: cancel)
                .keyboardShortcut(.cancelAction)
                .disabled(model.saving)
            Button(tr("_Save"), action: save)
                .keyboardShortcut(.defaultAction)
                .disabled(!model.canSave || model.prompt != nil)
        }
        .padding()
    }
}

/// The sheet's title, the character's name, and its pages.
struct EditHeading: View {
    @ObservedObject var model: EditModel

    var body: some View {
        VStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(tr("Edit Character"))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(model.session.title())
                    .font(.title2.bold())
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Picker(tr("Page"), selection: model.tabShown) {
                ForEach(0..<model.tabs.count, id: \.self) { (index: Int) in
                    Text(model.tabs[index]).tag(index)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .disabled(model.prompt != nil)
        }
        .padding(.horizontal, 20)
        .padding(.top, 16)
        .padding(.bottom, 8)
    }
}
