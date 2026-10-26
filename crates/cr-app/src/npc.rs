//! NPC Generation with no toolkit in it: the New NPC dialog (Create NPC,
//! with Ability Scores…) and the New NPC Group dialog (Create Multiple
//! NPCs) as forms. An interface draws their choices and passes back what
//! the user changes; the lists that follow a choice are filled again
//! here, so both apps offer the same ones.

use std::sync::Arc;

use cr_i18n::{tr, tr_noop, trn};
use cr_store::npc::{Abilities, Choice, GroupOptions, Npc, NpcOptions, NpcRules, Rand};
use cr_store::treasure::Flavor;

/// The class groups of Create Multiple NPCs, in combination order.
pub const GROUPS: [&str; 5] = [
    tr_noop!("Warrior"),
    tr_noop!("Rogue"),
    tr_noop!("Priest"),
    tr_noop!("Wizard"),
    tr_noop!("Psionicist"),
];

/// The highest level a class of a new NPC may have.
pub const MOST_LEVEL: i32 = 30;

/// The most NPCs one line of a group makes.
pub const MOST_NUMBER: i32 = 100;

/// "~Random" in the dialogs' choices.
const RANDOM: &str = tr_noop!("~Random");
/// The class choice that takes no class of a group.
const NO_CLASS: &str = tr_noop!("None");
/// The kit choice that takes no kit.
const NO_KIT: &str = "NONE";

/// The generator's notes on what it could not do (an item of the kit
/// missing from the database, no spellbook part), one line for each, as
/// the original's messages say them; `None` when there are none.
pub fn messages<'a>(npcs: impl IntoIterator<Item = &'a Npc>) -> Option<String> {
    let lines: Vec<String> = npcs
        .into_iter()
        .flat_map(|npc| {
            npc.messages
                .iter()
                .map(move |m| format!("{}: {m}", npc.character.data.name))
        })
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// What the New NPC Group's Create said: how many NPCs it made.
pub fn created_text(count: usize) -> String {
    trn!("{n} NPC was created", "{n} NPCs were created", count)
}

/// A change in the New NPC form, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum NpcInput {
    AllRaces(bool),
    Race(usize),
    AllClasses(bool),
    Class(usize),
    /// The level of the class at this place in the combination.
    Level(usize, i32),
    /// Player's Option rules rather than the Core Rules.
    PlayersOption(bool),
    /// Ability Scores…: rolls the scores alone, for Create to keep.
    Roll,
    /// Forgets the scores rolled, so that Create rolls new ones.
    Clear,
    Ability(usize, i32),
    Exceptional(i32),
    Party(String),
}

/// The New NPC dialog. In template mode (an encounter template's Add,
/// 0x4bd39a) it has no ability scores or party, and Create hands the NPC
/// back instead of saving it.
#[derive(Clone)]
pub struct NpcForm {
    rules: Arc<NpcRules>,
    pub template: bool,
    pub all_races: bool,
    races: Vec<String>,
    race: usize,
    pub all_classes: bool,
    /// The class combinations offered, single classes first.
    combinations: Vec<Vec<String>>,
    class: usize,
    /// One level for each class of the combination chosen.
    pub levels: Vec<i32>,
    pub players_option: bool,
    /// What Ability Scores… rolled, as edited since.
    pub abilities: Option<Abilities>,
    pub party: String,
    /// Why Roll or Create could not go on.
    pub error: Option<String>,
}

impl NpcForm {
    pub fn new(rules: Arc<NpcRules>, template: bool) -> Self {
        let mut form = Self {
            rules,
            template,
            all_races: false,
            races: Vec::new(),
            race: 0,
            all_classes: false,
            combinations: Vec::new(),
            class: 0,
            levels: Vec::new(),
            players_option: false,
            abilities: None,
            party: String::new(),
            error: None,
        };
        form.fill_races();
        form
    }

    pub fn rules(&self) -> &Arc<NpcRules> {
        &self.rules
    }

    /// The races offered: the standard ones, or every one with All
    /// Races (or when the data marks none standard).
    pub fn races(&self) -> &[String] {
        &self.races
    }

    pub fn race_index(&self) -> usize {
        self.race
    }

    pub fn race(&self) -> &str {
        self.races.get(self.race).map_or("", String::as_str)
    }

    /// The class combinations offered, as "Fighter/Thief".
    pub fn class_names(&self) -> Vec<String> {
        self.combinations.iter().map(|c| c.join("/")).collect()
    }

