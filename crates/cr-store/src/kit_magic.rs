//! What a kit does to a character's magic in the Expansion's Update
//! Character (and New Character's Spells page): the schools of magic it
//! bars or makes the character eliminate, the spheres it limits or gives
//! for free, and its never using spells.
//!
//! * Opening the Spells or Memorize page (62f1c8, 63488f) removes the
//!   schools the kit bars (+0x1dc) from the wizard class's schools, after
//!   the kit's warning, which may be ignored for good (`flag_0x1ea8`).
//!   The app removes them when the edit is saved, unless ignored.
//! * A kit limited to five schools (+0x1fc) has the player eliminate the
//!   schools beyond five once (6309df: "roll %d times to eliminate %d more
//!   schools"); they are kept in `unknown_0x1e00` and removed from the
//!   class's schools.
//! * The Memorize page (63286d) keeps only the spheres the kit limits the
//!   class to (+0x21c), lets the player keep at most the kit's numbers of
//!   minor and major spheres (+0x200, +0x204), and has the player pick
//!   the kit's free major sphere (+0x208), kept in `unknown_0x39c`. The
//!   free minor spheres (+0x230) are only told about.
//! * A kit that never uses spells (+0x5d7) warns before spells are
//!   memorized, which may be ignored for good (`unknown_0x1ea4.0`).
//!
//! The Skills & Powers Schools and Spheres pages (613302, 614532) only
//! tell about these, as the app's Abilities page does.

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::parts::PartsFile;
use cr_i18n::{tr, trn};

use crate::choices::{Caster, caster};
use crate::npc::data::Kit;
use crate::{Error, Installation, Result};

/// The most spheres without a kit that limits them.
const ANY_NUMBER: i32 = 9999;

/// What a kit does to magic.
#[derive(Debug, Clone, Default)]
struct KitMagic {
    barred_schools: Vec<String>,
    five_schools: bool,
    /// The most minor and major spheres.
    most_spheres: [i32; 2],
    limited_spheres: Vec<String>,
    free_major: Vec<String>,
    free_minor: Vec<String>,
    never_spells: bool,
}

/// The rules, read from the databases.
#[derive(Debug, Clone, Default)]
pub struct KitMagicRules {
    kits: HashMap<String, KitMagic>,
    class_ids: HashMap<String, i32>,
}

/// The spheres of a class the kit has the player choose from, at most
/// `most` of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpherePick {
    pub slot: usize,
    pub major: bool,
    pub most: usize,
    /// The class's spheres the kit allows.
    pub names: Vec<String>,
}

/// A character's choices about the kit's magic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterKitMagic {
    /// The kit's barred schools stay (`flag_0x1ea8`).
    pub barred_ignored: bool,
    /// The schools eliminated (`unknown_0x1e00`).
    pub eliminated: Vec<String>,
    /// The spheres kept where there were more than the kit allows.
    pub spheres: Vec<(usize, bool, Vec<String>)>,
    /// The kit's free major sphere (`unknown_0x39c`).
    pub free_sphere: String,
    /// The kit's warning about spells was ignored (`unknown_0x1ea4.0`).
    pub spell_use_ignored: bool,
}

impl Installation {
    /// Every kit's members by name, the first of each name kept
    /// (`Database/Parts.dat`, then `UserDbas/PartsU.dat`).
    pub(crate) fn kit_parts(&self) -> Result<HashMap<String, Kit>> {
        let mut kits = HashMap::new();
        for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            for object in file.of_type("Character Kits").unwrap_or_default() {
                if let Object::Part(p) = object
                    && let Some(Some(kit)) = &p.data.kit
                    && let Object::PartKit(kit) = &**kit
                    && let Some(data) = &kit.data
                {
                    kits.entry(p.head.name.clone())
                        .or_insert_with(|| Kit::of(data));
                }
            }
        }
        Ok(kits)
    }

    pub fn kit_magic_rules(&self) -> Result<KitMagicRules> {
        let kits = self
            .kit_parts()?
            .into_iter()
            .map(|(name, kit)| {
                let most = |off| match kit.i_or(off, ANY_NUMBER) {
                    n if n <= 0 => ANY_NUMBER,
                    n => n,
                };
                let magic = KitMagic {
                    barred_schools: kit.sa(0x1dc).to_vec(),
                    five_schools: kit.flag(0x1fc),
                    most_spheres: [most(0x200), most(0x204)],
                    limited_spheres: kit.sa(0x21c).to_vec(),
                    free_major: kit.sa(0x208).to_vec(),
                    free_minor: kit.sa(0x230).to_vec(),
                    never_spells: kit.flag(0x5d7),
                };
                (name, magic)
            })
            .collect();
        Ok(KitMagicRules {
            kits,
            class_ids: self.class_ids()?,
        })
    }
}

