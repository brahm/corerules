//! The encounter generator of the DM Toolkit (Monsters and Encounters):
//! encounter templates (`Database/EncPlat.dat`, `UserDbas/EncPlatU.dat`),
//! the encounters made from them (`UserDbas/EncountU.dat`), the editing the
//! Encounter dialog does, the 2-20 frequency tables
//! (`UserDbas/EncTabU.dat`) and the texts the originals show and print.
//!
//! Generation reproduces the originals for a given seed: it draws from the
//! MSVC `rand()` ([`Rand`]) in the same order, quirks included, and rolls
//! the treasure with the treasure engine from the same stream. NPC lines
//! of a template need the NPC generator, which is not ported yet: they
//! come back as [`PendingNpc`]s instead of NPC packages, and the rolls the
//! generator would make are missing from the stream (the NPCs are the last
//! step, so everything else matches). Addresses are the Expansion's unless
//! marked "2.0".
//!
//! ```no_run
//! # fn main() -> cr_store::Result<()> {
//! use cr_store::encounter::Rand;
//! let install = cr_store::Installation::open("/path/to/AD&D Core Rules 2.0")?;
//! let rules = install.encounter_rules()?;
//! let mut rand = Rand::seeded(628);
//! let generated = rules.generate("Orc, Wandering", &mut rand)?;
//! print!("{}", rules.encounter_text(&generated.encounter));
//! # Ok(())
//! # }
//! ```

mod generate;
mod tables;
mod text;

use std::cell::Cell;
use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::character::{CORE20_VERSION, Character};
use cr_formats::database::ObjectList;
use cr_formats::encounter as format;
use cr_formats::monster::Monster;
use cr_formats::parts::PartId;
use cr_formats::traps::Trap;

pub use crate::treasure::{Bundle, Flavor, Item, Rand};
pub use generate::{Generated, PendingNpc};
pub use tables::{
    CATEGORY_ROLLS, FREQUENCY_VERSION_20, FREQUENCY_VERSION_EXPANSION, FrequencyTable,
    FrequencyTables, LairMode, Source, TableFilter, frequency_version, roll_2_to_20, roll_and_pick,
};
pub use text::{dice_text, hit_dice_text, hit_point_text, npc_line, stat_line};

use crate::treasure::TreasureRules;
use crate::{Error, Installation, Result};

/// `N d S * M + B`, as the template dialogs enter it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dice {
    pub count: i32,
    pub sides: i32,
    pub multiplier: i32,
    pub bonus: i32,
}

impl Dice {
    pub fn new(count: i32, sides: i32, multiplier: i32, bonus: i32) -> Self {
        Self {
            count,
            sides,
            multiplier,
            bonus,
        }
    }

    fn of(values: &[i32]) -> Self {
        Self::new(values[0], values[1], values[2], values[3])
    }
}

/// What a dependent line counts: the monsters of a type when it has a
/// name, else the monsters of one name (§6.3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reference {
    pub monster_type: String,
    pub monster: String,
}

impl Reference {
    /// The name the template printer writes.
    pub fn name(&self) -> &str {
        if self.monster_type.is_empty() {
            &self.monster
        } else {
            &self.monster_type
        }
    }
}

/// One monster line of a template (`CEncTemplateMonsterPropertiesOb`).
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterLine {
    /// Percent chance; 100 rolls nothing.
    pub percent: i32,
    /// Runs in the second pass and counts the first pass's monsters.
    pub dependent: bool,
    /// "and": the second box appears too.
    pub and: bool,
    /// "or": a d100 over 50 takes the second box instead.
    pub or: bool,
    /// "for every [n]"; 0 adds once.
    pub per: i32,
    /// "beyond [n]".
    pub beyond: i32,
    pub reference: Reference,
    pub dice: Dice,
    pub monster: String,
    /// "randomly selected from the following list"; used from two
    /// entries on. Repeated entries weigh the draw.
    pub list: Vec<Monster>,
    pub second_dice: Dice,
    pub second_monster: String,
}

/// An NPC description of a template (`CEncounterNPCOb`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NpcDescription {
    pub name: String,
    pub race: String,
    /// Class and level, for the slots in use.
    pub classes: Vec<(String, i32)>,
}

