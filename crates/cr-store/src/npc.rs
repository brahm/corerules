//! The NPC generator of the Expansion (`CNPCGeneration`, Tsrp3 0x4bf418):
//! Create NPC, Ability Scores…, Create Multiple NPCs and the NPC packages
//! of generated encounters.
//!
//! Generation reproduces the original for a given seed: it draws from the
//! MSVC `rand()` ([`Rand`]) in the same order, one stage after the other
//! (document constructor, name, abilities, alignment, kit, social class,
//! background, height/weight/age, money, nonweapon and weapon
//! proficiencies, languages, hit points, equipment, spells, psionics).
//! Paths the specification leaves undecoded (Player's Option proficiencies
//! and equipment, some kit and class objects, dual-class hit points) take
//! a deterministic fallback and say so in the result's `messages`.
//!
//! Core Rules 2.0 installations (Evermore) roll the name and abilities
//! with their own code and the other stages with the Expansion's.
//!
//! ```no_run
//! # fn main() -> cr_store::Result<()> {
//! use cr_store::npc::{NpcOptions, Rand};
//! let install = cr_store::Installation::open("/path/to/AD&D Core Rules 2.0")?;
//! let rules = install.npc_rules()?;
//! let mut rand = Rand::seeded(628);
//! let npc = rules.generate(&mut rand, "Human", &[("Fighter".into(), 5)], &NpcOptions::default())?;
//! let file = install.save_npc(&npc, "")?;
//! # Ok(())
//! # }
//! ```

mod build;
mod data;
mod equip;
mod magic;
mod names;
mod prof;
mod program;
mod state;

use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

use cr_formats::character::{
    CORE20_VERSION, Character, CharacterEntry, CharacterIndex, EXPANSION_VERSION, EntryExtra,
};

use crate::create::{INDEX_SCHEMA, new_file_name, write_new};
use crate::edit::{CLASS_LINE, LEVEL_LINE, PLAYER_LINE, replace};
use crate::encounter::flavor_of;
use crate::lists::PartTable;
pub use crate::treasure::{Flavor, Rand};
use crate::{Error, Installation, Result};
use data::Data;
use state::Gen;

/// What lasts between the NPCs of one session: the kit lists the spell
/// stage filters in place (kit +0x230 and +0x208, 0x4df3ec), by kit.
#[derive(Debug, Default)]
pub(crate) struct Session {
    kit_lists: Mutex<HashMap<usize, KitLists>>,
}

/// A kit's lists +0x230 (minor spheres added) and +0x208 (major spheres
/// to pick from) as the spell stage left them.
type KitLists = (Vec<String>, Vec<String>);

/// A group member's race and classes, and the message the original shows.
type Pick = (String, Vec<(String, i32)>, Option<String>);

impl Session {
    fn kit_lists(&self, kit: usize) -> Option<KitLists> {
        self.kit_lists
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&kit)
            .cloned()
    }

    fn set_kit_lists(&self, kit: usize, lists: KitLists) {
        self.kit_lists
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(kit, lists);
    }
}

/// Ability scores as the generator rolls them; the "Ability Scores…"
/// button rolls them alone and Create NPC can then keep them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Abilities {
    /// Str, Dex, Con, Int, Wis, Cha.
    pub scores: [i32; 6],
    /// 1 to 100 for 18/01 to 18/00, else 0.
    pub exceptional: i32,
    /// Player's Option subabilities per ability: (score + d, score - d),
    /// that is muscle/stamina, aim/balance, health/fitness,
    /// reason/knowledge, intuition/willpower, leadership/appearance.
    pub subabilities: [[i32; 2]; 6],
}

/// Choices of the Create NPC dialog.
#[derive(Debug, Clone, Default)]
pub struct NpcOptions {
    /// Player's Option (Skills & Powers) rules.
    pub players_option: bool,
    /// Scores to keep instead of rolling new ones (what "Ability
    /// Scores…" rolled).
    pub abilities: Option<Abilities>,
}

