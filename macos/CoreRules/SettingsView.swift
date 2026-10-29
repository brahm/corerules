// Settings (Core Rules › Settings…): the interface language, New
// Character's message and where the data is, as the other apps' settings
// have them.

import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var model: AppModel
    /// The language chosen, "" for the system's.
    @State private var language = Prefs.language ?? ""
    @State private var coreVsOption = Prefs.coreVsOptionShown
    /// The language the interface started in.
    private let started = Prefs.startLanguage()

    var body: some View {
        Form {
            interface
            creation
            installation
        }
        .formStyle(.grouped)
        .frame(width: 520)
        .fixedSize(horizontal: false, vertical: true)
        .onAppear { coreVsOption = Prefs.coreVsOptionShown }
    }

    private var interface: some View {
        Section(tr("Interface")) {
            Picker(selection: $language) {
                Text(tr("System Language ({language})", ["language": systemLanguage])).tag("")
                ForEach(languages(), id: \.code) { (l: Language) in
                    Text(l.nativeName).tag(l.code)
                }
            } label: {
                Text(tr("Language"))
                Text(tr("Game data from the original files stays in English"))
            }
            .onChange(of: language) { (_: String, value: String) in
                Prefs.language = value.isEmpty ? nil : value
            }
            if Prefs.startLanguage() != started {
                Label(tr("Restart Core Rules to use the new language"), systemImage: "arrow.clockwise")
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var creation: some View {
        Section(tr("Character Creation")) {
            Toggle(isOn: $coreVsOption) {
                Text(tr("Core Rules Vs Player's Option Message"))
                Text(tr("Shown in New Character after the rules are chosen"))
            }
            .onChange(of: coreVsOption) { (_: Bool, value: Bool) in
                Prefs.coreVsOptionShown = value
            }
        }
    }

    private var installation: some View {
        Section(tr("Installation")) {
            LabeledContent(tr("Installation")) {
                Text(model.library?.root() ?? tr("No Core Rules Installation"))
                    .textSelection(.enabled)
                    .lineLimit(2)
                    .truncationMode(.middle)
            }
            LabeledContent(tr("Data folder"), value: dataPackageShown())
            Button(more(tr("_Open Folder…"))) { model.perform(Action.OPEN) }
        }
    }

    /// The system's language, by its name in itself.
    private var systemLanguage: String {
        let code = languageFor(preferred: Locale.preferredLanguages)
        return languages().first { $0.code == code }?.nativeName ?? code
    }
}