impl NpcDescription {
    fn of(npc: &format::EncounterNpc) -> Self {
        Self {
            name: npc.name.clone(),
            race: npc.race.clone(),
            classes: npc
                .classes
                .iter()
                .zip(npc.levels)
                .filter(|(c, _)| !c.is_empty())
                .map(|(c, l)| (c.clone(), l))
                .collect(),
        }
    }
}

/// One NPC line of a template (`CEncTemplateNPCPropertiesOb`).
#[derive(Debug, Clone, PartialEq)]
pub struct NpcLine {
    pub percent: i32,
    pub dependent: bool,
    pub and: bool,
    pub or: bool,
    pub per: i32,
    pub beyond: i32,
    pub reference: Reference,
    pub dice: Dice,
    pub npc: NpcDescription,
    pub list: Vec<NpcDescription>,
    pub second_dice: Dice,
    pub second_npc: NpcDescription,
}

/// Monsters that come with an item (`CEncTemplateNumAppearOb`).
#[derive(Debug, Clone, PartialEq)]
pub struct Escort {
    pub dice: Dice,
    pub monster: Monster,
}

/// The coin an item line holds (+0xdc0).
pub fn coin_name(coin: i32) -> &'static str {
    match coin {
        0 => "* Platinum Pieces",
        1 => "* Gold Pieces",
        3 => "* Copper Pieces",
        _ => "* Silver Pieces",
    }
}

/// One item line of a template (`CEncTemplateItemPropertiesOb`).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemLine {
    pub percent: i32,
    /// Shown as "Dependent" in the dialog; generation only looks at the
    /// reference.
    pub dependent: bool,
    pub per: i32,
    pub beyond: i32,
    /// No names: the item is not dependent.
    pub reference: Reference,
    /// The item's name (the part's).
    pub item: String,
    pub quantity: Dice,
    /// 0 platinum, 1 gold, 2 silver, 3 copper ([`coin_name`]).
    pub coin: i32,
    /// "containing" so many coins per item.
    pub coins: Dice,
    /// "These Monsters will appear if the previous Item(s) appear".
    pub escorts: Vec<Escort>,
    /// "For each Item".
    pub per_item: Vec<Escort>,
    /// "Double the number of these Monsters".
    pub doubled: Vec<String>,
    /// Groups of items (+0xe10), kept between generations as the original
    /// keeps it in the template.
    groups: Cell<i32>,
}

/// An encounter template (`CEncTemplateOb`).
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub name: String,
    /// From `UserDbas/EncPlatU.dat`.
    pub user: bool,
    pub monster_type: String,
    pub description: String,
    /// Climate/terrain category ("Any Land").
    pub climate_terrain: String,
    /// Frequency in percent; 0 is exempt from the 2-20 tables.
    pub frequency: i32,
    /// "Number Of Hit Dice" of the 2-20 tables.
    pub hit_dice: i32,
    /// Both lair and wandering, lair only, wandering only.
    pub placement: [bool; 3],
    pub climates: [i32; 150],
    /// Lair treasure, A to Z.
    pub lair_treasure: [i32; 26],
    pub monsters: Vec<MonsterLine>,
    pub npcs: Vec<NpcLine>,
    pub traps: Vec<Trap>,
    pub items: Vec<ItemLine>,
}

/// Monsters of one kind in an encounter (`CEncounterMonsterPackageOb`).
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterPackage {
    pub count: i32,
    pub monster: Monster,
    /// (hit points, how many), in the order the values first came up.
    /// "Double these monsters" leaves it as it was, so it may add up to
    /// less than `count`.
    pub hit_points: Vec<(i32, i32)>,
}

/// NPCs in an encounter (`CEncounterNPCPackageOb`): one character shared
/// by every individual.
#[derive(Debug, Clone, PartialEq)]
pub struct NpcPackage {
    pub count: i32,
    pub character: Character,
    /// One entry per individual, hit points in `part_type`.
    pub hit_points: Vec<PartId>,
}

impl NpcPackage {
    /// The Add NPC… button (0x63e814): a saved character, once, with no
    /// hit points rolled.
    pub fn of_character(character: Character) -> Self {
        Self {
            count: 1,
            character,
            hit_points: Vec::new(),
        }
    }

    /// The hit points of each individual.
    pub fn individual_hit_points(&self) -> Vec<i32> {
        self.hit_points.iter().map(|p| p.part_type).collect()
    }
}

/// A package of an encounter, as the dialog's list holds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Package {
    Monster(usize),
    Npc(usize),
}