/// A generated NPC.
#[derive(Debug, Clone, PartialEq)]
pub struct Npc {
    /// The character, ready to save like a hand-made one.
    pub character: Character,
    /// Classes and levels it was made with.
    pub classes: Vec<(String, i32)>,
    /// Maximum hit points of the rolls (0x42e0ab).
    pub hit_points: i32,
    /// Where the generator took a fallback for an undecoded path, and the
    /// original's own messages.
    pub messages: Vec<String>,
}

/// The NPCs of one line of an encounter template.
#[derive(Debug, Clone, PartialEq)]
pub struct EncounterNpc {
    pub character: Character,
    pub count: i32,
    /// Maximum hit points of each individual.
    pub hit_points: Vec<i32>,
    pub messages: Vec<String>,
}

/// Choices of the Create Multiple NPCs dialog (the combination branch:
/// the classes come from `AllowedU.dat`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupOptions {
    /// How many NPCs, low to high.
    pub number: (i32, i32),
    /// A race or subrace name; `None` is "~Random".
    pub race: Option<String>,
    /// Level ranges per class group: warrior, rogue, priest, wizard,
    /// psionicist.
    pub levels: [(i32, i32); 5],
    pub players_option: bool,
}

impl Default for GroupOptions {
    fn default() -> Self {
        Self {
            number: (1, 1),
            race: None,
            levels: [(1, 1); 5],
            players_option: false,
        }
    }
}

/// Everything the NPC generator reads, and its session state.
#[derive(Debug)]
pub struct NpcRules {
    data: Data,
    parts: PartTable,
    flavor: Flavor,
    session: Session,
}

const PO_FALLBACK: &str = "Player's Option proficiencies, equipment and later stages are not decoded (0x4d4a42 and on); the Core Rules paths were used.";
const EV_FALLBACK: &str = "Core Rules 2.0 generator: the name and abilities follow its own code (0x53a700); the later stages are the Expansion's.";

impl Installation {
    /// Loads the NPC generator's data.
    pub fn npc_rules(&self) -> Result<NpcRules> {
        Ok(NpcRules {
            data: Data::load(self)?,
            parts: self.part_table()?,
            flavor: flavor_of(self),
            session: Session::default(),
        })
    }

    /// Saves a generated NPC to the character list as an NPC (in `party`,
    /// which may be empty); returns its file name inside `Chars/`.
    pub fn save_npc(&self, npc: &Npc, party: &str) -> Result<String> {
        let format = |path: &str| {
            let path = path.to_owned();
            move |source| Error::Format {
                path: path.into(),
                source,
            }
        };
        let bytes = npc.character.to_bytes().map_err(format("NPC"))?;
        let chars = match self.path("Chars") {
            Some(dir) => dir,
            None => {
                let dir = self.root().join("Chars");
                fs::create_dir_all(&dir).map_err(|source| Error::Io {
                    path: dir.clone(),
                    source,
                })?;
                dir
            }
        };
        let mut index = self.characters()?;
        if index.schema == 0 {
            index.schema = INDEX_SCHEMA;
        }
        let file = new_file_name(&chars, &index);
        index.entries.push(npc_entry(npc, party, &file));
        let index_bytes = index.to_bytes().map_err(format("Chars/index.chr"))?;
        CharacterIndex::parse(&index_bytes).map_err(format("Chars/index.chr"))?;
        write_new(&chars.join(&file), &bytes)?;
        match self.path("Chars/index.chr") {
            Some(path) => replace(&path, &index_bytes)?,
            None => write_new(&chars.join("index.chr"), &index_bytes)?,
        }
        Ok(file)
    }
}

