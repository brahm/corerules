//! The Player's Option weapon groups: the broad and tight groups the
//! program holds (its part document's constructor 0x677978, CPartDoc
//! +0x234 and +0x248), which a Skills & Powers warrior or priest may buy
//! as weapon proficiencies covering every weapon in them (0x4798c2,
//! 0x479b07). A weapon names its tight groups (+0x1cc).

use crate::Installation;
use crate::npc::broad_has_tight;
use crate::treasure::Flavor;

/// The broad and tight groups, by name with their ids.
#[derive(Debug, Clone, Default)]
pub struct WeaponGroups {
    broad: Vec<(String, i32)>,
    tight: Vec<(String, i32)>,
}

impl WeaponGroups {
    /// The groups of the Expansion's program; none when it is missing.
    pub(crate) fn load(install: &Installation) -> Self {
        let Ok(image) = crate::exe::Image::open(install, Flavor::Expansion) else {
            return Self::default();
        };
        let mut tables = image.id_tables(0x67_7978, 0x3800, 0x57_2030);
        Self {
            broad: tables.remove(&0x234).unwrap_or_default(),
            tight: tables.remove(&0x248).unwrap_or_default(),
        }
    }

    /// The broad groups to buy, in the program's order.
    pub fn broad(&self) -> Vec<String> {
        names(&self.broad)
    }

    /// The tight groups to buy, in the program's order.
    pub fn tight(&self) -> Vec<String> {
        names(&self.tight)
    }

    /// A tight group's id.
    pub(crate) fn tight_id(&self, name: &str) -> Option<i32> {
        self.tight.iter().find(|g| g.0 == name).map(|g| g.1)
    }

    fn broad_id(&self, name: &str) -> Option<i32> {
        self.broad.iter().find(|g| g.0 == name).map(|g| g.1)
    }

    /// Whether one of the `broad` or `tight` groups holds a weapon in the
    /// tight groups `groups` (their ids).
    pub(crate) fn covers(&self, broad: &[String], tight: &[String], groups: &[i32]) -> bool {
        groups.iter().any(|&g| {
            tight.iter().any(|t| self.tight_id(t) == Some(g))
                || broad
                    .iter()
                    .filter_map(|b| self.broad_id(b))
                    .any(|b| broad_has_tight(g, b))
        })
    }
}

/// A table's names but its "None" (0x4B42BD).
fn names(table: &[(String, i32)]) -> Vec<String> {
    table
        .iter()
        .filter(|g| g.0 != "None")
        .map(|g| g.0.clone())
        .collect()
}
