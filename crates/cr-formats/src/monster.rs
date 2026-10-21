//! Monsters: `Database/Monster.dat` (`CMonsterOb`), `Database/Attack.dat`
//! (`CMonsterAttackOb`) and `Database/MonType.dat` (`CMonsterTypeOb`), each
//! a `CObList` read with [`ObjectList`](crate::database::ObjectList), plus
//! the matching empty `UserDbas/*U.dat` files.
//!
//! None of these classes reads a version. Members that are C++ sub-objects
//! (the arsenal, the inventory) are written inline by their own `Serialize`
//! with no class tag. Unnamed fields keep their member offset.

use crate::inventory::InventoryBundle;
use crate::{Object, record};

record! {
    /// One monster (`CMonsterOb`, schema 88, size 0x4e8, Serialize
    /// 0x67dde6). Examples are Aboleth and "Ankheg, 5 Hit Dice".
    pub struct Monster {
        /// "Aboleth"; unique in Monster.dat.
        name: String,
        /// "Description" (a placeholder) in 1502 records.
        description: String,
        /// Monster-type picture number (Aboleth 6 = MonType "6.bmp").
        picture: i32,
        /// Always equal to `picture` in the shipped data.
        unknown_0x3b8: i32,
        /// Frequency in percent: 65 common, 20 uncommon, 11 rare, 4 very
        /// rare.
        frequency: i32,
        /// "Solitary", "Tribal".
        organization: String,
        /// "Any", "Day", "Night".
        activity_cycle: String,
        /// "Omnivores", "Carnivores".
        diet: String,
        /// "13-14  Highly intelligent".
        intelligence: String,
        /// Number appearing, low end (Aboleth 1).
        number_appearing_min: i32,
        /// Number appearing, high end (Aboleth 4, Ankheg 6).
        number_appearing_max: i32,
        /// Aboleth 4, Ankheg 2; see `armor_class_notes`.
        armor_class: i32,
        /// Base movement rate (Aboleth 3, Ankheg 12).
        movement: i32,
        /// Maneuverability class "A" to "E" for `fly_rate`.
        fly_maneuverability: String,
        /// 30, 24, 18.
        fly_rate: i32,
        /// Always empty.
        unknown_0x414: String,
        /// Aboleth 18.
        swim_rate: i32,
        /// Always empty.
        unknown_0x41c: String,
        /// Ankheg 6.
        burrow_rate: i32,
        /// Always empty.
        unknown_0x424: String,
        /// A further movement rate, mostly 0.
        unknown_0x428: i32,
        /// Always empty.
        unknown_0x42c: String,
        /// A further movement rate, mostly 0.
        unknown_0x430: i32,
        /// Aboleth 8, Ankheg 5.
        hit_dice: i32,
        /// Added to the hit dice (a bonus or extra).
        unknown_0x438: i32,
        /// 8 in 1500 records.
        hit_die_size: i32,
        /// "Damage" (a placeholder).
        unknown_0x440: String,
        /// Always empty.
        unknown_0x444: String,
        /// Special-defense text ("Immune to cold.").
        defense_notes: String,
        /// Always empty.
        unknown_0x464: String,
        /// Percent: 0, 20, 30, 50.
        magic_resistance: i32,
        /// Aboleth 13, Ankheg 9.
        morale: i32,
        /// Aboleth 5000, "Ankheg, 5 Hit Dice" 420.
        experience: i32,
        /// Always 0.
        unknown_0x478: i32,
        /// Aboleth 13, "Ankheg, 5 Hit Dice" 15.
        thac0: i32,
        /// Usually equal to `morale`.
        unknown_0x470: i32,
        /// Mostly 100.
        unknown_0x3f8: i32,
        /// Mostly 100.
        unknown_0x3fc: i32,
        /// Always 100.
        unknown_0x400: i32,
        /// Always 100.
        unknown_0x404: i32,
        /// "Ankheg", "Fish", "Orc".
        monster_type: String,
        /// "Lawful Evil".
        alignment: String,
        /// "Any Land", "~Custom".
        climate_terrain: String,
        /// "String" (a placeholder).
        unknown_0x33c: String,
        /// "String" (a placeholder).
        unknown_0x340: String,
        /// "String" (a placeholder).
        unknown_0x344: String,
        /// Long description of special abilities.
        notes: String,
        /// 0/1 flags at +0x47c to +0x490.
        unknown_0x47c: [i32; 6],
        /// 0/1 flags at +0x44c to +0x460.
        unknown_0x44c: [i32; 6],
        /// 150 values, mostly 0/1 (Aboleth: indices 2 and 4 to 36 set).
        unknown_0x4: [i32; 150],
        /// 26 pairs (+0x25c[i], +0x2c4[i]), read interleaved.
        unknown_0x25c: [(i32, i32); 26],
        /// Qualifiers for `armor_class` ("Overall", "Underside", "", "",
        /// ""); 0 or 5 entries.
        armor_class_notes: Vec<String>,
        /// `CMonsterAttackOb`s; empty in all shipped data.
        unknown_0x4c0: Vec<Object>,
        /// `CSpellsOb`s (257 in Monster.dat).
        spells: Vec<Object>,
        arsenal: Arsenal,
        /// Carried items.
        inventory: InventoryBundle,
    }
}

