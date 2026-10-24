//! Encounter templates (`Database/EncPlat.dat`, `UserDbas/EncPlatU.dat`):
//! the Encounter Template dialog (IDD 289) and the dialogs its buttons
//! open, which edit the template's monster, NPC and item lines, its traps,
//! its lair treasure and its 2-20 table information.
//!
//! Unlike the other kinds, the original stores the name as typed (only the
//! check trims it), and a rename puts the template at the end of the user
//! file. The helpers here build the lines the Add buttons make, with the
//! constructors' defaults, and settle a line as each Properties dialog's
//! OK does.

use cr_formats::Object;
use cr_formats::encounter::{
    EncounterNpc, EncounterTemplate, TemplateItemProperties, TemplateMonsterProperties,
    TemplateNpcProperties, TemplateNumAppear,
};
use cr_formats::inventory::InventoryBundle;
use cr_formats::monster::{Arsenal, Monster, MonsterType};
use cr_formats::parts::Part;
use cr_i18n::tr_noop;

use super::{FieldDesc, Flavor, NewContext, UserKind};
use crate::{Installation, Result};

pub struct Templates;

impl UserKind for Templates {
    type Rec = EncounterTemplate;
    const NOUN: &'static str = tr_noop!("Encounter Template");
    const FILE: &'static str = "UserDbas/EncPlatU.dat";
    const TRIM_NAME: bool = false;
    const RENAME_TO_END: bool = true;

    fn name(r: &EncounterTemplate) -> &str {
        &r.name
    }

    fn set_name(r: &mut EncounterTemplate, name: &str) {
        name.clone_into(&mut r.name);
    }

    fn standard(install: &Installation) -> Result<Vec<EncounterTemplate>> {
        super::standard_list(install, "EncPlat", pick)
    }

    fn parse_user(data: &[u8]) -> std::result::Result<Vec<EncounterTemplate>, cr_formats::Error> {
        super::parse_list(data).map(|l| l.into_iter().filter_map(pick).collect())
    }

    fn user_bytes(
        records: &[EncounterTemplate],
    ) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        super::list_bytes(
            records
                .iter()
                .map(|t| Object::EncounterTemplate(Box::new(t.clone())))
                .collect(),
        )
    }

    /// New (0x6950a9): the constructor's template, of the monster type
    /// selected in the list.
    fn new_record(context: &NewContext) -> EncounterTemplate {
        new_template(&context.parent)
    }

    /// A line left with "and" or "or" but nothing in its second box
    /// would hang: it is stored as "appear." instead.
    fn prepare(r: &mut EncounterTemplate) {
        for object in r.monsters.iter_mut().chain(r.npcs.iter_mut()) {
            match object {
                Object::TemplateMonsterProperties(p) if p.second_name.trim().is_empty() => {
                    p.flags[1] = 0;
                    p.flags[2] = 0;
                }
                Object::TemplateNpcProperties(p)
                    if p.second_npc.name.trim().is_empty() && p.second_npc.race.is_empty() =>
                {
                    p.flags[1] = 0;
                    p.flags[2] = 0;
                }
                _ => {}
            }
        }
    }

    /// The template editor is the app's own (see the GTK front end).
    fn fields(_flavor: Flavor) -> Vec<FieldDesc<EncounterTemplate>> {
        Vec::new()
    }

    fn summary(r: &EncounterTemplate) -> String {
        r.monster_type.clone()
    }
}

fn pick(o: Object) -> Option<EncounterTemplate> {
    match o {
        Object::EncounterTemplate(t) => Some(*t),
        _ => None,
    }
}