    pub fn class_index(&self) -> usize {
        self.class
    }

    /// The classes of the combination chosen, in order.
    pub fn classes(&self) -> &[String] {
        self.combinations.get(self.class).map_or(&[], Vec::as_slice)
    }

    /// The classes chosen with their levels.
    pub fn chosen(&self) -> Vec<(String, i32)> {
        self.classes()
            .iter()
            .cloned()
            .zip(self.levels.iter().copied())
            .collect()
    }

    /// The party a new NPC goes into, which only the Expansion has.
    pub fn party_shown(&self) -> bool {
        !self.template && self.rules.flavor() == Flavor::Expansion
    }

    /// The race's lowest and highest score of each ability.
    pub fn ability_ranges(&self) -> [(i32, i32); 6] {
        self.rules.ability_range(self.race())
    }

    /// Exceptional strength shows for 18 Strength in a race that rolls
    /// it.
    pub fn exceptional_shown(&self) -> bool {
        self.abilities.is_some_and(|a| a.scores[0] == 18)
            && self.rules.exceptional_strength(self.race())
    }

    fn fill_races(&mut self) {
        let current = self.races.get(self.race).cloned();
        let standard = self.rules.standard_races();
        self.races = if self.all_races || standard.is_empty() {
            self.rules.races()
        } else {
            standard
        };
        self.race = current
            .and_then(|c| self.races.iter().position(|r| *r == c))
            .unwrap_or(0);
        self.fill_classes();
    }

    /// The race's class combinations, or every class alone with All
    /// Classes or when the race allows none; the levels follow.
    fn fill_classes(&mut self) {
        let mut list = if self.all_classes {
            Vec::new()
        } else {
            self.rules.class_options(self.race())
        };
        if list.is_empty() {
            list = self.rules.classes().into_iter().map(|c| vec![c]).collect();
        }
        // Single classes before multi-class combinations.
        list.sort_by_key(Vec::len);
        self.combinations = list;
        self.class = 0;
        self.fill_levels();
    }

    /// A level for each class chosen, keeping those already set.
    fn fill_levels(&mut self) {
        let n = self.classes().len();
        self.levels.resize(n, 1);
    }

    /// The scores typed stay within the race's limits, and 18 Strength
    /// brings exceptional Strength for a race that rolls it.
    fn fit_abilities(&mut self) {
        let ranges = self.ability_ranges();
        let shown = self.exceptional_shown();
        if let Some(a) = &mut self.abilities {
            for (score, (lo, hi)) in a.scores.iter_mut().zip(ranges) {
                *score = (*score).clamp(lo, hi.max(lo));
            }
            if shown && a.exceptional == 0 {
                a.exceptional = 1;
            }
            if a.scores[0] != 18 {
                a.exceptional = 0;
            }
        }
    }

    fn options(&self) -> NpcOptions {
        NpcOptions {
            players_option: self.players_option,
            abilities: self.abilities,
        }
    }

    pub fn input(&mut self, input: NpcInput, rand: &mut Rand) {
        self.error = None;
        match input {
            NpcInput::AllRaces(on) => {
                self.all_races = on;
                self.fill_races();
                self.fit_abilities();
            }
            NpcInput::Race(i) if i < self.races.len() && i != self.race => {
                self.race = i;
                self.fill_classes();
                self.fit_abilities();
            }
            NpcInput::Race(_) => {}
            NpcInput::AllClasses(on) => {
                self.all_classes = on;
                self.fill_classes();
            }
            NpcInput::Class(i) if i < self.combinations.len() => {
                self.class = i;
                self.fill_levels();
            }
            NpcInput::Class(_) => {}
            NpcInput::Level(i, level) => {
                if let Some(slot) = self.levels.get_mut(i) {
                    *slot = level.clamp(1, MOST_LEVEL);
                }
            }
            NpcInput::PlayersOption(on) => self.players_option = on,
            NpcInput::Roll => {
                let options = NpcOptions {
                    abilities: None,
                    ..self.options()
                };
                match self
                    .rules
                    .roll_abilities_only(rand, self.race(), &self.chosen(), &options)
                {
                    Ok(a) => self.abilities = Some(a),
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            NpcInput::Clear => self.abilities = None,
            NpcInput::Ability(i, score) => {
                if let Some(a) = &mut self.abilities
                    && let Some(slot) = a.scores.get_mut(i)
                {
                    *slot = score;
                }
                self.fit_abilities();
            }
            NpcInput::Exceptional(n) => {
                if let Some(a) = &mut self.abilities {
                    a.exceptional = n.clamp(0, 100);
                }
                self.fit_abilities();
            }
            NpcInput::Party(text) => self.party = text,
        }
    }

    /// Create: the NPC, with the scores rolled before if any.
    pub fn generate(&self, rand: &mut Rand) -> Result<Npc, String> {
        let classes = self.chosen();
        if classes.is_empty() {
            return Err(self.rules.no_class_message());
        }
        self.rules
            .generate(rand, self.race(), &classes, &self.options())
            .map_err(|e| e.to_string())
    }
}

/// The texts a choice shows for `items`: the first `fixed` are the
/// dialog's own choices ("~Random", "None"), translated; the rest are
/// names from the data.
fn shown(items: &[String], fixed: usize) -> Vec<String> {
    items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i < fixed {
                tr(s).to_owned()
            } else {
                s.clone()
            }
        })
        .collect()
}

