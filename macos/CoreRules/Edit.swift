// Edit Character, in a sheet over the main window. The form's checks and
// questions come from `cr_app::editor` through the bridge; the prototype
// draws the Details tab's description of the character.

import SwiftUI

@MainActor
final class EditModel: ObservableObject {
    let session: EditSession
    /// The character's file, to select it again after saving.
    let file: String
    @Published private(set) var details: EditDetails
    @Published private(set) var problem: String?
    @Published private(set) var error: String?
    @Published private(set) var canSave = false
    @Published var prompt: PromptInfo?

    init(_ session: EditSession, file: String) {
        self.session = session
        self.file = file
        details = session.details()
        refresh()
    }

    func refresh() {
        details = session.details()
        problem = session.problem()
        error = session.error()
        canSave = session.canSave()
        prompt = session.prompt()
    }

    /// A binding that hands each change to the session.
    func binding<T>(_ get: @escaping (EditDetails) -> T, _ set: @escaping (EditSession, T) -> Void)
        -> Binding<T>
    {
        Binding(
            get: { get(self.details) },
            set: {
                set(self.session, $0)
                self.refresh()
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
    @ObservedObject var model: EditModel
    /// Closes the sheet; true when the character was saved.
    let close: (Bool) -> Void
    @State private var discarding = false

    var body: some View {
        VStack(spacing: 0) {
            if let prompt = model.prompt {
                PromptView(
                    prompt: prompt, promptOff: { _ in },
                    answer: { yes in
                        if model.settle(model.session.answer(yes: yes)) { close(true) }
                    }
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            } else {
                form
            }
            Divider()
            HStack {
                if let message = model.error ?? model.problem {
                    Label(message, systemImage: "exclamationmark.triangle")
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }
                Spacer()
                Button(tr("_Cancel"), role: .cancel) {
                    if model.session.changed() {
                        discarding = true
                    } else {
                        close(false)
                    }
                }
                .keyboardShortcut(.cancelAction)
                Button(tr("_Save")) {
                    if model.settle(model.session.save()) { close(true) }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(!model.canSave || model.prompt != nil)
            }
            .padding()
        }
        .frame(minWidth: 520, minHeight: 560)
        .alert(tr("Discard Changes?"), isPresented: $discarding) {
            Button(tr("_Discard"), role: .destructive) { close(false) }
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(tr("The character has changes that are not saved."))
        }
    }

    private var form: some View {
        Form {
            Section(tr("Edit Character")) {
                TextField(tr("Name"), text: model.binding({ $0.name }, { $0.setName(name: $1) }))
                TextField(tr("Player"), text: model.binding({ $0.player }, { $0.setPlayer(player: $1) }))
                Picker(tr("Gender"), selection: model.binding({ $0.male }, { $0.setMale(male: $1) })) {
                    Text(tr("Male")).tag(true)
                    Text(tr("Female")).tag(false)
                }
                Picker(
                    tr("Alignment"),
                    selection: model.binding({ Int($0.alignment) }, { $0.setAlignment(index: UInt32($1)) })
                ) {
                    ForEach(Array(model.details.alignments.enumerated()), id: \.offset) { index, name in
                        Text(name).tag(index)
                    }
                }
                number(tr("Age"), model.binding({ $0.age }, { $0.setAge(years: $1) }))
                number(tr("Height"), model.binding({ $0.height }, { $0.setHeight(inches: $1) }))
                number(tr("Weight"), model.binding({ $0.weight }, { $0.setWeight(pounds: $1) }))
            }
            Section(tr("History")) {
                TextEditor(text: model.binding({ $0.history }, { $0.setHistory(history: $1) }))
                    .font(.body)
                    .frame(minHeight: 120)
            }
        }
        .formStyle(.grouped)
    }

    private func number(_ label: String, _ value: Binding<UInt32>) -> some View {
        TextField(label, value: value, format: .number)
            .multilineTextAlignment(.trailing)
    }
}
