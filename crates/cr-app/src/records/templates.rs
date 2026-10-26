//! The encounter template editor with no toolkit in it (IDD 289): the
//! template's name, monster type and notes, its monster, NPC and item
//! lines with their Properties (IDD 304, 384 and 309, with IDD 311's
//! lists), its traps, its lair treasure and its 2-20 table information
//! (IDD 325), and the pickers its Add buttons open (IDD 302, 306, 313 and
//! 314). NPCs come from NPC Creation in template mode, which the app
//! opens; standard templates are only copied, as in the original.

use std::sync::Arc;

use cr_formats::Object;
use cr_formats::encounter::{
    EncounterNpc, EncounterTemplate, TemplateItemProperties, TemplateMonsterProperties,
    TemplateNpcProperties,
};
use cr_formats::monster::{Monster, MonsterType};
use cr_formats::parts::Part;
use cr_formats::traps::Trap;
use cr_i18n::{tr, tr_noop};
use cr_store::Installation;
use cr_store::help::topic;
use cr_store::npc::Npc;
use cr_store::treasure::LETTERS;
use cr_store::userdb::monsters::{CLIMATES, CUSTOM_TERRAIN, MonsterTypes, Monsters, TERRAINS};
use cr_store::userdb::parts::{self, Parts};
use cr_store::userdb::templates::{self as tpl, Combo, Templates};
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{self as udb, Filter, Flavor, Origin, UserDb};

use super::form::Saved;
use crate::prompt::{self, Answer};

/// A question or message the template editor waits on.
pub type Prompt = prompt::Prompt<TemplateForm>;

/// The section of the encounter templates.
pub const SECTION: &str = "templates";

/// The most the boxes of a line take, as the original's fields allow.
pub const MOST: i32 = 1_000_000;
/// The most of each treasure type a lair holds.
pub const MOST_TREASURE: i32 = 100;
pub const MOST_HIT_DICE: i32 = 300_000_000;
/// The coins an item's Containing counts, in the combo's order: the
/// original's pp, gp, sp and cp.
pub const COINS: [&str; 4] = [
    tr_noop!("Platinum"),
    tr_noop!("Gold"),
    tr_noop!("Silver"),
    tr_noop!("Copper"),
];
/// The type a monster picker shows first when it shows every type.
pub const ALL_TYPES: &str = "~All";

/// What each box of "N d S * M + B" takes, lowest and highest.
pub type Ranges = [(i32, i32); 4];
/// The second box of a line, after "and" or "or".
pub const SECOND_RANGES: Ranges = [(0, MOST), (1, MOST), (1, MOST), (0, MOST)];
/// The coins an item holds.
pub const COIN_RANGES: Ranges = [(0, MOST), (0, MOST), (0, MOST), (0, 100_000)];
/// How many of a monster appear with an item (IDD 313).
pub const ESCORT_RANGES: Ranges = [(1, MOST), (1, MOST), (1, MOST), (0, MOST)];

/// Sets box `k` of `dice` to `value`, kept in its range.
fn set_die(dice: &mut [i32], k: usize, value: i32, ranges: &Ranges) {
    if let (Some(slot), Some((lo, hi))) = (dice.get_mut(k), ranges.get(k)) {
        *slot = value.clamp(*lo, *hi);
    }
}

/// What the editor offers to add, read when it opens: the monsters,
/// monster types, traps and parts of both databases, and the climate and
/// terrain categories.
pub struct TemplateData {
    pub flavor: Flavor,
    /// Every monster, by name, with where it comes from.
    pub monsters: Vec<(Origin, Monster)>,
    pub types: Vec<(Origin, MonsterType)>,
    /// The type names of IDD 279's list, sorted.
    pub type_names: Vec<String>,
    pub traps: UserDb<Traps>,
    /// Every trap, by name, with where it comes from and its damage.
    pub trap_rows: Vec<(Origin, String, String)>,
    /// Every part an item line may hold: its type, where it comes from
    /// and the part.
    pub parts: Vec<(usize, Origin, Part)>,
    /// The part types IDD 314 offers, with their names.
    pub categories: Vec<(usize, &'static str)>,
    pub terrains: Vec<String>,
}

impl TemplateData {
    pub fn load(install: &Installation) -> cr_store::Result<Self> {
        let flavor = cr_store::encounter::flavor_of(install);
        let monsters = UserDb::<Monsters>::load(install)?;
        let types = UserDb::<MonsterTypes>::load(install)?;
        let parts = UserDb::<Parts>::load(install)?;
        let traps = UserDb::<Traps>::load(install)?;
        let types: Vec<(Origin, MonsterType)> = types
            .list(Filter::All)
            .into_iter()
            .map(|e| (e.origin, e.record.clone()))
            .collect();
        let mut type_names: Vec<String> = types.iter().map(|(_, t)| t.name.clone()).collect();
        type_names.dedup();
        Ok(Self {
            flavor,
            monsters: monsters
                .list(Filter::All)
                .into_iter()
                .map(|e| (e.origin, e.record.clone()))
                .collect(),
            types,
            type_names,
            trap_rows: traps
                .list(Filter::All)
                .into_iter()
                .map(|e| (e.origin, e.record.name.clone(), e.record.damage.clone()))
                .collect(),
            traps,
            parts: parts
                .list(Filter::All)
                .into_iter()
                .map(|e| (e.record.part_type, e.origin, e.record.part.clone()))
                .collect(),
            categories: parts::editable_types(flavor),
            terrains: udb::monsters::distinct(install, "monster-terrain")?,
        })
    }