/// `CEncTemplateOb`'s constructor (0x647ec1) with `monster_type`: no
/// lines, exempt from the 2-20 tables, both lair and wandering.
pub fn new_template(monster_type: &str) -> EncounterTemplate {
    EncounterTemplate {
        name: String::new(),
        unknown_0x2d0: 0,
        frequency: 0,
        hit_dice: 0,
        monster_type: monster_type.to_owned(),
        notes: String::new(),
        climate_terrain: String::new(),
        placement: [1, 0, 0],
        monsters: Vec::new(),
        npcs: Vec::new(),
        traps: Vec::new(),
        items: Vec::new(),
        lair_treasure: [0; 26],
        climates: [0; 150],
    }
}

/// The empty monster type a line keeps when it does not count a type
/// (0x688d3e).
pub fn empty_type() -> MonsterType {
    MonsterType {
        name: String::new(),
        page_picture: String::new(),
        unknown_0x14: String::new(),
        unknown_0x10: String::new(),
        unknown_0x18: 0,
        unknown_0x1c: 0,
        picture: String::new(),
    }
}

/// `CMonsterOb`'s constructor (0x67c9a1), what an item line keeps when it
/// does not count a monster: every number 0 but the hit die, a d8.
pub fn empty_monster() -> Monster {
    Monster {
        name: String::new(),
        description: String::new(),
        picture: 0,
        unknown_0x3b8: 0,
        frequency: 0,
        organization: String::new(),
        activity_cycle: String::new(),
        diet: String::new(),
        intelligence: String::new(),
        number_appearing_min: 0,
        number_appearing_max: 0,
        armor_class: 0,
        movement: 0,
        fly_maneuverability: String::new(),
        fly_rate: 0,
        unknown_0x414: String::new(),
        swim_rate: 0,
        unknown_0x41c: String::new(),
        burrow_rate: 0,
        unknown_0x424: String::new(),
        unknown_0x428: 0,
        unknown_0x42c: String::new(),
        unknown_0x430: 0,
        hit_dice: 0,
        unknown_0x438: 0,
        hit_die_size: 8,
        unknown_0x440: String::new(),
        unknown_0x444: String::new(),
        defense_notes: String::new(),
        unknown_0x464: String::new(),
        magic_resistance: 0,
        morale: 0,
        experience: 0,
        unknown_0x478: 0,
        thac0: 0,
        unknown_0x470: 0,
        unknown_0x3f8: 0,
        unknown_0x3fc: 0,
        unknown_0x400: 0,
        unknown_0x404: 0,
        monster_type: String::new(),
        alignment: String::new(),
        climate_terrain: String::new(),
        unknown_0x33c: String::new(),
        unknown_0x340: String::new(),
        unknown_0x344: String::new(),
        notes: String::new(),
        unknown_0x47c: [0; 6],
        unknown_0x44c: [0; 6],
        unknown_0x4: [0; 150],
        unknown_0x25c: [(0, 0); 26],
        armor_class_notes: Vec::new(),
        unknown_0x4c0: Vec::new(),
        spells: Vec::new(),
        arsenal: Arsenal {
            damage: String::new(),
            attacks: String::new(),
            special_attacks: String::new(),
            special_defenses: String::new(),
            attack_list: Vec::new(),
        },
        inventory: InventoryBundle { items: Vec::new() },
    }
}

/// `CEncounterNPCOb`'s constructor (0x652ede): no name, no classes.
pub fn empty_npc() -> EncounterNpc {
    EncounterNpc {
        name: String::new(),
        race: String::new(),
        classes: Default::default(),
        levels: [-1; 5],
        unknown_0x30: Vec::new(),
    }
}

/// An NPC as NPC Creation's "Add to Template" stores it (0x653d12): the
/// generated name and race, and the classes and their levels.
pub fn npc_of(name: &str, race: &str, classes: &[(String, i32)]) -> EncounterNpc {
    let mut npc = empty_npc();
    name.clone_into(&mut npc.name);
    race.clone_into(&mut npc.race);
    for (i, (class, level)) in classes.iter().take(5).enumerate() {
        npc.classes[i].clone_from(class);
        npc.levels[i] = *level;
    }
    npc
}