impl KitMagicRules {
    fn kit(&self, name: &str) -> Option<&KitMagic> {
        self.kits.get(name)
    }

    fn caster(&self, class: &str) -> Caster {
        self.class_ids
            .get(class)
            .copied()
            .map_or(Caster::None, caster)
    }

    /// The class slots that cast wizard or priest spells.
    fn slots(&self, character: &Character, wanted: Caster) -> Vec<usize> {
        character
            .data
            .classes
            .names
            .iter()
            .enumerate()
            .take(5)
            .filter(|(_, c)| self.caster(c) == wanted)
            .map(|(slot, _)| slot)
            .collect()
    }

    pub fn magic_of(&self, character: &Character) -> CharacterKitMagic {
        let d = &character.data;
        CharacterKitMagic {
            barred_ignored: d.flag_0x1ea8.is_some_and(|f| f != 0),
            eliminated: d.unknown_0x1e00.clone().unwrap_or_default(),
            spheres: Vec::new(),
            free_sphere: d.unknown_0x39c.clone().unwrap_or_default(),
            spell_use_ignored: d.unknown_0x1ea4.as_ref().is_some_and(|f| f.0 != 0),
        }
    }

    /// The kit's warning about the schools it bars that a wizard class of
    /// the character has, unless ignored (63056f).
    pub fn barred_schools_warning(&self, character: &Character, kit: &str) -> Option<String> {
        let magic = self.kit(kit)?;
        let had: Vec<&String> = self
            .slots(character, Caster::Wizard)
            .into_iter()
            .flat_map(|s| &character.data.class_slots[s].unknown_0x270)
            .filter(|s| magic.barred_schools.contains(s))
            .collect();
        (!had.is_empty()).then(|| barred_text(kit, &magic.barred_schools))
    }

    /// The schools a wizard class keeps once the barred ones go, unless
    /// ignored, and the eliminated ones.
    fn kept_schools(&self, schools: &[String], kit: &str, new: &CharacterKitMagic) -> Vec<String> {
        let barred = self.kit(kit).map_or(&[][..], |k| &k.barred_schools[..]);
        kept(schools, barred, new.barred_ignored, &new.eliminated)
    }

    /// How many schools beyond five the character eliminates, and the
    /// schools to eliminate them from, while it has eliminated none
    /// (6309df).
    pub fn schools_to_eliminate(
        &self,
        character: &Character,
        kit: &str,
        new: &CharacterKitMagic,
    ) -> Option<(usize, Vec<String>)> {
        if !self.kit(kit)?.five_schools || !self.magic_of(character).eliminated.is_empty() {
            return None;
        }
        let slot = *self.slots(character, Caster::Wizard).first()?;
        let without = CharacterKitMagic {
            eliminated: Vec::new(),
            ..new.clone()
        };
        let schools = self.kept_schools(
            &character.data.class_slots[slot].unknown_0x270,
            kit,
            &without,
        );
        let count = schools.len().checked_sub(5).filter(|&n| n > 0)?;
        Some((count, schools))
    }