    /// The monster types the template's box offers, with the template's
    /// own first when it is not among them, and the one it has.
    pub fn type_choices(&self, current: &str) -> (Vec<String>, usize) {
        let mut types = self.type_names.clone();
        if !types.iter().any(|t| t == current) {
            types.insert(0, current.to_owned());
        }
        let chosen = types.iter().position(|t| t == current).unwrap_or(0);
        (types, chosen)
    }
}

/// The NPC as NPC Creation's "Add to Template" stores it.
pub fn template_npc(npc: &Npc) -> EncounterNpc {
    tpl::npc_of(
        &npc.character.data.name,
        &npc.character.data.race,
        &npc.classes,
    )
}

/// IDD 303's list: each monster line's text with its index, sorted by
/// the text.
pub fn monster_rows(t: &EncounterTemplate) -> Vec<(String, usize)> {
    let mut lines: Vec<(String, usize)> = t
        .monsters
        .iter()
        .enumerate()
        .filter_map(|(i, o)| match o {
            Object::TemplateMonsterProperties(p) => Some((tpl::monster_line_text(p), i)),
            _ => None,
        })
        .collect();
    lines.sort_by_cached_key(|(text, _)| text.to_lowercase());
    lines
}

/// IDD 383's list, in order.
pub fn npc_rows(t: &EncounterTemplate) -> Vec<String> {
    t.npcs
        .iter()
        .map(|o| match o {
            Object::TemplateNpcProperties(p) => tpl::npc_line_text(p),
            _ => String::new(),
        })
        .collect()
}

/// IDD 315's list: each item's name, and how many and how often.
pub fn item_rows(t: &EncounterTemplate) -> Vec<(String, String)> {
    t.items
        .iter()
        .map(|o| match o {
            Object::TemplateItemProperties(p) => (
                p.part.head.name.clone(),
                format!(
                    "{}d{} * {} + {} · {}%",
                    p.dice[0], p.dice[1], p.dice[2], p.dice[3], p.percent
                ),
            ),
            _ => Default::default(),
        })
        .collect()
}

/// The template's traps.
pub fn trap_list(t: &EncounterTemplate) -> Vec<Trap> {
    t.traps
        .iter()
        .filter_map(|o| match o {
            Object::Trap(t) => Some((**t).clone()),
            _ => None,
        })
        .collect()
}

fn set_traps(t: &mut EncounterTemplate, traps: Vec<Trap>) {
    t.traps = traps
        .into_iter()
        .map(|t| Object::Trap(Box::new(t)))
        .collect();
}

/// The random list of a monster line: each name, and whether Remove
/// takes it. The line's own monster stays while it is the only entry of
/// that name (0x72dde2).
pub fn random_monsters(p: &TemplateMonsterProperties) -> Vec<(String, bool)> {
    let names: Vec<String> = p
        .monsters
        .iter()
        .filter_map(|o| match o {
            Object::Monster(m) => Some(m.name.clone()),
            _ => None,
        })
        .collect();
    let main = &p.monster_name;
    let mains = names.iter().filter(|n| *n == main).count();
    names
        .iter()
        .map(|n| (n.clone(), n != main || mains > 1))
        .collect()
}

/// The random list of an NPC line: each NPC, and whether Remove takes
/// it. The line's own NPC stays while it is there only once (0x72f56f).
pub fn random_npcs(p: &TemplateNpcProperties) -> Vec<(EncounterNpc, bool)> {
    let npcs: Vec<EncounterNpc> = p
        .npcs
        .iter()
        .filter_map(|o| match o {
            Object::EncounterNpc(n) => Some((**n).clone()),
            _ => None,
        })
        .collect();
    let mains = npcs.iter().filter(|n| **n == p.npc).count();
    npcs.iter()
        .map(|n| (n.clone(), *n != p.npc || mains > 1))
        .collect()
}

/// What a dependent item counts, as IDD 309 shows it: the monster's
/// name, else the type's.
pub fn item_reference(p: &TemplateItemProperties) -> String {
    if p.monster.name.is_empty() {
        p.monster_type.name.clone()
    } else {
        p.monster.name.clone()
    }
}

/// The pages of the editor, one per button of the original's dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplatePage {
    Template,
    Monsters,
    Npcs,
    Items,
    Traps,
    Treasure,
    Table,
}

impl TemplatePage {
    pub const ALL: [Self; 7] = [
        Self::Template,
        Self::Monsters,
        Self::Npcs,
        Self::Items,
        Self::Traps,
        Self::Treasure,
        Self::Table,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Template => tr("Template"),
            Self::Monsters => tr("Monsters"),
            Self::Npcs => tr("NPC's"),
            Self::Items => tr("Items"),
            Self::Traps => tr("Traps"),
            Self::Treasure => tr("Treasure Types"),
            Self::Table => tr("2 - 20 Encounter Table"),
        }
    }

    pub fn help(self) -> u32 {
        match self {
            Self::Template => topic::ENCOUNTER_TEMPLATE,
            Self::Monsters => topic::TEMPLATE_MONSTERS,
            Self::Npcs => topic::TEMPLATE_NPCS,
            Self::Items => topic::TEMPLATE_ITEMS,
            Self::Traps => topic::TRAPS,
            Self::Treasure => topic::TREASURE_TYPES,
            Self::Table => topic::TEMPLATE_TABLE,
        }
    }
}

/// The Properties of a line, open in place of the page, on a copy of the
/// line that OK stores.
#[derive(Debug, Clone, PartialEq)]
pub enum LineEdit {
    Monster {
        index: usize,
        line: Box<TemplateMonsterProperties>,
    },
    Npc {
        index: usize,
        line: Box<TemplateNpcProperties>,
    },
    Item {
        index: usize,
        line: Box<TemplateItemProperties>,
    },
}

