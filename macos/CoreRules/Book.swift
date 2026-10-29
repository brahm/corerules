// A rule book of the Books section in a window of its own, as the GNOME
// app's reader shows it: the contents in the sidebar beside the text,
// Find with its matches, zoom, and the place the reader left kept between
// sessions. The text is the web page `cr-app` makes of the book, in a
// WKWebView; the bridge says which script the page runs next.

import AppKit
import SwiftUI
import WebKit

@MainActor
final class BookModel: ObservableObject {
    /// The book as it was asked for.
    let name: String
    @Published private(set) var session: BookSession?
    @Published private(set) var error: String?
    @Published private(set) var headings: [BookHeading] = []
    @Published private(set) var heading: UInt32?
    @Published private(set) var zoom = 1.0
    @Published var contents: NavigationSplitViewVisibility = .all
    @Published var finding: Bool
    /// Counts the times Find was asked for, to put the focus in its box.
    @Published private(set) var findAsked = 0
    @Published private(set) var query: String
    @Published private(set) var found = ""
    @Published private(set) var hasMatches = false
    /// The view the book is drawn in, which runs the scripts.
    weak var webView: WKWebView?
    /// Closes the book's window, and opens the help on the reader; its
    /// view sets them.
    var close: () -> Void = {}
    var help: () -> Void = {}

    init(name: String, find: String?, library: Library) {
        self.name = name
        finding = find != nil
        findAsked = find != nil ? 1 : 0
        query = find ?? ""
        let state = Prefs.readerState
        Task {
            let opened = await Task.detached {
                Result { try library.openBook(name: name, state: state, find: find) }
            }.value
            self.opened(opened)
        }
    }

    private func opened(_ result: Result<BookSession, Error>) {
        switch result {
        case .success(let session):
            self.session = session
            headings = session.headings()
            heading = session.heading()
            zoom = session.zoom()
        case .failure(let error):
            self.error = message(error)
        }
    }

    var title: String {
        session?.title() ?? name
    }

    private func run(_ scripts: [String]) {
        for script in scripts {
            webView?.evaluateJavaScript(script)
        }
        refreshFound()
    }

    private func refreshFound() {
        found = session?.found() ?? ""
        hasMatches = session?.hasMatches() ?? false
    }

    /// What the page posted.
    func posted(_ message: String) {
        guard let session else { return }
        switch session.posted(message: message) {
        case .ready(let scripts):
            run(scripts)
        case .moved(let heading):
            self.heading = heading
        case .key(let key):
            switch key {
            case "find": perform(Action.BOOK_FIND)
            case "zoom-in": perform(Action.BOOK_ZOOM_IN)
            case "zoom-out": perform(Action.BOOK_ZOOM_OUT)
            case "zoom-reset": perform(Action.BOOK_ZOOM_RESET)
            case "contents": perform(Action.BOOK_CONTENTS)
            case "close": perform(Action.BOOK_CLOSE)
            case "help": help()
            default: break
            }
        case .nothing:
            break
        }
    }

    /// Goes to heading `index` of the contents.
    func go(_ index: UInt32) {
        guard let script = session?.go(index: index) else { return }
        heading = index
        run([script])
    }

    func search(_ text: String) {
        guard let session else { return }
        query = text
        run(session.search(query: text))
    }

    /// Shows the match after (1) or before (-1) the one shown.
    func step(_ direction: Int32) {
        guard let script = session?.stepMatch(direction: direction) else { return }
        run([script])
    }

    func closeFind() {
        finding = false
        search("")
    }

    func enabled(_ action: Action) -> Bool {
        switch action {
        case Action.BOOK_ZOOM_RESET:
            return session != nil && abs(zoom - 1) > 0.001
        case Action.BOOK_CONTENTS:
            return !headings.isEmpty
        case Action.BOOK_CLOSE:
            return true
        default:
            return session != nil
        }
    }

