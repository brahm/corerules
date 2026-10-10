//! Campaign settings a new character can be made for.
//!
//! Neither original program knows about campaign settings: its databases
//! hold every race and class of the Player's Handbook and the Complete
//! handbooks, and the creation wizard offers all of them. What each
//! setting allows comes from its own campaign book instead, and only
//! narrows what the databases offer: a setting never adds a race or class
//! the databases lack (Dragonlance's kender, Dark Sun's muls and half-giants
//! are not in them, so they cannot be offered).

/// One campaign setting and what it allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setting {
    pub id: &'static str,
    pub name: &'static str,
    /// What the setting is and which book its rules come from.
    pub description: &'static str,
    /// Main races allowed; `None` allows every race in the databases.
    races: Option<&'static [&'static str]>,
    /// Subraces not found in the setting.
    excluded_subraces: &'static [&'static str],
    /// Only the default subrace of each race (the setting has no others).
    default_subraces_only: bool,
    /// Classes not found in the setting.
    excluded_classes: &'static [&'static str],
    /// Wizard classes allowed; `None` allows every one the race may take.
    wizards: Option<&'static [&'static str]>,
}

/// The races of the Player's Handbook.
const HANDBOOK_RACES: &[&str] = &["Dwarf", "Elf", "Gnome", "Half-elf", "Halfling", "Human"];

pub const SETTINGS: [Setting; 6] = [
    Setting {
        id: "generic",
        name: "Generic",
        description: "Every race and class in the databases, as the original program offers \
                      them",
        races: None,
        excluded_subraces: &[],
        default_subraces_only: false,
        excluded_classes: &[],
        wizards: None,
    },
    Setting {
        id: "forgotten-realms",
        name: "Forgotten Realms",
        description: "The Player's Handbook races and half-orcs, every class \
                      (Forgotten Realms Campaign Setting)",
        races: Some(&[
            "Dwarf", "Elf", "Gnome", "Half-elf", "Half-orc", "Halfling", "Human",
        ]),
        excluded_subraces: &[],
        default_subraces_only: false,
        excluded_classes: &[],
        wizards: None,
    },
    Setting {
        id: "greyhawk",
        name: "Greyhawk",
        description: "The Player's Handbook races and half-orcs, every class \
                      (From the Ashes)",
        races: Some(&[
            "Dwarf", "Elf", "Gnome", "Half-elf", "Half-orc", "Halfling", "Human",
        ]),
        excluded_subraces: &[],
        default_subraces_only: false,
        excluded_classes: &[],
        wizards: None,
    },
    Setting {
        id: "dragonlance",
        name: "Dragonlance",
        description: "Humans, dwarves, elves, half-elves, gnomes and minotaurs; no \
                      halflings (kender take their place), no half-orcs, no psionics, \
                      and none of the underground subraces (Tales of the Lance)",
        races: Some(&["Dwarf", "Elf", "Gnome", "Half-elf", "Human", "Minotaur"]),
        excluded_subraces: &[
            "Dark (drow) elf",
            "Deep dwarf",
            "Gray dwarf (duergar)",
            "Deep gnome (svirfneblin)",
        ],
        default_subraces_only: false,
        excluded_classes: &["Psionicist"],
        wizards: None,
    },
    Setting {
        id: "ravenloft",
        name: "Ravenloft",
        description: "The Player's Handbook races and every class \
                      (Ravenloft Campaign Setting)",
        races: Some(HANDBOOK_RACES),
        excluded_subraces: &[],
        default_subraces_only: false,
        excluded_classes: &[],
        wizards: None,
    },
    Setting {
        id: "dark-sun",
        name: "Dark Sun",
        description: "Humans, dwarves, elves, half-elves, halflings and thri-kreen; no \
                      gnomes, no paladins, and of the wizards only mages (preservers and \
                      defilers) and illusionists (Dark Sun Campaign Setting)",
        races: Some(&[
            "Dwarf",
            "Elf",
            "Half-elf",
            "Halfling",
            "Human",
            "Thri-kreen",
        ]),
        excluded_subraces: &[],
        default_subraces_only: true,
        excluded_classes: &["Paladin"],
        wizards: Some(&["Mage", "Illusionist"]),
    },
];

impl Setting {
    pub fn by_id(id: &str) -> Option<&'static Setting> {
        SETTINGS.iter().find(|s| s.id == id)
    }

    pub fn allows_race(&self, race: &str) -> bool {
        self.races.is_none_or(|r| r.contains(&race))
    }

    pub fn allows_subrace(&self, subrace: &str, default: bool) -> bool {
        (default || !self.default_subraces_only) && !self.excluded_subraces.contains(&subrace)
    }

    /// Whether the setting has this class; `wizard` tells whether it is
    /// one of the wizard group.
    pub fn allows_class(&self, class: &str, wizard: bool) -> bool {
        !self.excluded_classes.contains(&class)
            && (!wizard || self.wizards.is_none_or(|w| w.contains(&class)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_narrow_what_the_databases_offer() {
        let generic = Setting::by_id("generic").unwrap();
        assert!(generic.allows_race("Wemic"));
        assert!(generic.allows_class("Psionicist", false));

        let dragonlance = Setting::by_id("dragonlance").unwrap();
        assert!(dragonlance.allows_race("Minotaur"));
        assert!(!dragonlance.allows_race("Halfling"));
        assert!(!dragonlance.allows_class("Psionicist", false));
        assert!(!dragonlance.allows_subrace("Dark (drow) elf", false));
        assert!(dragonlance.allows_subrace("Sylvan (wood) elf", false));

        let dark_sun = Setting::by_id("dark-sun").unwrap();
        assert!(dark_sun.allows_race("Thri-kreen"));
        assert!(!dark_sun.allows_race("Gnome"));
        assert!(!dark_sun.allows_class("Paladin", false));
        assert!(dark_sun.allows_class("Illusionist", true));
        assert!(!dark_sun.allows_class("Necromancer", true));
        assert!(!dark_sun.allows_subrace("High elf", false));
        assert!(dark_sun.allows_subrace("High elf", true));
    }
}