impl LineEdit {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Monster { .. } => tr("Monster Properties for Encounter Templates"),
            Self::Npc { .. } => tr("Nonplayer Character Properties for Encounter Templates"),
            Self::Item { .. } => tr("Adding Items to an Encounter Template"),
        }
    }

    pub fn help(&self) -> u32 {
        match self {
            Self::Monster { .. } => topic::TEMPLATE_MONSTER,
            Self::Npc { .. } => topic::TEMPLATE_NPC,
            Self::Item { .. } => topic::TEMPLATE_ITEM,
        }
    }

    /// The first box: how many appear and the percent chance.
    pub fn first(&self) -> ([i32; 4], i32) {
        let (dice, percent) = match self {
            Self::Monster { line, .. } => (&line.dice[..4], line.percent),
            Self::Npc { line, .. } => (&line.dice[..4], line.percent),
            Self::Item { line, .. } => (&line.dice[..4], line.percent),
        };
        (std::array::from_fn(|k| dice[k]), percent)
    }

    /// What the first box's numbers take (IDD 304 and 384 allow fewer
    /// sides than IDD 309).
    pub fn first_ranges(&self) -> Ranges {
        let (sides, multiplier) = match self {
            Self::Item { .. } => (MOST, MOST),
            _ => (1000, 10_000),
        };
        [(1, MOST), (1, sides), (1, multiplier), (0, MOST)]
    }

    /// "Dependent on the following", for every how many, beyond how many.
    pub fn dependent(&self) -> (bool, i32, i32) {
        match self {
            Self::Monster { line, .. } => (line.flags[0] != 0, line.per, line.beyond),
            Self::Npc { line, .. } => (line.flags[0] != 0, line.per, line.beyond),
            Self::Item { line, .. } => (
                line.dependency[0] != 0,
                line.dependency[1],
                line.dependency[2],
            ),
        }
    }

    /// What a dependent line counts: a monster's name or a type's.
    pub fn reference(&self) -> String {
        match self {
            Self::Monster { line, .. } => {
                tpl::reference_text(&line.reference_name, &line.monster_type)
            }
            Self::Npc { line, .. } => tpl::reference_text(&line.reference_name, &line.monster_type),
            Self::Item { line, .. } => item_reference(line),
        }
    }

    /// The combo after the random list, for monsters and NPCs.
    pub fn combo(&self) -> Option<Combo> {
        match self {
            Self::Monster { line, .. } => Some(Combo::of_flags(&line.flags)),
            Self::Npc { line, .. } => Some(Combo::of_flags(&line.flags)),
            Self::Item { .. } => None,
        }
    }

    /// The second box's numbers.
    pub fn second_dice(&self) -> [i32; 4] {
        let dice = match self {
            Self::Monster { line, .. } => &line.dice[4..],
            Self::Npc { line, .. } => &line.dice[4..],
            Self::Item { .. } => return [1, 1, 1, 0],
        };
        std::array::from_fn(|k| dice[k])
    }

    /// Who the second box counts: the monster's name or the NPC's
    /// description.
    pub fn second(&self) -> String {
        match self {
            Self::Monster { line, .. } => line.second_name.clone(),
            Self::Npc { line, .. } => tpl::npc_description(&line.second_npc),
            Self::Item { .. } => String::new(),
        }
    }

    /// The random list: each entry's line and the line under it, and
    /// whether Remove takes it; none for an item.
    pub fn random(&self) -> Vec<(String, String, bool)> {
        match self {
            Self::Monster { line, .. } => random_monsters(line)
                .into_iter()
                .map(|(name, removable)| (name, String::new(), removable))
                .collect(),
            Self::Npc { line, .. } => random_npcs(line)
                .into_iter()
                .map(|(npc, removable)| (tpl::npc_description(&npc), npc.name, removable))
                .collect(),
            Self::Item { .. } => Vec::new(),
        }
    }

    /// The item line, for what only items have.
    pub fn item(&self) -> Option<&TemplateItemProperties> {
        match self {
            Self::Item { line, .. } => Some(line),
            _ => None,
        }
    }

    /// An item's coins: the coin, by index among [`COINS`], and its
    /// dice.
    pub fn coins(&self) -> (usize, [i32; 4]) {
        match self.item() {
            Some(p) => (
                p.dice[4].clamp(0, 3) as usize,
                std::array::from_fn(|k| p.dice[5 + k]),
            ),
            None => (0, [0; 4]),
        }
    }

    /// An item's monsters that appear with it (false) or with each one
    /// (true).
    pub fn escorts(&self, per_item: bool) -> Vec<String> {
        let Some(p) = self.item() else {
            return Vec::new();
        };
        let escorts = if per_item { &p.per_item } else { &p.escorts };
        escorts
            .iter()
            .filter_map(|o| match o {
                Object::TemplateNumAppear(n) => Some(tpl::escort_text(n)),
                _ => None,
            })
            .collect()
    }

    /// An item's "Double the number of these Monsters".
    pub fn doubles(&self) -> &[String] {
        self.item().map_or(&[], |p| p.monster_names.as_slice())
    }

    fn input(&mut self, input: LineInput) -> Option<Request> {
        let ranges = self.first_ranges();
        match (self, input) {
            (Self::Monster { line, .. }, LineInput::Dice(k, v)) => {
                set_die(&mut line.dice[..4], k, v, &ranges)
            }
            (Self::Npc { line, .. }, LineInput::Dice(k, v)) => {
                set_die(&mut line.dice[..4], k, v, &ranges)
            }
            (Self::Item { line, .. }, LineInput::Dice(k, v)) => {
                set_die(&mut line.dice[..4], k, v, &ranges)
            }
            (Self::Monster { line, .. }, LineInput::Percent(v)) => line.percent = v.clamp(1, 100),
            (Self::Npc { line, .. }, LineInput::Percent(v)) => line.percent = v.clamp(1, 100),
            (Self::Item { line, .. }, LineInput::Percent(v)) => line.percent = v.clamp(1, 100),
            (Self::Monster { line, .. }, LineInput::Dependent(on)) => line.flags[0] = i32::from(on),
            (Self::Npc { line, .. }, LineInput::Dependent(on)) => line.flags[0] = i32::from(on),
            (Self::Item { line, .. }, LineInput::Dependent(on)) => {
                line.dependency[0] = i32::from(on)
            }
            (Self::Monster { line, .. }, LineInput::Per(v)) => line.per = v.clamp(0, MOST),
            (Self::Npc { line, .. }, LineInput::Per(v)) => line.per = v.clamp(0, MOST),
            (Self::Item { line, .. }, LineInput::Per(v)) => line.dependency[1] = v.clamp(0, MOST),
            (Self::Monster { line, .. }, LineInput::Beyond(v)) => line.beyond = v.clamp(0, MOST),
            (Self::Npc { line, .. }, LineInput::Beyond(v)) => line.beyond = v.clamp(0, MOST),
            (Self::Item { line, .. }, LineInput::Beyond(v)) => {
                line.dependency[2] = v.clamp(0, MOST)
            }
            (_, LineInput::ChooseReference) => {
                return Some(Request::Monster(MonsterFor::Reference));
            }
            (Self::Monster { .. }, LineInput::AddRandom) => {
                return Some(Request::Monster(MonsterFor::Random));
            }
            (Self::Npc { .. }, LineInput::AddRandom) => return Some(Request::Npc(NpcFor::Random)),
            (this @ (Self::Monster { .. } | Self::Npc { .. }), LineInput::RemoveRandom(k)) => {
                if this
                    .random()
                    .get(k)
                    .is_some_and(|(.., removable)| *removable)
                {
                    match this {
                        Self::Monster { line, .. } => {
                            remove_nth(&mut line.monsters, k, |o| matches!(o, Object::Monster(_)))
                        }
                        Self::Npc { line, .. } => {
                            remove_nth(&mut line.npcs, k, |o| matches!(o, Object::EncounterNpc(_)))
                        }
                        Self::Item { .. } => {}
                    }
                }
            }
            (Self::Monster { line, .. }, LineInput::Combo(i)) => {
                line.flags = Combo::of(i as u32).flags(line.flags[0] != 0)
            }
            (Self::Npc { line, .. }, LineInput::Combo(i)) => {
                line.flags = Combo::of(i as u32).flags(line.flags[0] != 0)
            }
            (Self::Monster { line, .. }, LineInput::SecondDice(k, v)) => {
                set_die(&mut line.dice[4..], k, v, &SECOND_RANGES)
            }
            (Self::Npc { line, .. }, LineInput::SecondDice(k, v)) => {
                set_die(&mut line.dice[4..], k, v, &SECOND_RANGES)
            }
            (Self::Monster { .. }, LineInput::ChooseSecond) => {
                return Some(Request::Monster(MonsterFor::Second));
            }
            (Self::Npc { .. }, LineInput::ChooseSecond) => {
                return Some(Request::Npc(NpcFor::Second));
            }
            (Self::Item { line, .. }, LineInput::Coin(i)) => line.dice[4] = i.min(3) as i32,
            (Self::Item { line, .. }, LineInput::CoinDice(k, v)) => {
                set_die(&mut line.dice[5..], k, v, &COIN_RANGES)
            }
            (Self::Item { .. }, LineInput::AddEscort(per_item)) => {
                return Some(Request::Monster(MonsterFor::Escort(per_item)));
            }
            (Self::Item { line, .. }, LineInput::RemoveEscort(per_item, k)) => {
                let escorts = if per_item {
                    &mut line.per_item
                } else {
                    &mut line.escorts
                };
                remove_nth(escorts, k, |o| matches!(o, Object::TemplateNumAppear(_)));
            }
            (Self::Item { .. }, LineInput::AddDouble) => {
                return Some(Request::Monster(MonsterFor::Double));
            }
            (Self::Item { line, .. }, LineInput::RemoveDouble(k)) => {
                // The first of that name (0x7333ba).
                if let Some(name) = line.monster_names.get(k).cloned()
                    && let Some(first) = line.monster_names.iter().position(|n| *n == name)
                {
                    line.monster_names.remove(first);
                }
            }
            _ => {}
        }
        None
    }

    /// Keeps what a monster picker chose for this line.
    fn picked(&mut self, purpose: MonsterFor, picked: Picked) {
        match (self, purpose, picked) {
            (Self::Monster { line, .. }, MonsterFor::Random, Picked::Monster(m)) => {
                line.monsters.push(Object::Monster(m))
            }
            (Self::Monster { line, .. }, MonsterFor::Second, Picked::Monster(m)) => {
                line.second_name = m.name
            }
            (Self::Monster { line, .. }, MonsterFor::Reference, picked) => match picked {
                Picked::Monster(m) => {
                    line.reference_name = m.name;
                    line.monster_type = tpl::empty_type();
                }
                Picked::Type(t) => {
                    line.monster_type = t;
                    line.reference_name.clear();
                }
            },
            (Self::Npc { line, .. }, MonsterFor::Reference, picked) => match picked {
                Picked::Monster(m) => {
                    line.reference_name = m.name;
                    line.monster_type = tpl::empty_type();
                }
                Picked::Type(t) => {
                    line.monster_type = t;
                    line.reference_name.clear();
                }
            },
            (Self::Item { line, .. }, MonsterFor::Reference, picked) => match picked {
                Picked::Monster(m) => {
                    line.monster = m;
                    line.monster_type = tpl::empty_type();
                }
                Picked::Type(t) => {
                    line.monster_type = t;
                    *line.monster = tpl::empty_monster();
                }
            },
            (Self::Item { line, .. }, MonsterFor::Double, Picked::Monster(m)) => {
                line.monster_names.push(m.name)
            }
            _ => {}
        }
    }

    /// Keeps the NPC NPC Creation made for this line.
    fn npc_made(&mut self, purpose: NpcFor, npc: EncounterNpc) {
        if let Self::Npc { line, .. } = self {
            match purpose {
                NpcFor::Random => line.npcs.push(Object::EncounterNpc(Box::new(npc))),
                NpcFor::Second => line.second_npc = npc,
                NpcFor::Line => {}
            }
        }
    }
}

