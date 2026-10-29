// New NPC and New NPC Group, in a sheet over the main window. The form
// comes from `cr_app::npc` through the bridge; Create makes and saves the
// NPCs off the main thread.

import SwiftUI

@MainActor
final class NpcModel: ObservableObject {
    let session: NpcSession
    let title: String
    @Published private(set) var form = FormPage(groups: [], dialog: nil)
    @Published private(set) var creating = false
    /// Called with what Create made, once the NPCs are saved.
    var created: (NpcsCreated) -> Void = { _ in }
    var failed: (Error) -> Void = { _ in }

    init(_ session: NpcSession) {
        self.session = session
        title = session.title()
        refresh()
    }

    func refresh() {
        form = session.form()
    }

    func act(_ id: UInt32, _ value: FormValue) {
        session.act(id: id, value: value)
        refresh()
    }

    func create() {
        guard !creating else { return }
        creating = true
        let session = self.session
        Task {
            let made = await Task.detached { () -> Result<NpcsCreated, Error> in
                Result { try session.create() }
            }.value
            creating = false
            switch made {
            case .success(let npcs): created(npcs)
            case .failure(let error):
                refresh()
                failed(error)
            }
        }
    }
}

struct NpcView: View {
    @ObservedObject var model: NpcModel
    let close: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            Text(model.title)
                .font(.title2.bold())
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 20)
                .padding(.top, 16)
            FormView(groups: model.form.groups) { (id: UInt32, value: FormValue) in
                model.act(id, value)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            Divider()
            HStack {
                if model.creating {
                    ProgressView()
                        .controlSize(.small)
                }
                Spacer()
                Button(tr("_Cancel"), role: .cancel, action: close)
                    .keyboardShortcut(.cancelAction)
                Button(plain(tr("_Create"))) { model.create() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(model.creating)
            }
            .padding()
        }
        .frame(minWidth: 560, idealWidth: 640, minHeight: 560, idealHeight: 680)
    }
}
