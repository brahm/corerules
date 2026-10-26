//! The small dialogs some fields of the record editor open, with no
//! toolkit in them: a choose-N-of group (the ListOfListsOfOptions dialog,
//! 4b2d50), a class level's free spells (IDD 144), NWPs (IDD 172) and
//! class ability, and the Monster Attack dialog (IDD 269).

use cr_formats::monster::MonsterAttack;
use cr_store::help::topic;
use cr_store::userdb::Value;
use cr_store::userdb::classes::levels::{
    self, FREQUENCY_MAX, FreeSpell, FreeSpellLevel, LevelNwps, MODIFICATION_MAX,
};
use cr_store::userdb::monsters::{AREAS, SAVE_RESULTS, SAVES, TO_HIT, attack_problem};

/// The highest level a class level's dialog takes.
pub const MOST_LEVEL: i64 = 100;
/// The most uses a free spell takes.
pub const MOST_USES: i64 = 1000;
/// The highest spell level free spells' Add filters by.
pub const MOST_SPELL_LEVEL: i64 = 10;
/// The most of a group's names to choose.
pub const MOST_COUNT: i64 = 100;

/// The dialog a field opened.
#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    Group(GroupEdit),
    FreeSpells(FreeSpellsEdit),
    Nwps(NwpsEdit),
    Ability(AbilityEdit),
    Attack(AttackEdit),
}

/// A change in a dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum DialogInput {
    /// How many of a group's names to choose.
    Count(i64),
    /// What the Add box of the dialog's list `list` chose: 0, or 1 for
    /// the NWP score modifications.
    Adding(usize, String),
    /// Add the box's name to list `list`.
    Add(usize),
    /// Remove the row with this index of list `list`.
    Remove(usize, usize),
    /// The number of a row of list `list`: a spell's uses, an NWP's
    /// modification.
    Number(usize, usize, i64),
    /// The level the dialog is for.
    Level(i64),
    /// Free spells: how often the spells added may be cast.
    Uses(i64),
    Frequency(String),
    /// "Must select one spell from the list", "May only pick one from
    /// the list above".
    PickOne(bool),
    /// Free spells' Add: wizard spells (true) or priest spells.
    Wizard(bool),
    /// Free spells' Add: the spell level, 0 for all.
    SpellLevel(i64),
    /// The class ability, by its index among the choices.
    Ability(usize),
    Attack(AttackInput),
}

/// A change in the Monster Attack dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum AttackInput {
    Name(String),
    HelpId(i32),
    /// The damage's dice (0), sides (1) or bonus (2).
    Damage(usize, i32),
    /// "Must make a to hit roll against", by index of [`TO_HIT`].
    ToHit(usize),
    Effect(String),
    HitDice(i32),
    /// The area of effect, by index of [`AREAS`].
    Area(usize),
    /// One of the area's four measures (see
    /// [`cr_store::userdb::monsters::area_labels`]).
    Measure(usize, i32),
    /// "Save Against", by index of [`SAVES`].
    Save(usize),
    Modifier(i32),
    /// "To gain this", by index of [`SAVE_RESULTS`].
    Result(usize),
}

impl Dialog {
    /// The field the dialog edits.
    pub fn field(&self) -> usize {
        match self {
            Dialog::Group(d) => d.field,
            Dialog::FreeSpells(d) => d.field,
            Dialog::Nwps(d) => d.field,
            Dialog::Ability(d) => d.field,
            Dialog::Attack(d) => d.field,
        }
    }

    /// Its help topic, when it has one of its own.
    pub fn help(&self) -> Option<u32> {
        matches!(self, Dialog::Attack(_)).then_some(topic::MONSTER_ATTACK)
    }

    pub(super) fn input(&mut self, input: DialogInput) {
        match self {
            Dialog::Group(d) => d.input(input),
            Dialog::FreeSpells(d) => d.input(input),
            Dialog::Nwps(d) => d.input(input),
            Dialog::Ability(d) => d.input(input),
            Dialog::Attack(d) => {
                if let DialogInput::Attack(change) = input {
                    d.input(change);
                }
            }
        }
    }

