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
    static let IMPORT = Action("IMPORT")
    static let ABOUT = Action("ABOUT")
    static let HELP = Action("HELP")
    static let HELP_HERE = Action("HELP_HERE")
    static let HELP_CONTROL = Action("HELP_CONTROL")
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
