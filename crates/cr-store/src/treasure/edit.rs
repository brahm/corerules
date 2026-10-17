//! Editing a bundle as the Treasure Bundle Editor does (IDD 290,
//! `TreasureRollerEditorDlg`, 0x543790): the Encounter dialog's Items…
//! button opens it on the encounter's items, the roller on its treasure.
//! Nodes are addressed by paths of item indices from the top; the empty
//! path is the root ("Treasure"), which holds the bundle itself.

use cr_formats::inventory::InventoryExtra;

use super::{Bundle, Item, TreasureRules};

/// The editor's category drop-down (0x543fc2): "Money" and "Count", then
/// the app's category names, each with the part types it lists and
/// whether standard and custom parts are in (0x544105, 0x465218). The
/// drop-down sorts them; it starts on the first ("Armor, All").
pub const ADD_CATEGORIES: [(&str, usize, bool, bool); 17] = [
    ("Money", 16, true, true),
    ("Count", 17, true, true),
    ("Armor, All", 1, true, true),
    ("Armor, Standard", 1, true, false),
    ("Armor, Custom", 1, false, true),
    ("Weapons, All", 0, true, true),
    ("Weapons, Standard", 0, true, false),
    ("Weapons, Custom", 0, false, true),
    ("Equipment, All", 7, true, true),
    ("Equipment, Standard", 7, true, false),
    ("Equipment, Custom", 7, false, true),
    ("Gems and Art, All", 13, true, true),
    ("Gems and Art, Standard", 13, true, false),
    ("Gems and Art, Custom", 13, false, true),
    ("Magical Items, All", 8, true, true),
    ("Magical Items, Standard", 8, true, false),
    ("Magical Items, Custom", 8, false, true),
];

/// Edit Scrolls' limit (AddRemoveDlg 0x401010): more spells need a Yes to
/// "Scrolls should contain a maximum of 6 spells.  Do you want to ignore
/// this limit?".
pub const SCROLL_SPELL_LIMIT: usize = 6;

/// The order of a sorted Windows list box or combo box.
pub use crate::collate::list_cmp;

/// Why Edit Scrolls (0x5452aa) refuses, with the original's message box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollError {
    /// The root or a spell is selected.
    NotSelected,
    /// The item cannot hold spells.
    NotDesigned,
    /// The name has no "Scroll".
    NotAScroll,
    /// The name has neither "Priest" nor "Wizard".
    NotPriestOrWizard,
}

impl ScrollError {
    pub fn title(self) -> &'static str {
        match self {
            ScrollError::NotAScroll => "Not a Scroll",
            _ => "Not A Priest Or Wizard Scroll",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            ScrollError::NotSelected => {
                "You must first select a \"Spell Scroll, Priest\" or \"Spell Scroll, Wizard\"."
            }
            ScrollError::NotDesigned => {
                "Editing the scroll's spell list is only available for spell scrolls that are \
                 designed to hold spells (i.e. \"Spell Scroll, Priest\" and \"Spell Scroll, \
                 Wizard\"."
            }
            ScrollError::NotAScroll => "Editing of the spell list is only available for scrolls.",
            ScrollError::NotPriestOrWizard => {
                "Editing of the spell list is only available for Priest scrolls and Wizard scrolls."
            }
        }
    }
}

/// Why a drop (0x4ab3c2) is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoveError {
    #[error("You cannot place an item inside of itself.")]
    IntoItself,
    #[error("The selected destination is a spell. Spells cannot hold other objects.")]
    IntoSpell,
}

/// A move [`Bundle::begin_move`] has prepared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingMove {
    /// The item moved, where it now is.
    pub from: Vec<usize>,
    /// The destination.
    pub to: Vec<usize>,
    /// How many of the item there are.
    pub quantity: u32,
}

