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
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            Divider()
            buttons
        }
        .frame(minWidth: 560, minHeight: 460)
    }

    @ViewBuilder
    private var content: some View {
        if let prompt = model.prompt {
            PromptView(
                prompt: prompt,
                promptOff: { (off: Bool) in model.promptOff(off) },
                answer: { (yes: Bool) in model.answer(yes) })
        } else {
            page
        }
    }

    private var buttons: some View {
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

    /// Whether the Mac app draws the page in view yet.
    private var drawn: Bool {
        if case .setting = model.page { return true }
        return false
    }

    @ViewBuilder
    private var page: some View {
        switch model.page {
        case .setting(let expansion, let skillsAndPowers, let rulesText, let settings, let setting):
            SettingPage(
                model: model, expansion: expansion, skillsAndPowers: skillsAndPowers,
                rulesText: rulesText, settings: settings, setting: setting)
        case .other(let title):
            ContentUnavailableView {
                Label(title, systemImage: "hammer")
            } description: {
                Text(tr("The Mac version does not create characters yet"))
            }
        }
    }
}

/// The first page: the rules and the campaign setting.
struct SettingPage: View {
    @ObservedObject var model: WizardModel
    let expansion: Bool
    let skillsAndPowers: Bool
    let rulesText: String
    let settings: [SettingInfo]
    let setting: UInt32

    private var rules: Binding<Int> {
        Binding<Int>(
            get: { skillsAndPowers ? 1 : 0 },
            set: { (index: Int) in model.chooseRules(UInt32(index)) })
    }

    private var chosen: Binding<Int> {
        Binding<Int>(
            get: { Int(setting) },
            set: { (index: Int) in model.chooseSetting(UInt32(index)) })
    }

    var body: some View {
        Form {
            if expansion {
                Section {
                    Picker(tr("Rules"), selection: rules) {
                        Text(tr("Core Rules")).tag(0)
                        Text(tr("Player's Option")).tag(1)
                    }
                    .pickerStyle(.segmented)
                } footer: {
                    Text(rulesText).foregroundStyle(.secondary)
                }
            }
            Section(tr("Campaign Setting")) {
                Picker(tr("Campaign Setting"), selection: chosen) {
                    ForEach(0..<settings.count, id: \.self) { (index: Int) in
                        SettingRow(setting: settings[index]).tag(index)
                    }
                }
                .pickerStyle(.radioGroup)
                .labelsHidden()
            }
        }
        .formStyle(.grouped)
    }
}

struct SettingRow: View {
    let setting: SettingInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(setting.name)
            Text(setting.description)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
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
