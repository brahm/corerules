//! The app's actions, each with one name in every interface: the GNOME
//! app registers each under that name (`win.new-character`), and the
//! Windows app gives the command that does the same the same constant.
//! An interface refers to an action only through these constants, so a
//! test can tell when one interface has an action that the other lacks.

/// An action: the group GTK keeps it in (`win`, `sheet`) and its name
/// there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Action {
    pub group: &'static str,
    pub name: &'static str,
    /// The name with its group, as menus and buttons refer to it.
    pub detailed: &'static str,
    /// The name of its constant here, which the parity test looks for.
    pub constant: &'static str,
}

macro_rules! actions {
    ($($(#[$doc:meta])* $constant:ident = $group:literal . $name:literal;)*) => {
        $(
            $(#[$doc])*
            pub const $constant: Action = Action {
                group: $group,
                name: $name,
                detailed: concat!($group, ".", $name),
                constant: stringify!($constant),
            };
        )*
        /// Every action, in the order above.
        pub const ALL: &[Action] = &[$($constant),*];
    };
}

actions! {
    // The main window.
    /// Preferences (on Windows, Settings).
    PREFERENCES = "win"."preferences";
    /// Open Folder…: an installation or a CD.
    OPEN = "win"."open";
    /// Extract Data from CD…
    EXTRACT_DATA = "win"."extract-data";
    NEW_CHARACTER = "win"."new-character";
    ENTER_CHARACTER = "win"."enter-character";
    NEW_NPC = "win"."new-npc";
    NEW_NPC_GROUP = "win"."new-npc-group";
    ROLL_TREASURE = "win"."roll-treasure";
    NEW_ENCOUNTER = "win"."new-encounter";
    NEW_TABLE = "win"."new-table";
    ROLL_TABLE = "win"."roll-table";
    OPEN_BOOK = "win"."open-book";
    MANAGE_PARTIES = "win"."manage-parties";
    /// Export…: characters and user database entries to a package.
    EXPORT = "win"."export";
    /// Import…: a package.
    IMPORT = "win"."import";
    /// Import from Core Rules 2.0…
    IMPORT_CORE20 = "win"."import-core20";
    NEW_PACKAGE = "win"."new-package";
    EDIT_PACKAGE = "win"."edit-package";
    HELP = "win"."help";
    /// Help on the screen in front.
    HELP_HERE = "win"."help-here";
    /// What's This?: help on the control clicked next.
    HELP_CONTROL = "win"."help-control";
    ABOUT = "win"."about";

    // The user database editors.
    UDB_NEW = "win"."udb-new";
    UDB_COPY = "win"."udb-copy";
    UDB_EDIT = "win"."udb-edit";
    UDB_DELETE = "win"."udb-delete";
    UDB_SUBRACE = "win"."udb-subrace";
    UDB_SCHOOLS = "win"."udb-schools";
    UDB_ALLOWED = "win"."udb-allowed";
    UDB_RESTORE = "win"."udb-restore";

    // A book's window.
    BOOK_FIND = "win"."find";
    BOOK_ZOOM_IN = "win"."zoom-in";
    BOOK_ZOOM_OUT = "win"."zoom-out";
    BOOK_ZOOM_RESET = "win"."zoom-reset";
    BOOK_CONTENTS = "win"."toggle-contents";
    BOOK_CLOSE = "win"."close";

    // The character sheet.
    SHEET_PRINT = "sheet"."print";
    SHEET_PDF = "sheet"."pdf";
    SHEET_HTML = "sheet"."html";
    SHEET_TXT = "sheet"."txt";
    SHEET_MD = "sheet"."md";

    // A treasure.
    TREASURE_SAVE = "treasure"."save";
    TREASURE_PRINT = "treasure"."print";
    TREASURE_PDF = "treasure"."pdf";
    TREASURE_DELETE = "treasure"."delete";

    // An encounter, and a trap in its list.
    ENCOUNTER_PRINT = "encounter"."print";
    ENCOUNTER_PDF = "encounter"."pdf";
    ENCOUNTER_DELETE = "encounter"."delete";
    TRAP_EDIT = "trap"."edit";
    TRAP_COPY = "trap"."copy";
    TRAP_DELETE = "trap"."delete";
}

/// The actions the Windows app does not have yet. A screen ported to
/// Windows takes its actions off this list, and the release with Windows
/// waits until it is empty.
pub const NOT_YET_ON_WINDOWS: &[Action] = &[
    EXTRACT_DATA,
    ROLL_TREASURE,
    NEW_ENCOUNTER,
    NEW_TABLE,
    ROLL_TABLE,
    MANAGE_PARTIES,
    EXPORT,
    IMPORT_CORE20,
    NEW_PACKAGE,
    EDIT_PACKAGE,
    UDB_NEW,
    UDB_COPY,
    UDB_EDIT,
    UDB_DELETE,
    UDB_SUBRACE,
    UDB_SCHOOLS,
    UDB_ALLOWED,
    UDB_RESTORE,
    TREASURE_SAVE,
    TREASURE_PRINT,
    TREASURE_PDF,
    TREASURE_DELETE,
    ENCOUNTER_PRINT,
    ENCOUNTER_PDF,
    ENCOUNTER_DELETE,
    TRAP_EDIT,
    TRAP_COPY,
    TRAP_DELETE,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique_within_a_group() {
        let mut seen = HashSet::new();
        for action in ALL {
            assert!(seen.insert(action.detailed), "{action:?}");
        }
    }

    #[test]
    fn pending_actions_are_actions() {
        for action in NOT_YET_ON_WINDOWS {
            assert!(ALL.contains(action), "{action:?}");
        }
    }
}