/// The outcome of copying a spell onto an item ([`Bundle::copy_spell`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellCopy {
    /// Nothing to do (the spell's own scroll, or a bad path).
    Nothing,
    Copied,
    /// The target holds no spells: "The destination is not marked as
    /// being able to hold spells. Override?".
    NotHolding,
    /// "The target doesn't have pages left to copy the spell to.  Add
    /// needed pages?".
    NoPages,
}

fn new_extra() -> InventoryExtra {
    InventoryExtra {
        unknown_0x4: [0; 3],
        unknown_0x10: Vec::new(),
        unknown_0x24: Vec::new(),
        unknown_0x38: Vec::new(),
    }
}

impl Item {
    /// Pages a scroll has left (0x4adcb8): capacity less used.
    pub fn pages_left(&self) -> i32 {
        self.extra.as_ref().map_or(0, |e| {
            (e.unknown_0x4[0] as i32).wrapping_sub(e.unknown_0x4[1] as i32)
        })
    }

    /// Writes a spell on the scroll (0x4ad755): refused when it needs more
    /// pages than are left; otherwise the count and the pages used grow.
    pub fn add_spell(&mut self, name: &str, pages: u32, wizard: bool) -> bool {
        let Some(e) = &mut self.extra else {
            return false;
        };
        let [capacity, used, count] = e.unknown_0x4;
        if (pages as i32).wrapping_add(used as i32) > capacity as i32 {
            return false;
        }
        e.unknown_0x4 = [capacity, used.wrapping_add(pages), count.wrapping_add(1)];
        e.unknown_0x10.push(name.to_string());
        e.unknown_0x24.push(pages);
        e.unknown_0x38.push(u8::from(wizard));
        true
    }

    /// Erases spell `index` (0x4ad7d0): the count and the pages used go
    /// down and the name and pages are removed, but the wizard flags are
    /// left as they are, so the flags after it no longer line up.
    pub fn remove_spell(&mut self, index: usize) -> bool {
        let Some(e) = &mut self.extra else {
            return false;
        };
        let [capacity, used, count] = e.unknown_0x4;
        if count == 0 || index >= count as usize || index >= e.unknown_0x10.len() {
            return false;
        }
        let pages = e.unknown_0x24.get(index).copied().unwrap_or(0);
        e.unknown_0x4 = [capacity, used.wrapping_sub(pages), count - 1];
        if index < e.unknown_0x24.len() {
            e.unknown_0x24.remove(index);
        }
        e.unknown_0x10.remove(index);
        true
    }

    /// Edit Scrolls' checks (0x5452aa), which come before its dialog: a
    /// "Spell Scroll, Wizard" or "Spell Scroll, Priest" that holds no
    /// spells is made able to; then the scroll must hold spells and be
    /// named a priest or wizard scroll. Gives whether it is a wizard's.
    pub fn prepare_scroll_edit(&mut self) -> Result<bool, ScrollError> {
        if self.extra.is_none()
            && (self.name == "Spell Scroll, Wizard" || self.name == "Spell Scroll, Priest")
        {
            self.extra = Some(new_extra());
        }
        if self.extra.is_none() {
            return Err(ScrollError::NotDesigned);
        }
        if !self.name.contains("Scroll") {
            return Err(ScrollError::NotAScroll);
        }
        if self.name.contains("Priest") {
            Ok(false)
        } else if self.name.contains("Wizard") {
            Ok(true)
        } else {
            Err(ScrollError::NotPriestOrWizard)
        }
    }

    /// The names Edit Scrolls' dialog starts with.
    pub fn spell_names(&self) -> Vec<String> {
        self.extra
            .as_ref()
            .map_or_else(Vec::new, |e| e.unknown_0x10.clone())
    }

    /// Edit Scrolls' OK: every spell is erased, then the chosen ones are
    /// written in the dialog's sorted order, one page each, the capacity
    /// set to the number written so far before each (so it ends at their
    /// number; with none it is left as it was).
    pub fn set_scroll_spells(&mut self, names: &[String], wizard: bool) {
        let mut names: Vec<&String> = names.iter().collect();
        names.sort_by(|a, b| list_cmp(a, b));
        let old = self.extra.as_ref().map_or(0, |e| e.unknown_0x4[2]);
        for _ in 0..old {
            self.remove_spell(0);
        }
        for (i, name) in names.into_iter().enumerate() {
            if let Some(e) = &mut self.extra {
                e.unknown_0x4[0] = i as u32 + 1;
            }
            self.add_spell(name, 1, wizard);
        }
    }
}

