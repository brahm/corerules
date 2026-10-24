//! Building the sheet: one function per section, after the originals'
//! section emitters (addresses in each function's comment).

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;
use cr_formats::inventory::InventoryItem;
use cr_formats::parts::Part;

use super::tables::{self, at, ordinal, thousands};
use super::{
    Block, Cell, Flavor, ListItem, Section, SectionContent, Sheet, SheetBox, SheetRules, Table, ids,
};
use crate::Result;
use crate::lists::BUNDLE_NAMES;

pub(super) mod combat;
pub(super) mod group;
mod nwp;
mod rogue;
mod spells;

/// `inventory` indexes.
const CARRIED: usize = 2;
const READIED: usize = 3;
const WORN: usize = 5;

/// The abilities: Str, Dex, Con, Int, Wis, Cha.
const STR: usize = 0;
const DEX: usize = 1;
const CON: usize = 2;
const INT: usize = 3;
const WIS: usize = 4;
const CHA: usize = 5;

/// Subability indexes for [`Scores::sub`]: Strength's Stamina and
/// Muscle, Dexterity's Aim and Balance.
const STAMINA: usize = 1;
const MUSCLE: usize = 2;
const AIM: usize = 1;
const BALANCE: usize = 2;

/// The 2.0 sections, in the order its emitter (0x432e30) writes them; each
/// writes nothing when it does not apply.
const CORE20_SECTIONS: [u32; 14] = [
    ids::PERSONAL,
    ids::ABILITIES,
    ids::SAVES,
    ids::PROFICIENCIES,
    ids::WEAPONS,
    ids::TURNING,
    ids::ROGUE_SKILLS,
    ids::RACIAL,
    // 2.0's inventory lists the spells in books.
    ids::INVENTORY_SPELLS,
    ids::SPELLS,
    ids::MEMORIZED,
    ids::SPELLS_KNOWN,
    ids::PSIONICS,
    ids::HISTORY,
];

/// What one class of the character is, as the sheet uses it.
#[derive(Debug, Clone)]
pub(super) struct ClassInfo<'a> {
    pub name: &'a str,
    pub class: Option<&'a Class>,
    /// 0 warrior, 1 rogue, 2 priest, 3 wizard, 4 psionicist.
    pub group: i32,
    pub level: u32,
}

/// The scores as the sheet shows them: each ability with its two
/// subabilities (equal to the score with the Core Rules), after magical
/// items.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Scores {
    pub abilities: [[i32; 3]; 6],
    pub exceptional: i32,
    /// Whether the character can have exceptional Strength (0x43ac82).
    pub can_be_exceptional: bool,
}

impl Scores {
    pub fn score(&self, a: usize) -> i32 {
        self.abilities[a][0]
    }

    pub fn sub(&self, a: usize, s: usize) -> i32 {
        self.abilities[a][s]
    }

    /// The exceptional Strength to look tables up with, when it applies.
    pub fn exceptional_for(&self, strength: i32) -> Option<i32> {
        (strength == 18 && self.can_be_exceptional).then_some(self.exceptional)
    }

    /// Strength (or a Strength subability) as printed: `18/76`, `18/00`.
    pub fn strength_text(&self, strength: i32) -> String {
        match self.exceptional_for(strength) {
            Some(e) if e > 0 => {
                if e >= 100 {
                    "18/00".into()
                } else {
                    format!("18/{e}")
                }
            }
            _ => strength.to_string(),
        }
    }
}

/// Everything a section needs: the rules, the character and what is
/// worked out from it once.
pub(super) struct Ctx<'a> {
    pub rules: &'a SheetRules,
    pub ch: &'a Character,
    pub flavor: Flavor,
    pub sp: bool,
    pub classes: Vec<ClassInfo<'a>>,
    pub scores: Scores,
    /// Parts of the top-level items readied and worn.
    pub readied: Vec<&'a Part>,
    pub worn: Vec<&'a Part>,
    /// The kit's data, when the kit part has it.
    pub kit_data: Option<&'a cr_formats::parts::PartKitData>,
    pub kit: Option<crate::npc::data::Kit>,
}

