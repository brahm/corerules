// A page of a form as the bridge describes it (`cr-mac-ffi`'s form.rs):
// groups of rows in a grouped Form, each control saying what it was
// changed to by its number. New Character's pages, and the other forms as
// the Mac app gets them, are all drawn by this one view.

import AppKit
import SwiftUI
import UniformTypeIdentifiers

/// What a control was changed to, by the control's number.
typealias FormAct = (UInt32, FormValue) -> Void

struct FormView: View {
    let groups: [FormGroup]
    let act: FormAct

    var body: some View {
        Form {
            ForEach(Array(groups.enumerated()), id: \.offset) { item in
                FormGroupView(group: item.element, act: act)
            }
        }
        .formStyle(.grouped)
    }
}

struct FormGroupView: View {
    let group: FormGroup
    let act: FormAct

    var body: some View {
        Section {
            ForEach(Array(group.rows.enumerated()), id: \.offset) { item in
                FormRowView(row: item.element, act: act)
            }
        } header: {
            if !group.title.isEmpty || !group.caption.isEmpty || !group.buttons.isEmpty {
                FormHeading(group: group, act: act)
            }
        }
    }
}

/// A group's heading, the line under it, and its buttons.
struct FormHeading: View {
    let group: FormGroup
    let act: FormAct

