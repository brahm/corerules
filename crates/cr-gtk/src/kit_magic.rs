//! The character editor's kit magic on the Spells page: the schools the
//! kit bars or has the character eliminate, the spheres it lets the
//! character keep and the free one, as the original's Spells and Memorize
//! pages ask about them (see [`cr_store::kit_magic`]); and its warning
//! before spells are memorized.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::kit_magic::{CharacterKitMagic, KitMagicRules};
use gtk::glib;

use crate::create::{combo_row, combo_value, ignorable};

/// What is called when the choices change.
type Listeners = Rc<RefCell<Vec<Rc<dyn Fn()>>>>;

/// The page's state, cheap to clone into callbacks.
#[derive(Clone)]
pub struct KitMagicEditor {
    rules: Rc<KitMagicRules>,
    character: Rc<Character>,
    /// The kit chosen on the Details page.
    kit: Rc<dyn Fn() -> String>,
    new: Rc<RefCell<CharacterKitMagic>>,
    pub page: adw::PreferencesPage,
    /// The kit's groups, first on the page.
    groups: Rc<RefCell<Vec<adw::PreferencesGroup>>>,
    /// The page's other groups, after them.
    rest: Rc<RefCell<Vec<adw::PreferencesGroup>>>,
    /// Called when the choices change.
    listeners: Listeners,
}

/// A group of names to check, at most `most` of them; `changed` gets the
/// names checked.
pub(crate) fn check_group(
    title: &str,
    description: &str,
    names: &[String],
    checked: &[String],
    most: usize,
    changed: impl Fn(Vec<String>) + 'static,
) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(glib::markup_escape_text(title).as_str())
        .description(glib::markup_escape_text(description).as_str())
        .build();
    let checks: Rc<RefCell<Vec<(String, gtk::CheckButton)>>> = Rc::default();
    let limit = {
        let checks = checks.clone();
        move || {
            let checks = checks.borrow();
            let full = checks.iter().filter(|(_, c)| c.is_active()).count() >= most;
            for (_, check) in checks.iter() {
                check.set_sensitive(check.is_active() || !full);
            }
        }
    };
    let changed = Rc::new(changed);
    for name in names {
        let check = gtk::CheckButton::builder()
            .active(checked.contains(name))
            .valign(gtk::Align::Center)
            .build();
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(name).as_str())
            .activatable_widget(&check)
            .build();
        row.add_prefix(&check);
        group.add(&row);
        checks.borrow_mut().push((name.clone(), check.clone()));
        check.connect_toggled({
            let (checks, limit, changed) = (checks.clone(), limit.clone(), changed.clone());
            move |_| {
                let names = checks
                    .borrow()
                    .iter()
                    .filter(|(_, c)| c.is_active())
                    .map(|(n, _)| n.clone())
                    .collect();
                changed(names);
                limit();
            }
        });
    }
    limit();
    group
}

impl KitMagicEditor {
    pub fn new(
        rules: Rc<KitMagicRules>,
        character: &Character,
        kit: Rc<dyn Fn() -> String>,
    ) -> Self {
        let new = rules.magic_of(character);
        Self {
            rules,
            character: Rc::new(character.clone()),
            kit,
            new: Rc::new(RefCell::new(new)),
            page: adw::PreferencesPage::new(),
            groups: Rc::default(),
            rest: Rc::default(),
            listeners: Rc::default(),
        }
    }

    /// Puts `rest` on the page after the kit's groups.
    pub fn set_rest(&self, rest: Vec<adw::PreferencesGroup>) {
        *self.rest.borrow_mut() = rest;
        self.rebuild();
    }

    /// Calls `f` when the choices or the kit change.
    pub fn connect_changed(&self, f: impl Fn() + 'static) {
        self.listeners.borrow_mut().push(Rc::new(f));
    }

    fn changed(&self) {
        let listeners = self.listeners.borrow().clone();
        for f in listeners {
            f();
        }
    }

    /// The choices as edited.
    pub fn magic(&self) -> CharacterKitMagic {
        self.new.borrow().clone()
    }

    pub fn kit(&self) -> String {
        (self.kit)()
    }

    /// What the Abilities page tells about the kit's schools (`list` 0)
    /// or minor (1) or major (2) spheres, with `taken` of them.
    pub fn notices(&self, list: usize, taken: usize) -> Vec<String> {
        let (kit, new) = (self.kit(), self.new.borrow());
        match list {
            0 => self.rules.schools_notice(&kit, &new).into_iter().collect(),
            _ => self.rules.spheres_notice(&kit, &new, list == 2, taken),
        }
    }

    /// Shows the kit's groups again, once the kit or a choice changed.
    pub fn rebuild(&self) {
        for group in self.groups.borrow_mut().drain(..) {
            self.page.remove(&group);
        }
        for group in self.rest.borrow().iter() {
            self.page.remove(group);
        }
        let groups = self.kit_groups();
        for group in groups.iter().chain(self.rest.borrow().iter()) {
            self.page.add(group);
        }
        *self.groups.borrow_mut() = groups;
        self.changed();
    }