/// Removes the `k`th of the objects `wanted` takes.
fn remove_nth(objects: &mut Vec<Object>, k: usize, wanted: impl Fn(&Object) -> bool) {
    if let Some(i) = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| wanted(o))
        .nth(k)
        .map(|(i, _)| i)
    {
        objects.remove(i);
    }
}

/// What a monster picker's choice goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterFor {
    /// Add… of the Monsters page: a new line.
    Line,
    /// The random list of the line open.
    Random,
    /// The second box's monster.
    Second,
    /// What a dependent line counts: a monster or a monster type.
    Reference,
    /// An item's "Double the number of these Monsters".
    Double,
    /// A monster that appears with an item, with each one (true) or with
    /// them all.
    Escort(bool),
}

/// What NPC Creation's NPC goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcFor {
    /// Add… of the NPC's page: a new line.
    Line,
    /// The random list of the line open.
    Random,
    /// The second box's NPC.
    Second,
}

/// What a line asks of the editor.
enum Request {
    Monster(MonsterFor),
    Npc(NpcFor),
}

/// What a monster picker hands back.
enum Picked {
    Monster(Box<Monster>),
    Type(MonsterType),
}

/// IDD 302 / 306: a monster from either database, by origin, type and
/// name; or, with `instead`, a monster type.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterPicker {
    pub purpose: MonsterFor,
    pub origin: Filter,
    /// The type whose monsters are shown, by index among
    /// [`TemplateForm::picker_types`]: 0 for every type.
    pub kind: usize,
    /// "Select Type instead of Monster".
    pub instead: bool,
    pub search: String,
    /// The monster or type chosen, by index in the editor's data.
    pub chosen: Option<usize>,
}

/// IDD 314: a part of one category.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemPicker {
    /// By index among the data's categories.
    pub category: usize,
    pub search: String,
    pub chosen: Option<usize>,
}

/// The Traps page's Add: a trap of either database.
#[derive(Debug, Clone, PartialEq)]
pub struct TrapPicker {
    pub search: String,
    pub chosen: Option<usize>,
}

/// A picker over the page.
#[derive(Debug, Clone, PartialEq)]
pub enum Picker {
    Monster(MonsterPicker),
    Item(ItemPicker),
    Trap(TrapPicker),
}

impl Picker {
    fn search(&mut self) -> &mut String {
        match self {
            Self::Monster(p) => &mut p.search,
            Self::Item(p) => &mut p.search,
            Self::Trap(p) => &mut p.search,
        }
    }

    pub fn chosen(&self) -> Option<usize> {
        match self {
            Self::Monster(p) => p.chosen,
            Self::Item(p) => p.chosen,
            Self::Trap(p) => p.chosen,
        }
    }

    fn chosen_mut(&mut self) -> &mut Option<usize> {
        match self {
            Self::Monster(p) => &mut p.chosen,
            Self::Item(p) => &mut p.chosen,
            Self::Trap(p) => &mut p.chosen,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Monster(MonsterPicker {
                purpose: MonsterFor::Line,
                ..
            }) => tr("Monsters for this Encounter Template"),
            Self::Monster(_) => tr("More Monsters for the Encounter Template"),
            Self::Item(_) => tr("Add this Item to the Encounter Template"),
            Self::Trap(_) => tr("Add Trap"),
        }
    }

    pub fn help(&self) -> u32 {
        match self {
            Self::Monster(MonsterPicker {
                purpose: MonsterFor::Line,
                ..
            }) => topic::TEMPLATE_ADD_MONSTER,
            Self::Monster(_) => topic::TEMPLATE_MORE_MONSTERS,
            Self::Item(_) => topic::TEMPLATE_ADD_ITEM,
            Self::Trap(_) => topic::TRAPS,
        }
    }
}

/// An entry a picker lists.
#[derive(Debug, Clone, PartialEq)]
pub struct PickRow {
    pub title: String,
    pub subtitle: String,
    pub custom: bool,
    /// The entry, by index in the editor's data.
    pub index: usize,
}

/// IDD 313 once its monster is picked: how many appear with the item.
#[derive(Debug, Clone, PartialEq)]
pub struct Escort {
    /// In the per-item list, or the other.
    pub per_item: bool,
    pub monster: Monster,
    pub dice: [i32; 4],
}