/// A choice of a list whose first item is "~Random".
fn choice(items: &[String], selected: usize) -> Choice {
    match selected {
        0 => Choice::Random,
        i => items
            .get(i)
            .map_or(Choice::Random, |n| Choice::Name(n.clone())),
    }
}

/// A list of choices and the one chosen.
#[derive(Debug, Clone, Default, PartialEq)]
struct Pick {
    items: Vec<String>,
    selected: usize,
    /// How many of the first items are the dialog's own.
    fixed: usize,
}

impl Pick {
    /// Fills the list again, keeping the name chosen when it is still
    /// there and "~Random" (the first) otherwise.
    fn refill(&mut self, items: Vec<String>, fixed: usize) {
        let old = self.items.get(self.selected).cloned();
        self.selected = old
            .and_then(|o| items.iter().position(|i| *i == o))
            .unwrap_or(0);
        self.items = items;
        self.fixed = fixed;
    }

    fn shown(&self) -> Vec<String> {
        shown(&self.items, self.fixed)
    }

    fn choice(&self) -> Choice {
        choice(&self.items, self.selected)
    }
}

/// A change in the New NPC Group form, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupInput {
    Name(String),
    AllRaces(bool),
    Race(usize),
    Fewest(i32),
    Most(i32),
    PlayersOption(bool),
    /// Each NPC takes a class combination its race allows.
    Allowed(bool),
    /// The choice of a class group, by its index among
    /// [`GroupForm::class_items`].
    Class(usize, usize),
    Kit(usize),
    AllKits(bool),
    Alignment(usize),
    AnyAlignment(bool),
    /// The lowest level of a class group.
    Low(usize, i32),
    /// The highest level of a class group.
    High(usize, i32),
    /// Add Line: the choices above as a line of the list.
    AddLine,
}

/// The New NPC Group dialog: one or more lines (number, race, classes,
/// kit, alignment and levels), made in the order of the sorted list as
/// Create Group does. Race, kit and alignment take the original's
/// Standard/All, Standard/All and Allowed/Any choices; their lists follow
/// the rest.
#[derive(Clone)]
pub struct GroupForm {
    rules: Arc<NpcRules>,
    pub name: String,
    pub all_races: bool,
    race: Pick,
    pub number: (i32, i32),
    pub players_option: bool,
    pub allowed: bool,
    classes: [Pick; 5],
    kit: Pick,
    pub all_kits: bool,
    alignment: Pick,
    pub any_alignment: bool,
    /// "~Random" kit under Allowed holds the alignment at "~Random"
    /// (0x4e3721).
    alignment_locked: bool,
    pub levels: [(i32, i32); 5],
    lines: Vec<GroupOptions>,
    pub error: Option<String>,
}

impl GroupForm {
    pub fn new(rules: Arc<NpcRules>) -> Self {
        let classes = std::array::from_fn(|g| {
            let mut items = vec![RANDOM.to_owned(), NO_CLASS.to_owned()];
            items.extend(rules.group_classes(g));
            Pick {
                items,
                // Without a choice only the warrior group takes a class.
                selected: if g == 0 { 0 } else { 1 },
                fixed: 2,
            }
        });
        let mut form = Self {
            rules,
            name: String::new(),
            all_races: false,
            race: Pick::default(),
            number: (1, 1),
            players_option: false,
            allowed: true,
            classes,
            kit: Pick::default(),
            all_kits: false,
            alignment: Pick::default(),
            any_alignment: false,
            alignment_locked: false,
            levels: [(1, 1); 5],
            lines: Vec::new(),
            error: None,
        };
        form.fill_races();
        form
    }