/// What a line's second box or kind does: the combo of the Properties
/// dialogs ("appear.", "and", "or").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combo {
    Appear,
    And,
    Or,
}

impl Combo {
    /// The combo's items, in its order.
    pub const NAMES: [&str; 3] = [tr_noop!("appear."), tr_noop!("and"), tr_noop!("or")];

    pub fn of_flags(flags: &[i32; 3]) -> Self {
        if flags[1] != 0 {
            Self::And
        } else if flags[2] != 0 {
            Self::Or
        } else {
            Self::Appear
        }
    }

    pub fn of(index: u32) -> Self {
        match index {
            1 => Self::And,
            2 => Self::Or,
            _ => Self::Appear,
        }
    }

    pub fn index(self) -> u32 {
        self as u32
    }

    /// The flags OK stores with `dependent`: [dependent, and, or].
    pub fn flags(self, dependent: bool) -> [i32; 3] {
        [
            i32::from(dependent),
            i32::from(self == Self::And),
            i32::from(self == Self::Or),
        ]
    }
}

/// `CEncTemplateMonsterPropertiesOb`'s constructor (0x6519fe) as Add
/// (IDD 302, 0x72bf94) fills it: 1d1 of `monster`, always, with a copy of
/// it as the only entry of the random list.
pub fn new_monster_line(monster: &Monster) -> TemplateMonsterProperties {
    TemplateMonsterProperties {
        dice: [1, 1, 1, 0, 1, 1, 1, 0],
        flags: [0; 3],
        percent: 100,
        per: 0,
        beyond: 0,
        monster_name: monster.name.clone(),
        second_name: String::new(),
        reference_name: String::new(),
        monster_type: empty_type(),
        monsters: vec![Object::Monster(Box::new(monster.clone()))],
    }
}

/// `CEncTemplateNPCPropertiesOb`'s constructor (0x652421) as Add (IDD 383,
/// 0x729e0d) fills it: 1d1 of `npc`, always, and `npc` as the only entry
/// of the random list.
pub fn new_npc_line(npc: EncounterNpc) -> TemplateNpcProperties {
    TemplateNpcProperties {
        dice: [1, 1, 1, 0, 1, 1, 1, 0],
        flags: [0; 3],
        percent: 100,
        per: 0,
        beyond: 0,
        reference_name: String::new(),
        monster_type: empty_type(),
        npc: npc.clone(),
        second_npc: empty_npc(),
        npcs: vec![Object::EncounterNpc(Box::new(npc))],
    }
}

/// `CEncTemplateItemPropertiesOb`'s constructor (0x650761) as IDD 314's OK
/// (0x72feed) fills it: one of `part`, always, no coins.
pub fn new_item_line(part: Part) -> TemplateItemProperties {
    TemplateItemProperties {
        percent: 100,
        dice: [1, 1, 1, 0, 0, 0, 0, 0, 0],
        dependency: [0; 3],
        monster: Box::new(empty_monster()),
        monster_type: empty_type(),
        part: Box::new(part),
        monster_names: Vec::new(),
        escorts: Vec::new(),
        per_item: Vec::new(),
    }
}

/// IDD 313's Close (0x733d54): `dice` of `monster` with an item.
pub fn new_escort(dice: [i32; 4], monster: &Monster) -> TemplateNumAppear {
    TemplateNumAppear {
        dice,
        monster: Box::new(monster.clone()),
    }
}

/// What the Properties dialogs show when they open (OnInitDialog): each
/// box of four (N, S, M, B) at least 1d1 * 1 + 0.
pub fn clamp_dice(dice: &mut [i32]) {
    for d in dice.as_chunks_mut::<4>().0 {
        d[0] = d[0].max(1);
        d[1] = d[1].max(1);
        d[2] = d[2].max(1);
        d[3] = d[3].max(0);
    }
}