/// The character list entry of an NPC (+0x24 set, the party in +0x2c).
fn npc_entry(npc: &Npc, party: &str, file: &str) -> CharacterEntry {
    let c = &npc.character;
    let levels = npc
        .classes
        .iter()
        .map(|(_, l)| l.to_string())
        .collect::<Vec<_>>()
        .join("/");
    let race = format!("Race: {}", c.data.race);
    let class = format!("{CLASS_LINE}{}", c.data.class_title);
    let level = format!("{LEVEL_LINE}{levels}");
    if c.version == CORE20_VERSION {
        CharacterEntry {
            name: c.data.name.clone(),
            file: file.into(),
            race: c.data.race.clone(),
            version: 0,
            extra: None,
            unknown_0x24: 1,
            flag_0x28: 1,
            details: vec!["Core Rules".into(), race, class, level],
        }
    } else {
        CharacterEntry {
            name: c.data.name.clone(),
            file: file.into(),
            race: c.data.race.clone(),
            version: 3,
            extra: Some(EntryExtra {
                unknown: 8,
                unknown_0x2c: party.into(),
            }),
            unknown_0x24: 1,
            flag_0x28: 1,
            details: vec![
                "Core Rules".into(),
                PLAYER_LINE.into(),
                format!("Party: {party}"),
                race,
                class,
                level,
                format!("Kit: {}", c.data.kit),
            ],
        }
    }
}

impl NpcRules {
    /// Race and subrace names, in database order.
    pub fn races(&self) -> Vec<String> {
        self.data.races.iter().map(|r| r.name.clone()).collect()
    }

    /// Class names, in database order.
    pub fn classes(&self) -> Vec<String> {
        self.data.classes.iter().map(|c| c.name.clone()).collect()
    }

    pub fn flavor(&self) -> Flavor {
        self.flavor
    }

