// Manage Parties, in a sheet over the main window: the lists of
// `cr_app::parties` described as a form through the bridge, with Group
// Encounter's dialog and Delete Members.

import SwiftUI

@MainActor
final class PartiesModel: ObservableObject {
    let session: PartiesSession
    @Published private(set) var form = FormPage(groups: [], dialog: nil)
    @Published private(set) var party = ""
    @Published private(set) var hasMembers = false
    @Published var deleting = false
    @Published var error: String?
    /// Called with what Group Encounter's OK asked for.
    var grouped: (GroupRequest) -> Void = { _ in }
    /// Called with what Delete Members did, or why it could not.
    var deleted: (Result<String, Error>) -> Void = { _ in }

    init(_ session: PartiesSession) {
        self.session = session
        refresh()
    }

    func refresh() {
        form = session.form()
        party = session.party()
        hasMembers = session.hasMembers()
        if let failed = session.takeError() {
            error = failed
        }
        if let group = session.takeGroup() {
            grouped(group)
        }
    }

    func act(_ id: UInt32, _ value: FormValue) {
        session.act(id: id, value: value)
        refresh()
    }

    func delete() {
        deleted(Result { try session.deleteMembers() })
    }

    /// Whether Group Encounter's dialog is open.
    var dialogShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.form.dialog != nil },
            set: { (shown: Bool) in
                if !shown, let close = self.form.dialog?.close {
                    self.act(close.id, .press)
                }
            })
    }

    var errorShown: Binding<Bool> {
        Binding<Bool>(
            get: { self.error != nil },
            set: { (shown: Bool) in if !shown { self.error = nil } })
    }
}

struct PartiesView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var model: PartiesModel
    let close: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            Text(tr("Manage Parties"))
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
                Button(tr("Delete Members…"), role: .destructive) { model.deleting = true }
                    .disabled(model.party.isEmpty || !model.hasMembers)
                Spacer()
                Button(tr("_Done"), action: close)
                    .keyboardShortcut(.defaultAction)
            }
            .padding()
        }
        .frame(minWidth: 560, idealWidth: 640, minHeight: 560, idealHeight: 680)
        .sheet(isPresented: model.dialogShown) {
            if let dialog = model.form.dialog {
                FormDialogView(dialog: dialog) { (id: UInt32, value: FormValue) in
                    model.act(id, value)
                }
                .environmentObject(app)
            }
        }
        .alert(tr("Delete Party Members?"), isPresented: $model.deleting) {
            Button(tr("_Delete"), role: .destructive) { model.delete() }
            Button(tr("_Cancel"), role: .cancel) {}
        } message: {
            Text(
                tr(
                    "Every character in “{party}” will be deleted, and the party with them.",
                    ["party": model.party]))
        }
        .alert(tr("Manage Parties"), isPresented: model.errorShown) {
            Button(tr("_OK")) {}
        } message: {
            Text(model.error ?? "")
        }
    }
}