/// An encounter (`CEncounterOb`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Encounter {
    pub name: String,
    /// Always 0.
    pub unknown_0x7c: i32,
    pub monster_type: String,
    /// The template it was made from.
    pub template: String,
    pub description: String,
    pub monsters: Vec<MonsterPackage>,
    pub npcs: Vec<NpcPackage>,
    pub traps: Vec<Trap>,
    pub treasure: Bundle,
    pub items: Bundle,
    /// The template's lair treasure, A to Z.
    pub lair_treasure: [i32; 26],
}

/// Encounter Totals of the dialog (0x647983).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Totals {
    /// Treasure, items and what the monsters and NPCs carry, in gold
    /// pieces (each bundle's copper value floored separately).
    pub gold_value: i32,
    /// The monsters' experience times their number. NPCs count for
    /// nothing here (their experience value, 0x43b133, is not ported).
    pub monster_xp: i32,
    /// Experience of the magic items in the same bundles.
    pub treasure_xp: i32,
}

/// Why an encounter could not be saved, with the original's messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("A name must be supplied.")]
    NoName,
    #[error("Name already used.")]
    NameUsed,
}

fn wrong_object(what: &str, object: &Object) -> Error {
    Error::Invalid(format!("{what}: unexpected {}", object.class().0))
}

impl MonsterLine {
    fn of(p: &format::TemplateMonsterProperties) -> Result<Self> {
        Ok(Self {
            percent: p.percent,
            dependent: p.flags[0] != 0,
            and: p.flags[1] != 0,
            or: p.flags[2] != 0,
            per: p.per,
            beyond: p.beyond,
            reference: Reference {
                monster_type: p.monster_type.name.clone(),
                monster: p.reference_name.clone(),
            },
            dice: Dice::of(&p.dice[..4]),
            monster: p.monster_name.clone(),
            list: monsters_of(&p.monsters, "template monster list")?,
            second_dice: Dice::of(&p.dice[4..]),
            second_monster: p.second_name.clone(),
        })
    }
}

fn monsters_of(objects: &[Object], what: &str) -> Result<Vec<Monster>> {
    objects
        .iter()
        .map(|o| match o {
            Object::Monster(m) => Ok((**m).clone()),
            other => Err(wrong_object(what, other)),
        })
        .collect()
}

fn escorts_of(objects: &[Object]) -> Result<Vec<Escort>> {
    objects
        .iter()
        .map(|o| match o {
            Object::TemplateNumAppear(n) => Ok(Escort {
                dice: Dice::of(&n.dice),
                monster: (*n.monster).clone(),
            }),
            other => Err(wrong_object("item monsters", other)),
        })
        .collect()
}

impl Template {
    fn of(t: &format::EncounterTemplate, user: bool) -> Result<Self> {
        let mut monsters = Vec::new();
        for o in &t.monsters {
            match o {
                Object::TemplateMonsterProperties(p) => monsters.push(MonsterLine::of(p)?),
                other => return Err(wrong_object(&t.name, other)),
            }
        }
        let mut npcs = Vec::new();
        for o in &t.npcs {
            let Object::TemplateNpcProperties(p) = o else {
                return Err(wrong_object(&t.name, o));
            };
            let list = p
                .npcs
                .iter()
                .map(|o| match o {
                    Object::EncounterNpc(n) => Ok(NpcDescription::of(n)),
                    other => Err(wrong_object(&t.name, other)),
                })
                .collect::<Result<_>>()?;
            npcs.push(NpcLine {
                percent: p.percent,
                dependent: p.flags[0] != 0,
                and: p.flags[1] != 0,
                or: p.flags[2] != 0,
                per: p.per,
                beyond: p.beyond,
                reference: Reference {
                    monster_type: p.monster_type.name.clone(),
                    monster: p.reference_name.clone(),
                },
                dice: Dice::of(&p.dice[..4]),
                npc: NpcDescription::of(&p.npc),
                list,
                second_dice: Dice::of(&p.dice[4..]),
                second_npc: NpcDescription::of(&p.second_npc),
            });
        }
        let mut items = Vec::new();
        for o in &t.items {
            let Object::TemplateItemProperties(p) = o else {
                return Err(wrong_object(&t.name, o));
            };
            items.push(ItemLine {
                percent: p.percent,
                dependent: p.dependency[0] != 0,
                per: p.dependency[1],
                beyond: p.dependency[2],
                reference: Reference {
                    monster_type: p.monster_type.name.clone(),
                    monster: p.monster.name.clone(),
                },
                item: p.part.head.name.clone(),
                quantity: Dice::of(&p.dice[..4]),
                coin: p.dice[4],
                coins: Dice::of(&p.dice[5..]),
                escorts: escorts_of(&p.escorts)?,
                per_item: escorts_of(&p.per_item)?,
                doubled: p.monster_names.clone(),
                groups: Cell::new(0),
            });
        }
        let traps = t
            .traps
            .iter()
            .map(|o| match o {
                Object::Trap(trap) => Ok((**trap).clone()),
                other => Err(wrong_object(&t.name, other)),
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            name: t.name.clone(),
            user,
            monster_type: t.monster_type.clone(),
            description: t.notes.clone(),
            climate_terrain: t.climate_terrain.clone(),
            frequency: t.frequency,
            hit_dice: t.hit_dice,
            placement: t.placement.map(|f| f != 0),
            climates: t.climates,
            lair_treasure: t.lair_treasure,
            monsters,
            npcs,
            traps,
            items,
        })
    }
}

