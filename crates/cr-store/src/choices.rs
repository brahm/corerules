//! The names a character's lists can hold, from the rules databases and
//! the user's own additions (`UserDbas/PartsU.dat`, `UserDbas/SpellsU.dat`).

use cr_formats::Object;
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

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup();
    names
}