    func perform(_ action: Action) {
        guard enabled(action) else { return }
        switch action {
        case Action.BOOK_FIND:
            finding = true
            findAsked += 1
        case Action.BOOK_ZOOM_IN: zoomBy(1)
        case Action.BOOK_ZOOM_OUT: zoomBy(-1)
        case Action.BOOK_ZOOM_RESET: zoomBy(0)
        case Action.BOOK_CONTENTS:
            contents = contents == .detailOnly ? .all : .detailOnly
        case Action.BOOK_CLOSE: close()
        default: break
        }
    }

    private func zoomBy(_ direction: Int32) {
        guard let session, let script = session.stepZoom(direction: direction) else { return }
        zoom = session.zoom()
        run([script])
    }
}

/// The book's window.
struct BookWindow: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss
    let id: UUID

    var body: some View {
        if let book = app.book(id) {
            BookView(book: book)
                .onAppear {
                    book.close = { dismiss() }
                    book.help = { app.booksHelp() }
                }
                .onDisappear { app.closeBook(id) }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "book")
        }
    }
}

struct BookView: View {
    @ObservedObject var book: BookModel

    var body: some View {
        NavigationSplitView(columnVisibility: $book.contents) {
            BookContents(book: book)
                .navigationSplitViewColumnWidth(min: 200, ideal: 280, max: 420)
        } detail: {
            BookText(book: book)
        }
        .toolbar(removing: .sidebarToggle)
        .toolbar { BookTools(book: book) }
        .frame(minWidth: 760, minHeight: 520)
        .navigationTitle(book.title)
        .navigationSubtitle(book.session?.category() ?? "")
        .focusedSceneValue(\.book, book)
    }
}

/// The book's headings, the one the top of the text is under selected
/// and scrolled to.
struct BookContents: View {
    @ObservedObject var book: BookModel

    var body: some View {
        let selection = Binding<UInt32?>(
            get: { book.heading },
            set: { (value: UInt32?) in
                if let value { book.go(value) }
            })
        ScrollViewReader { (proxy: ScrollViewProxy) in
            List(selection: selection) {
                ForEach(book.headings, id: \.index) { (heading: BookHeading) in
                    Text(heading.title)
                        .fontWeight(heading.level == 0 ? .semibold : .regular)
                        .lineLimit(1)
                        .help(heading.title)
                        .padding(.leading, CGFloat(heading.level) * 12)
                        .tag(heading.index)
                }
            }
            .onChange(of: book.heading) { (_: UInt32?, value: UInt32?) in
                if let value { proxy.scrollTo(value) }
            }
        }
    }
}

struct BookText: View {
    @ObservedObject var book: BookModel