/// The monster databases: `Database/Monster.dat`, then
/// `UserDbas/MonsterU.dat`.
#[derive(Debug, Clone, Default)]
struct Monsters {
    list: Vec<Monster>,
    /// First monster of each name in the standard and the user database.
    by_name: [HashMap<String, usize>; 2],
}

impl Monsters {
    fn push(&mut self, monster: Monster, user: bool) {
        self.by_name[usize::from(user)]
            .entry(monster.name.clone())
            .or_insert(self.list.len());
        self.list.push(monster);
    }

    fn in_db(&self, name: &str, user: bool) -> Option<&Monster> {
        self.by_name[usize::from(user)]
            .get(name)
            .map(|&i| &self.list[i])
    }

    /// The template lines' lookup (0x46e395) and the Add dialog's
    /// (0x64f007): the standard database first.
    fn standard_first(&self, name: &str) -> Option<&Monster> {
        self.in_db(name, false).or_else(|| self.in_db(name, true))
    }

    /// The validation of a template (0x644d29): the user database first.
    fn user_first(&self, name: &str) -> Option<&Monster> {
        self.in_db(name, true).or_else(|| self.in_db(name, false))
    }

    /// Re-copies a template's monster from the database by name; a name
    /// not found keeps the copy stored in the template.
    fn refresh(&self, monster: &mut Monster) {
        if monster.name.is_empty() {
            return;
        }
        if let Some(m) = self.user_first(&monster.name) {
            *monster = m.clone();
        }
    }
}

/// What the encounter screens read: templates, monsters and the treasure
/// engine, loaded once per installation.
#[derive(Debug, Clone)]
pub struct EncounterRules {
    flavor: Flavor,
    templates: Vec<Template>,
    /// First template of each name, standard ones first (0x641c98).
    by_name: HashMap<String, usize>,
    monsters: Monsters,
    treasure: TreasureRules,
}

const TEMPLATES: [&str; 2] = ["Database/EncPlat.dat", "UserDbas/EncPlatU.dat"];
const MONSTERS: [&str; 2] = ["Database/Monster.dat", "UserDbas/MonsterU.dat"];
const SAVED: &str = "UserDbas/EncountU.dat";
const STANDARD: &str = "Database/Encount.dat";