impl<'a> Ctx<'a> {
    fn new(rules: &'a SheetRules, ch: &'a Character) -> Result<Self> {
        let levels = rules.advance.levels(ch)?;
        let classes = ch
            .data
            .classes
            .names
            .iter()
            .zip(&levels)
            .map(|(name, level)| {
                let class = rules.classes.get(name);
                ClassInfo {
                    name,
                    class,
                    group: class.map_or(-1, class_group),
                    level: level.level,
                }
            })
            .collect();
        let top = |b: usize| -> Vec<&'a Part> {
            ch.data.inventory[b]
                .items
                .iter()
                .filter_map(|i| rules.part(&i.name))
                .collect()
        };
        let mut ctx = Self {
            rules,
            ch,
            flavor: rules.flavor,
            sp: ch.skills_and_powers(),
            classes,
            scores: Scores::default(),
            readied: top(READIED),
            worn: top(WORN),
            kit_data: None,
            kit: None,
        };
        ctx.kit_data = ctx
            .part(&ch.data.kit)
            .and_then(|p| p.data.kit.as_ref())
            .and_then(|k| k.as_deref())
            .and_then(|o| match o {
                Object::PartKit(k) => k.data.as_ref(),
                _ => None,
            });
        ctx.kit = ctx.kit_data.map(crate::npc::data::Kit::of);
        ctx.scores = ctx.compute_scores();
        Ok(ctx)
    }

    /// A value cell: right-aligned in the Expansion, left in 2.0.
    fn value(&self, text: impl Into<String>) -> Cell {
        let cell = Cell::new(text);
        if self.expansion() { cell.right() } else { cell }
    }

    fn expansion(&self) -> bool {
        self.flavor == Flavor::Expansion
    }

    pub fn part(&self, name: &str) -> Option<&'a Part> {
        self.rules.part(name)
    }

    /// The readied and worn top-level item parts, readied first.
    pub fn equipped(&self) -> impl Iterator<Item = &'a Part> + '_ {
        self.readied.iter().chain(&self.worn).copied()
    }

    pub fn has_group(&self, group: i32) -> bool {
        self.classes.iter().any(|c| c.group == group)
    }

    pub fn race(&self) -> Option<&'a cr_formats::race::Race> {
        self.rules.races.get(&self.ch.data.race)
    }

    /// Whether the character is dual-classed (a race that may, more than
    /// one class).
    pub fn dual(&self) -> bool {
        self.rules.advance.dual_classed(self.ch)
    }

    /// The class ability parts of class `g`.
    pub fn class_abilities(&self, g: usize) -> Vec<&'a Part> {
        self.ch.data.class_slots[g]
            .abilities
            .iter()
            .filter_map(|a| self.part(a))
            .collect()
    }

    /// The classes whose abilities count: all of them, or only the new
    /// class of a dual-classed character that has not passed the old one.
    pub fn counted_classes(&self) -> Vec<usize> {
        let n = self.classes.len();
        if self.dual() && n > 1 && self.classes[n - 1].level <= self.classes[n - 2].level {
            vec![n - 1]
        } else {
            (0..n).collect()
        }
    }

    /// The character's racial ability parts.
    pub fn racial_abilities(&self) -> Vec<&'a Part> {
        self.ch
            .data
            .racial_abilities
            .iter()
            .filter_map(|a| self.part(a))
            .collect()
    }

    /// Displayed scores (getters at vtable 0xb4 to 0x114): the stored
    /// score raised to the floor magical items set, plus what additive
    /// items add, kept within 1 to 25 (exceptional Strength 0 to 100).
    fn compute_scores(&self) -> Scores {
        let d = &self.ch.data;
        let stored: [[i32; 3]; 6] = std::array::from_fn(|a| {
            let s = if a == STR {
                d.strength
            } else {
                d.abilities[a - 1]
            };
            s.map(|v| v as i32)
        });
        let mut add = [[0i32; 3]; 6];
        let mut floor = [[0i32; 3]; 6];
        let (mut add_exc, mut floor_exc) = (0, 0);
        if self.expansion() {
            for part in self.equipped() {
                let h = &part.head.unknown_0x30c;
                let main = [
                    part.unknown_0x728,
                    part.data.unknown_0x72c[0],
                    part.data.unknown_0x72c[1],
                    part.data.unknown_0x72c[2],
                    part.data.unknown_0x72c[3],
                    part.data.unknown_0x72c[4],
                ];
                // Subability fields: Stamina, Muscle, Aim, Balance, Health,
                // Fitness, Reason, Knowledge, Intuition, Willpower,
                // Leadership, Appearance.
                let subs = [
                    [h[1], h[2]],
                    [h[3], h[4]],
                    [h[6], h[5]],
                    [h[7], h[8]],
                    [h[10], h[9]],
                    [h[11], h[12]],
                ];
                let values: [[i32; 3]; 6] =
                    std::array::from_fn(|a| [main[a], subs[a][0] + main[a], subs[a][1] + main[a]]);
                let exc = part.data.unknown_0x72c[5];
                if h[0] != 0 {
                    for a in 0..6 {
                        for s in 0..3 {
                            add[a][s] += values[a][s];
                        }
                    }
                    add_exc += exc;
                } else {
                    for a in 0..6 {
                        for s in 0..3 {
                            floor[a][s] = floor[a][s].max(values[a][s]);
                        }
                    }
                    floor_exc = floor_exc.max(exc);
                }
            }
        }
        let mut abilities = [[0; 3]; 6];
        for a in 0..6 {
            for s in 0..3 {
                let base = if self.sp || s == 0 {
                    stored[a][s]
                } else {
                    stored[a][0]
                };
                let (fl, ad) = if self.sp || s == 0 {
                    (floor[a][s], add[a][s])
                } else {
                    (floor[a][0], add[a][0])
                };
                abilities[a][s] = (base.max(fl) + ad).clamp(1, 25);
            }
        }
        let exceptional = ((d.exceptional_strength as i32).max(floor_exc) + add_exc).clamp(0, 100);
        let race_allows = self.race().is_some_and(|r| r.unknown_0x40 != 0);
        let can_be_exceptional = self.classes.iter().any(|c| {
            (c.group == 0 && race_allows)
                || matches!(c.class, Some(Class::Expansion { data, .. }) if data.flag_0x3554.unwrap_or(0) != 0)
        }) || (0..self.classes.len()).any(|g| {
            self.class_abilities(g)
                .iter()
                .any(|p| p.data.unknown_0x7b4 != 0)
        }) || self.equipped().any(|p| p.data.unknown_0x72c[5] > 0);
        Scores {
            abilities,
            exceptional,
            can_be_exceptional,
        }
    }
}

