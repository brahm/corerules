//! What the GNOME and the Windows app share above the backend, with no
//! GTK and no Windows in it: the names of their actions, the library and
//! what each entry offers, the records of the user databases, the
//! original's pictures cut out, the help and About, the book reader's
//! zoom, find and reading positions, the character sheet
//! laid out on printed pages, and the sheet as a PDF. An interface draws, asks and reports;
//! what it decides comes from here or from `cr-store`, so that every
//! version of the app does the same.

pub mod about;
pub mod action;
pub mod books;
pub mod bundle;
pub mod create;
pub mod editor;
pub mod encounter;
pub mod help;
pub mod levels;
pub mod library;
pub mod npc;
pub mod packages;
pub mod pages;
pub mod parties;
pub mod pdf;
pub mod pictures;
pub mod prompt;
pub mod records;
pub mod sheets;
pub mod skills;
pub mod tables;
pub mod treasure;

/// The file name the original suggests when saving something called
/// `title`: `Borin_Pedraforte.pdf`.
pub fn file_name(title: &str, extension: &str) -> String {
    format!("{}.{extension}", title.replace(' ', "_"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn file_names_join_words_with_underscores() {
        assert_eq!(
            super::file_name("Borin Pedraforte", "pdf"),
            "Borin_Pedraforte.pdf"
        );
    }
}
