//! Editing a treasure bundle as the original's Treasure Bundle Editor
//! does (IDD 290), with no toolkit in it: adding parts, removing some or
//! all of an item, moving items into others, copying and removing scroll
//! spells, and Edit Scrolls. The treasure roller and the Encounter
//! dialog's Items use it.
//!
//! [`BundleEditor`] keeps the dialog the edit waits on (How many?, the
//! node to move into, a question), so that an interface only draws the
//! step it is at and passes back what the user did.

use std::sync::Arc;

use cr_i18n::{tr, trn};
use cr_store::treasure::{
    ADD_CATEGORIES, Bundle, Item, MoveError, PendingMove, SCROLL_SPELL_LIMIT, SpellCopy,
    TreasureRules, list_cmp,
};

/// Where an item is: the indices of the items that hold it, from the
/// top, then its own. The empty path is the bundle itself.
pub type Path = Vec<usize>;

/// The category whose parts are counted out: Add asks how many.
const MONEY: &str = "Money";

/// The categories of Add Item, in English for the lookups, sorted as they
/// are shown, translated.
pub fn categories() -> Vec<&'static str> {
    let mut categories: Vec<&str> = ADD_CATEGORIES.iter().map(|c| c.0).collect();
    categories.sort_by(|a, b| list_cmp(tr(a), tr(b)));
    categories
}

/// The parts `category` lists, sorted as the original's list box sorts
/// them.
pub fn parts(rules: &TreasureRules, category: &str) -> Vec<String> {
    let mut names = rules.category_parts(category);
    names.sort_by(|a, b| list_cmp(a, b));
    names
}

/// Whether Add asks how many of a part of `category`.
pub fn is_money(category: &str) -> bool {
    category == MONEY
}

/// Edit Scrolls refuses anything but a scroll (0x5452aa), so it is
/// offered only where it can work or explains why not.
pub fn edit_spells_offered(item: &Item) -> bool {
    item.extra.is_some()
        || item.name == "Spell Scroll, Wizard"
        || item.name == "Spell Scroll, Priest"
}

/// The pages a scroll or spellbook has used of its capacity, when it
/// holds spells or has pages; empty otherwise.
pub fn pages_text(item: &Item) -> String {
    match item.extra.as_ref().map(|e| e.unknown_0x4) {
        Some([capacity, used, _]) if !item.spells().is_empty() || capacity > 0 => {
            tr!("{used} of {capacity} pages used", used, capacity)
        }
        _ => String::new(),
    }
}

/// The pages a spell takes on a scroll.
pub fn spell_pages_text(pages: u32) -> String {
    trn!("{n} page", "{n} pages", pages)
}

/// A node a move or a copy may go to.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub path: Path,
    pub label: String,
    /// How deep it is: 0 for the top ("Treasure").
    pub depth: usize,
}

/// Every node a drop could go to: the top, then each item, under the one
/// that holds it.
pub fn destinations(bundle: &Bundle) -> Vec<Node> {
    fn walk(bundle: &Bundle, path: &mut Path, depth: usize, out: &mut Vec<Node>) {
        for (i, item) in bundle.items.iter().enumerate() {
            path.push(i);
            out.push(Node {
                path: path.clone(),
                label: item.label(),
                depth,
            });
            if let Some(contents) = &item.contents {
                walk(contents, path, depth + 1, out);
            }
            path.pop();
        }
    }
    let mut out = vec![Node {
        path: Vec::new(),
        label: tr("Treasure").to_owned(),
        depth: 0,
    }];
    walk(bundle, &mut Vec::new(), 1, &mut out);
    out
}

/// Where the item at `from` may move: anywhere but itself and the item
/// that holds it.
pub fn move_targets(bundle: &Bundle, from: &[usize]) -> Vec<Node> {
    let holder = &from[..from.len().saturating_sub(1)];
    destinations(bundle)
        .into_iter()
        .filter(|n| n.path != from && n.path != holder)
        .collect()
}

