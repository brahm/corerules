// Import Add-ons…: what a folder of add-ons holds, in a sheet over the
// main window, and Import to bring it all in at once. What is read, said
// and imported comes from `cr_app::addons` through the bridge.

import SwiftUI

@MainActor
final class AddonsModel: ObservableObject {
    let session: AddonsSession
    /// The import runs.
    @Published private(set) var importing = false

    init(_ session: AddonsSession) {
        self.session = session
    }

    /// Imports every add-on off the main thread, then hands over what it
    /// did.
    func importAll(done: @escaping (Result<ImportReport, Error>) -> Void) {
        guard !importing else { return }
        importing = true
        let session = self.session
        Task {
            let result = await Task.detached { () -> Result<ImportReport, Error> in
                Result { try session.`import`() }
            }.value
            importing = false
            done(result)
        }
    }
}

struct AddonsView: View {
    @ObservedObject var model: AddonsModel
    let close: () -> Void
    let imported: (ImportReport) -> Void
    let failed: (Error) -> Void

    var body: some View {
        VStack(spacing: 0) {
            ScrollView {
                Text(model.session.text())
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding()
            }
            Divider()
            buttons
        }
        .frame(minWidth: 520, minHeight: 480)
        .navigationTitle(model.session.title())
        .interactiveDismissDisabled(model.importing)
    }

    private func importAll() {
        model.importAll { (done: Result<ImportReport, Error>) in
            switch done {
            case .success(let report): imported(report)
            case .failure(let error): failed(error)
            }
        }
    }

    private var buttons: some View {
        HStack {
            if model.importing {
                ProgressView()
                    .controlSize(.small)
                Text(tr("Importing the add-ons…")).foregroundStyle(.secondary)
            } else {
                Text(model.session.title()).font(.headline)
            }
            Spacer()
            Button(tr("_Cancel"), role: .cancel, action: close)
                .keyboardShortcut(.cancelAction)
                .disabled(model.importing)
            Button(tr("_Import"), action: importAll)
                .keyboardShortcut(.defaultAction)
                .disabled(!model.session.canImport() || model.importing)
        }
        .padding()
    }
}
