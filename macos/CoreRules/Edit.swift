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
    private func binding<T>(_ get: @escaping (EditDetails) -> T, _ set: @escaping (T) -> Void)
        -> Binding<T>
    {
        Binding<T>(
            get: { get(self.details) },
            set: { (value: T) in
                set(value)
                self.refresh()
            })
    }

    var name: Binding<String> {
        binding({ $0.name }, { self.session.setName(name: $0) })
    }
    var player: Binding<String> {
        binding({ $0.player }, { self.session.setPlayer(player: $0) })
    }
    var male: Binding<Bool> {
        binding({ $0.male }, { self.session.setMale(male: $0) })
    }
    var alignment: Binding<Int> {
        binding({ Int($0.alignment) }, { self.session.setAlignment(index: UInt32($0)) })
    }
    var age: Binding<UInt32> {
        binding({ $0.age }, { self.session.setAge(years: $0) })
    }
    var height: Binding<UInt32> {
        binding({ $0.height }, { self.session.setHeight(inches: $0) })
    }
    var weight: Binding<UInt32> {
        binding({ $0.weight }, { self.session.setWeight(pounds: $0) })
    }
    var history: Binding<String> {
        binding({ $0.history }, { self.session.setHistory(history: $0) })
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
            content
            Divider()
            buttons
        }
        .frame(minWidth: 520, minHeight: 560)
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
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        } else {
            form
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
                Label(message, systemImage: "exclamationmark.triangle")
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            Spacer()
            Button(tr("_Cancel"), role: .cancel, action: cancel)
                .keyboardShortcut(.cancelAction)
            Button(tr("_Save"), action: save)
                .keyboardShortcut(.defaultAction)
                .disabled(!model.canSave || model.prompt != nil)
        }
        .padding()
    }

    private var form: some View {
        Form {
            Section(tr("Edit Character")) {
                TextField(tr("Name"), text: model.name)
                TextField(tr("Player"), text: model.player)
                Picker(tr("Gender"), selection: model.male) {
                    Text(tr("Male")).tag(true)
                    Text(tr("Female")).tag(false)
                }
                Picker(tr("Alignment"), selection: model.alignment) {
                    ForEach(0..<model.details.alignments.count, id: \.self) { (index: Int) in
                        Text(model.details.alignments[index]).tag(index)
                    }
                }
                number(tr("Age"), model.age)
                number(tr("Height"), model.height)
                number(tr("Weight"), model.weight)
            }
            Section(tr("History")) {
                TextEditor(text: model.history)
                    .font(.body)
                    .frame(minHeight: 120)
            }
        }
        .formStyle(.grouped)
    }

    private func number(_ label: String, _ value: Binding<UInt32>) -> some View {
        TextField(label, value: value, format: IntegerFormatStyle<UInt32>())
            .multilineTextAlignment(.trailing)
    }
}