    /// The field's `value` with what the dialog holds, or the dialog
    /// again when it has a problem to say first.
    pub(super) fn finish(self, value: Value) -> Result<Value, Box<Dialog>> {
        Ok(match self {
            Dialog::Group(d) => {
                let mut groups = value.groups();
                put_group(&mut groups, d.at, (d.count, d.names));
                Value::Groups(groups)
            }
            Dialog::FreeSpells(d) => {
                let Value::FreeSpells(class, mut list) = value else {
                    return Ok(value);
                };
                let mut level = d.level;
                level.frequency = level.frequency.chars().take(FREQUENCY_MAX).collect();
                put(&mut list, d.at, level, |l| l.level.into());
                Value::FreeSpells(class, list)
            }
            Dialog::Nwps(d) => {
                let mut list = value.level_nwps();
                put(&mut list, d.at, d.level, |l| l.level.into());
                Value::LevelNwps(list)
            }
            Dialog::Ability(d) => {
                let Some(name) = d.name else {
                    return Ok(value);
                };
                let mut list = value.pairs();
                put(&mut list, d.at, (name, d.level), |(_, l)| *l);
                Value::Pairs(list)
            }
            Dialog::Attack(mut d) => {
                let mut attacks = value.attacks();
                if let Some(problem) = attack_problem(&attacks, d.at, &d.attack.name) {
                    d.problem = Some(problem);
                    return Err(Box::new(Dialog::Attack(d)));
                }
                // The name trimmed (0x6bbb25).
                d.attack.name = d.attack.name.trim().to_owned();
                match d.at {
                    Some(i) if i < attacks.len() => attacks[i] = d.attack,
                    _ => attacks.push(d.attack),
                }
                Value::Attacks(attacks)
            }
        })
    }
}

/// Puts `entry` in `entries` in place of the one at `at` and of any other
/// at its level, keeping them by level.
fn put<T>(entries: &mut Vec<T>, at: Option<usize>, entry: T, level: impl Fn(&T) -> i64) {
    if let Some(i) = at
        && i < entries.len()
    {
        entries.remove(i);
    }
    let l = level(&entry);
    entries.retain(|e| level(e) != l);
    let at = entries.partition_point(|e| level(e) < l);
    entries.insert(at, entry);
}

/// The lowest level `used` lacks.
fn free_level(used: impl Iterator<Item = i64>) -> i64 {
    let used: Vec<i64> = used.collect();
    (1..=MOST_LEVEL).find(|l| !used.contains(l)).unwrap_or(1)
}

/// Adds the name `adding` holds to `names`, once.
fn add_name(names: &mut Vec<String>, adding: &mut Option<String>) {
    if let Some(name) = adding.take()
        && !names.contains(&name)
    {
        names.push(name);
    }
}

/// Puts `group` in `groups`, at `at` when it changes one, else at the
/// end. A group with nothing to choose from is not kept: an empty one
/// changed takes its group out, a new one adds nothing.
pub fn put_group(
    groups: &mut Vec<(i64, Vec<String>)>,
    at: Option<usize>,
    group: (i64, Vec<String>),
) {
    let at = at.filter(|&i| i < groups.len());
    match (at, group.1.is_empty()) {
        (Some(i), true) => {
            groups.remove(i);
        }
        (Some(i), false) => groups[i] = group,
        (None, true) => {}
        (None, false) => groups.push(group),
    }
}

/// A choose-N-of group: how many to choose, and out of which names.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupEdit {
    pub field: usize,
    /// The group's index, when one is changed rather than added.
    pub at: Option<usize>,
    pub count: i64,
    pub names: Vec<String>,
    pub adding: Option<String>,
}

impl GroupEdit {
    pub(super) fn new(field: usize, at: Option<usize>, count: i64, names: Vec<String>) -> Self {
        Self {
            field,
            at,
            count,
            names,
            adding: None,
        }
    }

    fn input(&mut self, input: DialogInput) {
        match input {
            DialogInput::Count(count) => self.count = count.clamp(1, MOST_COUNT),
            DialogInput::Adding(_, name) => self.adding = Some(name),
            DialogInput::Add(_) => add_name(&mut self.names, &mut self.adding),
            DialogInput::Remove(_, i) if i < self.names.len() => {
                self.names.remove(i);
            }
            _ => {}
        }
    }
}