/// IDD 309's OnInitDialog (0x7308f2): the item dice and the coin dice
/// clamped; the coin itself stays.
pub fn clamp_item_dice(dice: &mut [i32; 9]) {
    clamp_dice(&mut dice[..4]);
    clamp_dice(&mut dice[5..]);
}

/// IDD 304's OK (0x72d1ff): a line that is not dependent counts nothing,
/// once for every one beyond one; one that only appears has no second box.
pub fn settle_monster_line(p: &mut TemplateMonsterProperties) {
    if p.flags[0] == 0 {
        p.reference_name.clear();
        p.monster_type = empty_type();
        p.per = 1;
        p.beyond = 1;
    }
    if Combo::of_flags(&p.flags) == Combo::Appear {
        p.second_name.clear();
        p.dice[4..].copy_from_slice(&[1, 1, 1, 0]);
    }
}

/// IDD 384's OK (0x72eadc): as for monsters, but the second NPC stays.
pub fn settle_npc_line(p: &mut TemplateNpcProperties) {
    if p.flags[0] == 0 {
        p.reference_name.clear();
        p.monster_type = empty_type();
        p.per = 1;
        p.beyond = 1;
    }
    if Combo::of_flags(&p.flags) == Combo::Appear {
        p.dice[4..].copy_from_slice(&[1, 1, 1, 0]);
    }
}

/// IDD 309's OK (0x730c51): an item that is not dependent counts nothing.
pub fn settle_item_line(p: &mut TemplateItemProperties) {
    if p.dependency[0] == 0 {
        *p.monster = empty_monster();
        p.monster_type = empty_type();
        p.dependency[1] = 1;
        p.dependency[2] = 1;
    }
}

/// What a dependent line counts, as IDD 304 shows it: the monster's name,
/// else the type's.
pub fn reference_text(reference_name: &str, monster_type: &MonsterType) -> String {
    if reference_name.is_empty() {
        monster_type.name.clone()
    } else {
        reference_name.to_owned()
    }
}

/// `" (NdS * M + B)"`, with five spaces of padding added to `pad` for each
/// part left out.
fn dice_part(dice: &[i32], pad: &mut String) -> String {
    let mut s = format!(" ({}d{}", dice[0], dice[1]);
    if dice[2] > 1 {
        s += &format!(" * {}", dice[2]);
    } else {
        pad.push_str("     ");
    }
    if dice[3] > 0 {
        s += &format!(" + {}", dice[3]);
    } else {
        pad.push_str("     ");
    }
    s.push(')');
    s
}

/// The second box after " and " or " or ": padded as the first (NPCs), or
/// with one space before the name (monsters).
fn second_part(dice: &[i32], name: &str, padded: bool) -> String {
    let mut pad = String::from("  ");
    let s = dice_part(dice, &mut pad);
    if padded {
        format!("{s}{pad}{name}")
    } else {
        format!("{s} {name}")
    }
}

fn line_text(dice: &[i32], names: &[String], combo: Combo, second: &str, padded: bool) -> String {
    let mut pad = String::from("  ");
    let mut s = dice_part(&dice[..4], &mut pad);
    s += &pad;
    s += &names.join(" or ");
    let joint = match combo {
        Combo::Appear => return s,
        Combo::And => " and ",
        Combo::Or => " or ",
    };
    s + joint + &second_part(&dice[4..8], second, padded)
}

/// A line of IDD 303's list (0x7296c6): `" (1d1)            Aarakocra"`,
/// then the second box after " and " or " or ".
pub fn monster_line_text(p: &TemplateMonsterProperties) -> String {
    let names: Vec<String> = p
        .monsters
        .iter()
        .filter_map(|o| match o {
            Object::Monster(m) => Some(m.name.clone()),
            _ => None,
        })
        .collect();
    line_text(
        &p.dice,
        &names,
        Combo::of_flags(&p.flags),
        &p.second_name,
        false,
    )
}

