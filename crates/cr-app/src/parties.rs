//! Manage Parties (the Character dialog's Party Management…) with no
//! toolkit in it: the parties, the members of the one chosen and the
//! characters in no party, as the installation has them.

use cr_i18n::tr;
use cr_store::party::party_of;
use cr_store::{Installation, Result};

/// A character in the lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The character's file in `Chars/`.
    pub file: String,
    pub name: String,
    /// The race and class, NPC first for one the generator made.
    pub subtitle: String,
}

/// The lists, as read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartyLists {
    /// The parties, sorted, with those made here.
    pub parties: Vec<String>,
    /// The parties made here that have no members yet; they go with the
    /// lists.
    pub new: Vec<String>,
    /// The party chosen, empty when there is none.
    pub selected: String,
    pub members: Vec<Member>,
    /// The characters in no party.
    pub without: Vec<Member>,
    /// The files of the party's NPCs by name, for Group Encounter, as the
    /// NPC screen lists them (0x44b5c2).
    pub npcs: Vec<String>,
    /// Whether the installation has Group Encounter (the NPC screen's
    /// button 1576), which only the Expansion has.
    pub group_encounter: bool,
}

impl PartyLists {
    /// Reads the parties and characters of `install`, with the parties
    /// `new` made here, on the party `selected` or the first.
    pub fn read(install: &Installation, selected: &str, new: &[String]) -> Result<Self> {
        let mut parties = install.parties()?;
        let new: Vec<String> = new
            .iter()
            .filter(|p| !parties.iter().any(|q| q.eq_ignore_ascii_case(p)))
            .cloned()
            .collect();
        parties.extend(new.iter().cloned());
        parties.sort_by_cached_key(|p| cr_store::collate::sort_key(p));
        let position = parties
            .iter()
            .position(|p| p.eq_ignore_ascii_case(selected))
            .unwrap_or(0);
        let party = parties.get(position).cloned().unwrap_or_default();
        let index = install.characters()?;
        let (mut members, mut without) = (Vec::new(), Vec::new());
        let mut npcs: Vec<(String, String)> = Vec::new();
        for entry in &index.entries {
            let in_party = party_of(entry);
            let npc = entry.unknown_0x24 != 0;
            let list = if in_party.is_empty() {
                &mut without
            } else if !party.is_empty() && in_party.eq_ignore_ascii_case(&party) {
                if npc {
                    npcs.push((entry.name.clone(), entry.file.clone()));
                }
                &mut members
            } else {
                continue;
            };
            let subtitle = entry
                .details
                .iter()
                .find_map(|d| d.strip_prefix("Class: "))
                .map_or(entry.race.clone(), |class| {
                    format!("{} {class}", entry.race)
                });
            list.push(Member {
                file: entry.file.clone(),
                name: entry.name.trim().to_owned(),
                subtitle: if npc {
                    tr!("NPC · {description}", description = subtitle)
                } else {
                    subtitle
                },
            });
        }
        npcs.sort_by_cached_key(|(name, _)| cr_store::collate::sort_key(name));
        Ok(Self {
            parties,
            new,
            selected: party,
            members,
            without,
            npcs: npcs.into_iter().map(|(_, file)| file).collect(),
            group_encounter: cr_store::encounter::flavor_of(install)
                == cr_store::sheet::Flavor::Expansion,
        })
    }

    /// The party chosen, by its place in [`Self::parties`].
    pub fn position(&self) -> Option<usize> {
        self.parties
            .iter()
            .position(|p| p.eq_ignore_ascii_case(&self.selected))
    }

    /// What the members' list says over them, if anything.
    pub fn members_note(&self) -> Option<&'static str> {
        self.members
            .is_empty()
            .then(|| tr("Nobody is in this party yet."))
    }

    /// What the list of characters in no party says over them, if
    /// anything.
    pub fn without_note(&self) -> Option<&'static str> {
        if self.selected.is_empty() {
            Some(tr("Make a party to put characters in it."))
        } else if self.without.is_empty() {
            Some(tr("Every character is in a party."))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_party_is_listed_and_chosen() {
        let Some(dir) = std::env::var_os("CORERULES_DIR") else {
            return;
        };
        let Ok(install) = Installation::open(std::path::Path::new(&dir)) else {
            return;
        };
        let new = vec!["Zz New Party".to_owned()];
        let lists = PartyLists::read(&install, "zz new party", &new).unwrap();
        assert_eq!(lists.selected, "Zz New Party");
        assert_eq!(lists.new, new);
        assert!(lists.members.is_empty());
        assert_eq!(
            lists.members_note(),
            Some(tr("Nobody is in this party yet."))
        );
        let total = install.characters().unwrap().entries.len();
        assert!(lists.without.len() <= total);
    }
}
