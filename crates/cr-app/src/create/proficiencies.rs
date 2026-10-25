//! The Proficiencies page, and with Skills & Powers the Combat page after
//! it: nonweapon proficiencies, languages, weapons and their groups,
//! specialization and unarmed combat, with the original's questions and
//! the kit's warnings on the way.

use std::sync::Arc;

use cr_i18n::tr;
use cr_store::create::{
    CreationRules, DISADVANTAGE_POINTS, Draft, MOST_DISADVANTAGES, MOST_FIGHTING_STYLES,
    NonweaponPick, PickKind, SHIELDS, UNARMED, WeaponCheck, required_removal,
};

use super::{Input, Page, Prompt, Wizard, points, slots};

/// A list of the Proficiencies and Combat pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Nonweapon,
    Traits,
    Disadvantages,
    Learned,
    Weapons,
    Broad,
    Tight,
    Specializations,
    Armor,
    Styles,
    Spells,
    Disciplines,
    Sciences,
    Devotions,
    WildTalents,
    Attacks,
    Defenses,
    Items,
}

/// A list the user adds to and removes from, as the GNOME app's lists
/// with Add in their header.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct List {
    pub title: String,
    pub description: String,
    /// Each row's title and subtitle, and how many times it is taken
    /// when that can change.
    pub rows: Vec<(String, String, Option<u32>)>,
    /// What Add offers.
    pub choices: Vec<String>,
    pub shown: bool,
    /// Its rows are rolled, not added or removed.
    pub fixed: bool,
}

/// A list the classes or the kit have the user pick some proficiencies
/// from.
#[derive(Debug, Clone, PartialEq)]
pub struct PickGroup {
    pub title: String,
    pub description: String,
    pub names: Vec<String>,
    pub picked: Vec<bool>,
    /// As many are ticked as may be: the others are off.
    pub full: bool,
}

/// A choice of one, "None" first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Choice {
    pub title: String,
    pub description: String,
    pub items: Vec<String>,
    pub selected: usize,
    pub shown: bool,
}

/// The unarmed styles to tick, with what each costs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Unarmed {
    pub cost: String,
    pub taken: [bool; 3],
    pub shown: bool,
}

/// The nonweapon slots a kit lets the weapons use.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lending {
    pub description: String,
    pub least: u32,
    pub most: u32,
    pub value: u32,
}

/// What the lists hold until Next, which the draft does not have yet.
#[derive(Debug, Clone, Default, PartialEq)]
struct Lists {
    nonweapon: Vec<(String, u32)>,
    traits: Vec<String>,
    disadvantages: Vec<String>,
    learned: Vec<String>,
    weapons: Vec<String>,
    broad: Vec<String>,
    tight: Vec<String>,
    specializations: Vec<String>,
}

impl Lists {
    fn names(&self, kind: ListKind) -> Vec<String> {
        match kind {
            ListKind::Nonweapon => self.nonweapon.iter().map(|(n, _)| n.clone()).collect(),
            _ => self.list(kind).map(<[String]>::to_vec).unwrap_or_default(),
        }
    }

    fn list(&self, kind: ListKind) -> Option<&[String]> {
        Some(match kind {
            ListKind::Traits => &self.traits,
            ListKind::Disadvantages => &self.disadvantages,
            ListKind::Learned => &self.learned,
            ListKind::Weapons => &self.weapons,
            ListKind::Broad => &self.broad,
            ListKind::Tight => &self.tight,
            ListKind::Specializations => &self.specializations,
            _ => return None,
        })
    }

    fn list_mut(&mut self, kind: ListKind) -> Option<&mut Vec<String>> {
        Some(match kind {
            ListKind::Traits => &mut self.traits,
            ListKind::Disadvantages => &mut self.disadvantages,
            ListKind::Learned => &mut self.learned,
            ListKind::Weapons => &mut self.weapons,
            ListKind::Broad => &mut self.broad,
            ListKind::Tight => &mut self.tight,
            ListKind::Specializations => &mut self.specializations,
            _ => return None,
        })
    }
}

/// Nonweapon proficiencies, languages and weapon proficiencies; with
/// Skills & Powers also traits and disadvantages, all paid in character
/// points. The choices the GNOME wizard makes on the draft at once (the
/// kit's warnings ignored, the picks, the free specialization, unarmed
/// combat) go into the draft; the lists wait for Next.
#[derive(Debug, Clone, PartialEq)]
pub struct ProficienciesPage {
    lists: Lists,
    /// The draft as the page has it: the wizard's with the lists.
    pub probe: Draft,
    pub nonweapon: List,
    pub picks: Vec<PickGroup>,
    pub traits: List,
    pub disadvantages: List,
    pub native: Vec<String>,
    pub learned: List,
    /// The kit's weapon requirements, as the original's box.
    pub requirements: Vec<String>,
    pub requirements_text: String,
    pub lending: Option<Lending>,
    pub weapons: List,
    pub broad: List,
    pub tight: List,
    pub focus: Choice,
    pub specializations: List,
    pub free: Choice,
    pub unarmed: Unarmed,
    /// Skills & Powers: the character points left for later levels.
    pub points_left: Option<String>,
    pub ok: bool,
}

/// Skills & Powers: weapon of choice, expertise and specialization,
/// unarmed combat, shield and armor proficiencies and fighting styles,
/// paid from the points the proficiencies leave, as the original's
/// Specialization, Shield and Armor and Fighting Styles dialogs.
#[derive(Debug, Clone, PartialEq)]
pub struct CombatPage {
    /// The draft as the page has it.
    pub state: Draft,
    pub weapons_text: String,
    pub weapon_of_choice: Choice,
    pub expertise: Choice,
    pub specialization: Choice,
    pub unarmed: Unarmed,
    pub shields: Vec<(String, String, bool)>,
    pub armor: List,
    pub styles: List,
    pub points_left: String,
    pub problem: Option<String>,
    pub ok: bool,
}

/// What a nonweapon proficiency costs, as its row says: more slots with
/// the Core Rules, more ranks at 1 point each with Skills & Powers; the
/// classes and the kit make some free.
pub fn nonweapon_subtitle(rules: &CreationRules, probe: &Draft, name: &str) -> String {
    let cost = rules.nonweapon_cost(probe, name);
    if probe.skills_and_powers {
        tr!("{cost}, 1 more a further rank", cost = points(cost.into()))
    } else if !rules.language_nonweapon(name) {
        tr!("{cost}, 1 more a further rank", cost = slots(cost))
    } else if cost == 0 {
        tr("Free").to_owned()
    } else {
        tr!("{slots} each time", slots = slots(cost))
    }
}

/// What a weapon proficiency costs, as its row says: nothing said for
/// the Core Rules' one slot.
pub fn weapon_subtitle(rules: &CreationRules, probe: &Draft, name: &str) -> String {
    let cost = rules.weapon_cost_for(probe, name);
    if probe.skills_and_powers {
        points(cost.into())
    } else if cost == 1 {
        String::new()
    } else {
        slots(cost)
    }
}