    pub fn race_items(&self) -> Vec<String> {
        self.race.shown()
    }

    pub fn race_index(&self) -> usize {
        self.race.selected
    }

    /// The choices of class group `group`: "~Random", "None", then its
    /// classes.
    pub fn class_items(&self, group: usize) -> Vec<String> {
        self.classes[group].shown()
    }

    pub fn class_index(&self, group: usize) -> usize {
        self.classes[group].selected
    }

    /// The class groups take a choice only without Allowed Combination.
    pub fn classes_enabled(&self) -> bool {
        !self.allowed
    }

    pub fn kit_items(&self) -> Vec<String> {
        self.kit.shown()
    }

    pub fn kit_index(&self) -> usize {
        self.kit.selected
    }

    pub fn alignment_items(&self) -> Vec<String> {
        self.alignment.shown()
    }

    pub fn alignment_index(&self) -> usize {
        self.alignment.selected
    }

    pub fn alignment_enabled(&self) -> bool {
        !self.alignment_locked && self.alignment.items.len() > 1
    }

    /// The lines added, as the sorted list shows them.
    pub fn lines(&self) -> Vec<String> {
        self.lines.iter().map(GroupOptions::display_label).collect()
    }

    /// The choices above as one line.
    pub fn current(&self) -> GroupOptions {
        let mut options = GroupOptions {
            number: self.number,
            race: match self.race.selected {
                0 => None,
                i => self.race.items.get(i).cloned(),
            },
            allowed_combination: self.allowed,
            kit: self.kit.choice(),
            alignment: self.alignment.choice(),
            players_option: self.players_option,
            ..Default::default()
        };
        for (slot, pick) in options.classes.iter_mut().zip(&self.classes) {
            *slot = match pick.selected {
                1 => None,
                _ => Some(pick.choice()),
            };
        }
        for (slot, (low, high)) in options.levels.iter_mut().zip(self.levels) {
            *slot = (low.min(high), low.max(high));
        }
        options
    }

    /// 0x4e095e and 0x4e09ad, then the kits.
    fn fill_races(&mut self) {
        let mut items = vec![RANDOM.to_owned()];
        items.extend(if self.all_races {
            self.rules.races()
        } else {
            self.rules.group_standard_races(self.players_option)
        });
        self.race.refill(items, 1);
        self.fill_kits();
    }

    /// 0x4e15d3, then the alignments.
    fn fill_kits(&mut self) {
        let mut items = vec![RANDOM.to_owned(), NO_KIT.to_owned()];
        items.extend(self.rules.group_kits(!self.all_kits, &self.current()));
        self.kit.refill(items, 1);
        self.fill_alignments();
    }

    /// 0x4e1cc3: the alignments, which a "~Random" kit holds at
    /// "~Random" under Allowed (0x4e3721).
    fn fill_alignments(&mut self) {
        let options = self.current();
        let mut items = vec![RANDOM.to_owned()];
        items.extend(if self.any_alignment {
            self.rules.alignments()
        } else {
            self.rules.group_alignments(&options)
        });
        self.alignment.refill(items, 1);
        self.alignment_locked = !self.any_alignment && matches!(options.kit, Choice::Random);
        if self.alignment_locked {
            self.alignment.selected = 0;
        }
    }

    pub fn input(&mut self, input: GroupInput) {
        self.error = None;
        match input {
            GroupInput::Name(text) => self.name = text,
            GroupInput::AllRaces(on) => {
                self.all_races = on;
                self.fill_races();
            }
            GroupInput::PlayersOption(on) => {
                self.players_option = on;
                self.fill_races();
            }
            GroupInput::Race(i) => {
                if i < self.race.items.len() {
                    self.race.selected = i;
                    self.fill_kits();
                }
            }
            GroupInput::Fewest(n) => {
                let n = n.clamp(1, MOST_NUMBER);
                self.number = (n, self.number.1.max(n));
            }
            GroupInput::Most(n) => {
                let n = n.clamp(1, MOST_NUMBER);
                self.number = (self.number.0.min(n), n);
            }
            GroupInput::Allowed(on) => {
                self.allowed = on;
                self.fill_kits();
            }
            GroupInput::Class(group, i) => {
                if let Some(pick) = self.classes.get_mut(group)
                    && i < pick.items.len()
                {
                    pick.selected = i;
                    self.fill_kits();
                }
            }
            GroupInput::Kit(i) => {
                if i < self.kit.items.len() {
                    self.kit.selected = i;
                    self.fill_alignments();
                }
            }
            GroupInput::AllKits(on) => {
                self.all_kits = on;
                self.fill_kits();
            }
            GroupInput::Alignment(i) => {
                if i < self.alignment.items.len() && !self.alignment_locked {
                    self.alignment.selected = i;
                }
            }
            GroupInput::AnyAlignment(on) => {
                self.any_alignment = on;
                self.fill_alignments();
            }
            GroupInput::Low(group, level) => {
                if let Some(slot) = self.levels.get_mut(group) {
                    slot.0 = level.clamp(1, MOST_LEVEL);
                }
            }
            GroupInput::High(group, level) => {
                if let Some(slot) = self.levels.get_mut(group) {
                    slot.1 = level.clamp(1, MOST_LEVEL);
                }
            }
            GroupInput::AddLine => {
                self.lines.push(self.current());
                GroupOptions::sort(&mut self.lines);
            }
        }
    }