/// Moves `path` by one place when an item was put at `at` in the bundle
/// at `bundle`.
fn shift_after_insert(path: &mut [usize], bundle: &[usize], at: usize) {
    if path.len() > bundle.len() && path.starts_with(bundle) && path[bundle.len()] >= at {
        path[bundle.len()] += 1;
    }
}

/// Moves `path` back when the item at `removed` was taken out.
fn shift_after_remove(path: &mut [usize], removed: &[usize]) {
    let (parent, i) = (&removed[..removed.len() - 1], removed[removed.len() - 1]);
    if path.len() > parent.len() && path.starts_with(parent) && path[parent.len()] > i {
        path[parent.len()] -= 1;
    }
}

impl Bundle {
    /// The node at `path`.
    pub fn item(&self, path: &[usize]) -> Option<&Item> {
        let (&first, rest) = path.split_first()?;
        let item = self.items.get(first)?;
        if rest.is_empty() {
            Some(item)
        } else {
            item.contents.as_ref()?.item(rest)
        }
    }

    pub fn item_mut(&mut self, path: &[usize]) -> Option<&mut Item> {
        let (&first, rest) = path.split_first()?;
        let item = self.items.get_mut(first)?;
        if rest.is_empty() {
            Some(item)
        } else {
            item.contents.as_mut()?.item_mut(rest)
        }
    }

    /// What the node at `path` holds: the bundle itself for the root.
    fn contents_mut(&mut self, path: &[usize]) -> Option<&mut Bundle> {
        if path.is_empty() {
            Some(self)
        } else {
            self.item_mut(path)?.contents.as_mut()
        }
    }

    /// Like [`contents_mut`](Self::contents_mut), giving the node contents
    /// when it has none.
    fn ensure_contents(&mut self, path: &[usize]) -> Option<&mut Bundle> {
        if path.is_empty() {
            Some(self)
        } else {
            Some(
                self.item_mut(path)?
                    .contents
                    .get_or_insert_with(Bundle::default),
            )
        }
    }

    /// Where [`insert`](Self::insert) puts `item`, or `None` when it
    /// merges.
    fn insert_at(&mut self, item: Item, merge: bool) -> Option<usize> {
        let before = self.items.len();
        let key = super::ansi_bytes(&item.name);
        let at = self
            .items
            .iter()
            .position(|e| super::ansi_bytes(&e.name) > key)
            .unwrap_or(before);
        self.insert(item, merge);
        (self.items.len() > before).then_some(at)
    }

    /// Takes the item at `path` out.
    fn take(&mut self, path: &[usize]) -> Option<Item> {
        let (&last, parent) = path.split_last()?;
        let bundle = self.contents_mut(parent)?;
        (last < bundle.items.len()).then(|| bundle.items.remove(last))
    }

    /// The Add button (0x543a6b, 0x4aab4e): `quantity` of `name` (the
    /// "How many to add?" prompt, 0 to 10,000, is only for "Money"; it
    /// adds 1 otherwise) at the top of the bundle, merged with an item of
    /// that name.
    pub fn add_part(&mut self, name: &str, quantity: u32) {
        self.insert(Item::new(name, quantity), true);
    }

    /// The Remove button on an item (0x543c0f): `count` of it ("How many
    /// to delete?", 0 to its quantity, asked only when there is more than
    /// one). All of them removes the item and what it holds.
    pub fn remove_count(&mut self, path: &[usize], count: u32) {
        let Some(item) = self.item_mut(path) else {
            return;
        };
        if count == 0 {
        } else if count >= item.quantity {
            self.take(path);
        } else {
            item.quantity -= count;
        }
    }

