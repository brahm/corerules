//! Import…, Export…, New Package… and Edit Package… of the main menu, and
//! Import Core Rules 2.0…, with no toolkit in them: the DM Toolkit's
//! packages (`.xpx` from the Expansion, `.xpt` from Core Rules 2.0), what
//! to take into one or out of one, the names asked before an import
//! (0x49d704), and what an import did, in words.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cr_formats::export::ExportFile;
use cr_i18n::{tr, trn};
use cr_store::export::{Category, Clash, Content, Exportable, ImportReport, contents, remove};
use cr_store::userdb::UserDb;
use cr_store::userdb::parts::{Parts, name_taken};

/// The most items of one category an import lists one by one; a category
/// with more (a large package holds tens of thousands) is one choice that
/// takes or leaves them all, its items listed only when asked.
pub const ROWS_SHOWN: usize = 200;

/// The name the lists show for `category`, translated.
pub fn category_label(category: Category) -> &'static str {
    match category {
        Category::Part(_) => tr(category.label()),
        Category::Class => tr("Classes"),
        Category::Race => tr("Races"),
        Category::WizardSpell => tr("Wizard Spells"),
        Category::PriestSpell => tr("Priest Spells"),
        Category::Monster => tr("Monsters"),
        Category::MonsterAttack => tr("Monster Attacks"),
        Category::MonsterType => tr("Monster Types"),
        Category::Encounter => tr("Encounters"),
        Category::Template => tr("Encounter Templates"),
        Category::Trap => tr("Traps"),
        Category::PsionicPower => tr("Psionic Powers"),
        Category::TreasureBundle => tr("Treasure Bundles"),
        Category::Character => tr("Characters"),
    }
}

/// The categories of `items` in the order the lists show them: sorted by
/// label, each with its items' indexes.
fn by_category<T>(items: &[T], category: impl Fn(&T) -> Category) -> Vec<(Category, Vec<usize>)> {
    let mut categories: Vec<Category> = items.iter().map(&category).collect();
    categories.sort_by_key(|c| c.label());
    categories.dedup();
    categories
        .into_iter()
        .map(|c| {
            let of: Vec<usize> = (0..items.len())
                .filter(|&i| category(&items[i]) == c)
                .collect();
            (c, of)
        })
        .collect()
}

/// The file name of `path`, as titles and messages show it.
pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The extension of a new package of the Expansion (true) or of Core
/// Rules 2.0.
pub fn extension(expansion: bool) -> &'static str {
    if expansion { "xpx" } else { "xpt" }
}

/// Export… and Add Items…: the user's records and characters, by
/// category, each checked or not.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemChooser {
    pub items: Vec<Exportable>,
    pub checked: Vec<bool>,
}

impl ItemChooser {
    pub fn new(items: Vec<Exportable>) -> Self {
        Self {
            checked: vec![false; items.len()],
            items,
        }
    }

    /// The categories with their items' indexes, as listed.
    pub fn categories(&self) -> Vec<(Category, Vec<usize>)> {
        by_category(&self.items, |e| e.category)
    }

    pub fn set(&mut self, i: usize, on: bool) {
        if let Some(c) = self.checked.get_mut(i) {
            *c = on;
        }
    }

    /// Select All, on or off.
    pub fn set_all(&mut self, on: bool) {
        self.checked.fill(on);
    }

    pub fn all(&self) -> bool {
        !self.checked.is_empty() && self.checked.iter().all(|c| *c)
    }

    pub fn any(&self) -> bool {
        self.checked.iter().any(|c| *c)
    }

    /// The items checked, in their order.
    pub fn chosen(&self) -> Vec<Exportable> {
        self.items
            .iter()
            .zip(&self.checked)
            .filter(|(_, c)| **c)
            .map(|(e, _)| e.clone())
            .collect()
    }
}

/// What Import… does with an item a record's name clashes with, said
/// under it; nothing when no name clashes.
pub fn clash_note(clash: Clash, expansion: bool) -> &'static str {
    match (clash, expansion) {
        (Clash::None, _) => "",
        (Clash::Standard, _) => tr("Renamed with a * as a standard part has this name"),
        (Clash::Custom, true) => tr("Replaces your record of this name"),
        (Clash::Custom, false) => tr("Not imported: you have a record of this name"),
    }
}