/// The group of a class: 0 warrior, 1 rogue, 2 priest, 3 wizard, 4
/// psionicist.
pub(super) fn class_group(class: &Class) -> i32 {
    match class {
        Class::Expansion { data, .. } => data.unknown_0x14[0],
        Class::Core20(data) => data.unknown_0x10[0],
    }
}

/// `%+d`.
fn signed(n: i32) -> String {
    format!("{n:+}")
}

fn single(id: u32, title: &str, body: Vec<Block>) -> Section {
    Section {
        id,
        content: SectionContent::Columns(vec![vec![SheetBox {
            title: title.into(),
            body,
        }]]),
    }
}

fn table(columns: usize, rows: Vec<Vec<Cell>>) -> Block {
    Block::Table(Table { columns, rows })
}

/// Each rogue skill entry's score as Update Character's Rogue Skills
/// dialog shows it (0x59aa97 without armor), by entry name.
pub(super) fn rogue_scores(rules: &SheetRules, ch: &Character) -> Result<Vec<(String, i32)>> {
    let ctx = Ctx::new(rules, ch)?;
    Ok(rogue::skills(&ctx)
        .into_iter()
        .map(|s| (s.part_name, s.total - s.armor))
        .collect())
}

pub(super) fn death_save(rules: &SheetRules, ch: &Character) -> Result<i32> {
    let ctx = Ctx::new(rules, ch)?;
    Ok(combat::death_save(&ctx))
}

/// The Expansion's default sections (0x598347), for a character that has
/// never been through the sheet options.
pub(super) fn default_sections(rules: &SheetRules, ch: &Character) -> Result<Vec<u32>> {
    let ctx = Ctx::new(rules, ch)?;
    let mut list = vec![
        ids::PERSONAL,
        ids::ABILITIES,
        ids::SAVES,
        ids::PROFICIENCIES,
        ids::WEAPONS,
    ];
    if !ch.data.racial_abilities.is_empty() {
        list.push(ids::RACIAL);
    }
    list.push(ids::CLASS_ABILITIES);
    if !ch.data.unknown_0x1830[0].is_empty() {
        list.push(ids::TRAITS);
    }
    if !ch.data.unknown_0x1830[1].is_empty() {
        list.push(ids::DISADVANTAGES);
    }
    if !rogue::skills(&ctx).is_empty() {
        list.push(ids::ROGUE_SKILLS);
    }
    if combat::turn_level(&ctx) != 0 {
        list.push(ids::TURNING);
    }
    list.extend([ids::PAGE_BREAK, ids::INVENTORY_SPELLS, ids::MOVEMENT]);
    let wizard = ctx.classes.iter().any(|c| spells::casts(&ctx, c).1);
    let priest = ctx.classes.iter().any(|c| spells::casts(&ctx, c).0);
    if wizard || priest {
        list.extend([ids::PAGE_BREAK, ids::SPELLS, ids::MEMORIZED]);
        if wizard {
            list.extend([ids::PAGE_BREAK, ids::SPELLS_KNOWN]);
        }
        if priest {
            list.extend([ids::PAGE_BREAK, ids::SPELLS_GRANTED]);
        }
    }
    if ctx.has_group(4) {
        list.extend([ids::PAGE_BREAK, ids::PSIONICS]);
    }
    if !ch.data.unknown_0x94.is_empty() {
        list.extend([ids::PAGE_BREAK, ids::HISTORY]);
    }
    Ok(list)
}

pub(super) fn build(rules: &SheetRules, ch: &Character) -> Result<Sheet> {
    let ctx = Ctx::new(rules, ch)?;
    let ids = match rules.flavor {
        Flavor::Expansion => rules.section_ids(ch)?,
        Flavor::Core20 => CORE20_SECTIONS.to_vec(),
    };
    let mut sections = Vec::new();
    for id in ids {
        if let Some(section) = section(&ctx, id) {
            sections.push(section);
        }
    }
    let border = match rules.flavor {
        Flavor::Expansion => Some(rules.sheet_choice(ch)?.border),
        Flavor::Core20 => None,
    };
    Ok(Sheet {
        title: ch.data.name.clone(),
        flavor: rules.flavor,
        sections,
        border,
    })
}