    fn version(&self) -> &'static str {
        match self.flavor {
            Flavor::Core20 => CORE20_VERSION,
            Flavor::Expansion => EXPANSION_VERSION,
        }
    }

    fn start<'a>(
        &'a self,
        rand: &'a mut Rand,
        race: &str,
        classes: &[(String, i32)],
        options: &NpcOptions,
    ) -> Result<Gen<'a>> {
        let mut g = Gen::new(
            &self.data,
            rand,
            &self.session,
            race,
            classes,
            options.players_option,
        )?;
        if self.flavor == Flavor::Core20 {
            g.warn(EV_FALLBACK);
        }
        Ok(g)
    }

    /// The constructor, name and abilities (or the kept ones).
    fn front(&self, g: &mut Gen, options: &NpcOptions) {
        g.identity();
        match options.abilities {
            Some(a) => {
                g.ab = a.scores;
                g.exc = a.exceptional;
                g.subs = a.subabilities;
            }
            None if self.flavor == Flavor::Core20 => g.ev_roll_abilities(),
            None => g.roll_abilities(),
        }
    }

    fn stages(g: &mut Gen) {
        if g.po {
            g.warn(PO_FALLBACK);
        }
        g.st_alignment();
        g.st_kit();
        g.st_social();
        g.st_background();
        g.st_physical();
        g.st_money();
        g.st_nwp();
        g.st_wp();
        g.st_lang2();
        g.st_hp();
        g.st_equip();
        g.st_spells();
        g.st_psionics();
    }

    fn finish(&self, mut g: Gen, classes: &[(String, i32)]) -> Result<Npc> {
        let hit_points = g.max_hp();
        let character = g.character(&self.parts, self.version())?;
        Ok(Npc {
            character,
            classes: classes.to_vec(),
            hit_points,
            messages: g.messages,
        })
    }

    /// Create NPC (0x4bf418) for a race (or subrace) and classes with
    /// their levels.
    pub fn generate(
        &self,
        rand: &mut Rand,
        race: &str,
        classes: &[(String, i32)],
        options: &NpcOptions,
    ) -> Result<Npc> {
        let mut g = self.start(rand, race, classes, options)?;
        self.front(&mut g, options);
        Self::stages(&mut g);
        self.finish(g, classes)
    }

    /// Ability Scores… : the generator up to the abilities (constructor,
    /// name, abilities). [`generate`](Self::generate) with these in
    /// [`NpcOptions::abilities`] keeps them.
    pub fn roll_abilities_only(
        &self,
        rand: &mut Rand,
        race: &str,
        classes: &[(String, i32)],
        options: &NpcOptions,
    ) -> Result<Abilities> {
        let mut g = self.start(rand, race, classes, options)?;
        let options = NpcOptions {
            abilities: None,
            ..options.clone()
        };
        self.front(&mut g, &options);
        Ok(Abilities {
            scores: g.ab,
            exceptional: g.exc,
            subabilities: g.subs,
        })
    }

    /// One NPC of an encounter template line (0x65351a and 0x41adb5):
    /// the target document, the generator, then `count` (rolled after the
    /// character for "and"/"or" lines) and every individual's hit points.
    pub fn encounter_npc(
        &self,
        rand: &mut Rand,
        race: &str,
        classes: &[(String, i32)],
        count: impl FnOnce(&mut Rand) -> i32,
    ) -> Result<EncounterNpc> {
        let options = NpcOptions::default();
        let mut g = self.start(rand, race, classes, &options)?;
        // CCharacterDocNew of the package (0x59c290).
        g.char_ctor();
        self.front(&mut g, &options);
        Self::stages(&mut g);
        let n = count(g.r).max(1);
        let mut hit_points = Vec::new();
        for _ in 0..n {
            // 0x42d888 then 0x42e0ab per individual; the character keeps
            // the last rolls.
            g.st_hp();
            hit_points.push(g.max_hp());
        }
        let character = g.character(&self.parts, self.version())?;
        Ok(EncounterNpc {
            character,
            count: n,
            hit_points,
            messages: g.messages,
        })
    }

    /// Create Multiple NPCs (0x4e3480) for one group line.
    pub fn generate_group(&self, rand: &mut Rand, options: &GroupOptions) -> Result<Vec<Npc>> {
        let (lo, hi) = options.number;
        let n = if lo <= hi && hi >= 1 {
            between(rand, lo, hi)
        } else {
            1
        };
        let mut out = Vec::new();
        for _ in 0..n {
            // The target document (0x59c290).
            between(rand, 0, 3);
            between(rand, 0, 3);
            let (race, classes, message) = self.group_pick(rand, options)?;
            let npc_options = NpcOptions {
                players_option: options.players_option,
                abilities: None,
            };
            let mut npc = self.generate(rand, &race, &classes, &npc_options)?;
            if let Some(m) = message {
                npc.messages.insert(0, m);
            }
            out.push(npc);
        }
        Ok(out)
    }

    /// 0x4be9c0: the race (0x4d1f99) and an `AllowedU.dat` combination
    /// with levels (0x4d21c3).
    fn group_pick(&self, rand: &mut Rand, options: &GroupOptions) -> Result<Pick> {
        let d = &self.data;
        if d.races.is_empty() || d.classes.is_empty() {
            return Err(Error::Invalid(
                "the databases have no races or classes".into(),
            ));
        }
        let mut r = match &options.race {
            None => &d.races[between(rand, 0, d.races.len() as i32 - 1) as usize],
            Some(name) => d
                .race(name)
                .map(|i| &d.races[i])
                .ok_or_else(|| Error::Invalid(format!("no race named {name}")))?,
        };
        if r.unknown_0x184 != 0
            && let Some(i) = d.race(&r.unknown_0x188)
        {
            r = &d.races[i];
        }
        let Some(combos) = d.allowed.get(&r.name).filter(|c| !c.is_empty()) else {
            return Ok((
                r.name.clone(),
                vec![(d.classes[0].name.clone(), 1)],
                Some(format!(
                    "Race/Class combination information unavailable for race: {}",
                    r.name
                )),
            ));
        };
        let h = &combos[between(rand, 0, combos.len() as i32 - 1) as usize];
        let mut out: Vec<(String, i32)> = Vec::new();
        let mut lvl = 1;
        for (name, &(lo, hi)) in h.iter().zip(&options.levels) {
            if name.is_empty() {
                continue;
            }
            if out.is_empty() {
                if lo <= hi && hi > 0 {
                    lvl = between(rand, lo, hi);
                }
            } else if hi < lo || hi < 1 {
                lvl = 1;
            } else if !(lo <= lvl && lvl <= hi) {
                lvl = between(rand, lo, hi);
            }
            out.push((name.clone(), lvl));
        }
        Ok((r.name.clone(), out, None))
    }
}