/// A class level's free spells (IDD 144).
#[derive(Debug, Clone, PartialEq)]
pub struct FreeSpellsEdit {
    pub field: usize,
    pub at: Option<usize>,
    pub level: FreeSpellLevel,
    /// What Add offers: wizard spells or priest spells (IDD 144's radio
    /// buttons, Priest first), of one level or all (0).
    pub wizard: bool,
    pub spell_level: i64,
    pub adding: Option<String>,
}

impl FreeSpellsEdit {
    pub(super) fn new(field: usize, at: Option<usize>, list: &[FreeSpellLevel]) -> Self {
        let level = at.and_then(|i| list.get(i).cloned()).unwrap_or_else(|| {
            FreeSpellLevel::new(free_level(list.iter().map(|l| l.level.into())) as i32)
        });
        Self {
            field,
            at,
            level,
            wizard: false,
            spell_level: 0,
            adding: None,
        }
    }

    /// The spells Add offers out of `spells` (name, priest, level).
    pub fn choices(&self, spells: &[(String, bool, i32)]) -> Vec<String> {
        let mut names: Vec<String> = spells
            .iter()
            .filter(|(_, priest, level)| {
                *priest != self.wizard
                    && (self.spell_level == 0 || i64::from(*level) == self.spell_level)
            })
            .map(|(n, _, _)| n.clone())
            .filter(|n| {
                !self
                    .level
                    .spells
                    .iter()
                    .any(|s| s.name == *n && s.wizard == self.wizard)
            })
            .collect();
        names.dedup();
        names
    }

    /// A spell's line: as the wizard spell, or a priest's.
    pub fn spell_kind(spell: &FreeSpell) -> &'static str {
        if spell.wizard {
            cr_i18n::tr("As the Wizard Spell")
        } else {
            cr_i18n::tr("Priest")
        }
    }

    fn input(&mut self, input: DialogInput) {
        match input {
            DialogInput::Level(level) => self.level.level = level.clamp(1, MOST_LEVEL) as i32,
            DialogInput::Uses(uses) => self.level.uses = uses.clamp(0, MOST_USES) as i32,
            DialogInput::Frequency(text) => self.level.frequency = text,
            DialogInput::PickOne(on) => self.level.pick_one = on,
            DialogInput::Wizard(on) => {
                self.wizard = on;
                self.adding = None;
            }
            DialogInput::SpellLevel(level) => {
                self.spell_level = level.clamp(0, MOST_SPELL_LEVEL);
                self.adding = None;
            }
            DialogInput::Adding(_, name) => self.adding = Some(name),
            DialogInput::Add(_) => {
                if let Some(name) = self.adding.take() {
                    let taken = self
                        .level
                        .spells
                        .iter()
                        .any(|s| s.name == name && s.wizard == self.wizard);
                    if !taken {
                        self.level.spells.push(FreeSpell {
                            name,
                            uses: self.level.uses,
                            wizard: self.wizard,
                        });
                    }
                }
            }
            DialogInput::Remove(_, i) if i < self.level.spells.len() => {
                self.level.spells.remove(i);
            }
            DialogInput::Number(_, i, uses) => {
                if let Some(spell) = self.level.spells.get_mut(i) {
                    spell.uses = uses.clamp(0, MOST_USES) as i32;
                }
            }
            _ => {}
        }
    }
}

/// A class level's NWPs (IDD 172): those it grants (list 0) and the
/// score modifications it gives (list 1).
#[derive(Debug, Clone, PartialEq)]
pub struct NwpsEdit {
    pub field: usize,
    pub at: Option<usize>,
    pub level: LevelNwps,
    pub adding: [Option<String>; 2],
}

/// The highest NWP score modification.
pub const MOST_MODIFICATION: i64 = MODIFICATION_MAX as i64;

impl NwpsEdit {
    pub(super) fn new(field: usize, at: Option<usize>, list: &[LevelNwps]) -> Self {
        let level = at
            .and_then(|i| list.get(i).cloned())
            .unwrap_or_else(|| LevelNwps {
                level: free_level(list.iter().map(|l| l.level.into())) as i32,
                ..LevelNwps::default()
            });
        Self {
            field,
            at,
            level,
            adding: [None, None],
        }
    }

    /// The names list `list` has chosen.
    pub fn chosen(&self, list: usize) -> Vec<String> {
        if list == 0 {
            self.level.granted.clone()
        } else {
            self.level
                .modifications
                .iter()
                .map(|(n, _)| n.clone())
                .collect()
        }
    }