    /// The spheres of each priest class the kit has the player choose
    /// from, where the class has more of those it allows than it may
    /// keep (63286d).
    pub fn sphere_picks(&self, character: &Character, kit: &str) -> Vec<SpherePick> {
        let Some(magic) = self.kit(kit) else {
            return Vec::new();
        };
        let mut picks = Vec::new();
        for slot in self.slots(character, Caster::Priest) {
            let s = &character.data.class_slots[slot];
            for (major, list) in [(false, &s.unknown_0x2d4), (true, &s.unknown_0x338)] {
                let names = limited(list, &magic.limited_spheres);
                let most = magic.most_spheres[usize::from(major)].max(0) as usize;
                if names.len() > most {
                    picks.push(SpherePick {
                        slot,
                        major,
                        most,
                        names,
                    });
                }
            }
        }
        picks
    }

    /// The spheres the kit may give for free, while the character has none
    /// of them: those it lists that are not already a priest class's major
    /// spheres, within its limited spheres.
    pub fn free_sphere_choices(&self, character: &Character, kit: &str) -> Vec<String> {
        let Some(magic) = self.kit(kit) else {
            return Vec::new();
        };
        if !self.magic_of(character).free_sphere.is_empty() {
            return Vec::new();
        }
        let priests = self.slots(character, Caster::Priest);
        let Some(&slot) = priests.first() else {
            return Vec::new();
        };
        let major = &character.data.class_slots[slot].unknown_0x338;
        limited(&magic.free_major, &magic.limited_spheres)
            .into_iter()
            .filter(|s| !major.contains(s))
            .collect()
    }

    /// What the player is told about eliminating `count` schools (6309df).
    pub fn elimination_text(kit: &str, count: usize) -> String {
        trn!(
            "Your kit \"{kit}\" can learn spells from only five schools.  To determine which \
             schools are unavailable, roll {n} time to eliminate {n} more school.",
            "Your kit \"{kit}\" can learn spells from only five schools.  To determine which \
             schools are unavailable, roll {n} times to eliminate {n} more schools.",
            count,
            kit
        )
    }

    /// What the player is told about keeping at most `most` spheres
    /// (63286d).
    pub fn sphere_pick_text(kit: &str, major: bool, most: usize) -> String {
        if major {
            tr!(
                "The kit \"{kit}\" requires that you select a maximum of {most} major spheres \
                 from the list below.",
                kit,
                most
            )
        } else {
            tr!(
                "The kit \"{kit}\" requires that you select a maximum of {most} minor spheres \
                 from the list below.",
                kit,
                most
            )
        }
    }

    /// What the player is told about the free major sphere (63286d).
    pub fn free_sphere_text(kit: &str) -> String {
        tr!(
            "The kit \"{kit}\" lets you pick one extra sphere for free.",
            kit
        )
    }

    /// The kit's warning before spells are memorized, unless ignored.
    pub fn spell_use_warning(&self, kit: &str, new: &CharacterKitMagic) -> Option<String> {
        (self.kit(kit)?.never_spells && !new.spell_use_ignored)
            .then(|| tr("The selected kit never uses spells.").to_owned())
    }

    /// What the Skills & Powers Schools page tells about the kit's
    /// schools (613302), unless the warning was ignored.
    pub fn schools_notice(&self, kit: &str, new: &CharacterKitMagic) -> Option<String> {
        let magic = self.kit(kit)?;
        if magic.barred_schools.is_empty() || new.barred_ignored {
            return None;
        }
        Some(tr!(
            "Your kit \"{kit}\" bars you from the following schools: {schools}. You should not \
             select these schools.",
            kit,
            schools = magic.barred_schools.join(", ")
        ))
    }