    var body: some View {
        HStack(alignment: .bottom) {
            VStack(alignment: .leading, spacing: 2) {
                if !group.title.isEmpty {
                    Text(group.title)
                }
                if !group.caption.isEmpty {
                    Text(group.caption)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer()
            FormButtons(buttons: group.buttons, act: act)
        }
    }
}

struct FormRowView: View {
    let row: FormRow
    let act: FormAct

    var body: some View {
        switch row {
        case .note(let text, let secondary):
            Text(text)
                .foregroundStyle(secondary ? Color.secondary : Color.primary)
                .textSelection(.enabled)
        case .value(let title, let subtitle, let value, let buttons):
            ValueRow(title: title, subtitle: subtitle, value: value, buttons: buttons, act: act)
        case .choice(let id, let title, let subtitle, let options, let selected, let buttons):
            ChoiceRow(
                id: id, title: title, subtitle: subtitle, options: options, selected: selected,
                buttons: buttons, act: act)
        case .radio(let id, let options, let selected):
            RadioRow(id: id, options: options, selected: selected, act: act)
        case .number(let id, let title, let subtitle, let value, let min, let max, let enabled, let buttons):
            NumberRow(
                id: id, title: title, subtitle: subtitle, value: value, min: min, max: max,
                enabled: enabled, buttons: buttons, act: act)
        case .text(let id, let title, let value, let multiline, let buttons):
            TextRow(id: id, title: title, value: value, multiline: multiline, buttons: buttons, act: act)
        case .toggle(let id, let title, let subtitle, let on, let enabled):
            CheckRow(id: id, title: title, subtitle: subtitle, on: on, enabled: enabled, switched: true, act: act)
        case .check(let id, let title, let subtitle, let on, let enabled):
            CheckRow(id: id, title: title, subtitle: subtitle, on: on, enabled: enabled, switched: false, act: act)
        case .button(let title, let subtitle, let button):
            LabeledContent {
                FormButtonView(button: button, act: act)
            } label: {
                TwoLines(title: title, subtitle: subtitle)
            }
        case .item(let title, let subtitle, let number, let remove, let icon):
            ItemRow(title: title, subtitle: subtitle, number: number, remove: remove, icon: icon, act: act)
        case .tree(let title, let subtitle, let depth, let dimmed, let buttons):
            LabeledContent {
                HStack {
                    FormButtons(buttons: buttons, act: act)
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.borderless)
            } label: {
                TwoLines(title: title, subtitle: subtitle)
                    .padding(.leading, CGFloat(depth) * 20)
                    .opacity(dimmed ? 0.6 : 1)
            }
        case .add(let id, let label, let options):
            AddRow(id: id, label: label, options: options, act: act)
        case .portrait(let id, let path, let subtitle, let race):
            PortraitRow(id: id, path: path, subtitle: subtitle, race: race, act: act)
        case .banner(let text, let error):
            Label(text, systemImage: error ? "exclamationmark.octagon.fill" : "exclamationmark.triangle.fill")
                .foregroundStyle(error ? Color.red : Color.orange)
                .textSelection(.enabled)
        }
    }
}

/// A title with a line under it; a label of the catalogs loses its
/// mnemonic underscore.
struct TwoLines: View {
    let title: String
    let subtitle: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(plain(title))
            if !subtitle.isEmpty {
                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }
}

struct FormButtonView: View {
    let button: FormButton
    let act: FormAct

    var body: some View {
        let label = plain(button.label)
        Button {
            act(button.id, .press)
        } label: {
            if button.icon.isEmpty {
                Text(label)
            } else {
                Label(label, systemImage: button.icon)
                    .labelStyle(.iconOnly)
            }
        }
        .help(label)
        .disabled(!button.enabled)
    }
}

struct FormButtons: View {
    let buttons: [FormButton]
    let act: FormAct

    var body: some View {
        ForEach(Array(buttons.enumerated()), id: \.offset) { item in
            FormButtonView(button: item.element, act: act)
        }
    }
}

struct ValueRow: View {
    let title: String
    let subtitle: String
    let value: String
    let buttons: [FormButton]
    let act: FormAct

    var body: some View {
        LabeledContent {
            HStack {
                Text(value)
                    .monospacedDigit()
                    .textSelection(.enabled)
                FormButtons(buttons: buttons, act: act)
            }
        } label: {
            TwoLines(title: title, subtitle: subtitle)
        }
    }
}

struct ChoiceRow: View {
    let id: UInt32
    let title: String
    let subtitle: String
    let options: [String]
    let selected: UInt32?
    let buttons: [FormButton]
    let act: FormAct

    var body: some View {
        let selection = Binding<Int>(
            get: { selected.map { Int($0) } ?? -1 },
            set: { (index: Int) in
                if index >= 0 { act(id, .index(index: UInt32(index))) }
            })
        LabeledContent {
            HStack {
                Picker(title, selection: selection) {
                    ForEach(0..<options.count, id: \.self) { (index: Int) in
                        Text(options[index]).tag(index)
                    }
                }
                .labelsHidden()
                .fixedSize()
                FormButtons(buttons: buttons, act: act)
            }
        } label: {
            TwoLines(title: title, subtitle: subtitle)
        }
    }
}

struct RadioRow: View {
    let id: UInt32
    let options: [FormOption]
    let selected: UInt32?
    let act: FormAct

    var body: some View {
        let selection = Binding<Int>(
            get: { selected.map { Int($0) } ?? -1 },
            set: { (index: Int) in
                if index >= 0 { act(id, .index(index: UInt32(index))) }
            })
        Picker("", selection: selection) {
            ForEach(0..<options.count, id: \.self) { (index: Int) in
                TwoLines(title: options[index].title, subtitle: options[index].subtitle)
                    .tag(index)
            }
        }
        .pickerStyle(.radioGroup)
        .labelsHidden()
    }
}

struct NumberRow: View {
    let id: UInt32
    let title: String
    let subtitle: String
    let value: Int64
    let min: Int64
    let max: Int64
    let enabled: Bool
    let buttons: [FormButton]
    let act: FormAct

    var body: some View {
        LabeledContent {
            HStack {
                NumberField(id: id, title: title, value: value, min: min, max: max, act: act)
                    .disabled(!enabled)
                FormButtons(buttons: buttons, act: act)
            }
        } label: {
            TwoLines(title: title, subtitle: subtitle)
        }
    }
}

/// A whole number's box and stepper, kept within `min...max`.
struct NumberField: View {
    let id: UInt32
    let title: String
    let value: Int64
    let min: Int64
    let max: Int64
    let act: FormAct

    var body: some View {
        let number = Binding<Int64>(
            get: { value },
            set: { (new: Int64) in
                let kept = Swift.min(Swift.max(new, min), max)
                if kept != value { act(id, .number(value: kept)) }
            })
        HStack(spacing: 4) {
            TextField(title, value: number, format: IntegerFormatStyle<Int64>())
                .labelsHidden()
                .multilineTextAlignment(.trailing)
                .frame(width: max >= 100_000 ? 96 : 64)
            Stepper(title, value: number, in: min...Swift.max(min, max))
                .labelsHidden()
        }
    }
}

struct TextRow: View {
    let id: UInt32
    let title: String
    let value: String
    let multiline: Bool
    let buttons: [FormButton]
    let act: FormAct

    var body: some View {
        let text = Binding<String>(
            get: { value },
            set: { (new: String) in
                if new != value { act(id, .text(text: new)) }
            })
        if multiline {
            TextField(title, text: text, axis: .vertical)
                .labelsHidden()
                .lineLimit(4...12)
        } else {
            LabeledContent {
                HStack {
                    TextField(title, text: text)
                        .labelsHidden()
                        .frame(minWidth: 200)
                    FormButtons(buttons: buttons, act: act)
                }
            } label: {
                Text(title)
            }
        }
    }
}

/// A switch, or a check box of a list of them.
struct CheckRow: View {
    let id: UInt32
    let title: String
    let subtitle: String
    let on: Bool
    let enabled: Bool
    let switched: Bool
    let act: FormAct

    var body: some View {
        let isOn = Binding<Bool>(
            get: { on },
            set: { (new: Bool) in act(id, .on(on: new)) })
        if switched {
            Toggle(isOn: isOn) {
                TwoLines(title: title, subtitle: subtitle)
            }
            .toggleStyle(.switch)
            .disabled(!enabled)
        } else {
            Toggle(isOn: isOn) {
                TwoLines(title: title, subtitle: subtitle)
            }
            .toggleStyle(.checkbox)
            .disabled(!enabled)
        }
    }
}

/// A row of a list, with its number and Remove.
struct ItemRow: View {
    let title: String
    let subtitle: String
    let number: FormNumber?
    let remove: UInt32?
    let icon: Data?
    let act: FormAct

    var body: some View {
        LabeledContent {
            HStack {
                if let number {
                    NumberField(
                        id: number.id, title: number.label, value: number.value, min: number.min,
                        max: number.max, act: act)
                    .help(number.label)
                }
                if let remove {
                    Button(role: .destructive) {
                        act(remove, .press)
                    } label: {
                        Label(tr("Remove"), systemImage: "minus.circle")
                            .labelStyle(.iconOnly)
                    }
                    .buttonStyle(.borderless)
                    .help(tr("Remove"))
                }
            }
        } label: {
            HStack {
                if let icon, let image = NSImage(data: icon) {
                    Image(nsImage: image)
                        .interpolation(.none)
                        .frame(width: 32, height: 32)
                }
                TwoLines(title: title, subtitle: subtitle)
            }
        }
    }
}

/// Add on a list: a menu of what may be added.
struct AddRow: View {
    let id: UInt32
    let label: String
    let options: [String]
    let act: FormAct

    var body: some View {
        Menu {
            ForEach(0..<options.count, id: \.self) { (index: Int) in
                Button(options[index]) { act(id, .index(index: UInt32(index))) }
            }
        } label: {
            Label(label, systemImage: "plus")
        }
        .fixedSize()
        .disabled(options.isEmpty)
    }
}

/// The character's portrait, with Choose Portrait.
struct PortraitRow: View {
    let id: UInt32
    let path: String?
    let subtitle: String
    let race: String
    let act: FormAct
    @EnvironmentObject var app: AppModel
    @State private var choosing = false

    var body: some View {
        LabeledContent {
            HStack {
                PortraitImage(path: path)
                    .frame(width: 48, height: 64)
                Button(tr("_Choose…")) { choosing = true }
            }
        } label: {
            TwoLines(title: tr("Portrait"), subtitle: subtitle)
        }
        .sheet(isPresented: $choosing) {
            PortraitChooser(race: race) { (file: String) in
                act(id, .text(text: file))
            }
            .environmentObject(app)
        }
    }
}

/// A portrait from its file, or the placeholder.
struct PortraitImage: View {
    let path: String?

    var body: some View {
        if let path, let image = NSImage(contentsOfFile: path) {
            Image(nsImage: image)
                .resizable()
                .aspectRatio(contentMode: .fit)
        } else {
            Image(systemName: "person.crop.rectangle")
                .resizable()
                .aspectRatio(contentMode: .fit)
                .foregroundStyle(.tertiary)
        }
    }
}

/// Choose Portrait: the installation's portraits, the race's first, or
/// none, or one made from a picture of the user's.
struct PortraitChooser: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss
    let race: String
    let choose: (String) -> Void
    @State private var choices: [PortraitChoice]?
    @State private var error: String?

    var body: some View {
        VStack(spacing: 0) {
            content
                .frame(minWidth: 560, minHeight: 420)
            Divider()
            HStack {
                Button(tr("None")) { pick("") }
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
            PortraitGrid(choices: choices) { (file: String) in pick(file) }
        } else {
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private func load() async {
        guard let library = app.library else { return }
        let race = self.race
        let result = await Task.detached {
            Result { try library.portraitChoices(race: race) }
        }.value
        switch result {
        case .success(let found): choices = found
        case .failure(let failure): error = message(failure)
        }
    }

    private func pick(_ file: String) {
        choose(file)
        dismiss()
    }

    private func chooseFile() {
        guard let library = app.library else { return }
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.bmp, .jpeg, .gif, .png]
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            pick(try library.addPortrait(path: url.path))
        } catch {
            self.error = message(error)
        }
    }
}

/// The portraits in their groups, each a button that chooses it.
struct PortraitGrid: View {
    let choices: [PortraitChoice]
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
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 84))], spacing: 12) {
                        ForEach(choices.filter { $0.group == group }, id: \.path) { (choice: PortraitChoice) in
                            PortraitTile(choice: choice, pick: pick)
                        }
                    }
                }
            }
            .padding()
        }
    }
}

