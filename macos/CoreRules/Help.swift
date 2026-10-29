// Help, in a window of its own: the book chosen, its contents or what a
// search finds in the sidebar, and the topic in a web view with its links
// and popups, with Back and Print, as the other apps' help pages have
// them. The topics come from the installation's WinHelp files through the
// bridge.

import AppKit
import SwiftUI
import WebKit

@MainActor
final class HelpModel: ObservableObject {
    let session: HelpSession
    private weak var app: AppModel?
    let books = helpBooks()
    @Published private(set) var rows: [HelpRow] = []
    @Published private(set) var row: UInt32?
    @Published private(set) var listed = ""
    @Published private(set) var listedTitle = ""
    @Published private(set) var query = ""
    @Published private(set) var topic: HelpTopic?
    @Published private(set) var canGoBack = false
    /// Prints the next topic once it is drawn, as Print on a monster asks.
    var printNext = false
    /// The view the topic is drawn in, which popups and Print need.
    weak var webView: WKWebView?

    init(_ session: HelpSession, app: AppModel) {
        self.session = session
        self.app = app
    }

    func open(_ spec: String) throws {
        try session.open(spec: spec)
        refresh()
    }

    private func refresh() {
        rows = session.rows()
        row = session.row()
        listed = session.listed()
        listedTitle = session.listedTitle()
        query = session.query()
        topic = session.topic()
        canGoBack = session.canGoBack()
    }

    func search(_ text: String) {
        session.search(query: text)
        refresh()
    }

    func choose(_ index: UInt32) {
        do {
            try session.chooseRow(index: index)
        } catch {
            app?.fail(tr("Help"), error)
        }
        refresh()
    }

    /// A book of the book selector, by its file.
    func openBook(_ file: String) {
        let book = books.first { $0.file.lowercased() == file }?.file ?? file
        do {
            try open(book)
        } catch {
            app?.fail(tr("Help"), error)
        }
    }

    func back() {
        do {
            try session.back()
        } catch {
            app?.fail(tr("Help"), error)
        }
        refresh()
    }

    /// What a link the page posted leads to.
    func follow(_ id: String) {
        switch session.link(id: id) {
        case .shown:
            refresh()
        case .popup(let html):
            webView?.evaluateJavaScript("showPopup(\(jsString(text: html)))")
        case .web(let uri):
            if let url = URL(string: uri) {
                NSWorkspace.shared.open(url)
            }
        case .missing(let message):
            app?.notice = Notice(title: tr("Help"), text: message)
        case .nothing:
            break
        }
    }

    func print() {
        guard let webView else { return }
        let info = NSPrintInfo.shared.copy() as? NSPrintInfo ?? NSPrintInfo.shared
        info.horizontalPagination = .fit
        info.verticalPagination = .automatic
        let operation = webView.printOperation(with: info)
        operation.view?.frame = webView.bounds
        if let window = webView.window {
            operation.runModal(for: window, delegate: nil, didRun: nil, contextInfo: nil)
        } else {
            operation.run()
        }
    }

    /// The topic was drawn.
    func drawn() {
        if printNext {
            printNext = false
            print()
        }
    }
}

/// The help's window.
struct HelpWindow: View {
    @EnvironmentObject var app: AppModel

    var body: some View {
        if let help = app.help {
            HelpView(help: help)
        } else {
            ContentUnavailableView(
                tr("Open a Core Rules folder to read its help"), systemImage: "questionmark.circle")
        }
    }
}

struct HelpView: View {
    @ObservedObject var help: HelpModel

    var body: some View {
        NavigationSplitView {
            HelpSidebar(help: help)
                .navigationSplitViewColumnWidth(min: 200, ideal: 260, max: 400)
        } detail: {
            HelpDetail(help: help)
        }
        .frame(minWidth: 720, minHeight: 480)
        .navigationTitle(help.topic?.title ?? tr("Help"))
        .navigationSubtitle(help.topic?.fileTitle ?? "")
        .toolbar { HelpTools(help: help) }
    }
}