    fn input(&mut self, input: DialogInput) {
        match input {
            DialogInput::Level(level) => self.level.level = level.clamp(1, MOST_LEVEL) as i32,
            DialogInput::PickOne(on) => self.level.pick_one = on,
            DialogInput::Adding(list, name) if list < 2 => self.adding[list] = Some(name),
            DialogInput::Add(0) => add_name(&mut self.level.granted, &mut self.adding[0]),
            DialogInput::Add(1) => {
                if let Some(name) = self.adding[1].take()
                    && !self.level.modifications.iter().any(|(n, _)| *n == name)
                {
                    self.level.modifications.push((name, 0));
                }
            }
            DialogInput::Remove(0, i) if i < self.level.granted.len() => {
                self.level.granted.remove(i);
            }
            DialogInput::Remove(1, i) if i < self.level.modifications.len() => {
                self.level.modifications.remove(i);
            }
            DialogInput::Number(1, i, n) => {
                if let Some(entry) = self.level.modifications.get_mut(i) {
                    entry.1 = n.clamp(0, MOST_MODIFICATION) as i32;
                }
            }
            _ => {}
        }
    }
}

/// The class ability one level grants: the original's Add/Remove dialog
/// with one selection.
#[derive(Debug, Clone, PartialEq)]
pub struct AbilityEdit {
    pub field: usize,
    pub at: Option<usize>,
    pub level: i64,
    /// The ability chosen, by name, and the choices.
    pub name: Option<String>,
    pub choices: Vec<String>,
}

impl AbilityEdit {
    pub(super) fn new(
        field: usize,
        at: Option<usize>,
        list: &[(String, i64)],
        choices: &[String],
    ) -> Self {
        let (name, level) = at
            .and_then(|i| list.get(i).cloned())
            .unwrap_or_else(|| (String::new(), free_level(list.iter().map(|(_, l)| *l))));
        // The first is chosen when the name is not among the choices.
        let name = if choices.contains(&name) {
            Some(name)
        } else {
            choices.first().cloned()
        };
        Self {
            field,
            at,
            level,
            name,
            choices: choices.to_vec(),
        }
    }

    /// What the dialog asks, for its level.
    pub fn prompt(&self) -> String {
        levels::ability_prompt(self.level)
    }

    /// The ability chosen, by index among the choices.
    pub fn chosen(&self) -> Option<usize> {
        let name = self.name.as_ref()?;
        self.choices.iter().position(|c| c == name)
    }

    fn input(&mut self, input: DialogInput) {
        match input {
            DialogInput::Level(level) => self.level = level.clamp(1, MOST_LEVEL),
            DialogInput::Ability(i) => {
                if let Some(name) = self.choices.get(i) {
                    self.name = Some(name.clone());
                }
            }
            _ => {}
        }
    }
}

/// The Monster Attack dialog (IDD 269) on a copy of an attack.
#[derive(Debug, Clone, PartialEq)]
pub struct AttackEdit {
    pub field: usize,
    pub at: Option<usize>,
    pub attack: MonsterAttack,
    /// Why OK did not close the dialog.
    pub problem: Option<&'static str>,
}

/// The most of a number the attack's digits-only boxes take.
pub const MOST_ATTACK_NUMBER: i32 = i32::MAX;

impl AttackEdit {
    pub(super) fn new(field: usize, at: Option<usize>, attack: MonsterAttack) -> Self {
        Self {
            field,
            at,
            attack,
            problem: None,
        }
    }

    /// The index of the attack's "to hit" among [`TO_HIT`].
    pub fn to_hit(&self) -> usize {
        TO_HIT
            .iter()
            .position(|(v, _)| *v == self.attack.to_hit)
            .unwrap_or(0)
    }

    /// The attack's area, save and result as indices of their lists.
    pub fn area(&self) -> usize {
        index_in(self.attack.area, AREAS.len())
    }

    pub fn save(&self) -> usize {
        index_in(self.attack.save_against, SAVES.len())
    }

    pub fn result(&self) -> usize {
        index_in(self.attack.save_result, SAVE_RESULTS.len())
    }

    /// The area's four measures: range, width, end width and length.
    pub fn measures(&self) -> [i32; 4] {
        let a = &self.attack;
        [a.range, a.width, a.end_width, a.length]
    }