/// Import…: what a package would bring into the installation, each item
/// taken or left.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportChoice {
    pub file: ExportFile,
    /// The items, with the records whose names they share.
    pub preview: Vec<Content>,
    /// Whether the installation is the Expansion's.
    pub expansion: bool,
    pub checked: Vec<bool>,
}

impl ImportChoice {
    /// Every item of `preview` taken, as the dialog opens.
    pub fn new(file: ExportFile, preview: Vec<Content>, expansion: bool) -> Self {
        Self {
            checked: vec![true; preview.len()],
            file,
            preview,
            expansion,
        }
    }

    /// Core Rules 2.0 cannot import what the Expansion made.
    pub fn refused(&self) -> bool {
        self.file.is_expansion() && !self.expansion
    }

    /// The categories with their items' indexes, sorted by label.
    pub fn categories(&self) -> Vec<(Category, Vec<usize>)> {
        by_category(&self.preview, |c| c.category)
    }

    /// What importing item `n` does about a name it shares.
    pub fn note(&self, n: usize) -> &'static str {
        self.preview
            .get(n)
            .map_or("", |c| clash_note(c.clash, self.expansion))
    }

    pub fn set(&mut self, n: usize, on: bool) {
        if let Some(c) = self.checked.get_mut(n) {
            *c = on;
        }
    }

    /// Takes or leaves every item of `items`.
    pub fn set_many(&mut self, items: &[usize], on: bool) {
        for &n in items {
            self.set(n, on);
        }
    }

    /// Whether every item of `items` is taken.
    pub fn all(&self, items: &[usize]) -> bool {
        items.iter().all(|&n| self.checked.get(n) == Some(&true))
    }

    /// The items taken, by index into the preview.
    pub fn chosen(&self) -> Vec<usize> {
        (0..self.checked.len())
            .filter(|&n| self.checked[n])
            .collect()
    }

    /// "The package also holds 2 Map Maker II maps, …", when it does.
    pub fn maps_note(&self) -> Option<String> {
        let count = self.file.map_maker.count;
        (count > 0).then(|| {
            trn!(
                "The package also holds {n} Map Maker II map, which is not imported.",
                "The package also holds {n} Map Maker II maps, which are not imported.",
                count
            )
        })
    }

    pub fn can_import(&self) -> bool {
        !self.refused() && self.checked.iter().any(|c| *c)
    }
}

/// What the names asked before an import come to next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Name Conflict for item `n`, offering `name`.
    Ask { n: usize, name: String },
    /// Every item has its name: import with these.
    Import(HashMap<usize, String>),
}

/// The names the parts a standard part's name hides take, asked one by one
/// in the original's order before the import runs (0x49d704): the name
/// with a "*", or what Name Conflict asks for when a part (a standard one,
/// one of the user's or one imported before it) has that name.
#[derive(Debug, Clone, PartialEq)]
pub struct Renaming {
    chosen: Vec<usize>,
    /// The chosen item being looked at, by its place among them.
    at: usize,
    names: HashMap<usize, String>,
    /// The names of the parts chosen ahead of it.
    before: Vec<String>,
}

impl Renaming {
    /// The names of the items `chosen`, by index into the preview.
    pub fn new(chosen: Vec<usize>) -> Self {
        Self {
            chosen,
            at: 0,
            names: HashMap::new(),
            before: Vec::new(),
        }
    }

    /// Goes on to the next item that needs a name. A loop, not a
    /// recursion: a package holds tens of thousands of items, and only a
    /// Name Conflict question stops it until the user answers.
    pub fn next(&mut self, preview: &[Content], parts: &UserDb<Parts>) -> Step {
        while let Some(&n) = self.chosen.get(self.at) {
            let item = &preview[n];
            let part = matches!(item.category, Category::Part(_));
            // Only the Expansion renames (the preview says so).
            if item.clash != Clash::Standard {
                if part {
                    self.before.push(item.name.clone());
                }
                self.at += 1;
                continue;
            }
            let name = format!("{}*", item.name);
            if !name_taken(parts, &name, &self.before) {
                if part {
                    self.before.push(item.name.clone());
                }
                self.at += 1;
                continue;
            }
            return Step::Ask { n, name };
        }
        Step::Import(self.names.clone())
    }