    fn kit_groups(&self) -> Vec<adw::PreferencesGroup> {
        let kit = self.kit();
        let (rules, character) = (&self.rules, &*self.character);
        let mut groups = Vec::new();
        let new = self.magic();
        let class = |slot: usize| character.data.classes.names[slot].clone();

        // The barred schools go unless ignored (63056f).
        if let Some(warning) = rules.barred_schools_warning(character, &kit) {
            let group = adw::PreferencesGroup::builder()
                .title(tr("Kit Class Ability Requirements"))
                .description(glib::markup_escape_text(&warning).as_str())
                .build();
            let ignore = adw::SwitchRow::builder()
                .title(tr("Ignore"))
                .subtitle(tr(
                    "Keeps these schools; otherwise they are removed when the character is saved.",
                ))
                .active(new.barred_ignored)
                .build();
            ignore.connect_active_notify({
                let this = self.clone();
                move |row| {
                    this.new.borrow_mut().barred_ignored = row.is_active();
                    // The schools to eliminate depend on those kept.
                    glib::idle_add_local_once({
                        let this = this.clone();
                        move || this.rebuild()
                    });
                }
            });
            group.add(&ignore);
            groups.push(group);
        }

        // The schools beyond five, eliminated once (6309df).
        if let Some((count, schools)) = rules.schools_to_eliminate(character, &kit, &new) {
            let checked: Vec<String> = new
                .eliminated
                .iter()
                .filter(|s| schools.contains(s))
                .cloned()
                .collect();
            self.new.borrow_mut().eliminated.clone_from(&checked);
            let this = self.clone();
            groups.push(check_group(
                tr("Eliminated Schools"),
                &KitMagicRules::elimination_text(&kit, count),
                &schools,
                &checked,
                count,
                move |names| {
                    this.new.borrow_mut().eliminated = names;
                    this.changed();
                },
            ));
        } else if !rules.magic_of(character).eliminated.is_empty() {
            // Only once: what the file has stays.
        } else {
            self.new.borrow_mut().eliminated.clear();
        }

        // The spheres to keep where there are more than the kit allows
        // (63286d), the first ones kept until chosen.
        let mut spheres = Vec::new();
        for pick in rules.sphere_picks(character, &kit) {
            let chosen = new
                .spheres
                .iter()
                .find(|(s, m, _)| *s == pick.slot && *m == pick.major)
                .map_or_else(
                    || pick.names.iter().take(pick.most).cloned().collect(),
                    |p| p.2.clone(),
                );
            spheres.push((pick.slot, pick.major, chosen.clone()));
            let title = if pick.major {
                tr!("Major Spheres to Keep ({class})", class = class(pick.slot))
            } else {
                tr!("Minor Spheres to Keep ({class})", class = class(pick.slot))
            };
            let this = self.clone();
            let (slot, major) = (pick.slot, pick.major);
            groups.push(check_group(
                &title,
                &KitMagicRules::sphere_pick_text(&kit, major, pick.most),
                &pick.names,
                &chosen,
                pick.most,
                move |names| {
                    let mut new = this.new.borrow_mut();
                    if let Some(p) = new
                        .spheres
                        .iter_mut()
                        .find(|(s, m, _)| *s == slot && *m == major)
                    {
                        p.2 = names;
                    }
                },
            ));
        }
        self.new.borrow_mut().spheres = spheres;

        // The free major sphere, while the character has none.
        let free = rules.free_sphere_choices(character, &kit);
        if free.is_empty() {
            if rules.magic_of(character).free_sphere.is_empty() {
                self.new.borrow_mut().free_sphere.clear();
            }
        } else {
            let group = adw::PreferencesGroup::builder()
                .title(tr("Free Sphere"))
                .description(
                    glib::markup_escape_text(&KitMagicRules::free_sphere_text(&kit)).as_str(),
                )
                .build();
            let mut names = vec![tr("None").to_owned()];
            names.extend(free.iter().cloned());
            let selected = if free.contains(&new.free_sphere) {
                new.free_sphere.clone()
            } else {
                self.new.borrow_mut().free_sphere.clear();
                String::new()
            };
            let row = combo_row(tr("Selected Free Sphere"), &names, &selected);
            row.connect_selected_notify({
                let this = self.clone();
                move |row| {
                    let value = if row.selected() == 0 {
                        String::new()
                    } else {
                        combo_value(row)
                    };
                    this.new.borrow_mut().free_sphere = value;
                    this.changed();
                }
            });
            group.add(&row);
            groups.push(group);
        }
        groups
    }

    /// Runs `then` unless the kit never uses spells: its warning first,
    /// whose Ignore runs it and OK does not (5d7732).
    pub fn check_spell_use(&self, parent: &impl IsA<gtk::Widget>, then: impl Fn() + 'static) {
        let warning = self
            .rules
            .spell_use_warning(&self.kit(), &self.new.borrow());
        match warning {
            None => then(),
            Some(text) => {
                let this = self.clone();
                ignorable(parent, tr("Kit Spell Use"), &text, move || {
                    this.new.borrow_mut().spell_use_ignored = true;
                    then();
                });
            }
        }
    }
}
