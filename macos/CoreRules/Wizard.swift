// New Character and Enter Character, in a sheet over the main window. The
// pages and their questions come from `cr_app::create` through the
// bridge, each page described as a form; Create writes the character.

import SwiftUI

@MainActor
final class WizardModel: ObservableObject {
    let session: WizardSession
    @Published private(set) var title = ""
    @Published private(set) var pageTitle = ""
    @Published private(set) var form = FormPage(groups: [], dialog: nil)
    @Published private(set) var prompt: PromptInfo?
    @Published private(set) var depth: UInt32 = 1
    @Published private(set) var canGoOn = false
    @Published private(set) var last = false
    /// Called with the character's file once Create has written it.
    var created: (String) -> Void = { _ in }

    init(_ session: WizardSession) {
        self.session = session
        refresh()
    }

    /// Takes the session's state after an input.
    func refresh() {
        title = session.title()
        pageTitle = session.pageTitle()
        form = session.form()
        prompt = session.prompt()
        depth = session.depth()
        canGoOn = session.canGoOn()
        last = session.last()
        if let file = session.created() {
            created(file)
        }
    }

    func act(_ id: UInt32, _ value: FormValue) {
        session.act(id: id, value: value)
        refresh()
    }

    func promptOff(_ off: Bool) {
        session.promptOff(off: off)
        refresh()
    }

    func promptGroup(_ group: UInt32, _ index: UInt32) {
        session.promptGroup(group: group, index: index)
        refresh()
    }

    func next() {
        session.next()
        refresh()
    }

    func back() {
        session.back()
        refresh()
    }

    func answer(_ yes: Bool) {
        session.answer(yes: yes)
        refresh()
    }

    func answerChoice(_ index: UInt32) {
        session.answerChoice(index: index)
        refresh()
    }

    /// Whether the Levels dialog, or another over the page, is open.
    var dialogShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.form.dialog != nil },
            set: { (shown: Bool) in
                if !shown, let close = self.form.dialog?.close {
                    self.act(close.id, .press)
                }
            })
    }
}

struct WizardView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var model: WizardModel
    let close: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            WizardHeading(model: model)
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            Divider()
            WizardButtons(model: model, close: close)
        }
        .frame(minWidth: 640, idealWidth: 720, minHeight: 560, idealHeight: 680)
        .sheet(isPresented: model.dialogShown) {
            if let dialog = model.form.dialog {
                FormDialogView(dialog: dialog) { (id: UInt32, value: FormValue) in
                    model.act(id, value)
                }
                .environmentObject(app)
            }
        }
    }

    @ViewBuilder
    private var content: some View {
        if let prompt = model.prompt {
            PromptView(
                prompt: prompt,
                promptOff: { (off: Bool) in model.promptOff(off) },
                answer: { (yes: Bool) in model.answer(yes) },
                choose: { (index: UInt32) in model.answerChoice(index) },
                group: { (group: UInt32, index: UInt32) in model.promptGroup(group, index) })
        } else {
            FormView(groups: model.form.groups) { (id: UInt32, value: FormValue) in
                model.act(id, value)
            }
        }
    }
}

/// The wizard's title and the page's.
struct WizardHeading: View {
    @ObservedObject var model: WizardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(model.title)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(model.pageTitle)
                .font(.title2.bold())
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 20)
        .padding(.top, 16)
    }
}

/// Back, Cancel, and Next or Create.
struct WizardButtons: View {
    @ObservedObject var model: WizardModel
    let close: () -> Void

    var body: some View {
        HStack {
            if model.depth > 1 {
                Button(tr("Back")) { model.back() }
                    .disabled(model.prompt != nil)
            }
            Spacer()
            Button(tr("_Cancel"), role: .cancel, action: close)
                .keyboardShortcut(.cancelAction)
            Button(model.last ? tr("C_reate") : tr("_Next")) { model.next() }
                .keyboardShortcut(.defaultAction)
                .disabled(!model.canGoOn)
        }
        .padding()
    }
}

/// A question or message a form waits on, drawn in the form's place: the
/// Core Rules Vs Player's Option window has a check box, Add One Point
/// one button per ability, and All Classes… a menu per group, which an
/// alert cannot hold.
struct PromptView: View {
    let prompt: PromptInfo
    let promptOff: (Bool) -> Void
    let answer: (Bool) -> Void
    var choose: (UInt32) -> Void = { _ in }
    var group: (UInt32, UInt32) -> Void = { _, _ in }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(prompt.title).font(.title2.bold())
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    Text(prompt.text)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .textSelection(.enabled)
                    PromptControls(prompt: prompt, choose: choose, group: group)
                }
            }
            if case .coreVsOption(let off) = prompt.kind {
                Toggle(tr("_Do not show this window again"), isOn: Binding(get: { off }, set: promptOff))
            }
            PromptButtons(prompt: prompt, answer: answer)
        }
        .padding(20)
    }
}

/// What a prompt asks besides its buttons: a choice of abilities, or
/// one class of each group.
struct PromptControls: View {
    let prompt: PromptInfo
    let choose: (UInt32) -> Void
    let group: (UInt32, UInt32) -> Void

    var body: some View {
        switch prompt.kind {
        case .raiseOne(let labels):
            VStack(alignment: .leading, spacing: 6) {
                ForEach(Array(labels.enumerated()), id: \.offset) { item in
                    Button(item.element) { choose(UInt32(item.offset)) }
                        .frame(maxWidth: 320, alignment: .leading)
                }
            }
        case .allClasses(let groups):
            Form {
                ForEach(Array(groups.enumerated()), id: \.offset) { item in
                    if item.element.classes.count > 1 {
                        ClassGroupPicker(index: UInt32(item.offset), group: item.element, choose: group)
                    }
                }
            }
            .formStyle(.grouped)
            .frame(minHeight: 240)
        default:
            EmptyView()
        }
    }
}

struct ClassGroupPicker: View {
    let index: UInt32
    let group: PromptGroup
    let choose: (UInt32, UInt32) -> Void

    var body: some View {
        let selection = Binding<Int>(
            get: { Int(group.chosen) },
            set: { (value: Int) in choose(index, UInt32(value)) })
        Picker(group.name, selection: selection) {
            ForEach(0..<group.classes.count, id: \.self) { (i: Int) in
                Text(group.classes[i]).tag(i)
            }
        }
    }
}

/// A prompt's buttons; Add One Point has none, its choices answer it.
struct PromptButtons: View {
    let prompt: PromptInfo
    let answer: (Bool) -> Void

    var body: some View {
        HStack {
            Spacer()
            if let no = prompt.noLabel {
                Button(plain(no)) { answer(false) }
                    .keyboardShortcut(.cancelAction)
            }
            if !prompt.yesLabel.isEmpty {
                Button(plain(prompt.yesLabel)) { answer(true) }
                    .keyboardShortcut(.defaultAction)
            }
        }
    }
}