/// A change in the template editor, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateInput {
    /// Show the page with this index among [`TemplatePage::ALL`].
    Page(usize),
    Name(String),
    /// The monster type, by index among [`TemplateData::type_choices`].
    MonsterType(usize),
    Notes(String),
    /// The lair's count of the treasure type with this index.
    Treasure(usize, i32),
    /// Encountered, by index among the placements.
    Placement(usize),
    /// The frequency, by index among the frequencies.
    Frequency(usize),
    HitDice(i32),
    /// The climate and terrain category.
    Category(String),
    /// A climate or terrain of the ~Custom category, by index.
    Climate(usize, bool),
    AddMonster,
    AddNpc,
    AddItem,
    AddTrap,
    /// Open the Properties of the monster, NPC or item line with this
    /// index.
    OpenMonster(usize),
    OpenNpc(usize),
    OpenItem(usize),
    /// Remove the line, or trap, with this index.
    RemoveMonster(usize),
    RemoveNpc(usize),
    RemoveItem(usize),
    RemoveTrap(usize),
    Line(LineInput),
    /// OK (true) or Cancel on the Properties.
    CloseLine(bool),
    Picker(PickerInput),
    /// Add or Choose (true), or Cancel, on the picker.
    ClosePicker(bool),
    /// One of the escort's numbers.
    EscortDice(usize, i32),
    /// Add (true) or Cancel on the escort's numbers.
    CloseEscort(bool),
    Save,
}

/// A change in a line's Properties.
#[derive(Debug, Clone, PartialEq)]
pub enum LineInput {
    /// One of the first box's numbers: N, S, M or B.
    Dice(usize, i32),
    Percent(i32),
    Dependent(bool),
    Per(i32),
    Beyond(i32),
    /// Choose… for what a dependent line counts.
    ChooseReference,
    AddRandom,
    /// Remove the random list's entry with this index.
    RemoveRandom(usize),
    /// The combo, by index among [`Combo::NAMES`].
    Combo(usize),
    SecondDice(usize, i32),
    /// Monster… or NPC… of the second box.
    ChooseSecond,
    /// An item's coin, by index among [`COINS`], and its numbers.
    Coin(usize),
    CoinDice(usize, i32),
    /// Add Monster on the per-item list (true) or the other.
    AddEscort(bool),
    RemoveEscort(bool, usize),
    AddDouble,
    RemoveDouble(usize),
}

/// A change in a picker.
#[derive(Debug, Clone, PartialEq)]
pub enum PickerInput {
    /// Show Standard or Custom Records, by index among [`Filter::NAMES`].
    Origin(usize),
    /// The monster type shown, by index among
    /// [`TemplateForm::picker_types`].
    Kind(usize),
    Instead(bool),
    /// The item category, by index among the data's.
    Category(usize),
    Search(String),
    /// The entry chosen, by its [`PickRow::index`].
    Choose(usize),
}

/// What the app should do for the template editor.
#[derive(Clone)]
pub enum TemplateEffect {
    /// Open NPC Creation in template mode and pass the NPC "Add to
    /// Template" makes to [`TemplateForm::npc_made`] with this.
    Npc(NpcFor),
    /// Save `template` in place of the user template with index
    /// `editing`, or as a new one, and pass what it gives to
    /// [`TemplateForm::saved`].
    Save {
        editing: Option<usize>,
        template: Box<EncounterTemplate>,
    },
    /// Saved: leave the editor for the entry with `key`, saying `said`.
    Done { key: String, said: String },
}

/// The encounter template editor.
#[derive(Clone)]
pub struct TemplateForm {
    pub template: EncounterTemplate,
    kept: EncounterTemplate,
    /// The template's index in the user file, when it is changed rather
    /// than added.
    editing: Option<usize>,
    title: String,
    data: Arc<TemplateData>,
    pub page: TemplatePage,
    /// The Properties open.
    pub line: Option<LineEdit>,
    pub picker: Option<Picker>,
    pub escort: Option<Escort>,
    /// Counts the pickers and escorts opened, for a dialog of its own each
    /// time.
    pub dialog_serial: u64,
    prompt: Option<Prompt>,
    serial: u64,
    effects: Vec<TemplateEffect>,
    pub error: Option<String>,
    pub saving: bool,
}

