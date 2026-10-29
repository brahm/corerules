// Roll on a Table, in a window of its own: a book and one of its tables,
// the table's options, and the results of the roll, from
// `cr_app::tables` through the bridge. The toolbar rolls, rolls again,
// copies the results and reads the table in its book.

import AppKit
import SwiftUI

@MainActor
final class TableModel: ObservableObject {
    let session: TableSession
    @Published private(set) var books: [String] = []
    @Published private(set) var book = 0
    @Published private(set) var tables: [String] = []
    @Published private(set) var table = 0
    @Published private(set) var options: [TableOption] = []
    @Published private(set) var result: TableResult?
    @Published private(set) var againEnabled = false

    init(_ session: TableSession) {
        self.session = session
        refresh()
    }

    func refresh() {
        books = session.books()
        book = Int(session.book())
        tables = session.tables()
        table = Int(session.table())
        options = session.options()
        result = session.result()
        againEnabled = session.againEnabled()
    }

    var bookChosen: Binding<Int> {
        Binding<Int>(
            get: { self.book },
            set: { (index: Int) in
                self.session.setBook(index: UInt32(index))
                self.refresh()
            })
    }

    var tableChosen: Binding<Int> {
        Binding<Int>(
            get: { self.table },
            set: { (index: Int) in
                self.session.setTable(index: UInt32(index))
                self.refresh()
            })
    }

    func option(_ index: Int) -> Binding<Int> {
        Binding<Int>(
            get: { index < self.options.count ? Int(self.options[index].chosen) : 0 },
            set: { (chosen: Int) in
                self.session.setOption(index: UInt32(index), option: UInt32(chosen))
                self.refresh()
            })
    }

    func roll() {
        session.roll()
        refresh()
    }

    func again() {
        session.again()
        refresh()
    }

    func copy() {
        guard let text = result?.text else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}

struct TableWindow: View {
    @EnvironmentObject var app: AppModel

    var body: some View {
        if let tables = app.tables {
            TableView(tables: tables)
                .focusedSceneValue(\.tables, tables)
                .onDisappear { app.closeTables() }
        } else {
            ContentUnavailableView(tr("Nothing Selected"), systemImage: "list.bullet.rectangle")
        }
    }
}

struct TableView: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var tables: TableModel

    var body: some View {
        Form {
            Section {
                Picker(tr("Book"), selection: tables.bookChosen) {
                    ForEach(0..<tables.books.count, id: \.self) { (index: Int) in
                        Text(tables.books[index]).tag(index)
                    }
                }
                Picker(tr("Table"), selection: tables.tableChosen) {
                    ForEach(0..<tables.tables.count, id: \.self) { (index: Int) in
                        Text(tables.tables[index]).tag(index)
                    }
                }
                // A new list for each book.
                .id(tables.book)
            }
            if !tables.options.isEmpty {
                Section(tr("Options")) {
                    ForEach(Array(tables.options.enumerated()), id: \.offset) { item in
                        TableOptionRow(tables: tables, index: item.offset, option: item.element)
                    }
                }
                .id("\(tables.book).\(tables.table)")
            }
            Section(tr("Results")) {
                if let result = tables.result {
                    ForEach(Array(result.blocks.enumerated()), id: \.offset) { item in
                        TableBlockView(block: item.element)
                    }
                } else {
                    Text(tr("Choose a table and roll."))
                        .foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
        .frame(minWidth: 560, minHeight: 480)
        .navigationTitle(tr("Roll on a Table"))
        .toolbar {
            TableTools(tables: tables) { (reference: TableReference) in
                app.openBook(reference.book, find: reference.find)
            }
        }
    }
}

struct TableOptionRow: View {
    @ObservedObject var tables: TableModel
    let index: Int
    let option: TableOption

    var body: some View {
        Picker(option.label, selection: tables.option(index)) {
            ForEach(0..<option.options.count, id: \.self) { (i: Int) in
                Text(option.options[i]).tag(i)
            }
        }
    }
}

/// A block of the results: its title and subtitle, its columns, and its
/// notes.
struct TableBlockView: View {
    let block: TableBlock

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let title = block.title {
                Text(title).font(.headline)
            }
            if let subtitle = block.subtitle {
                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if !block.header.isEmpty {
                ScrollView(.horizontal) {
                    TableColumns(block: block)
                }
            }
            ForEach(Array(block.notes.enumerated()), id: \.offset) { item in
                Text(item.element)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .textSelection(.enabled)
    }
}

/// A block's columns, the header first; the columns never wrap.
struct TableColumns: View {
    let block: TableBlock

    var body: some View {
        let columns = block.header.count
        Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 4) {
            GridRow {
                ForEach(0..<columns, id: \.self) { (c: Int) in
                    Text(block.header[c])
                        .foregroundStyle(.secondary)
                }
            }
            ForEach(Array(block.rows.enumerated()), id: \.offset) { item in
                GridRow {
                    ForEach(0..<columns, id: \.self) { (c: Int) in
                        Text(c < item.element.count ? item.element[c] : "")
                            .fixedSize()
                    }
                }
            }
        }
        .padding(.vertical, 4)
    }
}

struct TableTools: ToolbarContent {
    @ObservedObject var tables: TableModel
    /// Opens the table's book at the table.
    let read: (TableReference) -> Void

    var body: some ToolbarContent {
        ToolbarItemGroup {
            Button {
                tables.roll()
            } label: {
                Label(tr("_Roll"), systemImage: "dice")
            }
            .keyboardShortcut("r")
            .help(tr("_Roll"))
            Button {
                tables.again()
            } label: {
                Label(tr("Roll _Again"), systemImage: "arrow.clockwise")
            }
            .keyboardShortcut("r", modifiers: [.command, .shift])
            .disabled(!tables.againEnabled)
            .help(tr("Roll _Again"))
            Button {
                tables.copy()
            } label: {
                Label(tr("Copy Results"), systemImage: "doc.on.doc")
            }
            .disabled(tables.result == nil)
            .help(tr("Copy Results"))
            if let reference = tables.result?.reference {
                Button {
                    tables.session.close()
                    read(reference)
                } label: {
                    Label(tr("Read in _Book"), systemImage: "book")
                }
                .help(tr("Read in _Book"))
            }
        }
    }
}

struct TablesFocusKey: FocusedValueKey {
    typealias Value = TableModel
}

extension FocusedValues {
    /// Roll on a Table, when its window is in front.
    var tables: TableModel? {
        get { self[TablesFocusKey.self] }
        set { self[TablesFocusKey.self] = newValue }
    }
}