    /// Create: the NPCs of every line, or of the line above when none was
    /// added, for the group named.
    pub fn generate(&self, rand: &mut Rand) -> Result<Vec<Npc>, String> {
        if self.name.trim().is_empty() {
            return Err(tr("A group name must be supplied.").to_owned());
        }
        let mut lines = self.lines.clone();
        if lines.is_empty() {
            lines.push(self.current());
        }
        self.rules
            .generate_groups(rand, &lines)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator of the installation `CORERULES_DIR` names, if it is
    /// set.
    fn rules() -> Option<Arc<NpcRules>> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        let install = cr_store::Installation::open(std::path::Path::new(&dir)).ok()?;
        install.npc_rules().ok().map(Arc::new)
    }

    #[test]
    fn a_new_npc_has_a_level_for_each_class() {
        let Some(rules) = rules() else {
            return;
        };
        let mut rand = Rand::seeded(7);
        let mut form = NpcForm::new(rules, false);
        assert!(!form.races().is_empty());
        let multi = form
            .class_names()
            .iter()
            .position(|c| c.contains('/'))
            .expect("a multi-class combination");
        form.input(NpcInput::Class(multi), &mut rand);
        assert_eq!(form.levels.len(), form.classes().len());
        assert!(form.levels.len() > 1);
        form.input(NpcInput::Level(1, 99), &mut rand);
        assert_eq!(form.levels[1], MOST_LEVEL);
        // Single classes come first.
        assert!(!form.class_names()[0].contains('/'));
        let npc = form.generate(&mut rand).unwrap();
        assert_eq!(npc.classes, form.chosen());
    }

    #[test]
    fn rolled_scores_stay_within_the_race() {
        let Some(rules) = rules() else {
            return;
        };
        let mut rand = Rand::seeded(3);
        let mut form = NpcForm::new(rules, false);
        form.input(NpcInput::Roll, &mut rand);
        assert!(form.abilities.is_some(), "{:?}", form.error);
        form.input(NpcInput::Ability(2, 99), &mut rand);
        let (_, most) = form.ability_ranges()[2];
        assert_eq!(form.abilities.unwrap().scores[2], most);
        assert!(form.generate(&mut rand).is_ok());
        form.input(NpcInput::Clear, &mut rand);
        assert!(form.abilities.is_none());
    }

    #[test]
    fn a_group_follows_its_choices_and_needs_a_name() {
        let Some(rules) = rules() else {
            return;
        };
        let mut rand = Rand::seeded(11);
        let mut form = GroupForm::new(rules);
        assert_eq!(form.race_items()[0], tr("~Random"));
        // A random kit holds the alignment at random.
        assert_eq!(form.kit_index(), 0);
        assert!(!form.alignment_enabled());
        form.input(GroupInput::AnyAlignment(true));
        assert!(form.alignment_enabled());
        form.input(GroupInput::Most(3));
        form.input(GroupInput::Fewest(5));
        assert_eq!(form.number, (5, 5));
        form.input(GroupInput::Fewest(2));
        form.input(GroupInput::Most(1));
        assert_eq!(form.number, (1, 1));
        assert!(form.generate(&mut rand).is_err());
        form.input(GroupInput::Name("Bandits".into()));
        form.input(GroupInput::AddLine);
        form.input(GroupInput::AddLine);
        assert_eq!(form.lines().len(), 2);
        let npcs = form.generate(&mut rand).unwrap();
        assert_eq!(npcs.len(), 2);
    }
}