/// What the Core Rules' weapon specializations cost, and the one free.
pub fn specialization_costs(rules: &CreationRules, probe: &Draft) -> String {
    let costs = tr!(
        "Bows take {bows}, other weapons 1 slot.",
        bows = slots(rules.bow_specialization_cost(probe))
    );
    match rules.free_specialization(probe) {
        Some(free) => format!("{costs} {}", tr!("{free} is free.", free)),
        None => costs,
    }
}

/// How many of the languages the slots allow are left.
pub fn languages_text(left: i64, room: usize) -> String {
    tr!(
        "{left} of {room} left: one for each slot or rank of Modern \
         Languages and the like",
        left,
        room
    )
}

/// Skills & Powers: how many disadvantages may be taken, and the points
/// those taken give.
pub fn disadvantages_text(rules: &CreationRules, probe: &Draft) -> String {
    let given: u32 = probe
        .disadvantages
        .iter()
        .map(|d| rules.disadvantage_points(d))
        .sum();
    tr!(
        "Up to {most}, giving at most {limit} points; these give {given}",
        most = MOST_DISADVANTAGES,
        limit = DISADVANTAGE_POINTS,
        given = given.min(DISADVANTAGE_POINTS)
    )
}

/// The title and description of a pick list.
pub fn pick_texts(pick: &NonweaponPick) -> (String, String) {
    match pick.kind {
        PickKind::Bonus(_) if pick.from_kit => (
            tr("Bonus Proficiencies").to_owned(),
            tr!(
                "The kit {kit} allows you to select {count} bonus NWPs from the list below.",
                kit = pick.source,
                count = pick.count
            ),
        ),
        PickKind::Bonus(_) => (
            tr("Bonus Proficiencies").to_owned(),
            tr!(
                "The class {class} allows you to select {count} bonus NWPs from the list \
                 below.",
                class = pick.source,
                count = pick.count
            ),
        ),
        PickKind::PlusTwo => (
            tr("+2 Proficiency").to_owned(),
            tr!(
                "The kit {kit} allows you to pick a NWP from this list to receive a +2 bonus.",
                kit = pick.source
            ),
        ),
        PickKind::Free => (
            tr("Free Proficiency").to_owned(),
            tr("Select a non-weapon proficiency that will be free.").to_owned(),
        ),
        PickKind::Level { level, .. } => (
            tr("Free Proficiency").to_owned(),
            tr!(
                "The class {class} lets you pick at level {level} one non-weapon proficiency \
                 that will be free.",
                class = pick.source,
                level
            ),
        ),
        PickKind::PlusOne => (
            tr("+1 Proficiencies").to_owned(),
            tr!(
                "Select non-weapon proficiencies that will receive +1 to their score, up \
                 to {count}.",
                count = pick.count
            ),
        ),
    }
}

/// "None", then `items`.
pub(crate) fn with_none(items: impl IntoIterator<Item = String>) -> Vec<String> {
    std::iter::once(tr("None").to_owned())
        .chain(items)
        .collect()
}

/// The index of `chosen` among `items`, which start with "None".
pub(crate) fn index_of(items: &[String], chosen: Option<&String>) -> usize {
    chosen
        .and_then(|c| items.iter().skip(1).position(|i| i == c))
        .map_or(0, |i| i + 1)
}

/// The item at `index` of `items`, none for "None".
pub(crate) fn item_at(items: &[String], index: usize) -> Option<String> {
    (index > 0).then(|| items.get(index).cloned()).flatten()
}

/// The points as the page shows them, with a true minus.
fn signed(n: i64) -> String {
    n.to_string().replace('-', "−")
}

impl Wizard {
    fn proficiencies(&self) -> Option<&ProficienciesPage> {
        match self.page() {
            Page::Proficiencies(page) => Some(page),
            _ => None,
        }
    }

    fn proficiencies_mut(&mut self) -> Option<&mut ProficienciesPage> {
        match self.page_mut() {
            Page::Proficiencies(page) => Some(page),
            _ => None,
        }
    }

    fn combat_mut(&mut self) -> Option<&mut CombatPage> {
        match self.page_mut() {
            Page::Combat(page) => Some(page),
            _ => None,
        }
    }

    /// The Proficiencies page, from the draft.
    pub(super) fn push_proficiencies(&mut self) {
        let d = &mut self.draft;
        d.proficiencies_started = true;
        // The proficiencies the classes and the kit require go in once
        // (0x4414e1).
        if !d.required_nonweapon_added {
            for name in self.rules.required_nonweapon(d) {
                if !d.nonweapon_proficiencies.contains(&name) {
                    d.nonweapon_proficiencies.push(name);
                }
            }
            d.required_nonweapon_added = true;
        }
        let d = self.draft.clone();
        let rules = &self.rules;
        let lists = Lists {
            nonweapon: rules.nonweapon_ranks(&d),
            traits: d.traits.clone(),
            disadvantages: d.disadvantages.clone(),
            learned: d.learned_languages.clone(),
            weapons: d.weapon_proficiencies.clone(),
            broad: d.broad_groups.clone(),
            tight: d.tight_groups.clone(),
            specializations: if rules.core_specializes(&d) {
                d.specializations.clone()
            } else {
                Vec::new()
            },
        };
        let requirements = rules.weapon_requirements(&d);
        let page = ProficienciesPage {
            lists,
            probe: d.clone(),
            nonweapon: List::default(),
            picks: Vec::new(),
            traits: List::default(),
            disadvantages: List::default(),
            native: rules.native_languages(&d),
            learned: List::default(),
            requirements_text: tr!(
                "The kit {kit} has the following weapon proficiency requirements:",
                kit = d.kit
            ),
            requirements,
            lending: None,
            weapons: List::default(),
            broad: List::default(),
            tight: List::default(),
            focus: Choice::default(),
            specializations: List::default(),
            free: Choice::default(),
            unarmed: Unarmed::default(),
            points_left: None,
            ok: false,
        };
        self.push(Page::Proficiencies(Box::new(page)));
        self.refresh_proficiencies();
        // The kit's starting slots, when it has them (5c458c).
        if let Some(base) = self.rules.kit_nonweapon_base(&self.draft)
            && self.draft.kit_nonweapon_base.is_none()
        {
            let choose = |choice: bool| -> super::Then {
                Arc::new(move |w: &mut Wizard| {
                    w.draft.kit_nonweapon_base = Some(choice);
                    w.refresh_proficiencies();
                })
            };
            let mut prompt = Prompt::ask(
                tr("Kit Specifies Starting NWP Slots"),
                tr!(
                    "Your kit specifies using a starting base of {slots} NWP slots. Do you \
                     want to use the kit's value?",
                    slots = base
                ),
                choose(true),
                Some(choose(false)),
            );
            prompt.kind = super::PromptKind::Question { yes_default: true };
            self.show(prompt);
        }
    }