/// `between(lo, hi)` (0x554d44).
fn between(rand: &mut Rand, lo: i32, hi: i32) -> i32 {
    let n = hi.wrapping_add(1).wrapping_sub(lo);
    if n < 1 {
        return 0;
    }
    (rand.rand() % n).wrapping_add(lo)
}

#[cfg(test)]
mod tests {
    use cr_formats::race::Race;
    use cr_formats::{ArchiveReader, Field};

    use super::data::Klass;
    use super::state::{It, inv_add};
    use super::*;

    fn race(name: &str) -> Race {
        let zeros = vec![0u8; 1 << 14];
        let mut r = Race::read(&mut ArchiveReader::new(&zeros)).unwrap();
        r.name = name.into();
        r.ability_limits = [[3, 18]; 6];
        r.physical = [60, 59, 2, 10, 140, 100, 6, 10, 15, 1, 4];
        r.movement = 12;
        r.languages = vec!["Common".into()];
        r.unknown_0x1e0 = [1, 1, 0];
        r
    }

    fn rules() -> NpcRules {
        let mut data = Data::empty();
        data.races = vec![race("Human"), race("Gnome")];
        let common = [
            (0x84, 10),
            (0x88, 9),
            (0x8c, 3),
            (0xc0, 3),
            (0xc4, 3),
            (0xd0, 2),
            (0xd4, 4),
            (0xd8, 3),
        ];
        let alignments: &[&str] = &["Lawful Good", "True Neutral"];
        let mut fighter = Klass::synthetic("Fighter", 0, &common, &[], &[(0xa0, alignments)]);
        fighter.c8 = 1;
        let mut mage = Klass::synthetic("Mage", 3, &common, &[], &[(0xa0, alignments)]);
        mage.wizard_caster = true;
        mage.slots = vec![[0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0]; 21];
        data.classes = vec![fighter, mage];
        NpcRules {
            data,
            parts: PartTable::default(),
            flavor: Flavor::Expansion,
            session: Session::default(),
        }
    }

    fn fighter(level: i32) -> Vec<(String, i32)> {
        vec![("Fighter".into(), level)]
    }

    #[test]
    fn generation_is_reproducible_and_saves_as_an_npc() {
        let rules = rules();
        let options = NpcOptions::default();
        let a = rules
            .generate(&mut Rand::seeded(42), "Human", &fighter(4), &options)
            .unwrap();
        let b = rules
            .generate(&mut Rand::seeded(42), "Human", &fighter(4), &options)
            .unwrap();
        assert_eq!(a, b);
        let d = &a.character.data;
        assert!(!d.name.is_empty());
        assert_eq!(d.race, "Human");
        assert_eq!(d.kit, "None");
        assert_eq!(d.unknown_0x1da4, 1);
        assert_eq!(d.class_title, "Fighter");
        assert_eq!(d.unknown_0x1618[0], 4000);
        assert!(["Lawful Good", "True Neutral"].contains(&d.alignment.as_str()));
        assert_eq!(d.unknown_0xc60[0].iter().filter(|&&h| h > 0).count(), 4);
        assert!(a.hit_points >= 4);
        let bytes = a.character.to_bytes().unwrap();
        assert_eq!(Character::parse(&bytes).unwrap(), a.character);
    }

    #[test]
    fn kept_abilities_are_not_rolled_again() {
        let rules = rules();
        let mut rand = Rand::seeded(5);
        let options = NpcOptions::default();
        let abilities = rules
            .roll_abilities_only(&mut rand, "Human", &fighter(1), &options)
            .unwrap();
        // Constructor (2 draws), name (1), 6 × 9 dice and the class bonus.
        assert_eq!(rand.calls(), 2 + 1 + 54 + 1);
        let keep = NpcOptions {
            abilities: Some(abilities),
            ..options
        };
        let npc = rules
            .generate(&mut rand, "Human", &fighter(1), &keep)
            .unwrap();
        let d = &npc.character.data;
        assert_eq!(d.strength[0] as i32, abilities.scores[0]);
        let rest: Vec<i32> = d.abilities.iter().map(|a| a[0] as i32).collect();
        assert_eq!(rest, abilities.scores[1..]);
    }