struct HelpSidebar: View {
    @ObservedObject var help: HelpModel

    var body: some View {
        let selection = Binding<UInt32?>(
            get: { help.row },
            set: { (value: UInt32?) in
                if let value { help.choose(value) }
            })
        let search = Binding<String>(
            get: { help.query },
            set: { (value: String) in help.search(value) })
        List(selection: selection) {
            ForEach(help.rows, id: \.index) { (row: HelpRow) in
                HelpRowView(row: row)
                    .tag(row.index)
            }
        }
        .searchable(text: search, placement: .sidebar, prompt: tr("Search"))
        .overlay {
            if help.rows.isEmpty && !help.query.isEmpty {
                ContentUnavailableView.search(text: help.query)
            }
        }
    }
}

struct HelpRowView: View {
    let row: HelpRow

    var body: some View {
        Label {
            Text(row.title)
                .fontWeight(row.topic ? .regular : .semibold)
        } icon: {
            Image(systemName: row.topic ? "doc.text" : "book.closed")
                .foregroundStyle(row.topic ? Color.secondary : Color.accentColor)
        }
        .padding(.leading, CGFloat(max(Int(row.level), 1) - 1) * 14)
    }
}

struct HelpDetail: View {
    @ObservedObject var help: HelpModel

    var body: some View {
        if let topic = help.topic {
            HelpWebView(html: topic.html, help: help)
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "questionmark.circle")
        }
    }
}

struct HelpTools: ToolbarContent {
    @ObservedObject var help: HelpModel

    var body: some ToolbarContent {
        ToolbarItem(placement: .navigation) {
            Button { help.back() } label: {
                Label(tr("Back"), systemImage: "chevron.backward")
            }
            .help(tr("Back"))
            .disabled(!help.canGoBack)
        }
        ToolbarItemGroup {
            BookPicker(help: help)
            Button { help.print() } label: {
                Label(more(tr("_Print…")), systemImage: "printer")
            }
            .help(more(tr("_Print…")))
            .disabled(help.topic == nil)
        }
    }
}

/// The book selector: the program help, the Online Manual and the Tome
/// of Help, and the file listed when it is none of them.
struct BookPicker: View {
    @ObservedObject var help: HelpModel

    var body: some View {
        let selection = Binding<String>(
            get: { help.listed },
            set: { (value: String) in help.openBook(value) })
        let other = !help.books.contains { $0.file.lowercased() == help.listed }
        Picker(tr("Book"), selection: selection) {
            ForEach(help.books, id: \.file) { (book: HelpBook) in
                Text(plain(book.label)).tag(book.file.lowercased())
            }
            if other && !help.listed.isEmpty {
                Divider()
                Text(help.listedTitle).tag(help.listed)
            }
        }
        .pickerStyle(.menu)
        .help(tr("Book"))
    }
}

/// The topic's page. Its links post their index to the `corerules`
/// handler, which the help follows.
struct HelpWebView: NSViewRepresentable {
    let html: String
    let help: HelpModel

    func makeCoordinator() -> Coordinator {
        Coordinator(help: help)
    }

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.userContentController.add(context.coordinator, name: "corerules")
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = context.coordinator
        view.setValue(false, forKey: "drawsBackground")
        help.webView = view
        return view
    }

    func updateNSView(_ view: WKWebView, context: Context) {
        context.coordinator.help = help
        help.webView = view
        if context.coordinator.loaded != html {
            context.coordinator.loaded = html
            view.loadHTMLString(html, baseURL: nil)
        }
    }

    static func dismantleNSView(_ view: WKWebView, coordinator: Coordinator) {
        view.configuration.userContentController.removeScriptMessageHandler(forName: "corerules")
    }

    @MainActor
    final class Coordinator: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
        weak var help: HelpModel?
        var loaded = ""

        init(help: HelpModel) {
            self.help = help
        }

        func userContentController(
            _ userContentController: WKUserContentController, didReceive message: WKScriptMessage
        ) {
            if let id = message.body as? String {
                help?.follow(id)
            }
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            help?.drawn()
        }
    }
}