/// A line of IDD 383's list (0x72a5ac): as for monsters with the NPCs'
/// descriptions, and the second box padded too.
pub fn npc_line_text(p: &TemplateNpcProperties) -> String {
    let names: Vec<String> = p
        .npcs
        .iter()
        .filter_map(|o| match o {
            Object::EncounterNpc(n) => Some(npc_description(n)),
            _ => None,
        })
        .collect();
    line_text(
        &p.dice,
        &names,
        Combo::of_flags(&p.flags),
        &npc_description(&p.second_npc),
        true,
    )
}

/// An NPC as the template dialogs describe it (0x6536e5): "Human Fighter
/// Level: 3", "Elf Fighter/Mage Levels: 3/4", up to the last class with a
/// level; "" without one.
pub fn npc_description(npc: &EncounterNpc) -> String {
    let Some(last) = npc.levels.iter().rposition(|l| *l > 0) else {
        return String::new();
    };
    let classes = npc.classes[..=last].join("/");
    let levels: Vec<String> = npc.levels[..=last].iter().map(i32::to_string).collect();
    if last == 0 {
        format!("{} {classes} Level: {}", npc.race, levels[0])
    } else {
        format!("{} {classes} Levels: {}", npc.race, levels.join("/"))
    }
}

/// A line of IDD 311's escort lists (0x733991).
pub fn escort_text(n: &TemplateNumAppear) -> String {
    format!(
        "Number Appearing:{}d{} * {} + {}  Name: {}",
        n.dice[0], n.dice[1], n.dice[2], n.dice[3], n.monster.name
    )
}

/// The frequencies of IDD 325's radios: Very Rare, Rare, Uncommon, Common
/// and Exempt from table.
pub const FREQUENCIES: [i32; 5] = [4, 11, 20, 65, 0];

pub const FREQUENCY_NAMES: [&str; 5] = [
    tr_noop!("Very Rare (4%)"),
    tr_noop!("Rare (11%)"),
    tr_noop!("Uncommon (20%)"),
    tr_noop!("Common (65%)"),
    tr_noop!("Exempt from table"),
];

/// The radio IDD 325 checks for `frequency` (0x72b793).
pub fn frequency_choice(frequency: i32) -> usize {
    match frequency {
        ..4 => 4,
        4..11 => 0,
        11..20 => 1,
        20..65 => 2,
        _ => 3,
    }
}

pub const PLACEMENT_NAMES: [&str; 3] = [
    tr_noop!("Both Lair and Wandering"),
    tr_noop!("Lair Only (Independent of the name assigned to this encounter)"),
    tr_noop!("Wandering Only (Independent of the name assigned to this encounter)"),
];

/// The placement radio IDD 325 checks: the first flag set.
pub fn placement_choice(placement: &[i32; 3]) -> usize {
    placement.iter().position(|f| *f != 0).unwrap_or(0)
}