    /// The Remove button on a spell (0x543c0f, 0x4ad7d0).
    pub fn remove_spell(&mut self, path: &[usize], index: usize) -> bool {
        self.item_mut(path).is_some_and(|i| i.remove_spell(index))
    }

    /// The first half of dropping item `from` on node `to` (0x4ab3c2):
    /// `Ok(None)` when there is nothing to do (`to` is `from` or its
    /// holder). Otherwise a stack of several `to` loses one, which is put
    /// beside it as the new destination, and the destination is made able
    /// to hold items; this stays even when the move goes no further. When
    /// the [`PendingMove`]'s quantity is more than one the program asks
    /// "How many to move?" (0 to all of them) before
    /// [`finish_move`](Self::finish_move).
    pub fn begin_move(
        &mut self,
        from: &[usize],
        to: &[usize],
    ) -> Result<Option<PendingMove>, MoveError> {
        if from.is_empty()
            || from == to
            || to == &from[..from.len() - 1]
            || self.item(from).is_none()
            || (!to.is_empty() && self.item(to).is_none())
        {
            return Ok(None);
        }
        if to.starts_with(from) {
            return Err(MoveError::IntoItself);
        }
        let mut from = from.to_vec();
        let mut to = to.to_vec();
        if let Some(target) = self.item_mut(&to)
            && target.quantity > 1
        {
            target.quantity -= 1;
            let mut single = target.clone();
            single.quantity = 1;
            single.contents.get_or_insert_with(Bundle::default);
            let parent = to[..to.len() - 1].to_vec();
            if let Some(at) = self
                .contents_mut(&parent)
                .and_then(|b| b.insert_at(single, true))
            {
                shift_after_insert(&mut from, &parent, at);
                to = [parent.as_slice(), &[at]].concat();
            }
        }
        self.ensure_contents(&to);
        let quantity = self.item(&from).map_or(0, |i| i.quantity);
        Ok(Some(PendingMove { from, to, quantity }))
    }

    /// The second half of a move: `count` of the item (all of it when
    /// `count` is its quantity) go into the destination, merged with an
    /// item of that name there. 0 does nothing.
    pub fn finish_move(&mut self, pending: &PendingMove, count: u32) {
        let Some(source) = self.item(&pending.from) else {
            return;
        };
        if count == 0 {
            return;
        }
        let mut moved = source.clone();
        let mut to = pending.to.clone();
        if count >= source.quantity {
            self.take(&pending.from);
            shift_after_remove(&mut to, &pending.from);
        } else {
            moved.quantity = count;
            if let Some(s) = self.item_mut(&pending.from) {
                s.quantity -= count;
            }
        }
        if let Some(b) = self.ensure_contents(&to) {
            b.insert(moved, true);
        }
    }

    /// Dropping spell `index` of the scroll at `from` on node `to`
    /// (0x4ab3c2): the spell is copied, keeping its pages and wizard flag.
    /// `make_holding` answers Yes to "...not marked as being able to hold
    /// spells. Override?" and `add_pages` to "...Add needed pages?", which
    /// grows the capacity by the pages missing.
    pub fn copy_spell(
        &mut self,
        from: &[usize],
        index: usize,
        to: &[usize],
        make_holding: bool,
        add_pages: bool,
    ) -> Result<SpellCopy, MoveError> {
        if to.is_empty() || from == to {
            return Ok(SpellCopy::Nothing);
        }
        let Some(spell) = self
            .item(from)
            .and_then(|i| i.spells().into_iter().nth(index))
        else {
            return Ok(SpellCopy::Nothing);
        };
        let Some(target) = self.item_mut(to) else {
            return Ok(SpellCopy::Nothing);
        };
        if target.extra.is_none() {
            if !make_holding {
                return Ok(SpellCopy::NotHolding);
            }
            target.extra = Some(new_extra());
        }
        if target.add_spell(&spell.name, spell.pages, spell.wizard) {
            return Ok(SpellCopy::Copied);
        }
        if !add_pages {
            return Ok(SpellCopy::NoPages);
        }
        let missing = (spell.pages as i32).wrapping_sub(target.pages_left());
        if let Some(e) = &mut target.extra {
            e.unknown_0x4[0] = (e.unknown_0x4[0] as i32).wrapping_add(missing) as u32;
        }
        Ok(
            if target.add_spell(&spell.name, spell.pages, spell.wizard) {
                SpellCopy::Copied
            } else {
                SpellCopy::NoPages
            },
        )
    }
}