    /// The draft with the lists as the page has them (the GNOME wizard's
    /// `current`).
    fn proficiencies_probe(&self, lists: &Lists) -> Draft {
        let rules = &self.rules;
        let mut probe = self.draft.clone();
        probe.nonweapon_proficiencies = lists
            .nonweapon
            .iter()
            .flat_map(|(name, times)| std::iter::repeat_n(name.clone(), *times as usize))
            .collect();
        probe.weapon_proficiencies = lists.weapons.clone();
        probe.broad_groups = lists.broad.clone();
        probe.tight_groups = lists.tight.clone();
        probe.languages = rules.native_languages(&self.draft);
        probe.learned_languages = lists.learned.clone();
        if probe.skills_and_powers {
            probe.traits = lists.traits.clone();
            probe.disadvantages = lists.disadvantages.clone();
            // The combat options keep only weapons still taken.
            let weapons = rules.covered_weapons(&probe);
            let kept = |w: &Option<String>| w.clone().filter(|w| weapons.contains(w));
            probe.weapon_of_choice = kept(&probe.weapon_of_choice);
            probe.expertise = kept(&probe.expertise);
            probe.specializations.retain(|w| weapons.contains(w));
        }
        if rules.core_specializes(&probe) {
            probe.specializations = lists.specializations.clone();
        }
        match rules.focus_choices(&probe) {
            Some((_, choices)) => {
                probe.weapon_focus = probe.weapon_focus.filter(|w| choices.contains(w));
            }
            None => probe.weapon_focus = None,
        }
        probe
    }

    /// Writes the page's texts and choices anew from its lists.
    pub(super) fn refresh_proficiencies(&mut self) {
        let Some(page) = self.proficiencies() else {
            return;
        };
        let lists = page.lists.clone();
        let probe = self.proficiencies_probe(&lists);
        // The kit's checks on adding a weapon count the groups'.
        self.draft.broad_groups.clone_from(&probe.broad_groups);
        self.draft.tight_groups.clone_from(&probe.tight_groups);
        let rules = self.rules.clone();
        let entering = self.entering();
        let d = &self.draft;
        let sp = probe.skills_and_powers;
        let core = rules.core_specializes(&probe);

        let nonweapon_subtitle = |name: &str| nonweapon_subtitle(&rules, &probe, name);
        let weapon_subtitle = |name: &str| weapon_subtitle(&rules, &probe, name);
        let without = |all: Vec<String>, taken: &[String]| -> Vec<String> {
            all.into_iter().filter(|n| !taken.contains(n)).collect()
        };
        let plain = |names: &[String], subtitle: &dyn Fn(&str) -> String| {
            names
                .iter()
                .map(|n| (n.clone(), subtitle(n), None))
                .collect::<Vec<_>>()
        };

        let nonweapon_names = lists.names(ListKind::Nonweapon);
        let mut nonweapon = List {
            title: tr("Nonweapon Proficiencies").to_owned(),
            rows: lists
                .nonweapon
                .iter()
                .map(|(n, times)| (n.clone(), nonweapon_subtitle(n), Some(*times)))
                .collect(),
            choices: without(rules.nonweapon_proficiencies(), &nonweapon_names),
            shown: true,
            ..List::default()
        };
        let picks = rules
            .nonweapon_picks(d)
            .into_iter()
            .map(|pick| {
                let (title, description) = pick_texts(&pick);
                let chosen = d.picked(pick.kind);
                let picked: Vec<bool> = pick.names.iter().map(|n| chosen.contains(n)).collect();
                let full = picked.iter().filter(|p| **p).count() >= pick.count;
                PickGroup {
                    title,
                    description,
                    names: pick.names,
                    picked,
                    full,
                }
            })
            .collect();
        let traits = List {
            title: tr("Traits").to_owned(),
            rows: plain(&lists.traits, &|n| points(rules.trait_cost(n).into())),
            choices: without(
                rules.traits().iter().map(|t| t.0.clone()).collect(),
                &lists.traits,
            ),
            shown: sp,
            ..List::default()
        };
        let disadvantages = List {
            title: tr("Disadvantages").to_owned(),
            description: if sp {
                disadvantages_text(&rules, &probe)
            } else {
                String::new()
            },
            rows: plain(&lists.disadvantages, &|n| {
                tr!(
                    "Gives {points}",
                    points = points(rules.disadvantage_points(n).into())
                )
            }),
            choices: if lists.disadvantages.len() >= MOST_DISADVANTAGES {
                Vec::new()
            } else {
                without(
                    rules.disadvantages().iter().map(|t| t.0.clone()).collect(),
                    &lists.disadvantages,
                )
            },
            shown: sp,
            fixed: false,
        };
        let native = rules.native_languages(d);
        let room = rules.language_slots(&probe);
        let languages_left = room as i64 - probe.learned_languages.len() as i64;
        let learned = List {
            title: tr("Learned Languages").to_owned(),
            description: languages_text(languages_left, room),
            rows: plain(&lists.learned, &|_| String::new()),
            choices: if lists.learned.len() >= room {
                Vec::new()
            } else {
                rules
                    .languages()
                    .iter()
                    .filter(|l| !lists.learned.contains(l) && !native.contains(l))
                    .cloned()
                    .collect()
            },
            shown: true,
            fixed: false,
        };
        let mut weapons = List {
            title: tr("Weapon Proficiencies").to_owned(),
            rows: plain(&lists.weapons, &weapon_subtitle),
            choices: without(rules.weapon_choices(d), &lists.weapons),
            shown: true,
            ..List::default()
        };
        let group = |broad: bool, taken: &[String]| List {
            title: if broad {
                tr("Broad Weapon Groups")
            } else {
                tr("Tight Weapon Groups")
            }
            .to_owned(),
            description: if broad { points(6) } else { points(4) },
            rows: plain(taken, &|n| rules.group_weapons(broad, n).join(", ")),
            choices: rules.group_choices(d, broad),
            shown: sp && rules.expansion,
            fixed: false,
        };
        let (broad, tight) = (group(true, &lists.broad), group(false, &lists.tight));
        let specializations = List {
            title: tr("Weapon Specialization").to_owned(),
            description: specialization_costs(&rules, &probe),
            rows: plain(&lists.specializations, &|n| {
                slots(rules.core_specialization_cost(&probe, n))
            }),
            choices: without(lists.weapons.clone(), &lists.specializations),
            shown: core,
            fixed: false,
        };
        let free_choices = rules.free_specialization_choices(d).to_vec();
        let free_items = with_none(free_choices.iter().cloned());
        let free = Choice {
            title: tr("Free Specialization").to_owned(),
            description: tr(
                "Your kit allows choosing a free weapon specialization from this list.",
            )
            .to_owned(),
            selected: index_of(&free_items, d.free_specialization.as_ref()),
            items: free_items,
            shown: !free_choices.is_empty(),
        };
        let lending = rules
            .nonweapon_lending(&probe)
            .map(|(least, most)| Lending {
                description: tr!(
                    "The kit {kit} lets weapon proficiencies use nonweapon proficiency slots.",
                    kit = d.kit
                ),
                least,
                most,
                value: rules.borrowed_nonweapon(&probe),
            });
        let focus = match rules.focus_choices(&probe) {
            Some((prompt, choices)) => {
                let items = with_none(choices);
                Choice {
                    title: tr("Weapon Focus").to_owned(),
                    description: prompt,
                    selected: index_of(&items, probe.weapon_focus.as_ref()),
                    items,
                    shown: true,
                }
            }
            None => Choice::default(),
        };
        let unarmed = self.unarmed(&probe, core);

        let mut points_left = None;
        let ok = if entering {
            // Enter Character's proficiencies take no slots or points
            // (0x5c30df, 0x5cd21c).
            let free = tr("Free with Enter Character, as in the original");
            nonweapon.description = free.to_owned();
            weapons.description = free.to_owned();
            true
        } else if !sp {
            let nonweapon_slots = rules.nonweapon_slots_for(&probe);
            let spent = rules.nonweapon_spent(&probe);
            let weapon_slots = rules.weapon_slots_for(&probe);
            let weapons_taken = rules.weapon_spent(&probe);
            nonweapon.description = tr!(
                "{left} of {slots} left: the class's slots and the languages Intelligence allows",
                left = nonweapon_slots as i64 - spent as i64,
                slots = slots(nonweapon_slots)
            );
            weapons.description = tr!(
                "{left} of {slots} left",
                left = weapon_slots as i64 - weapons_taken as i64,
                slots = slots(weapon_slots)
            );
            spent <= nonweapon_slots && weapons_taken <= weapon_slots && languages_left >= 0
        } else {
            let left = rules.proficiency_points_left(&probe);
            nonweapon.description = tr!(
                "{left} of {total} left for proficiencies and traits",
                left = points(left),
                total = rules.proficiency_points(&probe)
            );
            weapons.description = tr!(
                "{cost} each; {free} come with the class, and points left from the \
                 proficiencies pay for more",
                cost = points(rules.weapon_cost(&probe.classes).into()),
                free = points(rules.weapon_points_for(&probe).into())
            );
            let total = rules.points_left(&probe);
            points_left = Some(signed(total));
            left >= 0
                && total >= 0
                && languages_left >= 0
                && probe.disadvantages.len() <= MOST_DISADVANTAGES
        };
        let Some(page) = self.proficiencies_mut() else {
            return;
        };
        page.probe = probe;
        page.nonweapon = nonweapon;
        page.picks = picks;
        page.traits = traits;
        page.disadvantages = disadvantages;
        page.native = native;
        page.learned = learned;
        page.lending = lending;
        page.weapons = weapons;
        page.broad = broad;
        page.tight = tight;
        page.focus = focus;
        page.specializations = specializations;
        page.free = free;
        page.unarmed = unarmed;
        page.points_left = points_left;
        page.ok = ok;
    }

