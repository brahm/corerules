// The app's actions, as `cr_app::action` names them for every interface.
// The parity test finds each one this app has as `Action.NAME` in its
// sources; one not here yet stays on `NOT_YET_ON_MACOS`.

import Foundation

struct Action: Hashable {
    /// The name with its group, as the GNOME app has it:
    /// `win.new-character`.
    let detailed: String

    private init(_ constant: String) {
        detailed = actions().first { $0.constant == constant }?.detailed ?? constant
    }

    static let PREFERENCES = Action("PREFERENCES")
    static let OPEN = Action("OPEN")
    static let NEW_CHARACTER = Action("NEW_CHARACTER")
    static let ENTER_CHARACTER = Action("ENTER_CHARACTER")
    static let NEW_NPC = Action("NEW_NPC")
    static let NEW_NPC_GROUP = Action("NEW_NPC_GROUP")
    static let ROLL_TABLE = Action("ROLL_TABLE")
    static let ROLL_TREASURE = Action("ROLL_TREASURE")
    static let TREASURE_SAVE = Action("TREASURE_SAVE")
    static let TREASURE_PRINT = Action("TREASURE_PRINT")
    static let TREASURE_PDF = Action("TREASURE_PDF")
    static let TREASURE_DELETE = Action("TREASURE_DELETE")
    static let NEW_ENCOUNTER = Action("NEW_ENCOUNTER")
    static let NEW_TABLE = Action("NEW_TABLE")
    static let MANAGE_PARTIES = Action("MANAGE_PARTIES")
    static let ENCOUNTER_PRINT = Action("ENCOUNTER_PRINT")
    static let ENCOUNTER_PDF = Action("ENCOUNTER_PDF")
    static let ENCOUNTER_DELETE = Action("ENCOUNTER_DELETE")
    static let UDB_NEW = Action("UDB_NEW")
    static let UDB_COPY = Action("UDB_COPY")
    static let UDB_EDIT = Action("UDB_EDIT")
    static let UDB_DELETE = Action("UDB_DELETE")
    static let UDB_SUBRACE = Action("UDB_SUBRACE")
    static let UDB_SCHOOLS = Action("UDB_SCHOOLS")
    static let UDB_ALLOWED = Action("UDB_ALLOWED")
    static let UDB_RESTORE = Action("UDB_RESTORE")
    static let TRAP_EDIT = Action("TRAP_EDIT")
    static let TRAP_COPY = Action("TRAP_COPY")
    static let TRAP_DELETE = Action("TRAP_DELETE")
    static let IMPORT = Action("IMPORT")
    static let EXTRACT_DATA = Action("EXTRACT_DATA")
    static let IMPORT_CORE20 = Action("IMPORT_CORE20")
    static let EXPORT = Action("EXPORT")
    static let NEW_PACKAGE = Action("NEW_PACKAGE")
    static let EDIT_PACKAGE = Action("EDIT_PACKAGE")
    static let ABOUT = Action("ABOUT")
    static let HELP = Action("HELP")
    static let HELP_HERE = Action("HELP_HERE")
    static let HELP_CONTROL = Action("HELP_CONTROL")
    static let OPEN_BOOK = Action("OPEN_BOOK")
    static let BOOK_FIND = Action("BOOK_FIND")
    static let BOOK_ZOOM_IN = Action("BOOK_ZOOM_IN")
    static let BOOK_ZOOM_OUT = Action("BOOK_ZOOM_OUT")
    static let BOOK_ZOOM_RESET = Action("BOOK_ZOOM_RESET")
    static let BOOK_CONTENTS = Action("BOOK_CONTENTS")
    static let BOOK_CLOSE = Action("BOOK_CLOSE")
    static let SHEET_PRINT = Action("SHEET_PRINT")
    static let SHEET_PDF = Action("SHEET_PDF")
    static let SHEET_HTML = Action("SHEET_HTML")
    static let SHEET_TXT = Action("SHEET_TXT")
    static let SHEET_MD = Action("SHEET_MD")

    /// The export action of each file a sheet is exported to.
    static func export(_ export: SheetExport) -> Action {
        switch export {
        case .html: return Action.SHEET_HTML
        case .text: return Action.SHEET_TXT
        case .markdown: return Action.SHEET_MD
        }
    }
}