    /// What the Skills & Powers Spheres page tells about the kit's
    /// spheres (614532), for minor or major ones, with the count taken.
    pub fn spheres_notice(
        &self,
        kit: &str,
        new: &CharacterKitMagic,
        major: bool,
        taken: usize,
    ) -> Vec<String> {
        let Some(magic) = self.kit(kit) else {
            return Vec::new();
        };
        let mut notes = Vec::new();
        let most = magic.most_spheres[usize::from(major)];
        if most != ANY_NUMBER && taken as i64 > i64::from(most) {
            let over = taken as i64 - i64::from(most);
            notes.push(if major {
                tr!(
                    "Your kit \"{kit}\" specifies a maximum number of major spheres of {most}, \
                     however you have {taken} spheres selected. Please remove {over} major \
                     spheres.",
                    kit,
                    most,
                    taken,
                    over
                )
            } else {
                tr!(
                    "Your kit \"{kit}\" specifies a maximum number of minor spheres of {most}, \
                     however you have {taken} spheres selected. Please remove {over} minor \
                     spheres.",
                    kit,
                    most,
                    taken,
                    over
                )
            });
        }
        if !magic.limited_spheres.is_empty() {
            notes.push(tr!(
                "Your kit \"{kit}\" limits you to the following spheres: {spheres}. You should \
                 limit your selections to these spheres only.",
                kit,
                spheres = magic.limited_spheres.join(", ")
            ));
        }
        if !major && !magic.free_minor.is_empty() {
            notes.push(tr!(
                "Your kit \"{kit}\" gives you free minor access to the following spheres: \
                 {spheres}. You don't need to purchase minor access to these spheres.",
                kit,
                spheres = magic.free_minor.join(", ")
            ));
        }
        if major && !new.free_sphere.is_empty() {
            notes.push(tr!(
                "Your kit \"{kit}\" gives you free major access to the following sphere: \
                 {sphere}. You don't need to purchase major access to these spheres.",
                kit,
                sphere = new.free_sphere.as_str()
            ));
        }
        notes
    }

    /// Writes `new` into the character, the class lists trimmed as the
    /// Spells and Memorize pages trim them.
    pub fn apply(&self, character: &mut Character, new: &CharacterKitMagic) {
        let kit = character.data.kit.clone();
        let wizards = self.slots(character, Caster::Wizard);
        let priests = self.slots(character, Caster::Priest);
        let magic = self.kit(&kit).cloned().unwrap_or_default();
        let d = &mut character.data;
        for slot in wizards {
            let kept = self.kept_schools(&d.class_slots[slot].unknown_0x270, &kit, new);
            d.class_slots[slot].unknown_0x270 = kept;
        }
        for slot in priests {
            let s = &mut d.class_slots[slot];
            for (major, list) in [(false, &mut s.unknown_0x2d4), (true, &mut s.unknown_0x338)] {
                let mut kept = limited(list, &magic.limited_spheres);
                let most = magic.most_spheres[usize::from(major)].max(0) as usize;
                if kept.len() > most {
                    let chosen = new
                        .spheres
                        .iter()
                        .find(|(s, m, _)| *s == slot && *m == major)
                        .map(|p| &p.2);
                    if let Some(chosen) = chosen {
                        kept.retain(|s| chosen.contains(s));
                    }
                    kept.truncate(most);
                }
                *list = kept;
            }
        }
        if d.flag_0x1ea8.is_some() {
            d.flag_0x1ea8 = Some(u8::from(new.barred_ignored));
        }
        if d.unknown_0x1e00.is_some() {
            d.unknown_0x1e00 = Some(new.eliminated.clone());
        }
        if d.unknown_0x39c.is_some() {
            d.unknown_0x39c = Some(new.free_sphere.clone());
        }
        if let Some(flags) = &mut d.unknown_0x1ea4 {
            flags.0 = u8::from(new.spell_use_ignored);
        }
    }
}

/// The kit's warning about the schools it bars (63056f).
pub(crate) fn barred_text(kit: &str, barred: &[String]) -> String {
    let mut text = tr!(
        "The kit \"{kit}\" does not allow you to select schools from the following list.",
        kit
    );
    for school in barred {
        text.push('\n');
        text.push_str(school);
    }
    text
}

/// The schools of `schools` left once the barred ones go, unless ignored,
/// and the eliminated ones.
pub(crate) fn kept(
    schools: &[String],
    barred: &[String],
    ignored: bool,
    eliminated: &[String],
) -> Vec<String> {
    schools
        .iter()
        .filter(|s| ignored || !barred.contains(s))
        .filter(|s| !eliminated.contains(s))
        .cloned()
        .collect()
}

/// The spheres of `list` within `limit`, all of them when it is empty
/// (6719af).
fn limited(list: &[String], limit: &[String]) -> Vec<String> {
    list.iter()
        .filter(|s| limit.is_empty() || limit.contains(s))
        .cloned()
        .collect()
}