    /// The unarmed styles for `probe`; on the Proficiencies page only with
    /// the Core Rules (`shown`), Skills & Powers having them on Combat.
    fn unarmed(&self, probe: &Draft, shown: bool) -> Unarmed {
        let cost = self.rules.unarmed_cost(probe);
        Unarmed {
            cost: match cost {
                Some(cost) if probe.skills_and_powers => points(cost.into()),
                Some(cost) => slots(cost),
                None => String::new(),
            },
            taken: UNARMED.map(|name| probe.unarmed.iter().any(|u| u == name)),
            shown: shown && cost.is_some(),
        }
    }

    /// Changes the lists with `change`, and takes it back when the page
    /// no longer fits the slots or points, as the original refuses
    /// (Enter Character's are free).
    fn change_lists(&mut self, limited: bool, change: impl FnOnce(&mut Lists)) {
        let Some(page) = self.proficiencies_mut() else {
            return;
        };
        let before = page.lists.clone();
        change(&mut page.lists);
        let lists = page.lists.clone();
        if limited && !self.entering() {
            let probe = self.proficiencies_probe(&lists);
            if !self.rules.proficiencies_fit(&probe) {
                if let Some(page) = self.proficiencies_mut() {
                    page.lists = before;
                }
                self.refused();
                return;
            }
        }
        self.refresh_proficiencies();
    }

    /// Says a proficiency does not fit.
    fn refused(&mut self) {
        let text = if self.draft.skills_and_powers {
            tr("You do not have enough Character Points.")
        } else {
            tr("You do not have enough slots.")
        };
        self.message(tr("Not Allowed"), text);
    }

    /// Adds a row of `kind` named `name`.
    fn add_row(&mut self, kind: ListKind, name: String) {
        match kind {
            ListKind::Nonweapon => {
                self.change_lists(true, |lists| lists.nonweapon.push((name, 1)));
            }
            ListKind::Traits | ListKind::Learned => {
                self.change_lists(true, |lists| {
                    lists.list_mut(kind).expect("a list").push(name);
                });
            }
            _ => self.change_lists(false, |lists| {
                if let Some(list) = lists.list_mut(kind) {
                    list.push(name);
                }
            }),
        }
    }

    pub(super) fn proficiencies_input(&mut self, input: Input) {
        let Some(page) = self.proficiencies() else {
            return;
        };
        let (lists, probe) = (page.lists.clone(), page.probe.clone());
        match input {
            Input::Add(kind, name) => {
                let offered = match kind {
                    ListKind::Nonweapon => &page.nonweapon.choices,
                    ListKind::Traits => &page.traits.choices,
                    ListKind::Disadvantages => &page.disadvantages.choices,
                    ListKind::Learned => &page.learned.choices,
                    ListKind::Weapons => &page.weapons.choices,
                    ListKind::Broad => &page.broad.choices,
                    ListKind::Tight => &page.tight.choices,
                    ListKind::Specializations => &page.specializations.choices,
                    _ => return,
                };
                if !offered.contains(&name) {
                    return;
                }
                match kind {
                    ListKind::Nonweapon => self.add_nonweapon(name),
                    ListKind::Weapons => self.add_weapon(name, lists.weapons, false),
                    ListKind::Broad => self.add_group(true, name, false),
                    ListKind::Tight => self.add_group(false, name, false),
                    ListKind::Specializations => self.specialize(name, 0),
                    _ => self.add_row(kind, name),
                }
            }
            Input::Remove(kind, i) => self.remove_proficiency(kind, i, &lists, &probe),
            Input::Times(i, times) => {
                if !(1..=10).contains(&times) || i >= lists.nonweapon.len() {
                    return;
                }
                // A rank raised past the limit goes back.
                let raised = times > lists.nonweapon[i].1;
                self.change_lists(raised, |lists| lists.nonweapon[i].1 = times);
            }
            Input::Pick(g, i, on) => {
                let Some(pick) = self.rules.nonweapon_picks(&self.draft).into_iter().nth(g) else {
                    return;
                };
                let Some(name) = pick.names.get(i) else {
                    return;
                };
                let mut picked = self.draft.picked(pick.kind).to_vec();
                if on && !picked.contains(name) {
                    // No more than the count may be ticked.
                    if picked.len() >= pick.count {
                        return;
                    }
                    picked.push(name.clone());
                } else if !on {
                    picked.retain(|n| n != name);
                }
                self.draft.set_picked(pick.kind, picked);
                self.refresh_proficiencies();
            }
            Input::Lent(n) => {
                if let Some(lending) = &page.lending
                    && (lending.least..=lending.most).contains(&n)
                {
                    self.draft.lent_nonweapon = n;
                    self.refresh_proficiencies();
                }
            }
            Input::Focus(i) => {
                if page.focus.shown && i < page.focus.items.len() {
                    self.draft.weapon_focus = item_at(&page.focus.items, i);
                    self.refresh_proficiencies();
                }
            }
            Input::FreeSpecialization(i) => {
                if !page.free.shown || i >= page.free.items.len() {
                    return;
                }
                let chosen = item_at(&page.free.items, i);
                let conflict = chosen
                    .as_ref()
                    .and_then(|_| self.rules.specialization_conflict(&self.draft));
                match conflict {
                    Some(conflict) => self.show(Prompt::ask(
                        &conflict.title,
                        conflict.text,
                        Arc::new(move |w: &mut Wizard| {
                            w.draft.unarmed.clear();
                            w.draft.free_specialization = chosen.clone();
                            w.refresh_proficiencies();
                        }),
                        None,
                    )),
                    None => {
                        self.draft.free_specialization = chosen;
                        self.refresh_proficiencies();
                    }
                }
            }
            Input::Unarmed(i, on) => self.toggle_unarmed(i, on, probe),
            Input::Next if page.ok => self.proficiencies_next(),
            _ => {}
        }
    }