/// The hit dice a character counts as when the encounter screens value it
/// as a monster (Expansion 0x43b133, 2.0 0x41ff10), after the DMG's
/// creature experience adjustments: the highest class level, +1 for an
/// armor class of 0 or better (0x429ac5, 2.0 0x415250), +1 when the third
/// psionic list is not empty (member 0x199c, 2.0 0x18fc), +2 when the
/// second is not (0x1988, 2.0 0x18e8), per class +1 for slots at spell
/// level 1 or 2 and +1 for slots at 3 to 9 in the class table at its
/// level, and +1 for Intelligence above 12 (vtable 0xe8, 2.0 0xe4).
pub(super) fn npc_hit_dice(rules: &SheetRules, ch: &Character) -> Result<i32> {
    let ctx = Ctx::new(rules, ch)?;
    let level = |c: &ClassInfo| i32::try_from(c.level).unwrap_or(i32::MAX);
    let mut hd = ctx.classes.iter().map(level).max().unwrap_or(0).max(0);
    if combat::armor_class(&ctx) < 1 {
        hd += 1;
    }
    let psionic = &ch.data.unknown_0x196c;
    if !psionic[2].is_empty() {
        hd += 1;
    }
    if !psionic[1].is_empty() {
        hd += 2;
    }
    for g in 0..ctx.classes.len() {
        let slots = rules.advance.table_slots(ch, g)?;
        if slots[1] != 0 || slots[2] != 0 {
            hd += 1;
        }
        if slots[3..=9].iter().any(|&s| s != 0) {
            hd += 1;
        }
    }
    if ctx.scores.score(INT) > 12 {
        hd += 1;
    }
    Ok(hd)
}

fn section(ctx: &Ctx, id: u32) -> Option<Section> {
    let core20 = ctx.flavor == Flavor::Core20;
    Some(match id {
        ids::PERSONAL => personal(ctx),
        ids::ABILITIES => abilities(ctx),
        ids::SAVES => combat::saves(ctx),
        ids::PROFICIENCIES => proficiencies(ctx),
        ids::WEAPONS => {
            let section = combat::weapons(ctx, false);
            if core20 && combat::weapon_rows(ctx).is_empty() {
                return None;
            }
            section
        }
        ids::PRIMARY_OFFHAND => combat::weapons(ctx, true),
        ids::ROGUE_SKILLS => {
            if core20 && rogue::skills(ctx).is_empty() {
                return None;
            }
            rogue::section(ctx)
        }
        ids::TURNING => combat::turning(ctx)?,
        ids::RACIAL => {
            let text = racial_text(ctx);
            if core20 && text.is_empty() {
                return None;
            }
            single(id, "Racial Abilities", vec![Block::Paragraph(text)])
        }
        ids::CLASS_ABILITIES => single(
            id,
            "Class Abilities",
            vec![Block::Paragraph(class_text(ctx))],
        ),
        ids::TRAITS => single(
            id,
            "Traits",
            vec![Block::Paragraph(part_list_text(
                ctx,
                &ctx.ch.data.unknown_0x1830[0],
            ))],
        ),
        ids::DISADVANTAGES => single(
            id,
            "Disadvantages",
            vec![Block::Paragraph(part_list_text(
                ctx,
                &ctx.ch.data.unknown_0x1830[1],
            ))],
        ),
        ids::INVENTORY => inventory(ctx, id, false),
        ids::INVENTORY_SPELLS => inventory(ctx, id, true),
        ids::MOVEMENT => combat::movement(ctx),
        ids::PAGE_BREAK => Section {
            id,
            content: SectionContent::PageBreak,
        },
        ids::SPELLS => {
            if core20 && !(0..ctx.classes.len()).any(|g| spells::slots(ctx, g).is_some()) {
                return None;
            }
            spells::summary(ctx)
        }
        ids::MEMORIZED => {
            let section = spells::memorized(ctx);
            if core20 && spells::memorized_count(ctx) == 0 {
                return None;
            }
            section
        }
        ids::SPELLS_KNOWN => spells::known(ctx)?,
        ids::SPELLS_GRANTED => spells::granted(ctx)?,
        ids::PSIONICS => spells::psionics(ctx),
        ids::KIT => kit(ctx),
        ids::HISTORY => {
            if core20 && ctx.ch.data.unknown_0x94.is_empty() {
                return None;
            }
            single(
                id,
                "Character History",
                vec![Block::Paragraph(ctx.ch.data.unknown_0x94.clone())],
            )
        }
        ids::PORTRAIT => portrait(ctx),
        _ => return None,
    })
}