/// Where a spell of the scroll at `from` may be copied: any other item.
pub fn copy_targets(bundle: &Bundle, from: &[usize]) -> Vec<Node> {
    destinations(bundle)
        .into_iter()
        .filter(|n| !n.path.is_empty() && n.path != from)
        .collect()
}

/// Add Item (0x543a6b): a part of a category, at the top of the bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct AddItem {
    categories: Vec<&'static str>,
    pub category: usize,
    /// Every part of the category, sorted.
    parts: Vec<String>,
    pub search: String,
    /// The part chosen for Add.
    pub chosen: Option<String>,
}

impl AddItem {
    /// The categories, translated.
    pub fn category_names(&self) -> Vec<String> {
        self.categories.iter().map(|c| tr(c).to_owned()).collect()
    }

    /// The parts of the category that hold the search, case ignored.
    pub fn shown(&self) -> Vec<String> {
        let needle = self.search.to_lowercase();
        self.parts
            .iter()
            .filter(|p| p.to_lowercase().contains(&needle))
            .cloned()
            .collect()
    }

    fn money(&self) -> bool {
        self.categories
            .get(self.category)
            .is_some_and(|c| is_money(c))
    }
}

/// The "How many?" dialog (IDD 291): a number from `min` to `max`.
#[derive(Debug, Clone, PartialEq)]
pub struct HowMany {
    pub title: &'static str,
    /// The button that goes on: Add, Remove or Move.
    pub action: &'static str,
    pub min: u32,
    pub max: u32,
    pub value: u32,
    then: Count,
}

#[derive(Debug, Clone, PartialEq)]
enum Count {
    Add(String),
    Remove(Path),
    Move(PendingMove),
}

/// A list of nodes to choose one from: where to move an item or copy a
/// spell.
#[derive(Debug, Clone, PartialEq)]
pub struct Choose {
    pub title: &'static str,
    /// The button that goes on: Move or Copy.
    pub action: &'static str,
    pub nodes: Vec<Node>,
    /// The node chosen, by its place.
    pub chosen: Option<usize>,
    then: Target,
}

#[derive(Debug, Clone, PartialEq)]
enum Target {
    Move(Path),
    Spell(Path, usize),
}

/// A Yes/No question, No the default as in the original.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub title: &'static str,
    pub text: &'static str,
    then: Yes,
}

#[derive(Debug, Clone, PartialEq)]
enum Yes {
    /// Copy the spell again with these answers.
    Spell {
        from: Path,
        index: usize,
        to: Path,
        holding: bool,
        pages: bool,
    },
}

/// Edit Scrolls (0x5452aa, AddRemoveDlg IDD 408): the spells on a priest
/// or wizard scroll, chosen from every spell of its kind.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollEdit {
    pub path: Path,
    pub wizard: bool,
    /// The scroll's name.
    pub title: String,
    /// Every spell of the scroll's kind, sorted.
    pub choices: Vec<String>,
    /// The spells chosen, sorted.
    pub selected: Vec<String>,
    /// The spell the box has chosen for Add.
    pub adding: Option<String>,
    /// Add asks whether to go past the scroll's limit.
    pub asking: bool,
}

/// What Edit Scrolls asks before Add puts more spells on a scroll than
/// it should take.
pub fn scroll_limit_text() -> &'static str {
    tr("Scrolls should contain a maximum of 6 spells.  Do you want to ignore this limit?")
}

impl ScrollEdit {
    /// The heading of the spells chosen, with how many more the scroll
    /// should take.
    pub fn heading(&self) -> String {
        let left = SCROLL_SPELL_LIMIT as i64 - self.selected.len() as i64;
        tr!("Selected Spells (Spells Left: {left})", left)
    }

    fn add(&mut self) {
        if let Some(name) = self.adding.take() {
            self.selected.push(name);
            self.selected.sort_by(|a, b| list_cmp(a, b));
        }
    }
}