impl TreasureRules {
    /// The parts a category of [`ADD_CATEGORIES`] lists: standard ones
    /// then custom ones, in file order (the list box sorts them; see
    /// [`list_cmp`]).
    pub fn category_parts(&self, category: &str) -> Vec<String> {
        let Some(&(_, t, standard, custom)) = ADD_CATEGORIES.iter().find(|c| c.0 == category)
        else {
            return Vec::new();
        };
        let mut names = Vec::new();
        if standard {
            names.extend(self.part_names[1][t].iter().cloned());
        }
        if custom {
            names.extend(self.part_names[0][t].iter().cloned());
        }
        names
    }

    /// The spells Edit Scrolls offers for a priest or wizard scroll: every
    /// level, standard spells then custom ones (0x47298c, 0x47295c).
    pub fn scroll_spell_choices(&self, wizard: bool) -> Vec<String> {
        self.spells
            .iter()
            .flatten()
            .filter(|(w, _, _)| *w == wizard)
            .map(|(_, _, n)| n.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scroll(names: &[(&str, u32, bool)]) -> Item {
        let mut item = Item::new("Spell Scroll, Wizard", 1);
        item.extra = Some(new_extra());
        let pages: u32 = names.iter().map(|n| n.1).sum();
        item.extra.as_mut().unwrap().unknown_0x4[0] = pages;
        for &(n, p, w) in names {
            assert!(item.add_spell(n, p, w));
        }
        item
    }

    #[test]
    fn spells_add_and_remove() {
        let mut s = scroll(&[("A", 2, true), ("B", 3, false), ("C", 1, true)]);
        assert_eq!(s.extra.as_ref().unwrap().unknown_0x4, [6, 6, 3]);
        assert!(!s.add_spell("D", 1, true));
        assert!(s.remove_spell(1));
        let e = s.extra.as_ref().unwrap();
        assert_eq!(e.unknown_0x4, [6, 3, 2]);
        assert_eq!(e.unknown_0x10, ["A", "C"]);
        assert_eq!(e.unknown_0x24, [2, 1]);
        // The flags stay.
        assert_eq!(e.unknown_0x38, [1, 0, 1]);
        assert!(!s.remove_spell(2));
    }

    #[test]
    fn edit_scroll_rewrites_with_one_page_each() {
        let mut s = scroll(&[("Zeta", 4, true)]);
        assert_eq!(s.prepare_scroll_edit(), Ok(true));
        s.set_scroll_spells(&["b-c".into(), "Ab".into(), "ab".into()], true);
        let e = s.extra.as_ref().unwrap();
        assert_eq!(e.unknown_0x4, [3, 3, 3]);
        assert_eq!(e.unknown_0x10, ["Ab", "ab", "b-c"]);
        assert_eq!(e.unknown_0x24, [1, 1, 1]);
        assert_eq!(e.unknown_0x38, [1, 1, 1, 1]);
        // None chosen: the capacity stays.
        s.set_scroll_spells(&[], true);
        assert_eq!(s.extra.as_ref().unwrap().unknown_0x4, [3, 0, 0]);
    }

    #[test]
    fn edit_scroll_checks() {
        let mut i = Item::new("Spell Scroll, Priest", 1);
        assert_eq!(i.prepare_scroll_edit(), Ok(false));
        assert!(i.extra.is_some());
        let mut i = Item::new("Scroll of Protection", 1);
        assert_eq!(i.prepare_scroll_edit(), Err(ScrollError::NotDesigned));
        i.extra = Some(new_extra());
        assert_eq!(i.prepare_scroll_edit(), Err(ScrollError::NotPriestOrWizard));
        let mut i = Item::new("Wizard Book", 1);
        i.extra = Some(new_extra());
        assert_eq!(i.prepare_scroll_edit(), Err(ScrollError::NotAScroll));
    }

    fn tree() -> Bundle {
        let mut b = Bundle::default();
        b.insert(Item::new("Backpack", 2), true);
        b.insert(Item::new("Rope", 5), true);
        b.insert(Item::new("Torch", 1), true);
        b
    }

    #[test]
    fn add_and_remove() {
        let mut b = tree();
        b.add_part("Rope", 3);
        assert_eq!(b.items[1].quantity, 8);
        b.remove_count(&[1], 0);
        assert_eq!(b.items[1].quantity, 8);
        b.remove_count(&[1], 6);
        assert_eq!(b.items[1].quantity, 2);
        b.remove_count(&[1], 2);
        assert_eq!(b.items.len(), 2);
    }

    #[test]
    fn move_splits_the_destination() {
        let mut b = tree();
        // Rope x5 onto Backpack x2.
        let m = b.begin_move(&[1], &[0]).unwrap().unwrap();
        assert_eq!(m.quantity, 5);
        // Backpack x1, then the new Backpack holding nothing yet.
        assert_eq!((m.from.as_slice(), m.to.as_slice()), (&[2][..], &[1][..]));
        assert_eq!(b.items[0].quantity, 1);
        assert!(b.items[0].contents.is_none());
        assert_eq!(b.items[1].contents, Some(Bundle::default()));
        b.finish_move(&m, 2);
        assert_eq!(b.items[2].name, "Rope");
        assert_eq!(b.items[2].quantity, 3);
        let inside = &b.items[1].contents.as_ref().unwrap().items;
        assert_eq!((inside[0].name.as_str(), inside[0].quantity), ("Rope", 2));
        // All of the rest into the same backpack.
        let m = b.begin_move(&[2], &[1]).unwrap().unwrap();
        b.finish_move(&m, 3);
        assert_eq!(b.items.len(), 3);
        assert_eq!(b.items[1].contents.as_ref().unwrap().items[0].quantity, 5);
        // Into itself, and onto its holder.
        assert_eq!(b.begin_move(&[1], &[1, 0]), Err(MoveError::IntoItself));
        assert_eq!(b.begin_move(&[1, 0], &[1]), Ok(None));
        // Out to the top.
        let m = b.begin_move(&[1, 0], &[]).unwrap().unwrap();
        b.finish_move(&m, 5);
        assert_eq!(
            b.items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["Backpack", "Backpack", "Rope", "Torch"]
        );
    }

    #[test]
    fn spell_copies() {
        let mut b = Bundle::default();
        b.insert(scroll(&[("Sleep", 2, true)]), false);
        b.insert(Item::new("Torch", 1), true);
        assert_eq!(
            b.copy_spell(&[0], 0, &[1], false, false),
            Ok(SpellCopy::NotHolding)
        );
        assert!(b.items[1].extra.is_none());
        assert_eq!(
            b.copy_spell(&[0], 0, &[1], true, false),
            Ok(SpellCopy::NoPages)
        );
        assert_eq!(
            b.copy_spell(&[0], 0, &[1], true, true),
            Ok(SpellCopy::Copied)
        );
        let e = b.items[1].extra.as_ref().unwrap();
        assert_eq!(e.unknown_0x4, [2, 2, 1]);
        assert_eq!(b.items[0].spells().len(), 1);
    }

    #[test]
    fn word_sort() {
        let mut v = vec!["Wall of Fire", "wall-walk", "Wall", "Walls"];
        v.sort_by(|a, b| list_cmp(a, b));
        assert_eq!(v, ["Wall", "Wall of Fire", "Walls", "wall-walk"]);
    }
}