/// Personal Information (0x5a25f1).
fn personal(ctx: &Ctx) -> Section {
    let d = &ctx.ch.data;
    let multi = ctx.classes.len() > 1;
    let race = if d.subrace.is_empty() {
        &d.race
    } else {
        &d.subrace
    };
    let gender = if d.unknown_0x163c != 0 {
        "Male"
    } else {
        "Female"
    };
    let height = format!("Height: {}'{}\"", d.height / 12, d.height % 12);
    let class = format!(
        "{}{}",
        if multi { "Classes: " } else { "Class: " },
        d.class_title
    );
    let levels = ctx
        .classes
        .iter()
        .map(|c| c.level.to_string())
        .collect::<Vec<_>>()
        .join("/");
    let level = format!("{}{levels}", if multi { "Levels: " } else { "Level: " });
    let xp = format!(
        "XP: {}",
        (0..ctx.classes.len().max(1))
            .map(|g| thousands(i64::from(d.unknown_0x1618[g])))
            .collect::<Vec<_>>()
            .join("/")
    );
    let next = ctx
        .rules
        .advance
        .levels(ctx.ch)
        .unwrap_or_default()
        .iter()
        .map(|l| match l.next {
            Some(n) => thousands(i64::from(n)),
            None => "NA".into(),
        })
        .collect::<Vec<_>>()
        .join("/");
    let next = format!("Next Level: {next}");
    let kit = format!("Kit: {}", kit_name(ctx).unwrap_or("None"));
    let rows = match ctx.flavor {
        Flavor::Expansion => vec![
            vec![
                Cell::new(format!("Name: {}", d.name)),
                Cell::new(format!("Player: {}", d.player)).span(3),
            ],
            vec![
                Cell::new(format!("Race: {race}")),
                Cell::new(format!("Gender: {gender}")),
                Cell::new(height),
                Cell::new(format!("Age: {}", d.age)),
            ],
            vec![
                Cell::new(class),
                Cell::new(level),
                Cell::new(format!("Weight: {} lbs", d.weight)).span(2),
            ],
            vec![
                Cell::new(xp),
                Cell::new(format!("Alignment: {}", d.alignment)).span(3),
            ],
            vec![Cell::new(next), Cell::new(kit).span(3)],
        ],
        Flavor::Core20 => vec![
            vec![
                Cell::new(format!("Name: {}", d.name)),
                Cell::new(format!("Player: {}", d.player)).span(3),
            ],
            vec![
                Cell::new(format!("Race: {race}")),
                Cell::new(format!("Gender: {gender}")),
                Cell::new(height),
                Cell::new(format!("Weight: {} lbs", d.weight)),
            ],
            vec![
                Cell::new(class),
                Cell::new(level),
                Cell::new(format!("Alignment: {}", d.alignment)).span(2),
            ],
            vec![Cell::new(xp).span(2), Cell::new(next).span(2)],
            vec![Cell::new(kit).span(4)],
        ],
    };
    single(ids::PERSONAL, "Personal Information", vec![table(4, rows)])
}

/// The kit, when the character has one the databases know.
fn kit_name<'a>(ctx: &Ctx<'a>) -> Option<&'a str> {
    let kit = ctx.ch.data.kit.as_str();
    ctx.part(kit).map(|_| kit)
}