/// The dialog an edit waits on.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Add(AddItem),
    HowMany(HowMany),
    Choose(Choose),
    /// Why an edit could not be made, with OK.
    Notice {
        title: String,
        text: String,
    },
    Ask(Ask),
    Scroll(ScrollEdit),
}

/// What the user did to the bundle or in the dialog of its edit.
#[derive(Debug, Clone, PartialEq)]
pub enum BundleInput {
    /// Add Item….
    Add,
    /// Add Item's category, by its place in the list.
    Category(usize),
    Search(String),
    /// The part chosen to add.
    Part(String),
    /// Remove… on an item.
    Remove(Path),
    /// Move Into… on an item.
    MoveInto(Path),
    /// Edit Spells… on a scroll.
    EditSpells(Path),
    /// Remove on a spell of the scroll at the path.
    RemoveSpell(Path, usize),
    /// Copy To… on a spell of the scroll at the path.
    CopySpell(Path, usize),
    /// The number of How many?.
    Count(u32),
    /// The node of the list chosen, by its place.
    Choose(usize),
    /// Edit Scrolls: a spell removed, by its place.
    ScrollRemove(usize),
    /// Edit Scrolls: the spell the box chose for Add.
    ScrollAdding(String),
    ScrollAdd,
    /// The answer to whether Add goes past the scroll's limit.
    ScrollLimit(bool),
}

/// The bundle editor's state: the dialog it waits on, if any.
#[derive(Clone)]
pub struct BundleEditor {
    rules: Arc<TreasureRules>,
    step: Option<Step>,
    /// Counts the dialogs opened, so that one asked right after another
    /// is a dialog of its own and an answer to an old one is let go.
    serial: u64,
}

impl BundleEditor {
    pub fn new(rules: Arc<TreasureRules>) -> Self {
        Self {
            rules,
            step: None,
            serial: 0,
        }
    }

    pub fn rules(&self) -> &Arc<TreasureRules> {
        &self.rules
    }

    /// The dialog the edit waits on.
    pub fn step(&self) -> Option<&Step> {
        self.step.as_ref()
    }

    pub fn serial(&self) -> u64 {
        self.serial
    }

    fn open(&mut self, step: Step) {
        self.serial += 1;
        self.step = Some(step);
    }

    fn notice(&mut self, title: &str, text: impl Into<String>) {
        self.open(Step::Notice {
            title: title.to_owned(),
            text: text.into(),
        });
    }

    fn how_many(&mut self, title: &'static str, action: &'static str, max: u32, then: Count) {
        let value = if matches!(then, Count::Add(_)) {
            1
        } else {
            max
        };
        self.open(Step::HowMany(HowMany {
            title,
            action,
            min: 0,
            max,
            value,
            then,
        }));
    }