struct PortraitTile: View {
    let choice: PortraitChoice
    let pick: (String) -> Void

    var body: some View {
        Button {
            pick(choice.file)
        } label: {
            VStack(spacing: 4) {
                PortraitImage(path: choice.path)
                    .frame(width: 72, height: 96)
                Text(choice.file)
                    .font(.caption)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
        .buttonStyle(.plain)
        .help(choice.file)
    }
}

/// A dialog over a form, as Enter Character's Levels.
struct FormDialogView: View {
    let dialog: FormDialog
    let act: FormAct

    var body: some View {
        VStack(spacing: 0) {
            Text(dialog.title)
                .font(.headline)
                .padding(.top, 16)
            FormView(groups: dialog.groups, act: act)
                .frame(minWidth: 520, minHeight: 420)
            Divider()
            HStack {
                Spacer()
                if let accept = dialog.accept {
                    Button(plain(dialog.close.label)) { act(dialog.close.id, .press) }
                        .keyboardShortcut(.cancelAction)
                    Button(plain(accept.label)) { act(accept.id, .press) }
                        .keyboardShortcut(.defaultAction)
                        .disabled(!accept.enabled)
                } else {
                    Button(plain(dialog.close.label)) { act(dialog.close.id, .press) }
                        .keyboardShortcut(.defaultAction)
                }
            }
            .padding()
        }
    }
}