/// Ability Scores (0x5a29bf).
fn abilities(ctx: &Ctx) -> Section {
    let s = &ctx.scores;
    let muscle = s.sub(STR, 2);
    let stamina = s.sub(STR, 1);
    let exc_muscle = s.exceptional_for(muscle);
    let exc_stamina = s.exceptional_for(stamina);
    let doors = tables::strength(&tables::OPEN_DOORS, muscle, exc_muscle);
    let doors = if doors.1 > 0 {
        format!("{}({})", doors.0, doors.1)
    } else {
        doors.0.to_string()
    };
    let (aim, balance) = (s.sub(DEX, 1), s.sub(DEX, 2));
    let (health, fitness) = (s.sub(CON, 1), s.sub(CON, 2));
    let hp = {
        let normal = at(&tables::HIT_POINT_ADJUSTMENT, fitness);
        let warrior = at(&tables::WARRIOR_HIT_POINT_ADJUSTMENT, fitness);
        let minimum = at(&tables::MINIMUM_HIT_DIE_ROLL, fitness);
        if minimum != 0 {
            format!("Hit Point Adjustment: {normal:+}({warrior:+}) (Minimum roll {minimum})")
        } else if normal == warrior {
            format!("Hit Point Adjustment: {normal:+}")
        } else {
            format!("Hit Point Adjustment: {normal:+}({warrior:+})")
        }
    };
    let reason = s.sub(INT, 1).clamp(1, 25);
    let knowledge = s.sub(INT, 2);
    let max_level = at(&tables::MAXIMUM_SPELL_LEVEL, reason);
    let max_level = if max_level == 0 {
        "--".to_string()
    } else {
        ordinal(max_level)
    };
    let max_spells = match at(&tables::MAXIMUM_SPELLS, reason) {
        i32::MAX => "All".to_string(),
        n => n.to_string(),
    };
    let illusion = if reason >= 19 {
        format!("{} Level", ordinal(reason - 18))
    } else {
        "None".into()
    };
    let (intuition, willpower) = (s.sub(WIS, 1), s.sub(WIS, 2));
    let bonus = bonus_clerical_spells(ctx, intuition)
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let immunity = if willpower >= 19 {
        format!("Class {}", willpower - 18)
    } else {
        "None".into()
    };
    let (leadership, appearance) = (s.sub(CHA, 1), s.sub(CHA, 2));

    struct Ability {
        label: &'static str,
        score: String,
        subs: [(&'static str, String); 2],
        lines: [Vec<String>; 2],
    }
    let abilities = [
        Ability {
            label: "Str:",
            score: s.strength_text(s.score(STR)),
            subs: [
                ("Stamina:", s.strength_text(stamina)),
                ("Muscle:", s.strength_text(muscle)),
            ],
            lines: [
                vec![
                    format!(
                        "Weight Allowance: {} lbs",
                        thousands(
                            tables::strength(&tables::WEIGHT_ALLOWANCE, stamina, exc_stamina)
                                .into()
                        )
                    ),
                    format!(
                        "Bend Bars/Lift Gates: {}%",
                        tables::strength(&tables::BEND_BARS, muscle, exc_muscle)
                    ),
                ],
                vec![
                    format!(
                        "Attack Adj.: {}",
                        signed(tables::strength(
                            &tables::ATTACK_ADJUSTMENT,
                            muscle,
                            exc_muscle
                        ))
                    ),
                    format!(
                        "Damage Adj.: {}",
                        signed(tables::strength(
                            &tables::DAMAGE_ADJUSTMENT,
                            muscle,
                            exc_muscle
                        ))
                    ),
                    format!(
                        "Max. Press: {} lbs",
                        thousands(
                            tables::strength(&tables::MAXIMUM_PRESS, muscle, exc_muscle).into()
                        )
                    ),
                    format!("Open Doors: {doors}"),
                ],
            ],
        },
        Ability {
            label: "Dex:",
            score: s.score(DEX).to_string(),
            subs: [("Aim:", aim.to_string()), ("Balance:", balance.to_string())],
            lines: [
                vec![
                    format!(
                        "Missile Adjustment: {}",
                        signed(at(&tables::MISSILE_ADJUSTMENT, aim))
                    ),
                    format!("Pick Pockets: {}%", signed(at(&tables::PICK_POCKETS, aim))),
                    format!("Open Locks: {}%", signed(at(&tables::OPEN_LOCKS, aim))),
                ],
                vec![
                    format!(
                        "Reaction Adjustment: {}",
                        signed(at(&tables::REACTION_ADJUSTMENT, balance))
                    ),
                    format!(
                        "Armor Class: {}",
                        signed(at(&tables::DEFENSIVE_ADJUSTMENT, balance))
                    ),
                    format!(
                        "Move Silently: {}%",
                        signed(at(&tables::MOVE_SILENTLY, balance))
                    ),
                    format!(
                        "Climb Walls: {}%",
                        signed(at(&tables::CLIMB_WALLS, balance))
                    ),
                ],
            ],
        },
        Ability {
            label: "Con:",
            score: s.score(CON).to_string(),
            subs: [
                ("Health:", health.to_string()),
                ("Fitness:", fitness.to_string()),
            ],
            lines: [
                vec![
                    format!("System Shock: {}%", at(&tables::SYSTEM_SHOCK, health)),
                    format!("Poison Save: {}", signed(at(&tables::POISON_SAVE, health))),
                ],
                vec![
                    hp,
                    format!(
                        "Resurrection Chance: {}%",
                        at(&tables::RESURRECTION, fitness)
                    ),
                ],
            ],
        },
        Ability {
            label: "Int:",
            score: s.score(INT).to_string(),
            subs: [
                ("Reason:", s.sub(INT, 1).to_string()),
                ("Knowledge:", knowledge.to_string()),
            ],
            lines: [
                vec![
                    format!("Max. Spell Level: {max_level}"),
                    format!("Max. Spells Per Level: {max_spells}"),
                    format!("Illusion Immunity: {illusion}"),
                ],
                vec![
                    format!(
                        "Bonus Proficiencies: {}",
                        at(&tables::BONUS_PROFICIENCIES, knowledge)
                    ),
                    format!("Chance to Learn New Spell: {}%", learn_spell(ctx)),
                ],
            ],
        },
        Ability {
            label: "Wis:",
            score: s.score(WIS).to_string(),
            subs: [
                ("Intuition:", intuition.to_string()),
                ("Willpower:", willpower.to_string()),
            ],
            lines: [
                vec![
                    format!("Bonus Clerical Spells: {bonus}"),
                    format!(
                        "Clerical Spell Failure Chance: {}%",
                        at(&tables::SPELL_FAILURE, intuition)
                    ),
                ],
                vec![
                    format!(
                        "Magic Defense Adjustment: {}",
                        signed(at(&tables::MAGIC_DEFENSE, willpower))
                    ),
                    format!("Spell Immunity: {immunity}"),
                ],
            ],
        },
        Ability {
            label: "Cha:",
            score: s.score(CHA).to_string(),
            subs: [
                ("Leadership:", leadership.to_string()),
                ("Appearance:", appearance.to_string()),
            ],
            lines: [
                vec![
                    format!(
                        "Loyalty Base: {}",
                        signed(at(&tables::LOYALTY_BASE, leadership))
                    ),
                    format!(
                        "Maximum Number of Henchmen: {}",
                        at(&tables::HENCHMEN, leadership)
                    ),
                ],
                vec![format!(
                    "Initial Reaction Adjustment: {}",
                    signed(at(&tables::INITIAL_REACTION, appearance))
                )],
            ],
        },
    ];
    let mut rows = Vec::new();
    for ability in abilities {
        for (r, line) in ability.lines.into_iter().enumerate() {
            let mut row = if r == 0 {
                vec![
                    Cell::large(ability.label),
                    Cell::large(ability.score.clone()),
                ]
            } else {
                vec![Cell::new(""), Cell::new("")]
            };
            if ctx.sp {
                let (label, value) = &ability.subs[r];
                row.push(Cell::new(*label));
                row.push(Cell::new(value.clone()));
            }
            let n = line.len();
            let span = 12 / n as u32;
            for (i, text) in line.into_iter().enumerate() {
                let align = if n == 1 || i == 0 {
                    super::Align::Left
                } else if i == n - 1 {
                    super::Align::Right
                } else {
                    super::Align::Center
                };
                row.push(Cell::new(text).span(span).align(align));
            }
            rows.push(row);
        }
    }
    let columns = if ctx.sp { 16 } else { 14 };
    single(ids::ABILITIES, "Ability Scores", vec![table(columns, rows)])
}

/// Bonus clerical spells per spell level 1 to 7 (0x422638), doubled for
/// a priest class that doubles them.
fn bonus_clerical_spells(ctx: &Ctx, wisdom: i32) -> [i32; 7] {
    let row = if wisdom >= 13 {
        tables::BONUS_CLERICAL_SPELLS[(wisdom.min(25) - 13) as usize]
    } else {
        [0; 7]
    };
    let double = ctx.classes.iter().any(|c| {
        c.group == 2
            && matches!(c.class, Some(Class::Expansion { data, .. }) if data.flag_0x2790.unwrap_or(0) != 0)
    });
    row.map(|v| if double { v * 2 } else { v })
}

/// Chance to learn a new spell (0x422498), from Knowledge.
fn learn_spell(ctx: &Ctx) -> i32 {
    at(&tables::LEARN_SPELL, ctx.scores.sub(INT, 2))
}

/// Proficiencies (0x5a4b14): Combat and Armor, Weapon Proficiencies,
/// Non-Weapon Proficiencies, side by side.
fn proficiencies(ctx: &Ctx) -> Section {
    let first = vec![combat::combat_box(ctx), combat::armor_box(ctx)];
    let mut names: Vec<String> = ctx
        .ch
        .data
        .weapon_proficiencies
        .iter()
        .map(|w| weapon_proficiency_name(ctx, w))
        .collect();
    names.sort();
    let weapons = SheetBox {
        title: "Weapon Proficiencies".into(),
        body: vec![match ctx.flavor {
            Flavor::Expansion => table(1, names.into_iter().map(|w| vec![Cell::new(w)]).collect()),
            Flavor::Core20 => Block::Lines(names),
        }],
    };
    let nonweapon = match ctx.flavor {
        Flavor::Expansion => {
            let (list, secondary) = nwp::rows(ctx);
            let mut rows = Vec::new();
            for r in list {
                rows.push(vec![Cell::new(r.name).span(2), Cell::new(r.score).right()]);
                if !r.extra.is_empty() {
                    rows.push(vec![Cell::new(""), Cell::new(r.extra).span(2)]);
                }
            }
            SheetBox {
                title: if secondary {
                    "Secondary Skills"
                } else {
                    "Non-Weapon Proficiencies"
                }
                .into(),
                body: vec![table(3, rows)],
            }
        }
        Flavor::Core20 => SheetBox {
            title: "Non-Weapon Proficiencies".into(),
            body: vec![table(3, nonweapon_rows(ctx))],
        },
    };
    Section {
        id: ids::PROFICIENCIES,
        content: SectionContent::Columns(vec![first, vec![weapons], vec![nonweapon]]),
    }
}

fn weapon_proficiency_name(ctx: &Ctx, name: &str) -> String {
    if ctx.ch.data.unknown_0xac[3].iter().any(|s| s == name) {
        format!("{name} (Specialist)")
    } else {
        name.to_string()
    }
}

/// 2.0's nonweapon proficiencies with their scores, in the order of the
/// file.
fn nonweapon_rows(ctx: &Ctx) -> Vec<Vec<Cell>> {
    let d = &ctx.ch.data;
    d.nonweapon_proficiencies
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let score = rogue::nonweapon_score(ctx, i, name);
            vec![
                Cell::new(name.clone()).span(2),
                ctx.value(score.to_string()),
            ]
        })
        .collect()
}

