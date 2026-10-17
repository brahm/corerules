//! Races: `Database/Race.dat` and `UserDbas/RaceU.dat`, a `CObList` of
//! `CRaceOb` (schema 88, `CRaceOb::Serialize` at 0x686857 in TSRP3.EXE).
//!
//! Field names follow the evidence in the shipped data; fields without
//! enough evidence keep the member offset they have in the original object
//! (`unknown_0x2c` is the member at `this + 0x2c`), which is how
//! `docs/formats/race.md` refers to them as well.

use crate::Object;
use crate::record;

record! {
    /// One race (`CRaceOb`).
    pub struct Race {
        name: String,
        unknown_0x8: String,
        /// Minimum and maximum score per ability, in the order Str, Dex,
        /// Con, Int, Wis, Cha (Dwarf: 8/18, 3/17, 11/18, 3/18, 3/18, 3/17).
        ability_limits: [[i32; 2]; 6],
        /// Ability adjustments in the same order (Dwarf: Con +1, Cha -1).
        ability_adjustments: [i32; 6],
        /// Maximum level per class: Fighter, Ranger, Paladin, Thief, Bard,
        /// Cleric, Druid, Mage, specialist wizard (Dwarf: 15, -, -, 12, -,
        /// 10; Elf: ranger 15, no paladin). 0 is both "not allowed" and,
        /// for humans and the half-elf bard, "unlimited".
        level_limits: [i32; 9],
        /// Skills & Powers character points for racial abilities (Dwarf
        /// 45, Human 10, 0 for the monster races), and how many of them a
        /// character may keep for later steps (5 for demihumans, 10 for
        /// humans).
        unknown_0xa8: i32,
        unknown_0xac: i32,
        /// Database identifier (Dwarf 106, Elf 107, Human 119).
        id: i32,
        unknown_0x244: [i32; 5],
        unknown_0xdc: String,
        unknown_0xe0: i32,
        /// Rules text shown on the race page.
        description: String,
        unknown_0x188: String,
        unknown_0x2ac: String,
        unknown_0x16c: [i32; 2],
        /// Base movement rate (Dwarf 6, Human 12).
        movement: i32,
        unknown_0x178: [i32; 2],
        /// A dice expression, "1d2" in every shipped race.
        unknown_0x180: String,
        unknown_0x40: i32,
        /// Thief skill adjustments: pick pockets, open locks, find/remove
        /// traps, move silently, hide in shadows, detect noise, climb
        /// walls, read languages (Dwarf: 0, +10, +15, 0, 0, 0, -10, -5).
        thief_adjustments: [i32; 8],
        unknown_0x158: [i32; 5],
        unknown_0x18c: i32,
        unknown_0x1e0: [i32; 3],
        unknown_0x5c: i32,
        unknown_0x21c: [i32; 5],
        unknown_0x230: [String; 5],
        /// Height, weight and starting age: male and female base height,
        /// height dice (count, sides), male and female base weight, weight
        /// dice, base age, age dice (Dwarf: 43/41 + 1d10 in, 130/105 +
        /// 4d10 lb, 40 + 5d6 years).
        physical: [i32; 11],
        unknown_0x258: [i32; 10],
        /// Default portrait file and the folder it was authored in.
        portrait: String,
        portrait_folder: String,
        unknown_0x280: [String; 11],
        unknown_0x2b0: [i32; 9],
        /// Evermore's "Unlimited" level-limit checks, in the Level Limits
        /// dialog's row order: Bard, Cleric, Druid, Fighter, Specialist,
        /// Mage, Paladin, Ranger, Thief.
        unknown_0x84: [i32; 9],
        unknown_0x184: i32,
        unknown_0x190: [i32; 10],
        /// Languages a character of this race knows: the race's own
        /// language for a main race, the whole package of a subrace, which
        /// a Skills & Powers character gets as native languages.
        languages: Vec<String>,
        unknown_0xf8: Vec<String>,
        /// Racial abilities a new character starts with: a subrace's
        /// package (Hill dwarf: infravision, melee combat, mining detection,
        /// saving throw bonuses), a monster race's fixed abilities. Both
        /// programs write them for Core Rules characters too.
        abilities: Vec<String>,
        unknown_0x1b8: Vec<String>,
        /// Allowed multi-class combinations (`CClassMultiClassOb`).
        multi_classes: Vec<Object>,
        /// Portrait files offered for this race.
        portraits: Vec<String>,
        unknown_0x208: Vec<String>,
        /// Nine further `CObArray`s, empty in every shipped race; their
        /// element class is not known yet.
        unknown_0x2d4: [Vec<Object>; 9],
    }
}

record! {
    /// One allowed multi-class combination (`CClassMultiClassOb`, schema
    /// 94): a class id per class group, 0 for none (warrior 1000s, wizard
    /// 2000s, priest 3000s, rogue 4000s, psionicist 5000s).
    pub struct MultiClass {
        classes: [i32; 5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArchiveReader, ArchiveWriter, Field};

    #[test]
    fn multi_class_round_trips() {
        let m = Object::MultiClass(MultiClass {
            classes: [1000, 0, 3000, 0, 0],
        });
        let mut w = ArchiveWriter::new();
        m.write(&mut w).unwrap();
        let bytes = w.into_bytes();
        assert_eq!(&bytes[..2], &[0xFF, 0xFF]);
        assert_eq!(Object::read(&mut ArchiveReader::new(&bytes)).unwrap(), m);
    }
}