impl TemplateForm {
    /// The editor of `template` under `title`, the user template with
    /// index `editing` when it is changed rather than added.
    pub fn new(
        template: EncounterTemplate,
        editing: Option<usize>,
        title: String,
        data: Arc<TemplateData>,
    ) -> Self {
        Self {
            kept: template.clone(),
            template,
            editing,
            title,
            data,
            page: TemplatePage::Template,
            line: None,
            picker: None,
            escort: None,
            dialog_serial: 0,
            prompt: None,
            serial: 0,
            effects: Vec::new(),
            error: None,
            saving: false,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn data(&self) -> &TemplateData {
        &self.data
    }

    /// The help topic of what is in view: the picker or escort over the
    /// page, the Properties open, or the page.
    pub fn help(&self) -> u32 {
        if self.escort.is_some() {
            topic::TEMPLATE_ESCORT
        } else if let Some(picker) = &self.picker {
            picker.help()
        } else if let Some(line) = &self.line {
            line.help()
        } else {
            self.page.help()
        }
    }

    /// The monster types the template's box offers, and its own.
    pub fn type_choices(&self) -> (Vec<String>, usize) {
        self.data.type_choices(&self.template.monster_type)
    }

    /// Whether the climates and terrains are chosen one by one.
    pub fn custom_terrain(&self) -> bool {
        self.template.climate_terrain == CUSTOM_TERRAIN
    }

    /// The climates and terrains, each with whether the template has it.
    pub fn climates(&self) -> Vec<(&'static str, bool)> {
        CLIMATES
            .iter()
            .chain(TERRAINS.iter())
            .enumerate()
            .map(|(i, place)| (tr(place), self.template.climates[i] != 0))
            .collect()
    }

    /// The treasure types with the lair's count of each.
    pub fn treasure(&self) -> Vec<(char, i32)> {
        LETTERS
            .iter()
            .zip(self.template.lair_treasure)
            .map(|(l, n)| (*l, n))
            .collect()
    }

    /// The monster types a monster picker shows, every one first.
    pub fn picker_types(&self) -> Vec<String> {
        let mut types = vec![ALL_TYPES.to_owned()];
        types.extend(self.data.type_names.iter().cloned());
        types
    }

    /// What the picker lists, as its origin, type, category and search
    /// let through.
    pub fn picker_rows(&self) -> Vec<PickRow> {
        let Some(picker) = &self.picker else {
            return Vec::new();
        };
        let data = &self.data;
        let matches = |name: &str, search: &str| {
            name.to_lowercase()
                .contains(search.trim().to_lowercase().as_str())
        };
        match picker {
            Picker::Monster(p) if p.instead => data
                .types
                .iter()
                .enumerate()
                .filter(|(_, (o, t))| p.origin.accepts(*o) && matches(&t.name, &p.search))
                .map(|(i, (o, t))| PickRow {
                    title: t.name.clone(),
                    subtitle: String::new(),
                    custom: *o == Origin::Custom,
                    index: i,
                })
                .collect(),
            Picker::Monster(p) => {
                let wanted = p.kind.checked_sub(1).and_then(|k| data.type_names.get(k));
                data.monsters
                    .iter()
                    .enumerate()
                    .filter(|(_, (o, m))| {
                        p.origin.accepts(*o)
                            && wanted.is_none_or(|w| m.monster_type == *w)
                            && matches(&m.name, &p.search)
                    })
                    .map(|(i, (o, m))| PickRow {
                        title: m.name.clone(),
                        subtitle: m.monster_type.clone(),
                        custom: *o == Origin::Custom,
                        index: i,
                    })
                    .collect()
            }
            Picker::Item(p) => {
                let category = data.categories.get(p.category).map(|(c, _)| *c);
                data.parts
                    .iter()
                    .enumerate()
                    .filter(|(_, (t, _, part))| {
                        Some(*t) == category && matches(&part.head.name, &p.search)
                    })
                    .map(|(i, (_, o, part))| PickRow {
                        title: part.head.name.clone(),
                        subtitle: String::new(),
                        custom: *o == Origin::Custom,
                        index: i,
                    })
                    .collect()
            }
            Picker::Trap(p) => data
                .trap_rows
                .iter()
                .enumerate()
                .filter(|(_, (_, name, _))| matches(name, &p.search))
                .map(|(i, (o, name, damage))| PickRow {
                    title: name.clone(),
                    subtitle: damage.trim().to_owned(),
                    custom: *o == Origin::Custom,
                    index: i,
                })
                .collect(),
        }
    }

    /// Whether anything was changed.
    pub fn changed(&self) -> bool {
        self.template != self.kept
    }

    pub fn can_save(&self) -> bool {
        !self.saving
            && self.prompt.is_none()
            && self.line.is_none()
            && self.picker.is_none()
            && self.escort.is_none()
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    pub fn prompt_serial(&self) -> u64 {
        self.serial
    }

    pub fn take_effects(&mut self) -> Vec<TemplateEffect> {
        std::mem::take(&mut self.effects)
    }

    pub fn answer(&mut self, answer: Answer) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        if let Some(then) = prompt.then(answer) {
            then(self);
        }
    }

    fn show(&mut self, prompt: Prompt) {
        self.prompt = Some(prompt);
        self.serial += 1;
    }

    fn open_picker(&mut self, picker: Picker) {
        self.picker = Some(picker);
        self.dialog_serial += 1;
    }

    fn monster_picker(&mut self, purpose: MonsterFor) {
        // Add… starts on the template's type, the others on every type.
        let kind = match purpose {
            MonsterFor::Line => self
                .data
                .type_names
                .iter()
                .position(|t| *t == self.template.monster_type)
                .map_or(0, |k| k + 1),
            _ => 0,
        };
        self.open_picker(Picker::Monster(MonsterPicker {
            purpose,
            origin: Filter::All,
            kind,
            instead: false,
            search: String::new(),
            chosen: None,
        }));
    }

    fn request(&mut self, request: Request) {
        match request {
            Request::Monster(purpose) => self.monster_picker(purpose),
            Request::Npc(purpose) => self.effects.push(TemplateEffect::Npc(purpose)),
        }
    }

    pub fn input(&mut self, input: TemplateInput) {
        if self.prompt.is_some() || self.saving {
            return;
        }
        let t = &mut self.template;
        match input {
            TemplateInput::Page(p) => {
                if let Some(page) = TemplatePage::ALL.get(p) {
                    self.page = *page;
                }
            }
            TemplateInput::Name(name) => t.name = name,
            TemplateInput::MonsterType(i) => {
                if let Some(name) = self.data.type_choices(&t.monster_type).0.get(i) {
                    t.monster_type.clone_from(name);
                }
            }
            TemplateInput::Notes(notes) => t.notes = notes,
            TemplateInput::Treasure(i, n) => {
                if let Some(slot) = t.lair_treasure.get_mut(i) {
                    *slot = n.clamp(0, MOST_TREASURE);
                }
            }
            TemplateInput::Placement(i) => t.placement = tpl::placement_of(i),
            TemplateInput::Frequency(i) => {
                if let Some(f) = tpl::FREQUENCIES.get(i) {
                    t.frequency = *f;
                }
            }
            TemplateInput::HitDice(n) => t.hit_dice = n.clamp(0, MOST_HIT_DICE),
            TemplateInput::Category(text) => t.climate_terrain = text,
            TemplateInput::Climate(i, on) => {
                if let Some(slot) = t.climates.get_mut(i) {
                    *slot = i32::from(on);
                }
            }
            TemplateInput::AddMonster => self.monster_picker(MonsterFor::Line),
            TemplateInput::AddNpc => self.effects.push(TemplateEffect::Npc(NpcFor::Line)),
            TemplateInput::AddItem => self.open_picker(Picker::Item(ItemPicker {
                category: 0,
                search: String::new(),
                chosen: None,
            })),
            TemplateInput::AddTrap => self.open_picker(Picker::Trap(TrapPicker {
                search: String::new(),
                chosen: None,
            })),
            TemplateInput::OpenMonster(i) => {
                if let Some(Object::TemplateMonsterProperties(p)) = t.monsters.get(i) {
                    let mut line = p.clone();
                    tpl::clamp_dice(&mut line.dice);
                    self.line = Some(LineEdit::Monster { index: i, line });
                }
            }
            TemplateInput::OpenNpc(i) => {
                if let Some(Object::TemplateNpcProperties(p)) = t.npcs.get(i) {
                    let mut line = p.clone();
                    tpl::clamp_dice(&mut line.dice);
                    self.line = Some(LineEdit::Npc { index: i, line });
                }
            }
            TemplateInput::OpenItem(i) => {
                if let Some(Object::TemplateItemProperties(p)) = t.items.get(i) {
                    let mut line = p.clone();
                    tpl::clamp_item_dice(&mut line.dice);
                    self.line = Some(LineEdit::Item { index: i, line });
                }
            }
            TemplateInput::RemoveMonster(i) => {
                if i < t.monsters.len() {
                    t.monsters.remove(i);
                }
            }
            TemplateInput::RemoveNpc(i) => {
                if i < t.npcs.len() {
                    t.npcs.remove(i);
                }
            }
            TemplateInput::RemoveItem(i) => {
                if i < t.items.len() {
                    t.items.remove(i);
                }
            }
            TemplateInput::RemoveTrap(i) => {
                let mut traps = trap_list(t);
                let Some(name) = traps.get(i).map(|trap| trap.name.clone()) else {
                    return;
                };
                match cr_store::encounter::remove_trap(&mut traps, &name) {
                    Ok(()) => set_traps(t, traps),
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            TemplateInput::Line(change) => {
                if let Some(request) = self.line.as_mut().and_then(|l| l.input(change)) {
                    self.request(request);
                }
            }
            TemplateInput::CloseLine(ok) => {
                let Some(line) = self.line.take() else {
                    return;
                };
                if !ok {
                    return;
                }
                match line {
                    LineEdit::Monster { index, mut line } => {
                        tpl::settle_monster_line(&mut line);
                        if let Some(slot) = t.monsters.get_mut(index) {
                            *slot = Object::TemplateMonsterProperties(line);
                        }
                    }
                    LineEdit::Npc { index, mut line } => {
                        tpl::settle_npc_line(&mut line);
                        if let Some(slot) = t.npcs.get_mut(index) {
                            *slot = Object::TemplateNpcProperties(line);
                        }
                    }
                    LineEdit::Item { index, mut line } => {
                        tpl::settle_item_line(&mut line);
                        if let Some(slot) = t.items.get_mut(index) {
                            *slot = Object::TemplateItemProperties(line);
                        }
                    }
                }
            }
            TemplateInput::Picker(change) => self.pick(change),
            TemplateInput::ClosePicker(ok) => {
                let Some(picker) = self.picker.take() else {
                    return;
                };
                if let (true, Some(chosen)) = (ok, picker.chosen()) {
                    self.chose(picker, chosen);
                }
            }
            TemplateInput::EscortDice(k, v) => {
                if let Some(escort) = &mut self.escort {
                    set_die(&mut escort.dice, k, v, &ESCORT_RANGES);
                }
            }
            TemplateInput::CloseEscort(ok) => {
                let Some(escort) = self.escort.take() else {
                    return;
                };
                if let (true, Some(LineEdit::Item { line, .. })) = (ok, &mut self.line) {
                    let escorts = if escort.per_item {
                        &mut line.per_item
                    } else {
                        &mut line.escorts
                    };
                    escorts.push(Object::TemplateNumAppear(Box::new(tpl::new_escort(
                        escort.dice,
                        &escort.monster,
                    ))));
                }
            }
            TemplateInput::Save => {
                if self.can_save() {
                    self.saving = true;
                    self.error = None;
                    self.effects.push(TemplateEffect::Save {
                        editing: self.editing,
                        template: Box::new(self.template.clone()),
                    });
                }
            }
        }
    }

    fn pick(&mut self, change: PickerInput) {
        let Some(picker) = &mut self.picker else {
            return;
        };
        if let PickerInput::Choose(i) = change {
            *picker.chosen_mut() = Some(i);
            return;
        }
        // What was chosen may no longer be listed.
        *picker.chosen_mut() = None;
        match (picker, change) {
            (Picker::Monster(p), PickerInput::Origin(i)) => p.origin = Filter::of(i as u32),
            (Picker::Monster(p), PickerInput::Kind(i)) => p.kind = i,
            (Picker::Monster(p), PickerInput::Instead(on)) => {
                // Only what a dependent line counts may be a type.
                p.instead = on && p.purpose == MonsterFor::Reference
            }
            (Picker::Item(p), PickerInput::Category(i)) => p.category = i,
            (picker, PickerInput::Search(text)) => *picker.search() = text,
            _ => {}
        }
    }

    /// Keeps what the picker chose.
    fn chose(&mut self, picker: Picker, chosen: usize) {
        let data = self.data.clone();
        match picker {
            Picker::Monster(p) => {
                let picked = if p.instead {
                    data.types.get(chosen).map(|(_, t)| Picked::Type(t.clone()))
                } else {
                    data.monsters
                        .get(chosen)
                        .map(|(_, m)| Picked::Monster(Box::new(m.clone())))
                };
                let Some(picked) = picked else {
                    return;
                };
                match (p.purpose, picked) {
                    (MonsterFor::Line, Picked::Monster(m)) => {
                        self.template
                            .monsters
                            .push(Object::TemplateMonsterProperties(Box::new(
                                tpl::new_monster_line(&m),
                            )))
                    }
                    (MonsterFor::Escort(per_item), Picked::Monster(monster)) => {
                        self.escort = Some(Escort {
                            per_item,
                            monster: *monster,
                            dice: [1, 1, 1, 0],
                        });
                        self.dialog_serial += 1;
                    }
                    (purpose, picked) => {
                        if let Some(line) = &mut self.line {
                            line.picked(purpose, picked);
                        }
                    }
                }
            }
            Picker::Item(_) => {
                if let Some((_, _, part)) = data.parts.get(chosen) {
                    self.template
                        .items
                        .push(Object::TemplateItemProperties(Box::new(
                            tpl::new_item_line(part.clone()),
                        )));
                }
            }
            Picker::Trap(_) => {
                let Some((_, name, _)) = data.trap_rows.get(chosen) else {
                    return;
                };
                let mut traps = trap_list(&self.template);
                match cr_store::encounter::add_trap(&mut traps, &data.traps, name) {
                    Ok(()) => set_traps(&mut self.template, traps),
                    Err(e) => self.show(Prompt::message(tr("Unable to Add Trap"), e.to_string())),
                }
            }
        }
    }

    /// Keeps the NPC NPC Creation made for `purpose`: a new line, or the
    /// open line's.
    pub fn npc_made(&mut self, purpose: NpcFor, npc: EncounterNpc) {
        match purpose {
            NpcFor::Line => self
                .template
                .npcs
                .push(Object::TemplateNpcProperties(Box::new(tpl::new_npc_line(
                    npc,
                )))),
            _ => {
                if let Some(line) = &mut self.line {
                    line.npc_made(purpose, npc);
                }
            }
        }
    }

    /// What saving did: done, the editor leaves for the template; a
    /// refused name is said and the editor stays.
    pub fn saved(&mut self, result: Result<Saved, String>) {
        self.saving = false;
        match result {
            Ok(Saved::Done { name, key }) => self.effects.push(TemplateEffect::Done {
                key,
                said: tr!("“{name}” saved", name),
            }),
            Ok(Saved::Refused { title, text }) => self.show(Prompt::message(&title, text)),
            Err(why) => self.error = Some(why),
        }
    }
}

/// Saves `template` in place of the user template with index `editing`,
/// or as a new one.
pub fn save(
    install: &Installation,
    editing: Option<usize>,
    template: EncounterTemplate,
) -> Result<Saved, String> {
    let mut db = UserDb::<Templates>::load(install).map_err(|e| e.to_string())?;
    let stored = match editing {
        Some(i) if i < db.user.len() => db.replace(i, template.clone()),
        _ => db.add(template.clone()),
    };
    match stored {
        Ok(i) => {
            db.save(install).map_err(|e| e.to_string())?;
            let name = db.user[i].name.clone();
            Ok(Saved::Done {
                key: udb::key(SECTION, Origin::Custom, &name),
                name,
            })
        }
        Err(error) => Ok(Saved::Refused {
            title: tr("Not Saved").to_owned(),
            text: db.name_message(&error, &template, editing.is_none()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monster(name: &str, kind: &str) -> Monster {
        let mut m = tpl::empty_monster();
        name.clone_into(&mut m.name);
        kind.clone_into(&mut m.monster_type);
        m
    }

    fn data() -> TemplateData {
        let mut kind = tpl::empty_type();
        "Dragon".clone_into(&mut kind.name);
        TemplateData {
            flavor: Flavor::Expansion,
            monsters: vec![
                (Origin::Standard, monster("Orc", "Humanoid")),
                (Origin::Custom, monster("Red Dragon", "Dragon")),
            ],
            types: vec![(Origin::Standard, kind)],
            type_names: vec!["Dragon".into(), "Humanoid".into()],
            traps: UserDb::new(Flavor::Expansion, Vec::new(), Vec::new()),
            trap_rows: Vec::new(),
            parts: Vec::new(),
            categories: Vec::new(),
            terrains: Vec::new(),
        }
    }

    fn form() -> TemplateForm {
        let mut t = tpl::new_template("Humanoid");
        t.name = "Camp".into();
        TemplateForm::new(t, None, "Camp".into(), Arc::new(data()))
    }

    fn choose(form: &mut TemplateForm, title: &str) {
        let row = form
            .picker_rows()
            .into_iter()
            .find(|r| r.title == title)
            .unwrap_or_else(|| panic!("{title} is not listed"));
        form.input(TemplateInput::Picker(PickerInput::Choose(row.index)));
        form.input(TemplateInput::ClosePicker(true));
    }

    #[test]
    fn add_starts_on_the_templates_type_and_adds_a_line() {
        let mut form = form();
        form.input(TemplateInput::AddMonster);
        let titles: Vec<String> = form.picker_rows().into_iter().map(|r| r.title).collect();
        assert_eq!(titles, ["Orc"]);
        form.input(TemplateInput::Picker(PickerInput::Kind(0)));
        assert_eq!(form.picker_rows().len(), 2);
        form.input(TemplateInput::Picker(PickerInput::Origin(2)));
        choose(&mut form, "Red Dragon");
        assert_eq!(monster_rows(&form.template)[0].0, "(1d1) Red Dragon");
        assert!(form.changed());
    }

    #[test]
    fn properties_change_a_copy_that_ok_keeps() {
        let mut form = form();
        form.input(TemplateInput::AddMonster);
        choose(&mut form, "Orc");
        form.input(TemplateInput::OpenMonster(0));
        form.input(TemplateInput::Line(LineInput::Dice(0, 2)));
        form.input(TemplateInput::Line(LineInput::Dice(1, 5000)));
        form.input(TemplateInput::Line(LineInput::AddRandom));
        choose(&mut form, "Red Dragon");
        // The line's own monster stays while it is there once.
        let random = form.line.as_ref().unwrap().random();
        assert_eq!(random[0], ("Orc".into(), String::new(), false));
        form.input(TemplateInput::Line(LineInput::RemoveRandom(0)));
        assert_eq!(form.line.as_ref().unwrap().random().len(), 2);
        form.input(TemplateInput::Line(LineInput::Combo(2)));
        form.input(TemplateInput::Line(LineInput::ChooseSecond));
        choose(&mut form, "Orc");
        form.input(TemplateInput::Line(LineInput::Dependent(true)));
        form.input(TemplateInput::Line(LineInput::ChooseReference));
        form.input(TemplateInput::Picker(PickerInput::Instead(true)));
        choose(&mut form, "Dragon");
        assert_eq!(form.line.as_ref().unwrap().reference(), "Dragon");
        assert_eq!(monster_rows(&form.template)[0].0, "(1d1) Orc");
        form.input(TemplateInput::CloseLine(true));
        assert_eq!(
            monster_rows(&form.template)[0].0,
            "(2d1000) Orc or Red Dragon or (1d1) Orc"
        );
    }

    #[test]
    fn cancel_leaves_the_line_and_types_are_only_for_references() {
        let mut form = form();
        form.input(TemplateInput::AddMonster);
        choose(&mut form, "Orc");
        let kept = form.template.clone();
        form.input(TemplateInput::OpenMonster(0));
        form.input(TemplateInput::Line(LineInput::Percent(50)));
        form.input(TemplateInput::Line(LineInput::AddRandom));
        form.input(TemplateInput::Picker(PickerInput::Instead(true)));
        assert!(matches!(
            &form.picker,
            Some(Picker::Monster(MonsterPicker { instead: false, .. }))
        ));
        form.input(TemplateInput::ClosePicker(false));
        form.input(TemplateInput::CloseLine(false));
        assert_eq!(form.template, kept);
    }

    #[test]
    fn npcs_come_from_npc_creation() {
        let mut form = form();
        form.input(TemplateInput::AddNpc);
        assert!(matches!(
            form.take_effects().as_slice(),
            [TemplateEffect::Npc(NpcFor::Line)]
        ));
        let npc = tpl::npc_of("Al", "Dwarf", &[("Thief".into(), 2)]);
        form.npc_made(NpcFor::Line, npc);
        assert_eq!(npc_rows(&form.template), ["(1d1) Dwarf Thief Level: 2"]);
        form.input(TemplateInput::OpenNpc(0));
        form.input(TemplateInput::Line(LineInput::ChooseSecond));
        assert!(matches!(
            form.take_effects().as_slice(),
            [TemplateEffect::Npc(NpcFor::Second)]
        ));
        form.npc_made(
            NpcFor::Second,
            tpl::npc_of("", "Human", &[("Cleric".into(), 1)]),
        );
        assert_eq!(
            form.line.as_ref().unwrap().second(),
            "Human Cleric Level: 1"
        );
    }

    #[test]
    fn an_escort_is_picked_then_counted() {
        let mut form = form();
        let mut part = parts::new_part(Flavor::Expansion, parts::EQUIPMENT);
        "Chest".clone_into(&mut part.head.name);
        form.template
            .items
            .push(Object::TemplateItemProperties(Box::new(
                tpl::new_item_line(part),
            )));
        form.input(TemplateInput::OpenItem(0));
        form.input(TemplateInput::Line(LineInput::AddEscort(true)));
        form.input(TemplateInput::Picker(PickerInput::Kind(0)));
        choose(&mut form, "Orc");
        assert!(form.escort.is_some() && form.picker.is_none());
        form.input(TemplateInput::EscortDice(0, 3));
        form.input(TemplateInput::CloseEscort(true));
        let line = form.line.as_ref().unwrap();
        assert_eq!(
            line.escorts(true),
            ["Number Appearing: 3d1 * 1 + 0, Name: Orc"]
        );
        assert!(line.escorts(false).is_empty());
        assert!(!form.can_save());
    }
}
