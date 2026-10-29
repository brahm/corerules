// About Core Rules (Core Rules › About Core Rules): the app, its version
// and what it is, then What's New, Credits, Legal and Troubleshooting, as
// the GNOME app's About window and the Windows app's About have them.

import AppKit
import SwiftUI

struct AboutView: View {
    @EnvironmentObject var model: AppModel
    private let info = about()

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                AboutHeading(info: info)
                Form {
                    AboutWhatsNew(info: info)
                    AboutCredits(info: info, original: model.library?.original())
                    AboutLegal(info: info, original: model.library?.original())
                    AboutTroubleshooting(details: details)
                }
                .formStyle(.grouped)
            }
            .padding(.top, 24)
        }
        .frame(width: 520)
        .frame(minHeight: 420, idealHeight: 640)
    }

    /// What a bug report needs.
    private var details: String {
        debugInfo(
            data: model.library?.root(),
            macos: ProcessInfo.processInfo.operatingSystemVersionString)
    }
}

/// The icon, the name and the version, and what the app is.
struct AboutHeading: View {
    let info: AboutInfo

    var body: some View {
        VStack(spacing: 8) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .frame(width: 96, height: 96)
            Text(tr("Core Rules"))
                .font(.title.bold())
            Text(tr("Version {version}", ["version": info.version]))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
            Text("\(info.copyright) · MIT")
                .font(.caption)
                .foregroundStyle(.secondary)
            ForEach(Array(info.paragraphs.enumerated()), id: \.offset) { item in
                Text(item.element)
                    .fixedSize(horizontal: false, vertical: true)
                    .multilineTextAlignment(.center)
            }
            .padding(.horizontal, 32)
            if let url = URL(string: info.website) {
                Link(url.host() ?? info.website, destination: url)
            }
        }
    }
}

struct AboutWhatsNew: View {
    let info: AboutInfo

    var body: some View {
        if !info.releaseNotes.isEmpty {
            Section {
                DisclosureGroup(tr("What's New in {version}", ["version": info.version])) {
                    ForEach(Array(info.releaseNotes.enumerated()), id: \.offset) { item in
                        Text(item.element)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
            }
        }
    }
}

/// Who made the app, and the original program's Credits window.
struct AboutCredits: View {
    let info: AboutInfo
    let original: OriginalInfo?

    var body: some View {
        Section {
            DisclosureGroup(tr("Credits")) {
                Credit(heading: tr("Developer")) {
                    Link(info.developer, destination: URL(string: info.developerUrl)!)
                }
                Credit(heading: tr("Assistant")) {
                    Link(info.assistant, destination: URL(string: info.assistantUrl)!)
                }
                if let translators = info.translators {
                    Credit(heading: tr("Translators")) { Text(translators) }
                }
                if let original, let title = original.creditsTitle {
                    Credit(heading: title) {
                        Text(original.credits.joined(separator: "\n"))
                    }
                }
            }
        }
    }
}

/// The notice on the original's content, the original program's About
/// box and license, and the license of the icons the sidebar draws.
struct AboutLegal: View {
    let info: AboutInfo
    let original: OriginalInfo?

    var body: some View {
        Section {
            DisclosureGroup(tr("Legal")) {
                Credit(heading: tr("AD&D Core Rules")) { Text(info.legal) }
                if let original, let title = original.aboutTitle {
                    Credit(heading: title) { Text(original.aboutText) }
                }
                if let icons = iconsLicense {
                    Credit(heading: "Material Design Icons") { Text(icons) }
                }
            }
        }
    }

    /// Shipped with the app, as the sword the sidebar draws is theirs.
    private var iconsLicense: String? {
        guard
            let url = Bundle.main.url(
                forResource: "MaterialDesignIcons-LICENSE", withExtension: "txt"),
            let text = try? String(contentsOf: url, encoding: .utf8)
        else { return nil }
        return text.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

/// What a bug report needs, to copy, and where reports go.
struct AboutTroubleshooting: View {
    let details: String

    var body: some View {
        Section {
            DisclosureGroup(tr("Troubleshooting")) {
                Text(
                    tr(
                        "A bug report helps most with these details and what you did when it happened."
                    )
                )
                .fixedSize(horizontal: false, vertical: true)
                Text(details)
                    .font(.callout.monospaced())
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                HStack {
                    Button(tr("Copy Details")) {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(details, forType: .string)
                    }
                    Spacer()
                    if let url = URL(string: about().issues) {
                        Link(tr("Report an Issue"), destination: url)
                    }
                }
            }
        }
    }
}

/// A credit: who or what, under its heading.
struct Credit<Content: View>: View {
    let heading: String
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(heading)
                .font(.caption)
                .foregroundStyle(.secondary)
            content()
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