/// A part's name as the ability boxes show it: without the "Race, " or
/// "Class, " prefix and the cost in parentheses. A name whose comma comes
/// after a colon ("Phobia: Snakes, severe") has no such prefix.
pub(crate) fn ability_display_name(name: &str) -> String {
    let name = match name.split_once(", ") {
        Some((prefix, rest)) if !prefix.contains(':') => rest,
        _ => name,
    };
    let name = match name.rfind(" (") {
        Some(i) if name.ends_with(')') && name[i + 2..name.len() - 1].parse::<i32>().is_ok() => {
            &name[..i]
        }
        _ => name,
    };
    name.to_string()
}

/// Racial Abilities (0x5a67c2): name - description, one per line.
fn racial_text(ctx: &Ctx) -> String {
    let mut text = String::new();
    for part in ctx.racial_abilities() {
        let name = ability_display_name(&part.head.name);
        let value = rogue::ability_value(ctx, part, None);
        text.push_str(&name);
        if let Some(v) = value {
            text.push(' ');
            text.push_str(&v);
        }
        text.push_str(" - ");
        text.push_str(&part.head.description);
        text.push('\n');
    }
    text
}

/// Class Abilities (0x5a6a1e): per class its name, then its abilities
/// indented, then its spheres or schools.
fn class_text(ctx: &Ctx) -> String {
    let mut text = String::new();
    for (g, class) in ctx.classes.iter().enumerate() {
        text.push_str(class.name);
        text.push('\n');
        let mut abilities = ctx.class_abilities(g);
        abilities.sort_by(|a, b| a.head.name.cmp(&b.head.name));
        for part in abilities {
            let name = ability_display_name(&part.head.name);
            text.push('\t');
            text.push_str(&name);
            if let Some(v) = rogue::ability_value(ctx, part, Some(g)) {
                text.push(' ');
                text.push_str(&v);
            }
            text.push_str(" - ");
            text.push_str(&part.head.description);
            text.push('\n');
        }
        text.push_str(&spells::spheres_text(ctx, g));
        // Every free spell of the class, whatever the level (0x667936
        // with 100).
        if let Some(Class::Expansion { data, .. }) = class.class {
            for level in crate::userdb::classes::levels::free_spells(data) {
                for line in crate::userdb::classes::levels::free_spell_lines_untranslated(
                    class.name, &level,
                ) {
                    text.push_str("\tFree Spell - ");
                    text.push_str(&line);
                    text.push('\n');
                }
            }
        }
    }
    text
}