/// What IDD 325's OK stores for radio `choice`.
pub fn placement_of(choice: usize) -> [i32; 3] {
    let mut placement = [0; 3];
    placement[choice.min(2)] = 1;
    placement
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::UserDb;
    use crate::userdb::testing::TempInstall;

    fn monster(name: &str) -> Monster {
        let mut m = empty_monster();
        name.clone_into(&mut m.name);
        m
    }

    #[test]
    fn a_hanging_or_is_stored_as_appear() {
        let mut t = new_template("");
        let mut line = new_monster_line(&monster("Orc"));
        line.flags = Combo::Or.flags(false);
        let mut both = line.clone();
        "Goblin".clone_into(&mut both.second_name);
        t.monsters = vec![
            Object::TemplateMonsterProperties(Box::new(both)),
            Object::TemplateMonsterProperties(Box::new(line)),
        ];
        Templates::prepare(&mut t);
        let combos: Vec<Combo> = t
            .monsters
            .iter()
            .map(|o| match o {
                Object::TemplateMonsterProperties(p) => Combo::of_flags(&p.flags),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(combos, [Combo::Or, Combo::Appear]);
    }

    #[test]
    fn lines_read_like_the_list() {
        let mut line = new_monster_line(&monster("Orc"));
        assert_eq!(monster_line_text(&line), " (1d1)            Orc");
        line.dice[..4].copy_from_slice(&[2, 4, 10, 1]);
        line.flags = Combo::Or.flags(false);
        line.second_name = "Ogre".into();
        line.monsters
            .push(Object::Monster(Box::new(monster("Goblin"))));
        assert_eq!(
            monster_line_text(&line),
            " (2d4 * 10 + 1)  Orc or Goblin or  (1d1) Ogre"
        );
        let npc = npc_of("Bob", "Elf", &[("Fighter".into(), 3), ("Mage".into(), 4)]);
        assert_eq!(npc_description(&npc), "Elf Fighter/Mage Levels: 3/4");
        assert_eq!(npc_description(&empty_npc()), "");
        let mut line = new_npc_line(npc_of("Al", "Dwarf", &[("Thief".into(), 2)]));
        line.flags = Combo::And.flags(false);
        line.second_npc = npc_of("", "Human", &[("Cleric".into(), 1)]);
        assert_eq!(
            npc_line_text(&line),
            " (1d1)            Dwarf Thief Level: 2 and  (1d1)            Human Cleric Level: 1"
        );
    }

    #[test]
    fn ok_settles_lines() {
        let mut line = new_monster_line(&monster("Orc"));
        line.reference_name = "Goblin".into();
        line.second_name = "Ogre".into();
        line.dice[4..].copy_from_slice(&[3, 3, 3, 3]);
        settle_monster_line(&mut line);
        assert_eq!((line.per, line.beyond), (1, 1));
        assert!(line.reference_name.is_empty() && line.second_name.is_empty());
        assert_eq!(line.dice[4..], [1, 1, 1, 0]);

        let mut line = new_npc_line(empty_npc());
        line.second_npc = npc_of("", "Human", &[("Cleric".into(), 1)]);
        line.dice[4..].copy_from_slice(&[3, 3, 3, 3]);
        settle_npc_line(&mut line);
        assert_eq!(line.second_npc.race, "Human");
        assert_eq!(line.dice[4..], [1, 1, 1, 0]);

        let mut dice = [0; 9];
        clamp_item_dice(&mut dice);
        assert_eq!(dice, [1, 1, 1, 0, 0, 1, 1, 1, 0]);
    }

    #[test]
    fn frequency_and_placement_round_trip() {
        for (i, f) in FREQUENCIES.iter().enumerate() {
            assert_eq!(frequency_choice(*f), i);
        }
        for i in 0..3 {
            assert_eq!(placement_choice(&placement_of(i)), i);
        }
    }

    #[test]
    fn renaming_moves_a_template_to_the_end() {
        let t = TempInstall::new("templates", &[]);
        let mut db = UserDb::<Templates>::load(&t.install).unwrap();
        for name in ["First", " Second "] {
            let mut template = db.new_record(0, "Orc");
            template.name = name.into();
            db.add(template).unwrap();
        }
        // The name is stored as typed and checked trimmed, ignoring case.
        assert_eq!(db.user[1].name, " Second ");
        let mut clash = db.new_record(0, "Orc");
        clash.name = "second".into();
        assert!(db.add(clash).is_err());

        let mut kept = db.user[0].clone();
        kept.notes = "Same place".into();
        assert_eq!(db.replace(0, kept), Ok(0));
        let mut renamed = db.user[0].clone();
        renamed.name = "Third".into();
        assert_eq!(db.replace(0, renamed), Ok(1));
        let names: Vec<&str> = db.user.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, [" Second ", "Third"]);
        assert_eq!(db.user[1].notes, "Same place");

        db.save(&t.install).unwrap();
        let again = UserDb::<Templates>::load(&t.install).unwrap();
        assert_eq!(again.user, db.user);
    }
}