    fn input(&mut self, input: AttackInput) {
        // The digits-only edits take no minus sign; Bonus Hit Dice does.
        let digits = |v: i32| v.max(0);
        let a = &mut self.attack;
        match input {
            AttackInput::Name(name) => a.name = name,
            AttackInput::HelpId(v) => a.help_id = digits(v),
            AttackInput::Damage(0, v) => a.damage_dice = digits(v),
            AttackInput::Damage(1, v) => a.damage_sides = digits(v),
            AttackInput::Damage(_, v) => a.damage_bonus = digits(v),
            AttackInput::ToHit(i) => a.to_hit = TO_HIT.get(i).map_or(0, |(v, _)| *v),
            AttackInput::Effect(text) => a.special_effect = text,
            AttackInput::HitDice(v) => a.bonus_hit_dice = v,
            AttackInput::Area(i) if i < AREAS.len() => a.area = i as i32,
            AttackInput::Measure(0, v) => a.range = digits(v),
            AttackInput::Measure(1, v) => a.width = digits(v),
            AttackInput::Measure(2, v) => a.end_width = digits(v),
            AttackInput::Measure(_, v) => a.length = digits(v),
            AttackInput::Save(i) if i < SAVES.len() => a.save_against = i as i32,
            AttackInput::Modifier(v) => a.save_modifier = digits(v),
            AttackInput::Result(i) if i < SAVE_RESULTS.len() => a.save_result = i as i32,
            _ => {}
        }
    }
}

/// `value` as an index of a list of `len`, the first when it is not one.
fn index_in(value: i32, len: usize) -> usize {
    usize::try_from(value)
        .ok()
        .filter(|&i| i < len)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_entry_replaces_the_one_at_its_level_and_keeps_the_order() {
        let mut list = vec![("A".to_owned(), 1), ("B".to_owned(), 3)];
        put(&mut list, None, ("C".to_owned(), 2), |(_, l)| *l);
        assert_eq!(list[1].0, "C");
        put(&mut list, Some(0), ("D".to_owned(), 3), |(_, l)| *l);
        assert_eq!(
            list,
            vec![("C".to_owned(), 2), ("D".to_owned(), 3)],
            "the edited entry and the one at its new level both go"
        );
        assert_eq!(free_level([1, 2, 4].into_iter()), 3);
    }

    #[test]
    fn a_group_takes_names_once() {
        let mut dialog = Dialog::Group(GroupEdit::new(0, None, 1, Vec::new()));
        for _ in 0..2 {
            dialog.input(DialogInput::Adding(0, "Bow".into()));
            dialog.input(DialogInput::Add(0));
        }
        dialog.input(DialogInput::Count(2));
        let value = dialog.finish(Value::Groups(Vec::new())).unwrap();
        assert_eq!(value, Value::Groups(vec![(2, vec!["Bow".to_owned()])]));
    }

    #[test]
    fn a_group_with_nothing_to_choose_is_not_kept() {
        let bow = (1, vec!["Bow".to_owned()]);
        let dialog = Dialog::Group(GroupEdit::new(0, None, 1, Vec::new()));
        let value = dialog.finish(Value::Groups(vec![bow.clone()])).unwrap();
        assert_eq!(value, Value::Groups(vec![bow.clone()]));
        let dialog = Dialog::Group(GroupEdit::new(0, Some(0), 1, Vec::new()));
        let value = dialog.finish(Value::Groups(vec![bow])).unwrap();
        assert_eq!(value, Value::Groups(Vec::new()));
    }

    #[test]
    fn an_attack_without_a_name_keeps_the_dialog_open() {
        let dialog = Dialog::Attack(AttackEdit::new(0, None, MonsterAttack::default()));
        let Err(again) = dialog.finish(Value::Attacks(Vec::new())) else {
            panic!("a nameless attack was taken");
        };
        let Dialog::Attack(again) = *again else {
            panic!("a nameless attack was taken");
        };
        assert!(again.problem.is_some());
        let mut dialog = Dialog::Attack(again);
        dialog.input(DialogInput::Attack(AttackInput::Name(" Bite ".into())));
        let Ok(Value::Attacks(attacks)) = dialog.finish(Value::Attacks(Vec::new())) else {
            panic!("the attack was refused");
        };
        assert_eq!(attacks[0].name, "Bite");
    }
}