    var body: some View {
        VStack(spacing: 0) {
            if book.finding {
                FindBar(book: book)
                Divider()
            }
            if let error = book.error {
                ContentUnavailableView(
                    tr("Could Not Open Book"), systemImage: "book.closed",
                    description: Text(error))
            } else if let session = book.session {
                BookWebView(path: session.path(), book: book)
            } else {
                ProgressView(tr("Opening Book…"))
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }
}

/// Find: its box, what it found, and the matches before and after.
struct FindBar: View {
    @ObservedObject var book: BookModel
    @FocusState private var focused: Bool

    var body: some View {
        let text = Binding<String>(
            get: { book.query },
            set: { (value: String) in book.search(value) })
        HStack(spacing: 8) {
            TextField(tr("Find in Book"), text: text)
                .textFieldStyle(.roundedBorder)
                .focused($focused)
                .onSubmit { book.step(1) }
                .frame(maxWidth: 400)
            Text(book.found)
                .foregroundStyle(.secondary)
                .frame(minWidth: 80, alignment: .leading)
            ControlGroup {
                Button { book.step(-1) } label: {
                    Label(tr("Previous Match"), systemImage: "chevron.left")
                }
                .help(tr("Previous Match"))
                Button { book.step(1) } label: {
                    Label(tr("Next Match"), systemImage: "chevron.right")
                }
                .help(tr("Next Match"))
            }
            .disabled(!book.hasMatches)
            .fixedSize()
            Spacer()
            Button(tr("_Close")) { book.closeFind() }
                .keyboardShortcut(.cancelAction)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .onAppear { focused = true }
        .onChange(of: book.findAsked) { (_: Int, _: Int) in focused = true }
    }
}

struct BookTools: ToolbarContent {
    @ObservedObject var book: BookModel

    var body: some ToolbarContent {
        ToolbarItem(placement: .navigation) {
            tool(tr("Contents"), "sidebar.left", Action.BOOK_CONTENTS)
        }
        ToolbarItemGroup {
            ControlGroup {
                tool(plain(tr("Zoom _Out")), "minus.magnifyingglass", Action.BOOK_ZOOM_OUT)
                tool(plain(tr("Zoom _In")), "plus.magnifyingglass", Action.BOOK_ZOOM_IN)
            }
            tool(more(tr("_Find…")), "magnifyingglass", Action.BOOK_FIND)
        }
    }

    private func tool(_ title: String, _ image: String, _ action: Action) -> some View {
        Button { book.perform(action) } label: {
            Label(title, systemImage: image)
        }
        .help(title)
        .disabled(!book.enabled(action))
    }
}

/// The book's page. It posts to the `corerules` handler once it has
/// loaded, where the reader is as it scrolls, and the reader's keys.
struct BookWebView: NSViewRepresentable {
    let path: String
    let book: BookModel

    func makeCoordinator() -> Coordinator {
        Coordinator(book: book)
    }

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.userContentController.add(context.coordinator, name: "corerules")
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.setValue(false, forKey: "drawsBackground")
        book.webView = view
        return view
    }

    func updateNSView(_ view: WKWebView, context: Context) {
        context.coordinator.book = book
        book.webView = view
        if context.coordinator.loaded != path {
            context.coordinator.loaded = path
            let url = URL(fileURLWithPath: path)
            view.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
        }
    }

    static func dismantleNSView(_ view: WKWebView, coordinator: Coordinator) {
        view.configuration.userContentController.removeScriptMessageHandler(forName: "corerules")
    }

    @MainActor
    final class Coordinator: NSObject, WKScriptMessageHandler {
        weak var book: BookModel?
        var loaded = ""

        init(book: BookModel) {
            self.book = book
        }

        func userContentController(
            _ userContentController: WKUserContentController, didReceive message: WKScriptMessage
        ) {
            if let text = message.body as? String {
                book?.posted(text)
            }
        }
    }
}

struct BookFocusKey: FocusedValueKey {
    typealias Value = BookModel
}

extension FocusedValues {
    /// The book in the window in front, which the reader's menu items act
    /// on.
    var book: BookModel? {
        get { self[BookFocusKey.self] }
        set { self[BookFocusKey.self] = newValue }
    }
}

/// The reader's menu items: Find in the Edit menu, the zoom and the
/// contents in the View menu, for the book in front.
struct BookCommands: Commands {
    @FocusedValue(\.book) private var book: BookModel?

    var body: some Commands {
        CommandGroup(replacing: .textEditing) {
            item(more(tr("_Find…")), Action.BOOK_FIND)
                .keyboardShortcut("f")
            item(tr("Next Match"), Action.BOOK_FIND, step: 1)
                .keyboardShortcut("g")
            item(tr("Previous Match"), Action.BOOK_FIND, step: -1)
                .keyboardShortcut("g", modifiers: [.command, .shift])
        }
        CommandGroup(before: .sidebar) {
            item(tr("Contents"), Action.BOOK_CONTENTS)
            Divider()
            item(plain(tr("Zoom _In")), Action.BOOK_ZOOM_IN)
                .keyboardShortcut("+")
            item(plain(tr("Zoom _Out")), Action.BOOK_ZOOM_OUT)
                .keyboardShortcut("-")
            item(plain(tr("_Reset Zoom")), Action.BOOK_ZOOM_RESET)
                .keyboardShortcut("0")
            Divider()
        }
    }

    /// The item of `action`; `step` makes it go to the next or previous
    /// match instead.
    private func item(_ title: String, _ action: Action, step: Int32 = 0) -> some View {
        Button(title) {
            if step != 0 {
                book?.step(step)
            } else {
                book?.perform(action)
            }
        }
        .disabled(book.map { step != 0 ? $0.hasMatches : $0.enabled(action) } != true)
    }
}