impl Installation {
    fn object_list(&self, relative: &str) -> Result<Option<ObjectList>> {
        if self.path(relative).is_none() {
            return Ok(None);
        }
        ObjectList::parse(&self.read(relative)?)
            .map(Some)
            .map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })
    }

    /// Loads the encounter templates, the monsters and the treasure tables.
    /// Every template's monster copies are refreshed from the monster
    /// databases as the original does before each generation (0x6449dc).
    pub fn encounter_rules(&self) -> Result<EncounterRules> {
        let treasure = self.treasure_rules()?;
        let mut monsters = Monsters::default();
        for (user, relative) in MONSTERS.into_iter().enumerate() {
            for o in self
                .object_list(relative)?
                .map_or(Vec::new(), |l| l.objects)
            {
                match o {
                    Object::Monster(m) => monsters.push(*m, user == 1),
                    other => return Err(wrong_object(relative, &other)),
                }
            }
        }
        let mut templates = Vec::new();
        for (user, relative) in TEMPLATES.into_iter().enumerate() {
            for o in self
                .object_list(relative)?
                .map_or(Vec::new(), |l| l.objects)
            {
                match &o {
                    Object::EncounterTemplate(t) => templates.push(Template::of(t, user == 1)?),
                    other => return Err(wrong_object(relative, other)),
                }
            }
        }
        for t in &mut templates {
            for line in &mut t.monsters {
                line.list.iter_mut().for_each(|m| monsters.refresh(m));
            }
            for item in &mut t.items {
                for e in item.escorts.iter_mut().chain(&mut item.per_item) {
                    monsters.refresh(&mut e.monster);
                }
            }
        }
        let mut by_name = HashMap::new();
        for (i, t) in templates.iter().enumerate() {
            by_name.entry(t.name.clone()).or_insert(i);
        }
        Ok(EncounterRules {
            flavor: treasure.flavor(),
            templates,
            by_name,
            monsters,
            treasure,
        })
    }

    /// The user's encounters (`UserDbas/EncountU.dat`), in file order.
    pub fn saved_encounters(&self) -> Result<Vec<Encounter>> {
        self.encounters_in(SAVED)
    }

    /// The standard encounters (`Database/Encount.dat`, shipped empty).
    pub fn standard_encounters(&self) -> Result<Vec<Encounter>> {
        self.encounters_in(STANDARD)
    }

    fn encounters_in(&self, relative: &str) -> Result<Vec<Encounter>> {
        if self.path(relative).is_none() {
            return Ok(Vec::new());
        }
        parse_encounters(&self.read(relative)?)
            .map_err(|e| Error::Invalid(format!("{relative}: {e}")))
    }

    /// Replaces `UserDbas/EncountU.dat` with `list`, as the original does
    /// when the Encounters screen closes; the old file is kept as
    /// `EncountU.dat.bak`. Core Rules 2.0 cannot read Expansion NPCs, so
    /// for it any NPC character that is not `TSRP2 V29` is refused.
    pub fn write_saved_encounters(&self, list: &[Encounter]) -> Result<()> {
        let flavor = flavor_of(self);
        check_flavor(list, flavor)?;
        let bytes = encounters_to_bytes(list)?;
        replace_or_create(self, SAVED, "EncountU.dat", &bytes)
    }
}

/// The program an installation belongs to, as the treasure engine decides
/// it: the Expansion has `UserDbas/AllowedU.dat`.
pub fn flavor_of(install: &Installation) -> Flavor {
    if install.path("UserDbas/AllowedU.dat").is_some() {
        Flavor::Expansion
    } else {
        Flavor::Core20
    }
}