    #[test]
    fn undecoded_paths_fall_back_with_a_message() {
        let rules = rules();
        let npc = rules
            .generate(
                &mut Rand::seeded(1),
                "Human",
                &[("Mage".into(), 3)],
                &NpcOptions::default(),
            )
            .unwrap();
        assert!(npc.messages.iter().any(|m| m.contains("Spellbook")));
        let po = NpcOptions {
            players_option: true,
            abilities: None,
        };
        let npc = rules
            .generate(&mut Rand::seeded(1), "Human", &fighter(2), &po)
            .unwrap();
        assert!(npc.messages.iter().any(|m| m.contains("Player's Option")));
    }

    #[test]
    fn unknown_names_are_errors_before_any_draw() {
        let rules = rules();
        let mut rand = Rand::seeded(1);
        let options = NpcOptions::default();
        assert!(
            rules
                .generate(&mut rand, "Human", &[("Bard".into(), 1)], &options)
                .is_err()
        );
        assert!(
            rules
                .generate(&mut rand, "Elf", &fighter(1), &options)
                .is_err()
        );
        assert!(rules.generate(&mut rand, "Human", &[], &options).is_err());
        assert_eq!(rand.calls(), 0);
    }

    #[test]
    fn groups_without_combinations_use_the_first_class() {
        let rules = rules();
        let options = GroupOptions {
            number: (2, 2),
            race: Some("Human".into()),
            ..GroupOptions::default()
        };
        let npcs = rules
            .generate_group(&mut Rand::seeded(3), &options)
            .unwrap();
        assert_eq!(npcs.len(), 2);
        for npc in &npcs {
            assert_eq!(npc.classes, fighter(1));
            assert_eq!(
                npc.messages[0],
                "Race/Class combination information unavailable for race: Human"
            );
        }
    }

    #[test]
    fn encounter_npcs_roll_their_number_after_the_character() {
        let rules = rules();
        let mut rand = Rand::seeded(9);
        let mut drawn = 0;
        let made = rules
            .encounter_npc(&mut rand, "Human", &fighter(2), |r| {
                drawn = r.calls();
                3
            })
            .unwrap();
        assert!(drawn > 2);
        assert_eq!(made.count, 3);
        assert_eq!(made.hit_points.len(), 3);
        assert!(made.hit_points.iter().all(|&h| h >= 2));
    }

    #[test]
    fn gnome_names_are_fixed() {
        let rules = rules();
        let mut rand = Rand::seeded(11);
        let mut g = Gen::new(
            &rules.data,
            &mut rand,
            &rules.session,
            "Gnome",
            &fighter(1),
            false,
        )
        .unwrap();
        let male = g.make_name("Gnome", true);
        let female = g.make_name("Gnome", false);
        assert!(["Norm", "David"].contains(&male.as_str()));
        assert!(["Norma", "Louise"].contains(&female.as_str()));
        assert_eq!(rand.calls(), 2);
    }

    #[test]
    fn items_merge_and_stay_sorted() {
        let mut list = Vec::new();
        inv_add(&mut list, It::new("Torch", 2), true);
        inv_add(&mut list, It::new("Backpack", 1), true);
        inv_add(&mut list, It::new("Torch", 3), true);
        let mut book = It::new("Spellbook", 1);
        book.bundle = Some(Vec::new());
        inv_add(&mut list, book.clone(), true);
        inv_add(&mut list, book, true);
        let names: Vec<(&str, i32)> = list.iter().map(|i| (i.name.as_str(), i.n)).collect();
        assert_eq!(
            names,
            [
                ("Backpack", 1),
                ("Spellbook", 1),
                ("Spellbook", 1),
                ("Torch", 5)
            ]
        );
    }

    #[test]
    fn kit_edits_last_for_the_session() {
        let s = Session::default();
        assert_eq!(s.kit_lists(3), None);
        s.set_kit_lists(3, (vec!["Animal".into()], Vec::new()));
        assert_eq!(s.kit_lists(3), Some((vec!["Animal".into()], Vec::new())));
    }
}