    /// Adds a nonweapon proficiency; one the kit bars needs the warning
    /// ignored (5c857b).
    fn add_nonweapon(&mut self, name: String) {
        match self.rules.barred_nonweapon_warning(&self.draft, &name) {
            None => self.add_row(ListKind::Nonweapon, name),
            Some(text) => self.show(Prompt::ignorable(
                tr("Kit Non-Weapon Proficiency Requirements"),
                &text,
                Arc::new(move |w: &mut Wizard| {
                    w.draft.barred_nonweapon_ignored = true;
                    w.add_row(ListKind::Nonweapon, name.clone());
                }),
            )),
        }
    }

    /// Adds a weapon proficiency once the questions buying it asks are
    /// answered Yes (0x444079), then each of the kit's warnings, which
    /// Ignore passes for good (5d4aa7).
    fn add_weapon(&mut self, name: String, taken: Vec<String>, asked: bool) {
        if !asked {
            let questions = self.rules.weapon_questions(&self.draft, &taken, &name);
            self.ask_each(
                questions.into(),
                Arc::new(move |w: &mut Wizard| w.add_weapon(name.clone(), taken.clone(), true)),
            );
            return;
        }
        let warning = self
            .rules
            .weapon_warnings(&self.draft, &taken, &name)
            .into_iter()
            .next();
        let Some(warning) = warning else {
            let rules = self.rules.clone();
            rules.weapon_added(&mut self.draft, &taken, &name);
            self.change_lists(true, |lists| lists.weapons.push(name));
            return;
        };
        let check = warning.check;
        self.show(Prompt::ignorable(
            tr("Kit Weapon Proficiency Requirements"),
            &warning.text,
            Arc::new(move |w: &mut Wizard| {
                w.draft.ignored_weapon_checks.push(check);
                w.add_weapon(name.clone(), taken.clone(), true);
            }),
        ));
    }

    /// Buys a broad (else tight) weapon group once the kit's warning is
    /// passed, Ignore passing it for good (5d3d63, 5d4121), and the
    /// questions are answered Yes; refuses one the classes may not buy
    /// (5cde66).
    fn add_group(&mut self, broad: bool, name: String, warned: bool) {
        if !warned
            && let Some(warning) = self
                .rules
                .group_warnings(&self.draft, broad, &name)
                .into_iter()
                .next()
        {
            let check = warning.check;
            self.show(Prompt::ignorable(
                tr("Kit Weapon Proficiency Requirements"),
                &warning.text,
                Arc::new(move |w: &mut Wizard| {
                    w.draft.ignored_weapon_checks.push(check);
                    w.add_group(broad, name.clone(), true);
                }),
            ));
            return;
        }
        match self.rules.group_offer(&self.draft, broad) {
            Ok(offer) => {
                let kind = if broad {
                    ListKind::Broad
                } else {
                    ListKind::Tight
                };
                self.ask_each(
                    offer.questions.into(),
                    Arc::new(move |w: &mut Wizard| w.add_row(kind, name.clone())),
                );
            }
            Err(text) => self.message(tr("Weapon Groups"), text),
        }
    }

    /// Core Rules: specializes in a weapon once the questions (from the
    /// `asked`th on) are answered Yes, the kit's warning passed and the
    /// slots suffice (61e897).
    fn specialize(&mut self, name: String, asked: usize) {
        let Some(d) = self.proficiencies().map(|p| p.probe.clone()) else {
            return;
        };
        let rules = self.rules.clone();
        // Unarmed combat goes first, once Yes is answered (61edf1).
        if let Some(conflict) = rules.specialization_conflict(&d) {
            self.show(Prompt::ask(
                &conflict.title,
                conflict.text,
                Arc::new(move |w: &mut Wizard| {
                    w.draft.unarmed.clear();
                    w.refresh_proficiencies();
                    w.specialize(name.clone(), asked);
                }),
                None,
            ));
            return;
        }
        if let Some(question) = rules.specialization_questions(&d, &name).get(asked) {
            self.show(Prompt::ask(
                &question.title,
                question.text.clone(),
                Arc::new(move |w: &mut Wizard| w.specialize(name.clone(), asked + 1)),
                None,
            ));
            return;
        }
        if let Some(warning) = rules.specialization_warnings(&d, &name).into_iter().next() {
            let check = warning.check;
            self.show(Prompt::ignorable(
                tr("Kit Specialization Requirements"),
                &warning.text,
                Arc::new(move |w: &mut Wizard| {
                    w.draft.ignored_weapon_checks.push(check);
                    w.refresh_proficiencies();
                    w.specialize(name.clone(), asked);
                }),
            ));
            return;
        }
        // Enter Character's slots are free (0x5cd21c).
        if !self.entering() && !rules.specialization_fits(&d, &name) {
            self.message(tr("Not Allowed"), tr("You do not have enough slots."));
            return;
        }
        self.add_row(ListKind::Specializations, name);
    }

    /// Removes row `i` of `kind`, asking first or refusing as the
    /// original does.
    fn remove_proficiency(&mut self, kind: ListKind, i: usize, lists: &Lists, probe: &Draft) {
        let names = lists.names(kind);
        let Some(name) = names.get(i).cloned() else {
            return;
        };
        let rules = self.rules.clone();
        let remove = move |w: &mut Wizard| {
            w.change_lists(false, |lists| match kind {
                ListKind::Nonweapon => {
                    lists.nonweapon.remove(i);
                }
                _ => {
                    if let Some(list) = lists.list_mut(kind) {
                        list.remove(i);
                    }
                }
            });
        };
        match kind {
            // Removing a required proficiency asks first (5c2856).
            ListKind::Nonweapon if rules.required_nonweapon(&self.draft).contains(&name) => {
                let question = required_removal();
                self.show(Prompt::ask(
                    &question.title,
                    question.text,
                    Arc::new(remove),
                    None,
                ));
            }
            // Core Rules: a specialized weapon keeps its proficiency until
            // the specialization goes (5cae2c).
            ListKind::Weapons
                if rules.core_specializes(probe)
                    && rules.all_specializations(probe).contains(&name) =>
            {
                self.message(
                    tr("Specialized Weapon"),
                    tr(
                        "You must remove all specialization information for this weapon \
                         before you can remove its proficiency.",
                    ),
                );
            }
            ListKind::Broad | ListKind::Tight => {
                match rules.group_removal(probe, kind == ListKind::Broad) {
                    Some(text) => self.message(tr("Weapon Groups"), text),
                    None => remove(self),
                }
            }
            _ => remove(self),
        }
    }