record! {
    /// `CMonsterArsenal` (size 0x28, Serialize 0x68a30a), only ever
    /// embedded in a [`Monster`] (+0x494).
    pub struct Arsenal {
        /// "1-6 (x4)", "3-18 (crush) + 1-4 (acid)".
        damage: String,
        /// Number of attacks ("4", "1 or 2").
        attacks: String,
        /// "Psionics, enslavement, tentacles".
        special_attacks: String,
        /// "Slime", "+1 or better weapon to hit".
        special_defenses: String,
        /// `CMonsterAttackOb`s (107 in Monster.dat).
        attack_list: Vec<Object>,
    }
}

record! {
    /// One attack (`CMonsterAttackOb`, schema 88, size 0x48, Serialize
    /// 0x6889ac), edited by the Monster Attack dialog (IDD 269). All
    /// integers but `bonus_hit_dice` are 0 in the shipped data.
    #[derive(Default)]
    pub struct MonsterAttack {
        /// "Blood drain", "Employs psionic devotions".
        name: String,
        /// No dialog shows it; empty in the shipped data.
        unknown_0xc: String,
        /// "Help ID".
        help_id: i32,
        /// Damage dice: how many, their sides and the bonus.
        damage_dice: i32,
        damage_sides: i32,
        damage_bonus: i32,
        /// 0 Single Target, 1 Cone, 2 Beam, 3 Rectangle, 4 Circle.
        area: i32,
        /// Feet.
        range: i32,
        /// The start width of a cone, the width of a beam or rectangle,
        /// the diameter of a circle, in feet.
        width: i32,
        /// The end width of a cone, in feet.
        end_width: i32,
        /// The length of a cone, beam or rectangle, in feet.
        length: i32,
        /// 0 None, then Paralyzation, Poison, Death Magic, Rod, Staff,
        /// Wand, Petrification, Polymorph, Breath Weapon, Spell.
        save_against: i32,
        /// What a save gains: 0 Not Applicable, 1 Half Damage, 2 Negate
        /// Effect.
        save_result: i32,
        /// The roll to hit: 0 Normal AC, 1 Automatically Hits, 2 AC
        /// Excluding Armor, 10 Armor Class 10.
        to_hit: i32,
        /// What the saving throw is modified by.
        save_modifier: i32,
        /// Hit dice the attack adds for experience: 1 to 3 in the shipped
        /// data ("Energy drain" 3).
        bonus_hit_dice: i32,
        /// "Special Effect".
        special_effect: String,
    }
}

record! {
    /// One monster type (`CMonsterTypeOb`, schema 95, size 0x20, Serialize
    /// 0x6890dd). Also embedded in encounter templates.
    pub struct MonsterType {
        /// "Aarakocra", "Aboleth".
        name: String,
        /// Bitmap of the picture page ("5to34.bmp"); empty in the types
        /// the original's editor writes.
        page_picture: String,
        /// Empty in the shipped data. The original's type editor writes the
        /// file name of `picture` here ("teste.bmp").
        unknown_0x14: String,
        /// Empty in the shipped data. The original's type editor writes the
        /// folder of `picture` here, with its last backslash
        /// ("C:\Program Files\AD&D Core Rules 2.0\Userbmp\").
        unknown_0x10: String,
        /// 0 to 12.
        unknown_0x18: i32,
        /// 0 or small.
        unknown_0x1c: i32,
        /// Picture bitmap ("6.bmp"); the original's type editor writes the
        /// whole path ("C:\Program Files\AD&D Core Rules 2.0\Userbmp\teste.bmp").
        picture: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArchiveReader, ArchiveWriter, Field};

    #[test]
    fn monster_type_round_trips() {
        let t = MonsterType {
            name: "Aboleth".into(),
            page_picture: "5to34.bmp".into(),
            unknown_0x14: String::new(),
            unknown_0x10: String::new(),
            unknown_0x18: 3,
            unknown_0x1c: 0,
            picture: "6.bmp".into(),
        };
        let mut w = ArchiveWriter::new();
        t.write(&mut w).unwrap();
        let bytes = w.into_bytes();
        // Strings with a length byte each, then two 4-byte integers.
        assert_eq!(bytes.len(), 8 + 10 + 1 + 1 + 4 + 4 + 6);
        let mut r = ArchiveReader::new(&bytes);
        assert_eq!(MonsterType::read(&mut r).unwrap(), t);
        assert!(r.is_at_end());
    }
}