    /// Whether `name` is taken for the item asked about.
    pub fn taken(&self, name: &str, parts: &UserDb<Parts>) -> bool {
        name_taken(parts, name, &self.before)
    }

    /// The name given in Name Conflict for the item asked about, `n`; the
    /// caller asks again while [`taken`](Self::taken) says so.
    pub fn named(&mut self, n: usize, name: String) {
        self.before.push(name.clone());
        self.names.insert(n, name);
        self.at += 1;
    }
}

/// What an import did, as its report says it: the heading and the lines
/// under it (none when there is nothing more to say).
pub fn report(report: &ImportReport) -> (String, Vec<String>) {
    let label = |c: &Content| format!("{} ({})", c.name, category_label(c.category));
    let mut lines = Vec::new();
    for (c, name) in &report.renamed {
        lines.push(tr!("{item} was renamed {name}.", item = label(c), name));
    }
    if !report.replaced.is_empty() {
        lines.push(tr!(
            "Replaced your records: {names}.",
            names = report
                .replaced
                .iter()
                .map(|(c, n)| format!("{n} ({})", category_label(*c)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !report.skipped.is_empty() {
        lines.push(tr!(
            "Not imported, as you have records of these names: {names}.",
            names = report
                .skipped
                .iter()
                .map(label)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    lines.extend(report.warnings.iter().cloned());
    let heading = trn!(
        "Imported {n} Item",
        "Imported {n} Items",
        report.imported.len()
    );
    (heading, lines)
}

/// The package being edited by New Package… or Edit Package….
#[derive(Debug, Clone, PartialEq)]
pub struct PackageEditor {
    pub file: ExportFile,
    /// Where it is saved; `None` until a new package is saved.
    pub path: Option<PathBuf>,
    /// Whether the installation is the Expansion's.
    pub expansion: bool,
    pub changed: bool,
    /// What the changes left out, said when it is saved.
    pub warnings: Vec<String>,
    /// Saving is under way.
    pub saving: bool,
    pub error: Option<String>,
}

impl PackageEditor {
    /// The editor of `file`, saved at `path` (none for a new package).
    pub fn new(file: ExportFile, path: Option<PathBuf>, expansion: bool) -> Self {
        Self {
            file,
            changed: path.is_none(),
            path,
            expansion,
            warnings: Vec::new(),
            saving: false,
            error: None,
        }
    }

    pub fn title(&self) -> String {
        match &self.path {
            Some(path) => file_name(path),
            None => tr("New Package").to_owned(),
        }
    }

    /// Whether the package is in the installation's layout, so that its
    /// records can go into it.
    pub fn same_layout(&self) -> bool {
        self.file.is_expansion() == self.expansion
    }

    /// Why items cannot be added, when they cannot.
    pub fn layout_note(&self) -> Option<&'static str> {
        if self.same_layout() {
            None
        } else if self.file.is_expansion() {
            Some(tr(
                "This package was made by the Expansion; items of Core Rules 2.0 cannot be \
                 added to it.",
            ))
        } else {
            Some(tr(
                "This package was made by Core Rules 2.0; items of the Expansion cannot be \
                 added to it.",
            ))
        }
    }

    /// The items, by category, each with the button that takes it out.
    pub fn items(&self) -> Vec<(Category, Vec<Content>)> {
        let items = contents(&self.file);
        by_category(&items, |c| c.category)
            .into_iter()
            .map(|(c, of)| (c, of.into_iter().map(|i| items[i].clone()).collect()))
            .collect()
    }

    /// Takes `item` out.
    pub fn remove(&mut self, item: &Content) {
        let warnings = remove(&mut self.file, std::slice::from_ref(item));
        self.warnings.extend(warnings);
        self.changed = true;
    }

    /// Items were added, with these warnings.
    pub fn added(&mut self, warnings: Vec<String>) {
        self.warnings.extend(warnings);
        self.changed = true;
    }

    /// "The package also holds 2 Map Maker II maps, …", when it does.
    pub fn maps_note(&self) -> Option<String> {
        let count = self.file.map_maker.count;
        (count > 0).then(|| {
            trn!(
                "The package also holds {n} Map Maker II map, which a change to the \
                 package leaves out.",
                "The package also holds {n} Map Maker II maps, which a change to the \
                 package leaves out.",
                count
            )
        })
    }

    /// It was saved at `path`: what to say, a heading and the warnings.
    pub fn saved(&mut self, path: PathBuf) -> (String, Vec<String>) {
        self.saving = false;
        self.changed = false;
        let said = tr!("Saved {name}", name = file_name(&path));
        self.path = Some(path);
        (said, std::mem::take(&mut self.warnings))
    }
}

/// Writes `file` at `path`.
pub fn write(path: &Path, file: &ExportFile) -> Result<(), String> {
    let bytes = file.to_bytes().map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads the package at `path`.
pub fn read(path: &Path) -> Result<ExportFile, String> {
    std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|d| ExportFile::parse(&d).map_err(|e| e.to_string()))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// What Edit Package… says of a package of Core Rules 2.0, which is only
/// imported: its title and text.
pub fn core20_only() -> (&'static str, &'static str) {
    (
        tr("Core Rules 2.0 Package"),
        tr(
            "Packages made by Core Rules 2.0 are only imported. Use Import… to bring \
             their content into this installation, then make a new package from it.",
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_store::export::Source;
    use cr_store::userdb::Flavor;

    fn content(category: Category, name: &str, clash: Clash) -> Content {
        Content {
            category,
            name: name.into(),
            index: 0,
            clash,
        }
    }

    #[test]
    fn the_chooser_groups_by_category_and_takes_what_is_checked() {
        let item = |category, name: &str| Exportable {
            category,
            name: name.into(),
            source: Source::Record(0),
        };
        let mut chooser = ItemChooser::new(vec![
            item(Category::Trap, "Pit"),
            item(Category::Race, "Elf"),
            item(Category::Trap, "Dart"),
        ]);
        let categories = chooser.categories();
        assert_eq!(categories[0], (Category::Race, vec![1]));
        assert_eq!(categories[1], (Category::Trap, vec![0, 2]));
        assert!(!chooser.any());
        chooser.set(2, true);
        assert_eq!(chooser.chosen()[0].name, "Dart");
        chooser.set_all(true);
        assert!(chooser.all());
    }

    #[test]
    fn renaming_asks_only_when_the_starred_name_is_taken() {
        let parts = UserDb::<Parts>::new(Flavor::Expansion, Vec::new(), Vec::new());
        let preview = vec![
            content(Category::Part(0), "Sword", Clash::Standard),
            content(Category::Part(0), "Sword*", Clash::None),
            content(Category::Part(0), "Sword", Clash::Standard),
        ];
        let mut renaming = Renaming::new(vec![0, 1, 2]);
        // "Sword*" is free for the first; the second is named so, so the
        // third must be asked.
        assert_eq!(
            renaming.next(&preview, &parts),
            Step::Ask {
                n: 2,
                name: "Sword*".into()
            }
        );
        assert!(renaming.taken("Sword*", &parts));
        assert!(!renaming.taken("Old Sword", &parts));
        renaming.named(2, "Old Sword".into());
        let Step::Import(names) = renaming.next(&preview, &parts) else {
            panic!("asked again");
        };
        assert_eq!(names.get(&2).map(String::as_str), Some("Old Sword"));
    }

    #[test]
    fn the_report_says_what_was_renamed_replaced_and_skipped() {
        let report = ImportReport {
            imported: vec![content(Category::Trap, "Pit", Clash::None)],
            replaced: vec![(Category::Trap, "Pit".into())],
            renamed: vec![(
                content(Category::Race, "Elf", Clash::Standard),
                "Elf*".into(),
            )],
            ..Default::default()
        };
        let (heading, lines) = super::report(&report);
        assert_eq!(heading, "Imported 1 Item");
        assert_eq!(lines[0], "Elf (Races) was renamed Elf*.");
        assert_eq!(lines[1], "Replaced your records: Pit (Traps).");
    }

    #[test]
    fn the_editor_says_why_nothing_can_be_added() {
        let editor = PackageEditor::new(ExportFile::new(false), None, true);
        assert!(editor.changed && !editor.same_layout());
        assert!(editor.layout_note().is_some());
        assert_eq!(editor.title(), "New Package");
        let choice = ImportChoice::new(ExportFile::new(true), Vec::new(), false);
        assert!(choice.refused() && !choice.can_import());
    }
}