    /// Does what `input` asks to `bundle`; the dialogs' lists and boxes
    /// change the step.
    pub fn input(&mut self, bundle: &mut Bundle, input: BundleInput) {
        match input {
            BundleInput::Add => {
                let categories = categories();
                let parts = categories
                    .first()
                    .map(|c| parts(&self.rules, c))
                    .unwrap_or_default();
                self.open(Step::Add(AddItem {
                    categories,
                    category: 0,
                    parts,
                    search: String::new(),
                    chosen: None,
                }));
            }
            BundleInput::Category(i) => {
                if let Some(Step::Add(add)) = &mut self.step
                    && let Some(category) = add.categories.get(i)
                {
                    add.category = i;
                    add.parts = parts(&self.rules, category);
                    add.chosen = None;
                }
            }
            BundleInput::Search(text) => {
                if let Some(Step::Add(add)) = &mut self.step {
                    add.search = text;
                }
            }
            BundleInput::Part(name) => {
                if let Some(Step::Add(add)) = &mut self.step {
                    add.chosen = Some(name);
                }
            }
            // Remove (0x543c0f): asks how many when there is more than
            // one.
            BundleInput::Remove(path) => match bundle.item(&path).map(|i| i.quantity) {
                Some(quantity) if quantity > 1 => self.how_many(
                    tr("How many to delete?"),
                    tr("_Remove"),
                    quantity,
                    Count::Remove(path),
                ),
                Some(_) => bundle.remove_count(&path, 1),
                None => {}
            },
            BundleInput::MoveInto(path) if !path.is_empty() => {
                let nodes = move_targets(bundle, &path);
                self.open(Step::Choose(Choose {
                    title: tr("Move Into"),
                    action: tr("_Move"),
                    nodes,
                    chosen: None,
                    then: Target::Move(path),
                }));
            }
            BundleInput::MoveInto(_) => {}
            BundleInput::CopySpell(path, index) => {
                let nodes = copy_targets(bundle, &path);
                self.open(Step::Choose(Choose {
                    title: tr("Copy Spell To"),
                    action: tr("_Copy"),
                    nodes,
                    chosen: None,
                    then: Target::Spell(path, index),
                }));
            }
            BundleInput::RemoveSpell(path, index) => {
                bundle.remove_spell(&path, index);
            }
            BundleInput::EditSpells(path) => self.edit_spells(bundle, path),
            BundleInput::Count(n) => {
                if let Some(Step::HowMany(how)) = &mut self.step {
                    how.value = n.clamp(how.min, how.max);
                }
            }
            BundleInput::Choose(i) => {
                if let Some(Step::Choose(choose)) = &mut self.step
                    && i < choose.nodes.len()
                {
                    choose.chosen = Some(i);
                }
            }
            BundleInput::ScrollRemove(i) => {
                if let Some(Step::Scroll(scroll)) = &mut self.step
                    && i < scroll.selected.len()
                {
                    scroll.selected.remove(i);
                }
            }
            BundleInput::ScrollAdding(name) => {
                if let Some(Step::Scroll(scroll)) = &mut self.step {
                    scroll.adding = Some(name);
                }
            }
            BundleInput::ScrollAdd => {
                if let Some(Step::Scroll(scroll)) = &mut self.step
                    && scroll.adding.is_some()
                {
                    if scroll.selected.len() >= SCROLL_SPELL_LIMIT {
                        scroll.asking = true;
                    } else {
                        scroll.add();
                    }
                }
            }
            BundleInput::ScrollLimit(yes) => {
                if let Some(Step::Scroll(scroll)) = &mut self.step
                    && scroll.asking
                {
                    scroll.asking = false;
                    if yes {
                        scroll.add();
                    }
                }
            }
        }
    }

    /// The answer to the dialog numbered `serial`: `yes` for the button
    /// that goes on (OK, Yes, Done, How many?'s Add, Remove or Move), no
    /// for Cancel, No or a dialog closed. An answer to a dialog no longer
    /// open is let go.
    pub fn answer(&mut self, bundle: &mut Bundle, serial: u64, yes: bool) {
        if serial != self.serial {
            return;
        }
        let Some(step) = self.step.take() else {
            return;
        };
        match step {
            Step::Add(add) if yes => {
                let Some(name) = add.chosen.clone() else {
                    return;
                };
                if add.money() {
                    self.how_many(tr("How many to add?"), tr("_Add"), 10000, Count::Add(name));
                } else {
                    bundle.add_part(&name, 1);
                }
            }
            Step::Choose(choose) if yes => {
                let Some(to) = choose
                    .chosen
                    .and_then(|i| choose.nodes.get(i))
                    .map(|n| n.path.clone())
                else {
                    return;
                };
                match choose.then {
                    Target::Move(from) => self.begin_move(bundle, &from, &to),
                    Target::Spell(from, index) => {
                        self.copy_spell(bundle, from, index, to, false, false)
                    }
                }
            }
            Step::HowMany(how) if yes => match how.then {
                Count::Add(name) => bundle.add_part(&name, how.value),
                Count::Remove(path) => bundle.remove_count(&path, how.value),
                Count::Move(pending) => bundle.finish_move(&pending, how.value),
            },
            Step::Ask(ask) if yes => match ask.then {
                Yes::Spell {
                    from,
                    index,
                    to,
                    holding,
                    pages,
                } => self.copy_spell(bundle, from, index, to, holding, pages),
            },
            Step::Scroll(scroll) if yes => {
                if let Some(item) = bundle.item_mut(&scroll.path) {
                    item.set_scroll_spells(&scroll.selected, scroll.wizard);
                }
            }
            _ => {}
        }
    }