/// Traits or disadvantages: name - description per line.
fn part_list_text(ctx: &Ctx, names: &[String]) -> String {
    let mut text = String::new();
    for name in names {
        let description = ctx.part(name).map_or("", |p| p.head.description.as_str());
        text.push_str(&ability_display_name(name));
        text.push_str(" - ");
        text.push_str(description);
        text.push('\n');
    }
    text
}

/// Inventory (0x5a70c2): the nonempty bundles and their items, with the
/// spells written in books for "with Spells".
fn inventory(ctx: &Ctx, id: u32, with_spells: bool) -> Section {
    let d = &ctx.ch.data;
    let mut bundles = Vec::new();
    for (b, bundle) in d.inventory.iter().enumerate() {
        if bundle.items.is_empty() {
            continue;
        }
        bundles.push(ListItem {
            text: BUNDLE_NAMES[b].to_string(),
            children: Some(items(ctx, &bundle.items, with_spells)),
        });
    }
    single(id, "Inventory", vec![Block::List(bundles)])
}

/// The items of a bundle, a book's spells "name (pages)" one entry each.
/// The Expansion writes quantities with thousands separators.
fn items(ctx: &Ctx, list: &[InventoryItem], with_spells: bool) -> Vec<ListItem> {
    list.iter()
        .map(|item| {
            let text = if item.quantity > 1 {
                let n = if ctx.expansion() {
                    thousands(i64::from(item.quantity))
                } else {
                    item.quantity.to_string()
                };
                format!("{} x{n}", item.name)
            } else {
                item.name.clone()
            };
            let mut children = match item.contents.as_deref() {
                Some(Object::InventoryBundle(bundle)) if !bundle.items.is_empty() => {
                    Some(items(ctx, &bundle.items, with_spells))
                }
                _ => None,
            };
            if with_spells
                && let Some(extra) = &item.extra
                && !extra.unknown_0x10.is_empty()
            {
                let entries = extra.unknown_0x10.iter().enumerate().map(|(s, name)| {
                    let pages = extra.unknown_0x24.get(s).copied().unwrap_or(0);
                    ListItem {
                        text: format!("{name} ({pages})\n"),
                        children: None,
                    }
                });
                children.get_or_insert_with(Vec::new).extend(entries);
            }
            ListItem { text, children }
        })
        .collect()
}

/// Kit Description (0x5aa192): the kit part's description.
fn kit(ctx: &Ctx) -> Section {
    let (title, text) = match kit_name(ctx) {
        Some(name) => (
            format!("Kit - {name}"),
            ctx.part(name)
                .map(|p| p.head.description.clone())
                .unwrap_or_default(),
        ),
        None => ("Kit - None".to_string(), String::new()),
    };
    single(ids::KIT, &title, vec![Block::Paragraph(text)])
}

/// Character Portrait (0x5aa24d).
fn portrait(ctx: &Ctx) -> Section {
    // The folder comes first.
    let d = &ctx.ch.data;
    let (dir, file) = (&d.unknown_0x1db8[0], &d.unknown_0x1db8[1]);
    let windows = if file.is_empty() || dir.is_empty() {
        format!("{}\\Bitmaps\\Portrait\\NA.bmp", ctx.rules.windows_root)
    } else {
        crate::pictures::joined(dir, file)
    };
    let file = crate::pictures::Finder::new(&ctx.rules.root)
        .picture(&windows, crate::pictures::Kind::Portrait);
    single(
        ids::PORTRAIT,
        "Portrait",
        vec![Block::Image { windows, file }],
    )
}

#[cfg(test)]
mod tests {
    use super::ability_display_name;

    #[test]
    fn display_names_drop_the_prefix_and_cost_only() {
        assert_eq!(
            ability_display_name("Dwarf, Infravision (5)"),
            "Infravision"
        );
        assert_eq!(
            ability_display_name("Phobia: Snakes, severe"),
            "Phobia: Snakes, severe"
        );
    }
}