    /// Ticks or unticks unarmed style `i`; one taken while a weapon is
    /// specialized asks first, and Yes drops the specialization (6208ac).
    /// The Core Rules check the slots.
    fn toggle_unarmed(&mut self, i: usize, on: bool, probe: Draft) {
        let Some(name) = UNARMED.get(i) else {
            return;
        };
        if on == probe.unarmed.iter().any(|u| u == name) {
            return;
        }
        if !on {
            self.set_unarmed(|unarmed| unarmed.retain(|u| u != name));
            return;
        }
        let take = move |w: &mut Wizard| {
            w.set_unarmed(|unarmed| unarmed.push(UNARMED[i].to_owned()));
        };
        if let Some(conflict) = self.rules.unarmed_conflict(&probe) {
            self.show(Prompt::ask(
                &conflict.title,
                conflict.text,
                Arc::new(move |w: &mut Wizard| {
                    w.drop_specialization();
                    take(w);
                }),
                None,
            ));
        } else if !probe.skills_and_powers && !self.entering() && !self.rules.unarmed_fits(&probe) {
            self.message(tr("Not Allowed"), tr("You do not have enough slots."));
        } else {
            take(self);
        }
    }

    /// Changes the unarmed styles of the page in view.
    fn set_unarmed(&mut self, change: impl FnOnce(&mut Vec<String>)) {
        if let Some(page) = self.combat_mut() {
            change(&mut page.state.unarmed);
            self.refresh_combat();
        } else {
            change(&mut self.draft.unarmed);
            self.refresh_proficiencies();
        }
    }

    /// Drops the weapon specializations of the page in view, for unarmed
    /// combat.
    fn drop_specialization(&mut self) {
        if let Some(page) = self.combat_mut() {
            page.state.specializations.clear();
            self.refresh_combat();
        } else if let Some(page) = self.proficiencies_mut() {
            page.lists.specializations.clear();
            self.draft.free_specialization = None;
            self.refresh_proficiencies();
        }
    }

    /// Next on Proficiencies.
    fn proficiencies_next(&mut self) {
        let Some(probe) = self.proficiencies().map(|p| p.probe.clone()) else {
            return;
        };
        let rules = self.rules.clone();
        // The proficiency the kit has the character learn by a level
        // (5c8e84): Yes stays, No goes on.
        if let Some(question) = rules.nonweapon_by_level(&probe) {
            self.show(Prompt::ask(
                &question.title,
                question.text,
                Arc::new(|_: &mut Wizard| {}),
                Some(Arc::new(|w: &mut Wizard| {
                    w.draft.by_level_declined = true;
                    w.refresh_proficiencies();
                    w.proficiencies_next();
                })),
            ));
            return;
        }
        // The specializations the kit requires (5d2bef, 5d2f71); Ignore
        // leaves the page.
        if rules.core_specializes(&probe)
            && let Some(warning) = rules.leaving_warnings(&probe).into_iter().next()
        {
            let check: WeaponCheck = warning.check;
            self.show(Prompt::ignorable(
                tr("Kit Weapon Proficiency Requirements"),
                &warning.text,
                Arc::new(move |w: &mut Wizard| {
                    w.draft.ignored_weapon_checks.push(check);
                    w.refresh_proficiencies();
                    w.proficiencies_next();
                }),
            ));
            return;
        }
        let d = &mut self.draft;
        if d.weapon_proficiencies != probe.weapon_proficiencies {
            // The equipment page starts with one of each.
            d.purchases.clear();
        }
        d.nonweapon_proficiencies = probe.nonweapon_proficiencies;
        d.languages = probe.languages;
        d.learned_languages = probe.learned_languages;
        d.weapon_proficiencies = probe.weapon_proficiencies;
        d.broad_groups = probe.broad_groups;
        d.tight_groups = probe.tight_groups;
        d.traits = probe.traits;
        d.disadvantages = probe.disadvantages;
        d.specializations = probe.specializations;
        d.weapon_of_choice = probe.weapon_of_choice;
        d.expertise = probe.expertise;
        d.weapon_focus = probe.weapon_focus;
        if d.skills_and_powers {
            self.push_combat();
        } else {
            self.after_combat();
        }
    }

    fn push_combat(&mut self) {
        let d = self.draft.clone();
        let page = CombatPage {
            state: d,
            weapons_text: String::new(),
            weapon_of_choice: Choice::default(),
            expertise: Choice::default(),
            specialization: Choice::default(),
            unarmed: Unarmed::default(),
            shields: Vec::new(),
            armor: List::default(),
            styles: List::default(),
            points_left: String::new(),
            problem: None,
            ok: false,
        };
        self.push(Page::Combat(Box::new(page)));
        self.refresh_combat();
    }

    /// Writes the Combat page's texts and choices anew from its state.
    fn refresh_combat(&mut self) {
        let Page::Combat(page) = self.page() else {
            return;
        };
        let rules = self.rules.clone();
        let entering = self.entering();
        let d = page.state.clone();
        let classes = &d.classes;
        let weapon_items = with_none(rules.covered_weapons(&d));
        let choice =
            |title: &str, description: String, chosen: Option<&String>, shown: bool| Choice {
                title: title.to_owned(),
                description,
                items: weapon_items.clone(),
                selected: index_of(&weapon_items, chosen),
                shown,
            };
        let specialization_cost = rules.specialization_cost(&d);
        let weapon_of_choice = choice(
            tr("Weapon of Choice"),
            points(rules.weapon_of_choice_cost(classes).into()),
            d.weapon_of_choice.as_ref(),
            true,
        );
        let expertise = choice(
            tr("Weapon Expertise"),
            tr!(
                "{cost}, 1 less in the weapon of choice",
                cost = points(
                    rules
                        .expertise_cost(&Draft {
                            expertise: None,
                            ..d.clone()
                        })
                        .into()
                )
            ),
            d.expertise.as_ref(),
            true,
        );
        let specialization = choice(
            tr("Weapon Specialization"),
            points(specialization_cost.unwrap_or(0).into()),
            d.specializations.first(),
            specialization_cost.is_some(),
        );
        let weapons_text = if specialization_cost.is_none() {
            tr(
                "This character does not have any Class Abilities allowing specialization at \
                1st level.",
            )
        } else {
            tr("Among the weapon proficiencies taken")
        }
        .to_owned();
        let shield_cost = points(rules.shield_cost(classes).into());
        let shields = SHIELDS
            .iter()
            .enumerate()
            .map(|(i, name)| ((*name).to_owned(), shield_cost.clone(), d.shields[i]))
            .collect();
        let armor = List {
            title: tr("Armor Proficiencies").to_owned(),
            description: String::new(),
            rows: d
                .armor_proficiencies
                .iter()
                .map(|n| (n.clone(), shield_cost.clone(), None))
                .collect(),
            choices: rules
                .armors()
                .into_iter()
                .filter(|a| !d.armor_proficiencies.contains(a))
                .collect(),
            shown: true,
            fixed: false,
        };
        let styles = List {
            title: tr("Fighting Styles").to_owned(),
            description: tr(
                "The styles the class groups may learn; the \u{201c}(+2)\u{201d} ones are \
                 specializations in them",
            )
            .to_owned(),
            rows: d
                .fighting_styles
                .iter()
                .map(|n| {
                    let cost = rules.style_cost(classes, n).unwrap_or(0);
                    (n.clone(), points(cost.into()), None)
                })
                .collect(),
            choices: if d.fighting_styles.len() >= MOST_FIGHTING_STYLES {
                Vec::new()
            } else {
                rules
                    .styles(classes)
                    .into_iter()
                    .filter(|s| !d.fighting_styles.contains(s))
                    .collect()
            },
            shown: true,
            fixed: false,
        };
        let unarmed = self.unarmed(&d, true);
        let left = rules.points_left(&d);
        // Enter Character charges no points (0x5d09e6).
        let problem = rules.combat_problem(&d).or_else(|| {
            (left < 0 && !entering).then(|| {
                tr!(
                    "Not enough character points: the new choices need {more} more",
                    more = -left
                )
            })
        });
        let Some(page) = self.combat_mut() else {
            return;
        };
        page.weapons_text = weapons_text;
        page.weapon_of_choice = weapon_of_choice;
        page.expertise = expertise;
        page.specialization = specialization;
        page.unarmed = unarmed;
        page.shields = shields;
        page.armor = armor;
        page.styles = styles;
        page.points_left = signed(left);
        page.ok = problem.is_none();
        page.problem = problem;
    }