    /// Moving an item into another, the original's drag and drop
    /// (0x4ab3c2).
    fn begin_move(&mut self, bundle: &mut Bundle, from: &[usize], to: &[usize]) {
        match bundle.begin_move(from, to) {
            Err(e @ (MoveError::IntoItself | MoveError::IntoSpell)) => {
                self.notice(tr("Cannot Move"), e.to_string())
            }
            Ok(None) => {}
            Ok(Some(pending)) if pending.quantity > 1 => self.how_many(
                tr("How many to move?"),
                tr("_Move"),
                pending.quantity,
                Count::Move(pending),
            ),
            Ok(Some(pending)) => bundle.finish_move(&pending, 1),
        }
    }

    fn copy_spell(
        &mut self,
        bundle: &mut Bundle,
        from: Path,
        index: usize,
        to: Path,
        holding: bool,
        pages: bool,
    ) {
        let Ok(outcome) = bundle.copy_spell(&from, index, &to, holding, pages) else {
            return;
        };
        let again = |holding, pages| Yes::Spell {
            from: from.clone(),
            index,
            to: to.clone(),
            holding,
            pages,
        };
        match outcome {
            SpellCopy::NotHolding => self.open(Step::Ask(Ask {
                title: tr("Cannot Hold Spells"),
                text: tr("The destination is not marked as being able to hold spells. Override?"),
                then: again(true, false),
            })),
            SpellCopy::NoPages => self.open(Step::Ask(Ask {
                title: tr("No Pages Left"),
                text: tr(
                    "The target doesn't have pages left to copy the spell to.  Add needed pages?",
                ),
                then: again(true, true),
            })),
            SpellCopy::Nothing | SpellCopy::Copied => {}
        }
    }

    /// Edit Scrolls: the check may make the scroll able to hold spells,
    /// which stays even when it refuses.
    fn edit_spells(&mut self, bundle: &mut Bundle, path: Path) {
        let Some(item) = bundle.item_mut(&path) else {
            return;
        };
        let prepared = item.prepare_scroll_edit();
        let mut selected = item.spell_names();
        let title = item.name.clone();
        let wizard = match prepared {
            Ok(wizard) => wizard,
            Err(e) => return self.notice(e.title(), e.message()),
        };
        let mut choices = self.rules.scroll_spell_choices(wizard);
        choices.sort_by(|a, b| list_cmp(a, b));
        selected.sort_by(|a, b| list_cmp(a, b));
        self.open(Step::Scroll(ScrollEdit {
            path,
            wizard,
            title,
            choices,
            selected,
            adding: None,
            asking: false,
        }));
    }
}

/// A bundle's item as a list shows it, with what it holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub path: Path,
    pub title: String,
    /// The pages used, for a scroll or spellbook.
    pub subtitle: String,
    /// A line that is not an item ("Gems and Art", a weapon's ability),
    /// shown dimmed.
    pub flag: bool,
    pub edit_spells: bool,
    /// The spells on it, with the pages each takes.
    pub spells: Vec<(String, String)>,
    pub rows: Vec<Row>,
}

