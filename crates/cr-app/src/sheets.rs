//! The character sheet as both apps offer it: a character's sheet with
//! the options that can be changed, the files it is exported to, and the
//! lists of Character Sheet Options (dialog 414).

use cr_i18n::tr;
use cr_store::sheet::{Flavor, SECTIONS, Sheet, SheetChoice, Timestamp, ids};
use cr_store::{Installation, Result};

/// The sheet of the character in `Chars/<file>`, with its options when
/// they can be changed: an Expansion character's sections and border, in
/// an installation that takes changes.
pub fn character(install: &Installation, file: &str) -> Result<(Sheet, Option<SheetChoice>)> {
    let character = install.character(file)?;
    let rules = install.sheet_rules()?;
    let choice = (rules.flavor() == Flavor::Expansion && !install.is_core20())
        .then(|| rules.sheet_choice(&character))
        .transpose()?;
    Ok((rules.sheet(&character)?, choice))
}

/// Keeps new options for the sheet of the character in `Chars/<file>`
/// and makes its sheet again with them.
pub fn choose(install: &Installation, file: &str, choice: &SheetChoice) -> Result<Sheet> {
    install.save_sheet_choice(file, choice)?;
    install.sheet(&install.character(file)?)
}

/// A file a character's sheet is exported to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Export {
    Html,
    Text,
    Markdown,
}

impl Export {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Text => "txt",
            Self::Markdown => "md",
        }
    }

    /// Its menu item.
    pub fn label(self) -> &'static str {
        match self {
            Self::Html => tr("Export as _HTML…"),
            Self::Text => tr("Export as _Text…"),
            Self::Markdown => tr("Export as _Markdown…"),
        }
    }

    /// The files of its type, as the Save dialog filters them.
    pub fn kind(self) -> &'static str {
        match self {
            Self::Html => tr("HTML documents"),
            Self::Text => tr("Text files"),
            Self::Markdown => tr("Markdown documents"),
        }
    }

    /// The exports of a sheet of `flavor`: HTML and Markdown, and the
    /// text file the Expansion writes.
    pub fn of(flavor: Flavor) -> Vec<Self> {
        let mut exports = vec![Self::Html];
        if flavor == Flavor::Expansion {
            exports.push(Self::Text);
        }
        exports.push(Self::Markdown);
        exports
    }

    /// The file's contents, dated now as the originals date them.
    pub fn write(self, sheet: &Sheet) -> Vec<u8> {
        let date = Timestamp::now_utc().format(sheet.flavor);
        match self {
            Self::Markdown => sheet.to_markdown(&date).into_bytes(),
            _ => sheet.export(self == Self::Html, &date),
        }
    }
}

/// A section of the sheet as the options list it; the page break is set
/// apart from the sections.
pub fn section_name(id: u32) -> String {
    if id == ids::PAGE_BREAK {
        return format!("— {} —", tr("Page Break"));
    }
    SECTIONS
        .iter()
        .find(|(s, _)| *s == id)
        .map_or_else(String::new, |(_, n)| (*n).to_owned())
}

/// Every section the options offer to add, in the original's sorted
/// list, the page break first. Adding one leaves it there, so any can be
/// added again.
pub fn offered() -> Vec<u32> {
    let mut offered: Vec<u32> = SECTIONS.iter().map(|(id, _)| *id).collect();
    offered.sort_by_key(|&id| (id != ids::PAGE_BREAK, section_name(id)));
    offered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_break_comes_first_then_the_sections_by_name() {
        let offered = offered();
        assert_eq!(offered[0], ids::PAGE_BREAK);
        let names: Vec<String> = offered[1..].iter().map(|&id| section_name(id)).collect();
        assert!(names.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(offered.len(), SECTIONS.len());
    }

    #[test]
    fn text_is_an_export_of_the_expansion_only() {
        assert_eq!(Export::of(Flavor::Core20), [Export::Html, Export::Markdown]);
        assert_eq!(Export::of(Flavor::Expansion).len(), 3);
    }
}