fn check_flavor(list: &[Encounter], flavor: Flavor) -> Result<()> {
    if flavor == Flavor::Expansion {
        return Ok(());
    }
    for e in list {
        for npc in &e.npcs {
            if npc.character.version != CORE20_VERSION {
                return Err(Error::Invalid(format!(
                    "{}: the NPC {} is a {} character, which Core Rules 2.0 cannot read",
                    e.name, npc.character.data.name, npc.character.version
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn replace_or_create(
    install: &Installation,
    relative: &str,
    file: &str,
    bytes: &[u8],
) -> Result<()> {
    if let Some(path) = install.path(relative) {
        return crate::edit::replace(&path, bytes);
    }
    let folder = install
        .path("UserDbas")
        .ok_or_else(|| Error::Invalid("the installation has no UserDbas folder".into()))?;
    crate::create::write_new(&folder.join(file), bytes)
}

/// Reads an `EncountU.dat` (a `CObList` of `CEncounterOb`).
pub fn parse_encounters(data: &[u8]) -> std::result::Result<Vec<Encounter>, String> {
    let list = ObjectList::parse(data).map_err(|e| e.to_string())?;
    list.objects
        .iter()
        .map(|o| match o {
            Object::Encounter(e) => Encounter::from_record(e).map_err(|e| e.to_string()),
            other => Err(format!("not an encounter: {}", other.class().0)),
        })
        .collect()
}

/// The bytes of an `EncountU.dat` holding `list`, as the original writes
/// it.
pub fn encounters_to_bytes(list: &[Encounter]) -> Result<Vec<u8>> {
    ObjectList {
        objects: list
            .iter()
            .map(|e| Object::Encounter(Box::new(e.to_record())))
            .collect(),
    }
    .to_bytes()
    .map_err(|source| Error::Format {
        path: SAVED.into(),
        source,
    })
}

/// Checks a name as the dialog's OK does (0x647542 on both databases):
/// trimmed and without regard to case, it must be new among `list`
/// (`except` is the encounter being edited).
pub fn check_name(
    list: &[Encounter],
    name: &str,
    except: Option<usize>,
) -> std::result::Result<(), NameError> {
    let key = |n: &str| n.trim().to_ascii_uppercase();
    let wanted = key(name);
    if wanted.is_empty() {
        return Err(NameError::NoName);
    }
    let used = list
        .iter()
        .enumerate()
        .any(|(i, e)| Some(i) != except && key(&e.name) == wanted);
    if used {
        Err(NameError::NameUsed)
    } else {
        Ok(())
    }
}

/// OK in the Encounter dialog (0x46ef0e): checks the name, then replaces
/// the encounter at `except` or adds it at the end.
pub fn store_encounter(
    list: &mut Vec<Encounter>,
    encounter: Encounter,
    except: Option<usize>,
) -> std::result::Result<(), NameError> {
    check_name(list, &encounter.name, except)?;
    match except {
        Some(i) if i < list.len() => list[i] = encounter,
        _ => list.push(encounter),
    }
    Ok(())
}

impl Encounter {
    pub fn from_record(r: &format::Encounter) -> Result<Self> {
        let monsters = r
            .monsters
            .iter()
            .map(|o| match o {
                Object::MonsterPackage(p) => Ok(MonsterPackage {
                    count: p.count,
                    monster: (*p.monster).clone(),
                    hit_points: p
                        .hit_points
                        .iter()
                        .map(|o| match o {
                            Object::MonsterHp(h) => Ok((h.hit_points, h.count)),
                            other => Err(wrong_object(&r.name, other)),
                        })
                        .collect::<Result<_>>()?,
                }),
                other => Err(wrong_object(&r.name, other)),
            })
            .collect::<Result<_>>()?;
        let npcs = r
            .npcs
            .iter()
            .map(|o| match o {
                Object::NpcPackage(p) => Ok(NpcPackage {
                    count: p.count,
                    character: (*p.character).clone(),
                    hit_points: p
                        .hit_points
                        .iter()
                        .map(|o| match o {
                            Object::PartId(id) => Ok(id.clone()),
                            other => Err(wrong_object(&r.name, other)),
                        })
                        .collect::<Result<_>>()?,
                }),
                other => Err(wrong_object(&r.name, other)),
            })
            .collect::<Result<_>>()?;
        let traps = r
            .traps
            .iter()
            .map(|o| match o {
                Object::Trap(t) => Ok((**t).clone()),
                other => Err(wrong_object(&r.name, other)),
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            name: r.name.clone(),
            unknown_0x7c: r.unknown_0x7c,
            monster_type: r.monster_type.clone(),
            template: r.template.clone(),
            description: r.description.clone(),
            monsters,
            npcs,
            traps,
            treasure: Bundle::from_inventory(&r.treasure)?,
            items: Bundle::from_inventory(&r.items)?,
            lair_treasure: r.lair_treasure,
        })
    }

    pub fn to_record(&self) -> format::Encounter {
        format::Encounter {
            name: self.name.clone(),
            unknown_0x7c: self.unknown_0x7c,
            monster_type: self.monster_type.clone(),
            template: self.template.clone(),
            description: self.description.clone(),
            monsters: self
                .monsters
                .iter()
                .map(|p| {
                    Object::MonsterPackage(Box::new(format::MonsterPackage {
                        count: p.count,
                        monster: Box::new(p.monster.clone()),
                        hit_points: p
                            .hit_points
                            .iter()
                            .map(|&(hit_points, count)| {
                                Object::MonsterHp(format::MonsterHp { hit_points, count })
                            })
                            .collect(),
                    }))
                })
                .collect(),
            npcs: self
                .npcs
                .iter()
                .map(|p| {
                    Object::NpcPackage(Box::new(format::NpcPackage {
                        count: p.count,
                        character: Box::new(p.character.clone()),
                        hit_points: p.hit_points.iter().cloned().map(Object::PartId).collect(),
                    }))
                })
                .collect(),
            traps: self
                .traps
                .iter()
                .map(|t| Object::Trap(Box::new(t.clone())))
                .collect(),
            treasure: self.treasure.to_inventory(),
            items: self.items.to_inventory(),
            lair_treasure: self.lair_treasure,
        }
    }

    /// The packages in the dialog's order: monsters, then NPCs.
    pub fn packages(&self) -> Vec<Package> {
        (0..self.monsters.len())
            .map(Package::Monster)
            .chain((0..self.npcs.len()).map(Package::Npc))
            .collect()
    }

    /// A package as the dialog's list writes it (0x63d876): "9 - Orc".
    pub fn package_line(&self, package: Package) -> Option<String> {
        Some(match package {
            Package::Monster(i) => {
                let p = self.monsters.get(i)?;
                format!("{} - {}", p.count, p.monster.name)
            }
            Package::Npc(i) => {
                let p = self.npcs.get(i)?;
                format!("{} - {}", p.count, p.character.data.name)
            }
        })
    }

    /// Edit… (0x63dba3): a new number for a package, 1 to 100,000,000. A
    /// monster package rolls all its hit points again; an NPC package only
    /// takes the number. The same number, or 0, changes nothing.
    pub fn set_count(
        &mut self,
        package: Package,
        count: i32,
        rules: &EncounterRules,
        rand: &mut Rand,
    ) {
        let count = count.clamp(0, 100_000_000);
        match package {
            Package::Monster(i) => {
                if let Some(p) = self.monsters.get_mut(i)
                    && count != 0
                    && count != p.count
                {
                    p.count = count;
                    rules.roll_hit_points(p, rand);
                }
            }
            Package::Npc(i) => {
                if let Some(p) = self.npcs.get_mut(i)
                    && count != 0
                {
                    p.count = count;
                }
            }
        }
    }

    /// Delete (0x63e29a).
    pub fn remove(&mut self, package: Package) {
        match package {
            Package::Monster(i) if i < self.monsters.len() => {
                self.monsters.remove(i);
            }
            Package::Npc(i) if i < self.npcs.len() => {
                self.npcs.remove(i);
            }
            _ => {}
        }
    }

    /// Add NPC… (0x63e814): `character` once. Core Rules 2.0 can only hold
    /// its own characters (`TSRP2 V29`).
    pub fn add_npc(&mut self, character: Character, flavor: Flavor) -> Result<()> {
        if flavor == Flavor::Core20 && character.version != CORE20_VERSION {
            return Err(Error::Invalid(format!(
                "{} is a {} character; Core Rules 2.0 reads only {CORE20_VERSION}",
                character.data.name, character.version
            )));
        }
        self.npcs.push(NpcPackage::of_character(character));
        Ok(())
    }

    /// The treasure types the Treasure… button opens the roller with
    /// (0x63de14): the lair treasure plus each monster's letters times its
    /// number.
    pub fn treasure_counts(&self) -> [i32; 26] {
        let mut counts = self.lair_treasure;
        for p in &self.monsters {
            for (c, (_, letters)) in counts.iter_mut().zip(p.monster.unknown_0x25c) {
                *c = c.wrapping_add(letters.wrapping_mul(p.count));
            }
        }
        counts
    }
}

impl EncounterRules {
    /// The program whose rules apply.
    pub fn flavor(&self) -> Flavor {
        self.flavor
    }

    /// Follows another program's rules (for comparing the two).
    pub fn set_flavor(&mut self, flavor: Flavor) {
        self.flavor = flavor;
        self.treasure.set_flavor(flavor);
    }

    /// The treasure engine the encounters roll with.
    pub fn treasure_rules(&self) -> &TreasureRules {
        &self.treasure
    }

    /// Every template, standard ones first, in file order.
    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    /// The template of that name, the standard one first (0x641c98).
    pub fn template(&self, name: &str) -> Option<&Template> {
        self.by_name.get(name).map(|&i| &self.templates[i])
    }

    /// The templates the template-select dialog lists for a monster type
    /// ("~All" lists every one).
    pub fn templates_of_type<'a>(
        &'a self,
        monster_type: &'a str,
    ) -> impl Iterator<Item = &'a Template> {
        self.templates
            .iter()
            .filter(move |t| monster_type == "~All" || t.monster_type == monster_type)
    }

    /// Every monster, `Monster.dat` then `MonsterU.dat`.
    pub fn monsters(&self) -> &[Monster] {
        &self.monsters.list
    }

    /// A monster by name as the Add dialog finds it: the standard database
    /// first.
    pub fn monster(&self, name: &str) -> Option<&Monster> {
        self.monsters.standard_first(name)
    }

    /// `rand_between(lo, hi)` (0x554d44): `rand() % (hi - lo + 1) + lo`.
    /// With an empty range the Expansion returns 0 without drawing; 2.0
    /// inlines it without the check (a range of 0 would divide by zero
    /// there and gives 0 here too).
    pub fn between(&self, rand: &mut Rand, lo: i32, hi: i32) -> i32 {
        let n = hi.wrapping_add(1).wrapping_sub(lo);
        if n == 0 || (n < 0 && self.flavor == Flavor::Expansion) {
            return 0;
        }
        (rand.rand() % n).wrapping_add(lo)
    }

    /// `(Σ count dice of sides) × multiplier + bonus` (0x6521a9).
    pub fn roll(&self, rand: &mut Rand, dice: Dice) -> i32 {
        let mut sum = 0i32;
        for _ in 0..dice.count {
            sum = sum.wrapping_add(self.between(rand, 1, dice.sides));
        }
        sum.wrapping_mul(dice.multiplier).wrapping_add(dice.bonus)
    }

    /// A number in the monster's own No. Appearing range.
    fn number_appearing(&self, rand: &mut Rand, monster: &Monster) -> i32 {
        self.between(
            rand,
            monster.number_appearing_min,
            monster.number_appearing_max,
        )
    }

    /// Rolls the hit points of every monster of a package (0x6406f4):
    /// `Σ hit dice + bonus` each, equal values merged.
    pub fn roll_hit_points(&self, package: &mut MonsterPackage, rand: &mut Rand) {
        let m = &package.monster;
        let (dice, bonus, sides) = (m.hit_dice, m.unknown_0x438, m.hit_die_size);
        package.hit_points.clear();
        for _ in 0..package.count {
            let mut hp = 0i32;
            for _ in 0..dice {
                hp = hp.wrapping_add(self.between(rand, 1, sides));
            }
            hp = hp.wrapping_add(bonus);
            match package.hit_points.iter_mut().find(|(h, _)| *h == hp) {
                Some(entry) => entry.1 = entry.1.wrapping_add(1),
                None => package.hit_points.push((hp, 1)),
            }
        }
    }

    /// A package as the Add… dialog makes it (0x64f007): the monster from
    /// the database, `count` kept only within its No. Appearing range
    /// (else its low end), hit points rolled.
    pub fn new_monster_package(
        &self,
        name: &str,
        count: Option<i32>,
        rand: &mut Rand,
    ) -> Option<MonsterPackage> {
        let monster = self.monster(name)?.clone();
        let (min, max) = (monster.number_appearing_min, monster.number_appearing_max);
        let count = match count {
            Some(n) if (min..=max).contains(&n) => n,
            _ => min,
        };
        let mut package = MonsterPackage {
            count,
            monster,
            hit_points: Vec::new(),
        };
        self.roll_hit_points(&mut package, rand);
        Some(package)
    }

    /// Encounter Totals (0x647983).
    pub fn totals(&self, encounter: &Encounter) -> Totals {
        let mut t = Totals::default();
        let mut add = |bundle: &Bundle, times: i32| {
            let s = self.treasure.summary(bundle);
            t.gold_value = t
                .gold_value
                .wrapping_add((s.value_cp / 100).wrapping_mul(times));
            t.treasure_xp = t.treasure_xp.wrapping_add(s.xp.wrapping_mul(times));
        };
        add(&encounter.treasure, 1);
        add(&encounter.items, 1);
        for p in &encounter.monsters {
            if let Ok(b) = Bundle::from_inventory(&p.monster.inventory) {
                add(&b, p.count);
            }
        }
        for p in &encounter.npcs {
            add(&npc_inventory(&p.character), p.count);
        }
        for p in &encounter.monsters {
            t.monster_xp = t
                .monster_xp
                .wrapping_add(p.monster.experience.wrapping_mul(p.count));
        }
        t
    }
}

/// Everything an NPC carries, its nine equipment groups in one bundle.
fn npc_inventory(character: &Character) -> Bundle {
    let mut all = Bundle::default();
    for b in &character.data.inventory {
        if let Ok(b) = Bundle::from_inventory(b) {
            for item in b.items {
                all.insert(item, true);
            }
        }
    }
    all
}
