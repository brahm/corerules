//! The records of the user databases behind the catalog's sections:
//! spells, parts, races, monsters, treasure tables and the rest, each
//! kept in a user file of the original next to its standard records;
//! what New, Copy, Edit and Delete do to them, and their editors.

pub mod actions;
pub mod allowed;
pub mod dialogs;
pub mod form;
pub mod tables;
pub mod templates;

use cr_i18n::tr;
use cr_store::userdb::{self, Origin, parts};

/// The sections backed by a user file, for a `'static` id.
const SECTIONS: [&str; 16] = [
    "wizard-spells",
    "priest-spells",
    "traps",
    "psionics",
    "monster-types",
    "monsters",
    "races",
    "classes",
    "weapons",
    "armor",
    "equipment",
    "magic-items",
    "kits",
    "nonweapon",
    "treasure-tables",
    "templates",
];

/// The id of section `id` as one that lives as long as the program, for
/// the sections backed by a user file.
pub fn static_section(id: &str) -> Option<&'static str> {
    SECTIONS
        .iter()
        .copied()
        .chain(parts::OTHER_TYPES.iter().map(|(id, ..)| *id))
        .chain([parts::OTHER_SECTION])
        .find(|s| *s == id)
}

/// The record kind behind a catalog section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Wizard (0) or priest (1) spells.
    Spells(usize),
    Traps,
    Powers,
    MonsterTypes,
    Monsters,
    Races,
    Classes,
    /// Parts of one type.
    Parts(usize),
    /// The section of the other part types, where New asks for the type.
    OtherParts,
    Tables,
    Templates,
}

/// The kind of the records of catalog section `section`, for the
/// sections backed by a user file.
pub fn kind_of(section: &str) -> Option<Kind> {
    Some(match section {
        "wizard-spells" => Kind::Spells(0),
        "priest-spells" => Kind::Spells(1),
        "traps" => Kind::Traps,
        "psionics" => Kind::Powers,
        "monster-types" => Kind::MonsterTypes,
        "monsters" => Kind::Monsters,
        "races" => Kind::Races,
        "classes" => Kind::Classes,
        "weapons" => Kind::Parts(parts::WEAPONS),
        "armor" => Kind::Parts(parts::ARMOR),
        "equipment" => Kind::Parts(parts::EQUIPMENT),
        "magic-items" => Kind::Parts(parts::MAGICAL_ITEMS),
        "kits" => Kind::Parts(parts::KITS),
        "nonweapon" => Kind::Parts(4),
        "treasure-tables" => Kind::Tables,
        "templates" => Kind::Templates,
        parts::OTHER_SECTION => Kind::OtherParts,
        id => Kind::Parts(parts::other_type(id)?),
    })
}

/// The title of a new record's editor and the tooltip of New.
pub fn new_title(kind: Kind) -> &'static str {
    match kind {
        Kind::Spells(0) => tr("New Wizard Spell"),
        Kind::Spells(_) => tr("New Priest Spell"),
        Kind::Traps => tr("New Trap"),
        Kind::Powers => tr("New Psionic Power"),
        Kind::MonsterTypes => tr("New Monster Type"),
        Kind::Monsters => tr("New Monster"),
        Kind::Races => tr("New Race"),
        Kind::Classes => tr("New Class"),
        Kind::Parts(parts::WEAPONS) => tr("New Weapon"),
        Kind::Parts(parts::ARMOR) => tr("New Armor"),
        Kind::Parts(parts::EQUIPMENT) => tr("New Equipment"),
        Kind::Parts(parts::MAGICAL_ITEMS) => tr("New Magical Item"),
        Kind::Parts(parts::KITS) => tr("New Kit"),
        Kind::Parts(4) => tr("New Proficiency"),
        Kind::Parts(5) => tr("New Racial Ability"),
        Kind::Parts(6) => tr("New Language"),
        Kind::Parts(9) => tr("New School of Magic"),
        Kind::Parts(11) => tr("New Trait"),
        Kind::Parts(12) => tr("New Disadvantage"),
        Kind::Parts(13) => tr("New Gem or Art Object"),
        Kind::Parts(parts::CLASS_ABILITIES) => tr("New Class Ability"),
        Kind::Parts(16) => tr("New Currency Type"),
        Kind::Parts(17) => tr("New Count"),
        Kind::Parts(18) => tr("New Sphere"),
        Kind::Parts(_) | Kind::OtherParts => tr("New Part"),
        Kind::Tables => tr("New Treasure Table"),
        Kind::Templates => tr("New Encounter Template"),
    }
}

/// The New item of the detail page's menu.
pub fn new_menu_label(kind: Option<Kind>) -> &'static str {
    match kind {
        None => tr("_New Record…"),
        Some(Kind::Spells(0)) => tr("_New Wizard Spell…"),
        Some(Kind::Spells(_)) => tr("_New Priest Spell…"),
        Some(Kind::Traps) => tr("_New Trap…"),
        Some(Kind::Powers) => tr("_New Psionic Power…"),
        Some(Kind::MonsterTypes) => tr("_New Monster Type…"),
        Some(Kind::Monsters) => tr("_New Monster…"),
        Some(Kind::Races) => tr("_New Race…"),
        Some(Kind::Classes) => tr("_New Class…"),
        Some(Kind::Parts(parts::WEAPONS)) => tr("_New Weapon…"),
        Some(Kind::Parts(parts::ARMOR)) => tr("_New Armor…"),
        Some(Kind::Parts(parts::EQUIPMENT)) => tr("_New Equipment…"),
        Some(Kind::Parts(parts::MAGICAL_ITEMS)) => tr("_New Magical Item…"),
        Some(Kind::Parts(parts::KITS)) => tr("_New Kit…"),
        Some(Kind::Parts(4)) => tr("_New Proficiency…"),
        // New in the other parts' section asks for the type.
        Some(Kind::Parts(_) | Kind::OtherParts) => tr("_New Part…"),
        Some(Kind::Tables) => tr("_New Treasure Table…"),
        Some(Kind::Templates) => tr("_New Encounter Template…"),
    }
}

/// Whether the section's list has a New button.
pub fn can_create(section: &str) -> bool {
    kind_of(section).is_some()
}

/// The tooltip of the section's New button.
pub fn new_tooltip(section: &str) -> String {
    kind_of(section).map_or_else(|| tr("New").into(), |k| new_title(k).into())
}

/// Whether a section lists standard and custom records.
pub fn has_origins(section: &str) -> bool {
    kind_of(section).is_some()
}

/// Whether the entry with `key` can be edited: custom records, and every
/// treasure table (standard ones are edited in the user copy).
pub fn editable(key: &str) -> bool {
    match userdb::parse_key(key) {
        Some((section, origin, _)) => origin == Origin::Custom || section == "treasure-tables",
        None => false,
    }
}

/// The origin of a catalog entry, for the All/Standard/Custom filter.
pub fn origin_of(key: &str) -> Option<Origin> {
    userdb::parse_key(key).map(|(_, origin, _)| origin)
}