    pub(super) fn combat_input(&mut self, input: Input) {
        let Page::Combat(page) = self.page() else {
            return;
        };
        let page = page.clone();
        match input {
            Input::WeaponOfChoice(i) | Input::Expertise(i) => {
                let Some(choice) = (match input {
                    Input::WeaponOfChoice(_) => Some(&page.weapon_of_choice),
                    _ => Some(&page.expertise),
                })
                .filter(|c| i < c.items.len()) else {
                    return;
                };
                let value = item_at(&choice.items, i);
                let of_choice = matches!(input, Input::WeaponOfChoice(_));
                if let Some(page) = self.combat_mut() {
                    if of_choice {
                        page.state.weapon_of_choice = value;
                    } else {
                        page.state.expertise = value;
                    }
                }
                self.refresh_combat();
            }
            Input::Specialization(i) => {
                if !page.specialization.shown || i >= page.specialization.items.len() {
                    return;
                }
                match item_at(&page.specialization.items, i) {
                    None => {
                        if let Some(page) = self.combat_mut() {
                            page.state.specializations.clear();
                        }
                        self.refresh_combat();
                    }
                    Some(name) => self.po_specialize(name),
                }
            }
            Input::Unarmed(i, on) => self.toggle_unarmed(i, on, page.state),
            Input::Shield(i, on) => {
                if i < SHIELDS.len()
                    && let Some(page) = self.combat_mut()
                {
                    page.state.shields[i] = on;
                    self.refresh_combat();
                }
            }
            Input::Add(kind @ (ListKind::Armor | ListKind::Styles), name) => {
                let list = if kind == ListKind::Armor {
                    &page.armor
                } else {
                    &page.styles
                };
                if !list.choices.contains(&name) {
                    return;
                }
                if let Some(page) = self.combat_mut() {
                    match kind {
                        ListKind::Armor => page.state.armor_proficiencies.push(name),
                        _ => page.state.fighting_styles.push(name),
                    }
                }
                self.refresh_combat();
            }
            Input::Remove(kind @ (ListKind::Armor | ListKind::Styles), i) => {
                if let Some(page) = self.combat_mut() {
                    let list = match kind {
                        ListKind::Armor => &mut page.state.armor_proficiencies,
                        _ => &mut page.state.fighting_styles,
                    };
                    if i < list.len() {
                        list.remove(i);
                    }
                }
                self.refresh_combat();
            }
            Input::Next if page.ok => {
                let state = page.state;
                let d = &mut self.draft;
                d.weapon_of_choice = state.weapon_of_choice;
                d.expertise = state.expertise;
                d.specializations = state.specializations;
                d.unarmed = state.unarmed;
                d.shields = state.shields;
                d.armor_proficiencies = state.armor_proficiencies;
                d.fighting_styles = state.fighting_styles;
                self.after_combat();
            }
            _ => {}
        }
    }

    /// Skills & Powers: specializes in a weapon once the question about
    /// unarmed combat is answered Yes (which drops it), the kit's
    /// questions too and its warning passed (61edf1).
    fn po_specialize(&mut self, name: String) {
        let Page::Combat(page) = self.page() else {
            return;
        };
        let probe = page.state.clone();
        let rules = self.rules.clone();
        if let Some(conflict) = rules.specialization_conflict(&probe) {
            self.show(Prompt::ask(
                &conflict.title,
                conflict.text,
                Arc::new(move |w: &mut Wizard| {
                    w.set_unarmed(Vec::clear);
                    w.po_specialize(name.clone());
                }),
                None,
            ));
            return;
        }
        let questions = rules.specialization_questions(&probe, &name);
        self.ask_each(
            questions.into(),
            Arc::new(move |w: &mut Wizard| w.po_specialization_warning(name.clone())),
        );
    }