/// The bundle's items as rows, those an item holds under it.
pub fn rows(bundle: &Bundle) -> Vec<Row> {
    fn of(bundle: &Bundle, path: &mut Path) -> Vec<Row> {
        bundle
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                path.push(i);
                let row = Row {
                    path: path.clone(),
                    title: item.label(),
                    subtitle: pages_text(item),
                    flag: item.flag,
                    edit_spells: edit_spells_offered(item),
                    spells: item
                        .spells()
                        .into_iter()
                        .map(|s| (s.name, spell_pages_text(s.pages)))
                        .collect(),
                    rows: item
                        .contents
                        .as_ref()
                        .map(|c| of(c, path))
                        .unwrap_or_default(),
                };
                path.pop();
                row
            })
            .collect()
    }
    of(bundle, &mut Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Option<Arc<TreasureRules>> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        let install = cr_store::Installation::open(std::path::Path::new(&dir)).ok()?;
        install.treasure_rules().ok().map(Arc::new)
    }

    #[test]
    fn money_is_counted_out_and_items_are_added_one_at_a_time() {
        let Some(rules) = rules() else {
            return;
        };
        let mut editor = BundleEditor::new(rules);
        let mut bundle = Bundle::default();
        editor.input(&mut bundle, BundleInput::Add);
        let Some(Step::Add(add)) = editor.step() else {
            panic!("Add Item opens");
        };
        let money = add
            .categories
            .iter()
            .position(|c| is_money(c))
            .expect("Money");
        let other = usize::from(money == 0);
        editor.input(&mut bundle, BundleInput::Category(other));
        let Some(Step::Add(add)) = editor.step() else {
            panic!()
        };
        let part = add.shown()[0].clone();
        editor.input(&mut bundle, BundleInput::Part(part.clone()));
        editor.answer(&mut bundle, editor.serial(), true);
        assert!(editor.step().is_none());
        assert_eq!(bundle.items[0].name, part);

        editor.input(&mut bundle, BundleInput::Add);
        editor.input(&mut bundle, BundleInput::Category(money));
        let Some(Step::Add(add)) = editor.step() else {
            panic!()
        };
        let coin = add.shown()[0].clone();
        editor.input(&mut bundle, BundleInput::Part(coin.clone()));
        editor.answer(&mut bundle, editor.serial(), true);
        let serial = editor.serial();
        assert!(matches!(editor.step(), Some(Step::HowMany(_))));
        editor.input(&mut bundle, BundleInput::Count(250));
        // A late answer to Add Item changes nothing.
        editor.answer(&mut bundle, serial - 1, true);
        editor.answer(&mut bundle, serial, true);
        let coins = bundle.items.iter().find(|i| i.name == coin).unwrap();
        assert_eq!(coins.quantity, 250);

        // Removing some of them asks how many.
        let at = bundle.items.iter().position(|i| i.name == coin).unwrap();
        editor.input(&mut bundle, BundleInput::Remove(vec![at]));
        editor.input(&mut bundle, BundleInput::Count(50));
        editor.answer(&mut bundle, editor.serial(), true);
        assert_eq!(bundle.items[at].quantity, 200);
    }

    #[test]
    fn an_item_moves_into_another_and_not_into_itself() {
        let Some(rules) = rules() else {
            return;
        };
        let mut editor = BundleEditor::new(rules);
        let mut bundle = Bundle::default();
        bundle.add_part("Backpack", 1);
        bundle.add_part("Rope", 1);
        let rope = bundle.items.iter().position(|i| i.name == "Rope").unwrap();
        let backpack = 1 - rope;
        assert!(
            !move_targets(&bundle, &[rope])
                .iter()
                .any(|n| n.path == [rope])
        );
        editor.input(&mut bundle, BundleInput::MoveInto(vec![rope]));
        let Some(Step::Choose(choose)) = editor.step() else {
            panic!("Move Into opens");
        };
        let to = choose
            .nodes
            .iter()
            .position(|n| n.path == [backpack])
            .unwrap();
        editor.input(&mut bundle, BundleInput::Choose(to));
        editor.answer(&mut bundle, editor.serial(), true);
        assert_eq!(bundle.items.len(), 1);
        let rows = rows(&bundle);
        assert_eq!(rows[0].rows[0].title, "Rope");
        assert_eq!(rows[0].rows[0].path, [0, 0]);
    }
}
