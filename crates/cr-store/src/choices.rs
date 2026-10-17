//! The names a character's lists can hold, from the rules databases and
//! the user's own additions (`UserDbas/PartsU.dat`, `UserDbas/SpellsU.dat`).

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::classes::Class;
use cr_formats::database::ObjectList;
use cr_formats::parts::PartsFile;
use cr_formats::spells::SpellsFile;

use crate::{Error, Installation, Result};

impl Installation {
    /// Names of the parts of one type (a [`cr_formats::parts::PART_TYPES`]
    /// name such as "Languages"), sorted, without duplicates.
    pub fn part_names(&self, part_type: &str) -> Result<Vec<String>> {
        self.part_names_where(part_type, |_| true)
    }

    /// Names of the weapons a character can be proficient with: the
    /// Weapons parts that are not magical (part member 0x224).
    pub fn weapon_proficiency_names(&self) -> Result<Vec<String>> {
        self.part_names_where("Weapons", |p| p.head.unknown_0x224 == 0)
    }

    fn part_names_where(
        &self,
        part_type: &str,
        keep: impl Fn(&cr_formats::parts::Part) -> bool,
    ) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            names.extend(
                file.of_type(part_type)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|o| match o {
                        Object::Part(p) if !p.head.name.is_empty() && keep(p) => {
                            Some(p.head.name.clone())
                        }
                        _ => None,
                    }),
            );
        }
        Ok(sorted(names))
    }

    /// Names of the wizard or priest spells, sorted, without duplicates.
    pub fn spell_names(&self, wizard: bool) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for relative in ["Database/Spells.dat", "UserDbas/SpellsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file =
                SpellsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                    path: relative.into(),
                    source,
                })?;
            let spells = if wizard { &file.wizard } else { &file.priest };
            names.extend(spells.iter().filter_map(|o| match o {
                Object::Spell(s) if !s.name.is_empty() => Some(s.name.clone()),
                _ => None,
            }));
        }
        Ok(sorted(names))
    }
}

/// What kind of spells a class casts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Caster {
    Wizard,
    Priest,
    None,
}

/// The kind of spells the class with this database identifier casts:
/// wizards (4000s) and bards (2001) cast wizard spells; priests (3000s),
/// paladins (1001) and rangers (1002) priest spells.
pub fn caster(class_id: i32) -> Caster {
    match (class_id / 1000, class_id) {
        (4, _) | (_, 2001) => Caster::Wizard,
        (3, _) | (_, 1001 | 1002) => Caster::Priest,
        _ => Caster::None,
    }
}

impl Installation {
    /// Spell levels by name, wizard or priest.
    pub fn spell_levels(&self, wizard: bool) -> Result<HashMap<String, u32>> {
        let mut levels = HashMap::new();
        for relative in ["Database/Spells.dat", "UserDbas/SpellsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file =
                SpellsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                    path: relative.into(),
                    source,
                })?;
            for o in if wizard { &file.wizard } else { &file.priest } {
                if let Object::Spell(s) = o {
                    levels
                        .entry(s.name.clone())
                        .or_insert(s.level.max(0) as u32);
                }
            }
        }
        Ok(levels)
    }

    /// Class database identifiers by class name (Fighter 1000).
    pub fn class_ids(&self) -> Result<HashMap<String, i32>> {
        let mut ids = HashMap::new();
        for relative in ["Database/Class.dat", "UserDbas/ClassU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let list =
                ObjectList::parse(&self.read(relative)?).map_err(|source| Error::Format {
                    path: relative.into(),
                    source,
                })?;
            for o in &list.objects {
                if let Object::Class(class) = o {
                    let id = match &**class {
                        Class::Expansion { data, .. } => data.id,
                        // The 2.0 layout keeps the identifier at value 22.
                        Class::Core20(data) => data.unknown_0x10[22],
                    };
                    ids.entry(class.name().to_owned()).or_insert(id);
                }
            }
        }
        Ok(ids)
    }
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
    names.dedup();
    names
}
