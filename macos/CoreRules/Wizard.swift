// New Character, in a sheet over the main window. The pages and their
// questions come from `cr_app::create` through the bridge; the prototype
// draws the first page, and names the pages after it.

import SwiftUI

@MainActor
final class WizardModel: ObservableObject {
    let session: WizardSession
    @Published private(set) var page: WizardPage
    @Published private(set) var prompt: PromptInfo?
    @Published private(set) var depth: UInt32 = 1
    @Published private(set) var canGoOn = false

    init(_ session: WizardSession) {
        self.session = session
        page = session.page()
        refresh()
    }

    /// Takes the session's state after an input.
    func refresh() {
        page = session.page()
        prompt = session.prompt()
        depth = session.depth()
        canGoOn = session.canGoOn()
    }

    func chooseRules(_ index: UInt32) {
        session.chooseRules(index: index)
        refresh()
    }

    func chooseSetting(_ index: UInt32) {
        session.chooseSetting(index: index)
        refresh()
    }

    func promptOff(_ off: Bool) {
        session.promptOff(off: off)
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
}

struct WizardView: View {
    @ObservedObject var model: WizardModel
    let close: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            Group {
                if let prompt = model.prompt {
                    PromptView(prompt: prompt, promptOff: { model.promptOff($0) }, answer: { model.answer($0) })
                } else {
                    page
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            Divider()
            HStack {
                if model.depth > 1 {
                    Button(tr("Back")) { model.back() }
                        .disabled(model.prompt != nil)
                }
                Spacer()
                Button(tr("_Cancel"), role: .cancel, action: close)
                    .keyboardShortcut(.cancelAction)
                Button(tr("_Next")) { model.next() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(!model.canGoOn || !drawn)
            }
            .padding()
        }
        .frame(minWidth: 560, minHeight: 460)
        .navigationTitle(model.session.title())
    }

    /// Whether the Mac app draws the page in view yet.
    private var drawn: Bool {
        if case .setting = model.page { return true }
        return false
    }

    @ViewBuilder
    private var page: some View {
        switch model.page {
        case .setting(let expansion, let skillsAndPowers, let rulesText, let settings, let setting):
            Form {
                if expansion {
                    Section {
                        Picker(
                            tr("Rules"),
                            selection: Binding(
                                get: { skillsAndPowers ? 1 : 0 },
                                set: { model.chooseRules(UInt32($0)) })
                        ) {
                            Text(tr("Core Rules")).tag(0)
                            Text(tr("Player's Option")).tag(1)
                        }
                        .pickerStyle(.segmented)
                    } footer: {
                        Text(rulesText).foregroundStyle(.secondary)
                    }
                }
                Section(tr("Campaign Setting")) {
                    Picker(
                        tr("Campaign Setting"),
                        selection: Binding(
                            get: { Int(setting) },
                            set: { model.chooseSetting(UInt32($0)) })
                    ) {
                        ForEach(Array(settings.enumerated()), id: \.offset) { index, s in
                            VStack(alignment: .leading, spacing: 2) {
                                Text(s.name)
                                Text(s.description)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .fixedSize(horizontal: false, vertical: true)
                            }
                            .tag(index)
                        }
                    }
                    .pickerStyle(.radioGroup)
                    .labelsHidden()
                }
            }
            .formStyle(.grouped)
        case .other(let title):
            ContentUnavailableView {
                Label(title, systemImage: "hammer")
            } description: {
                Text(tr("The Mac version does not create characters yet"))
            }
        }
    }
}

/// A question or message a form waits on, drawn in the form's place: the
/// Core Rules Vs Player's Option window has a check box, which an alert
/// cannot hold.
struct PromptView: View {
    let prompt: PromptInfo
    let promptOff: (Bool) -> Void
    let answer: (Bool) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(prompt.title).font(.title2.bold())
            ScrollView {
                Text(prompt.text)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .textSelection(.enabled)
            }
            if case .coreVsOption(let off) = prompt.kind {
                Toggle(tr("_Do not show this window again"), isOn: Binding(get: { off }, set: promptOff))
            }
            HStack {
                Spacer()
                if let no = prompt.noLabel {
                    Button(plain(no)) { answer(false) }
                        .keyboardShortcut(.cancelAction)
                }
                Button(plain(prompt.yesLabel)) { answer(true) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
    }
}