    /// The kit's warning about the specialization, if any, then the
    /// specialization.
    fn po_specialization_warning(&mut self, name: String) {
        let Page::Combat(page) = self.page() else {
            return;
        };
        let probe = page.state.clone();
        let warning = self
            .rules
            .specialization_warnings(&probe, &name)
            .into_iter()
            .next();
        let choose = move |w: &mut Wizard| {
            if let Some(page) = w.combat_mut() {
                page.state.specializations = vec![name.clone()];
            }
            w.refresh_combat();
        };
        let Some(warning) = warning else {
            choose(self);
            return;
        };
        let check = warning.check;
        self.show(Prompt::ignorable(
            tr("Kit Specialization Requirements"),
            &warning.text,
            Arc::new(move |w: &mut Wizard| {
                if let Some(page) = w.combat_mut() {
                    page.state.ignored_weapon_checks.push(check);
                }
                w.draft.ignored_weapon_checks.push(check);
                choose(w);
            }),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::super::details::tests::at_details_with;
    use super::super::{Answer, Effect, MOST_ITEMS, PromptKind};
    use super::*;

    /// A human fighter on the Proficiencies page.
    fn at_proficiencies(sp: bool) -> Option<Wizard> {
        let mut w = at_details_with(false, sp)?;
        w.input(Input::Next);
        // The kit's starting slots, if it asks.
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        matches!(w.page(), Page::Proficiencies(_)).then_some(w)
    }

    fn page(w: &Wizard) -> &ProficienciesPage {
        match w.page() {
            Page::Proficiencies(page) => page,
            _ => panic!("not the proficiencies"),
        }
    }

    /// Answers Yes (or Ignore) to whatever is asked.
    fn yes(w: &mut Wizard) {
        for _ in 0..10 {
            if w.prompt().is_none() {
                return;
            }
            w.answer(Answer::Yes);
        }
    }

    #[test]
    fn a_fighter_takes_a_weapon_and_specializes_in_it() {
        let Some(mut w) = at_proficiencies(false) else {
            return;
        };
        let p = page(&w);
        assert!(p.ok);
        assert!(p.specializations.shown);
        assert!(!p.traits.shown);
        let sword = p
            .weapons
            .choices
            .iter()
            .find(|c| c.contains("Long sword") || c.contains("Long Sword"))
            .or(p.weapons.choices.first())
            .unwrap()
            .clone();
        w.input(Input::Add(ListKind::Weapons, sword.clone()));
        yes(&mut w);
        let p = page(&w);
        assert_eq!(p.weapons.rows[0].0, sword);
        assert!(p.specializations.choices.contains(&sword));
        w.input(Input::Add(ListKind::Specializations, sword.clone()));
        yes(&mut w);
        assert_eq!(page(&w).specializations.rows[0].0, sword);
        // A specialized weapon keeps its proficiency.
        w.input(Input::Remove(ListKind::Weapons, 0));
        assert_eq!(w.prompt().unwrap().kind, PromptKind::Message);
        w.answer(Answer::Yes);
        assert_eq!(page(&w).weapons.rows.len(), 1);
        w.input(Input::Next);
        yes(&mut w);
        // An Expansion fighter may test for wild talents.
        assert!(matches!(w.page(), Page::Wild(_)));
        let d = w.draft();
        assert_eq!(d.weapon_proficiencies, std::slice::from_ref(&sword));
        assert_eq!(d.specializations, [sword]);
        // Back keeps the lists as Next left them.
        assert!(w.back());
        assert_eq!(page(&w).weapons.rows.len(), 1);
    }

    #[test]
    fn a_fighter_tests_for_wild_talents_buys_equipment_and_is_created() {
        let Some(mut w) = at_proficiencies(false) else {
            return;
        };
        let weapon = page(&w).weapons.choices[0].clone();
        w.input(Input::Add(ListKind::Weapons, weapon.clone()));
        yes(&mut w);
        w.input(Input::Next);
        yes(&mut w);
        let Page::Wild(wild) = w.page() else {
            panic!("not wild talents");
        };
        assert!(wild.test_shown && wild.can_test && !wild.can_roll);
        // New Character takes no talent but those rolled.
        assert!(wild.talents.fixed && wild.talents.choices.is_empty());
        w.input(Input::Test);
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        let Page::Wild(wild) = w.page() else {
            panic!();
        };
        let tested = wild.can_roll;
        w.input(Input::Next);
        assert!(w.draft().wild.tested);
        assert_eq!(w.draft().wild.wild_talent, tested);
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        let Page::Equipment(equipment) = w.page() else {
            panic!("not equipment");
        };
        // One of each weapon taken.
        assert_eq!(equipment.items.rows[0].0, weapon);
        assert_eq!(equipment.items.rows[0].2, Some(1));
        w.input(Input::RollMoney);
        let Page::Equipment(equipment) = w.page() else {
            panic!();
        };
        assert!(equipment.ok, "{}", equipment.money);
        // More than the money buys is refused.
        w.input(Input::Times(0, MOST_ITEMS));
        assert_eq!(w.prompt().unwrap().text, tr("This item is too expensive."));
        w.answer(Answer::Yes);
        let Page::Equipment(equipment) = w.page() else {
            panic!();
        };
        assert_eq!(equipment.items.rows[0].2, Some(1));
        w.input(Input::Next);
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        let effects = w.take_effects();
        let Some(Effect::Create(draft)) = effects.last() else {
            panic!("not created: {effects:?}");
        };
        assert_eq!(draft.purchases, [(weapon, 1)]);
        assert!(draft.gold > 0);
        assert_eq!(w.rules().problem(draft), None);
        assert!(w.rules().build(draft).is_ok());
    }

    #[test]
    fn back_to_the_details_and_next_keep_the_proficiencies() {
        let Some(mut w) = at_proficiencies(false) else {
            return;
        };
        let weapon = page(&w).weapons.choices[0].clone();
        w.input(Input::Add(ListKind::Weapons, weapon.clone()));
        yes(&mut w);
        let nonweapon = page(&w).nonweapon.choices.first().cloned();
        if let Some(name) = nonweapon {
            w.input(Input::Add(ListKind::Nonweapon, name));
            yes(&mut w);
        }
        let before = page(&w).clone();
        assert!(w.back());
        assert!(matches!(w.page(), Page::Details(_)));
        w.input(Input::Next);
        yes(&mut w);
        assert_eq!(page(&w).weapons.rows, before.weapons.rows);
        assert_eq!(page(&w).nonweapon.rows, before.nonweapon.rows);
        // A new kit, once said yes to, starts them over.
        assert!(w.back());
        let Page::Details(details) = w.page() else {
            panic!("not the details");
        };
        let Some(kit) = (0..details.kits.len()).find(|&k| Some(k) != details.kit_index()) else {
            return;
        };
        w.input(Input::Kit(kit));
        assert_eq!(w.prompt().unwrap().title, tr("Warning"));
        w.answer(Answer::Yes);
        yes(&mut w);
        w.input(Input::Next);
        yes(&mut w);
        if let Page::Proficiencies(p) = w.page() {
            assert!(!p.weapons.rows.iter().any(|(name, ..)| *name == weapon));
        }
    }

    #[test]
    fn a_proficiency_past_the_slots_is_refused() {
        let Some(mut w) = at_proficiencies(false) else {
            return;
        };
        for _ in 0..40 {
            let p = page(&w);
            let rows = p.nonweapon.rows.len();
            let Some(name) = p.nonweapon.choices.first().cloned() else {
                return;
            };
            w.input(Input::Add(ListKind::Nonweapon, name));
            if let Some(prompt) = w.prompt() {
                if prompt.title == tr("Not Allowed") {
                    w.answer(Answer::Yes);
                    assert_eq!(page(&w).nonweapon.rows.len(), rows);
                    assert!(page(&w).ok);
                    return;
                }
                // A kit's warning: OK leaves it out.
                w.answer(Answer::No);
            }
        }
        panic!("never refused");
    }

    #[test]
    fn skills_and_powers_go_on_to_combat() {
        let Some(mut w) = at_proficiencies(true) else {
            return;
        };
        let p = page(&w);
        assert!(p.traits.shown);
        assert!(p.points_left.is_some());
        let weapon = p.weapons.choices.first().unwrap().clone();
        w.input(Input::Add(ListKind::Weapons, weapon.clone()));
        yes(&mut w);
        assert!(page(&w).ok);
        w.input(Input::Next);
        yes(&mut w);
        let Page::Combat(combat) = w.page() else {
            panic!("not combat");
        };
        assert!(combat.weapon_of_choice.items.contains(&weapon));
        let i = combat
            .weapon_of_choice
            .items
            .iter()
            .position(|i| *i == weapon)
            .unwrap();
        w.input(Input::WeaponOfChoice(i));
        let Page::Combat(combat) = w.page() else {
            panic!();
        };
        assert_eq!(combat.state.weapon_of_choice.as_ref(), Some(&weapon));
        // Back to the proficiencies and Next keep the combat choices.
        assert!(w.back());
        w.input(Input::Next);
        yes(&mut w);
        let Page::Combat(combat) = w.page() else {
            panic!("not combat again");
        };
        assert_eq!(combat.state.weapon_of_choice.as_ref(), Some(&weapon));
        if combat.ok {
            w.input(Input::Next);
            assert!(matches!(w.page(), Page::Wild(_)));
            assert_eq!(w.draft().weapon_of_choice, Some(weapon));
        }
    }
}
