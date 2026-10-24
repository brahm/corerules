//! The DM Toolkit's Export and Import: packages of custom records and
//! characters (`.xpx` from the Expansion, `.xpt` from Core Rules 2.0; the
//! layout is in [`cr_formats::export`]).
//!
//! Export offers what the user made: the records of the user files and the
//! characters of `Chars/index.chr`. In the Expansion a race takes its
//! `AllowedU.dat` record along, and a class the combinations of every race
//! that names it. A package can also be edited: [`Installation::add_to`]
//! puts more items in, [`remove`] takes items out.
//!
//! The Campaign Mapper drawings a package may hold are neither offered nor
//! imported; an edited package keeps them as they are.
//!
//! Import does what the original's Import All does (0x49d529, 0x49d704 in
//! TSRP3.EXE, 0x4def60 in EVERMORE.EXE), taking the items sorted by name:
//!
//! * In the Expansion a record replaces the user record of the same name
//!   (trimmed, case ignored): the old one is removed and the new one
//!   appended. Standard records are not looked at, so a user record may
//!   share a standard one's name. Core Rules 2.0 keeps the user record
//!   and drops the imported one, though its dialog lists both as done.
//! * In the Expansion a part named like a standard part gets "*" appended
//!   (0x49d704); when a part already has that name, the original's Name
//!   Conflict asks for another until none has it (`Installation::import_named`).
//! * In the Expansion an imported race replaces its `AllowedU.dat` record
//!   with the one the file carries; a file without combinations (from
//!   Core Rules 2.0) gives a race without a record one built from the
//!   race's own class lists (0x47c91e). An imported class adds the file's
//!   combinations that name it to each race's record (0x49f33c).
//! * A character gets a new file in `Chars/` and an entry in the list. The
//!   Expansion leaves the class and level lines out of that entry.
//! * The Expansion stores characters and parts from a Core Rules 2.0
//!   package in its own layout.
//!
//! The `.mon` files the monster editor's Export and Import buttons mention
//! do not exist: in both programs those buttons only point at the DM
//! Toolkit, so monsters travel in these packages.

use cr_i18n::{tr, trn};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use cr_formats::Object;
use cr_formats::allowed::{ClassesAllowed, ClassesAllowedHelperData};
use cr_formats::character::{Character, CharacterEntry, CharacterIndex, EntryExtra};
use cr_formats::database::ObjectList;
use cr_formats::export::ExportFile;
use cr_formats::parts::{PART_TYPE_COUNT, PART_TYPES, PartVersion, PartsFile};
use cr_formats::spells::SpellsFile;

use crate::create::{new_file_name, write_new};
use crate::edit::{CLASS_LINE, LEVEL_LINE, PLAYER_LINE, replace};
use crate::encounter::{flavor_of, replace_or_create};
use crate::exe::Image;
use crate::userdb::Flavor;
use crate::userdb::allowed::{self, Allowed, AllowedDb};
use crate::userdb::classes::{ClassEntry, class_entries, class_list};
use crate::userdb::parts::{CLASS_ABILITIES, KITS, VERSION_MARKER, VERSION_STRING};
use crate::{Error, Installation, Result};

/// What an item of a package is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    /// A part of the type with this array index ([`PART_TYPES`]).
    Part(usize),
    Class,
    Race,
    WizardSpell,
    PriestSpell,
    Monster,
    MonsterAttack,
    MonsterType,
    Encounter,
    Template,
    Trap,
    PsionicPower,
    TreasureBundle,
    Character,
}

/// The user files kept as one `CObList`, by category.
const LISTS: [(Category, &str); 10] = [
    (Category::PsionicPower, "UserDbas/PsionicU.dat"),
    (Category::Trap, "UserDbas/TrapsU.dat"),
    (Category::Race, "UserDbas/RaceU.dat"),
    (Category::Class, "UserDbas/ClassU.dat"),
    (Category::Monster, "UserDbas/MonsterU.dat"),
    (Category::MonsterAttack, "UserDbas/AttackU.dat"),
    (Category::MonsterType, "UserDbas/MonTypeU.dat"),
    (Category::Encounter, "UserDbas/EncountU.dat"),
    (Category::Template, "UserDbas/EncPlatU.dat"),
    (Category::TreasureBundle, "UserDbas/TreaBunU.dat"),
];

const SPELLS: &str = "UserDbas/SpellsU.dat";
const PARTS: &str = "UserDbas/PartsU.dat";

impl Category {
    /// The name the lists show for the category.
    pub fn label(self) -> &'static str {
        match self {
            Self::Part(t) => PART_TYPES.get(t).copied().flatten().unwrap_or("Parts"),
            Self::Class => "Classes",
            Self::Race => "Races",
            Self::WizardSpell => "Wizard Spells",
            Self::PriestSpell => "Priest Spells",
            Self::Monster => "Monsters",
            Self::MonsterAttack => "Monster Attacks",
            Self::MonsterType => "Monster Types",
            Self::Encounter => "Encounters",
            Self::Template => "Encounter Templates",
            Self::Trap => "Traps",
            Self::PsionicPower => "Psionic Powers",
            Self::TreasureBundle => "Treasure Bundles",
            Self::Character => "Characters",
        }
    }

    /// The categories a package of `flavor` holds, sorted by label.
    pub fn all(flavor: Flavor) -> Vec<Self> {
        let mut all: Vec<Self> = (0..PART_TYPE_COUNT)
            .filter(|&t| matches!(PART_TYPES[t], Some(n) if n != "NOT"))
            .filter(|&t| flavor == Flavor::Expansion || t != CLASS_ABILITIES)
            .map(Self::Part)
            .collect();
        all.extend([
            Self::Race,
            Self::WizardSpell,
            Self::PriestSpell,
            Self::Monster,
            Self::MonsterAttack,
            Self::MonsterType,
            Self::Encounter,
            Self::Template,
            Self::Trap,
            Self::PsionicPower,
            Self::TreasureBundle,
            Self::Character,
        ]);
        if flavor == Flavor::Expansion {
            all.push(Self::Class);
        }
        all.sort_by_key(|c| c.label());
        all
    }

    /// The category a command line names: its label or a short word
    /// ("race", "wizard-spell", "weapons"), case ignored.
    pub fn parse(text: &str) -> Option<Self> {
        let wanted = text.trim().to_lowercase().replace(['-', '_'], " ");
        Self::all(Flavor::Expansion).into_iter().find(|c| {
            // The same spelling rules for the label: "Non-Weapon
            // Proficiencies" as `cr import --list` prints it.
            let label = c.label().to_lowercase().replace('-', " ");
            label == wanted
                || label.strip_suffix('s') == Some(wanted.as_str())
                || label.strip_suffix("es") == Some(wanted.as_str())
        })
    }
}

/// Something the export can take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exportable {
    pub category: Category,
    pub name: String,
    pub source: Source,
}

/// Where an [`Exportable`] comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The record at this index of its user list (of its part type, of
    /// the wizard or priest spells).
    Record(usize),
    /// A character file inside `Chars/`.
    Character(String),
}

/// What [`Installation::export`] made.
#[derive(Debug, Clone)]
pub struct Export {
    pub file: ExportFile,
    /// Races without race/class combinations to take along.
    pub warnings: Vec<String>,
}

/// An item of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub category: Category,
    pub name: String,
    /// Its index in its section of the file.
    pub index: usize,
    /// Which record has the name already.
    pub clash: Clash,
}

/// A record whose name an imported one has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Clash {
    #[default]
    None,
    /// A user record: the Expansion replaces it, Core Rules 2.0 skips the
    /// imported one.
    Custom,
    /// A standard part: the Expansion renames the imported one with "*".
    Standard,
}

/// What an import did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: Vec<Content>,
    /// The user records the imported ones replaced (category, name).
    pub replaced: Vec<(Category, String)>,
    /// Records Core Rules 2.0 does not import because a user record has
    /// the name.
    pub skipped: Vec<Content>,
    /// Parts renamed because a standard part has the name: the item and
    /// its new name.
    pub renamed: Vec<(Content, String)>,
    /// The character files written into `Chars/`.
    pub characters: Vec<String>,
    pub warnings: Vec<String>,
}

/// The name of a record of a package.
pub fn object_name(object: &Object) -> Option<&str> {
    Some(match object {
        Object::Part(p) => &p.head.name,
        Object::Spell(s) => &s.name,
        Object::Race(r) => &r.name,
        Object::Class(c) => c.name(),
        Object::Monster(m) => &m.name,
        Object::MonsterAttack(a) => &a.name,
        Object::MonsterType(t) => &t.name,
        Object::Encounter(e) => &e.name,
        Object::EncounterTemplate(t) => &t.name,
        Object::Trap(t) => &t.name,
        Object::PsionicPower(p) => &p.name,
        Object::Treasure(t) => &t.name,
        _ => return None,
    })
}

fn same_name(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn named(objects: &[Object], name: &str) -> Option<usize> {
    objects
        .iter()
        .position(|o| object_name(o).is_some_and(|n| same_name(n, name)))
}

/// The section of `file` holding `category`.
fn section(file: &ExportFile, category: Category) -> &[Object] {
    match category {
        Category::Part(t) => &file.parts[t],
        Category::Class => file.classes.as_deref().unwrap_or(&[]),
        Category::Race => &file.races,
        Category::WizardSpell => &file.wizard_spells,
        Category::PriestSpell => &file.priest_spells,
        Category::Monster => &file.monsters,
        Category::MonsterAttack => &file.monster_attacks,
        Category::MonsterType => &file.monster_types,
        Category::Encounter => &file.encounters,
        Category::Template => &file.templates,
        Category::Trap => &file.traps,
        Category::PsionicPower => &file.psionics,
        Category::TreasureBundle => &file.treasure,
        Category::Character => &[],
    }
}

fn section_mut(file: &mut ExportFile, category: Category) -> Option<&mut Vec<Object>> {
    Some(match category {
        Category::Part(t) => &mut file.parts[t],
        Category::Class => file.classes.as_mut()?,
        Category::Race => &mut file.races,
        Category::WizardSpell => &mut file.wizard_spells,
        Category::PriestSpell => &mut file.priest_spells,
        Category::Monster => &mut file.monsters,
        Category::MonsterAttack => &mut file.monster_attacks,
        Category::MonsterType => &mut file.monster_types,
        Category::Encounter => &mut file.encounters,
        Category::Template => &mut file.templates,
        Category::Trap => &mut file.traps,
        Category::PsionicPower => &mut file.psionics,
        Category::TreasureBundle => &mut file.treasure,
        Category::Character => return None,
    })
}

/// Every item of `file`, sorted by name, case ignored, as the Import
/// dialog lists them (`clash` is not set; see
/// [`Installation::import_preview`]).
pub fn contents(file: &ExportFile) -> Vec<Content> {
    let mut items = Vec::new();
    let mut categories = Category::all(Flavor::Expansion);
    categories.retain(|c| *c != Category::Character);
    for category in categories {
        for (index, object) in section(file, category).iter().enumerate() {
            if let Some(name) = object_name(object) {
                items.push(Content {
                    category,
                    name: name.to_owned(),
                    index,
                    clash: Clash::None,
                });
            }
        }
    }
    for (index, c) in file.characters.iter().enumerate() {
        items.push(Content {
            category: Category::Character,
            name: c.data.name.clone(),
            index,
            clash: Clash::None,
        });
    }
    items.sort_by_cached_key(|c| crate::collate::sort_key(&c.name));
    items
}

/// Takes `items` (as [`contents`] lists them) out of `file`. In the
/// Expansion a race takes its combinations along, unless a class the
/// package holds is named in them, and a class the combinations of the
/// races the package does not hold that name it. Returns the warnings.
pub fn remove(file: &mut ExportFile, items: &[Content]) -> Vec<String> {
    let mut warnings = Vec::new();
    if items.is_empty() {
        return warnings;
    }
    let mut items = items.to_vec();
    // Later indexes first, so the earlier ones still point at their items.
    items.sort_by_key(|c| std::cmp::Reverse(c.index));
    let (mut removed, mut combinations) = (false, false);
    for item in &items {
        match item.category {
            Category::Character => {
                if file
                    .characters
                    .get(item.index)
                    .is_some_and(|c| c.data.name == item.name)
                {
                    file.characters.remove(item.index);
                    removed = true;
                }
            }
            category => {
                let Some(section) = section_mut(file, category) else {
                    continue;
                };
                if item.index < section.len()
                    && object_name(&section[item.index]).is_some_and(|n| n == item.name)
                {
                    section.remove(item.index);
                    removed = true;
                    combinations |= matches!(category, Category::Race | Category::Class);
                }
            }
        }
    }
    if combinations {
        prune_combinations(file);
    }
    if removed {
        drop_map_maker(file, &mut warnings);
    }
    warnings
}

/// Keeps the combinations of the races `file` holds, and of the others
/// those naming a class it holds.
fn prune_combinations(file: &mut ExportFile) {
    let races: Vec<String> = file
        .races
        .iter()
        .filter_map(object_name)
        .map(str::to_owned)
        .collect();
    let classes: Vec<String> = file
        .classes
        .iter()
        .flatten()
        .filter_map(object_name)
        .map(str::to_owned)
        .collect();
    let Some(allowed) = file.allowed.as_mut() else {
        return;
    };
    allowed.retain_mut(|o| {
        let Object::ClassesAllowed(record) = o else {
            return true;
        };
        if races.contains(&record.race) {
            return true;
        }
        record.classes.retain(|h| {
            helper_data(h).is_none_or(|d| {
                helper_slots(d)
                    .iter()
                    .any(|s| classes.iter().any(|c| c == s))
            })
        });
        !record.classes.is_empty()
    });
}

/// The Map Maker II maps come last and may refer to what the sections
/// before them define, so they do not survive a change to the package.
fn drop_map_maker(file: &mut ExportFile, warnings: &mut Vec<String>) {
    if file.map_maker.count > 0 {
        warnings.push(trn!(
            "The package's Map Maker II map was left out",
            "The package's {n} Map Maker II maps were left out",
            file.map_maker.count as u64
        ));
        file.map_maker = Default::default();
    }
}

/// The class names of a combination, in group order.
fn helper_names(d: &ClassesAllowedHelperData) -> Vec<&str> {
    [&d.warrior, &d.rogue, &d.priest, &d.wizard, &d.psionicist]
        .into_iter()
        .filter(|n| !n.is_empty())
        .map(String::as_str)
        .collect()
}

fn helper_data(o: &Object) -> Option<&ClassesAllowedHelperData> {
    match o {
        Object::ClassesAllowedHelper(h) => Some(&h.data),
        _ => None,
    }
}

fn helper_slots(d: &ClassesAllowedHelperData) -> [&str; 5] {
    [&d.warrior, &d.rogue, &d.priest, &d.wizard, &d.psionicist].map(String::as_str)
}

fn empty_record(race: &str) -> ClassesAllowed {
    ClassesAllowed {
        version: 0,
        race: race.to_owned(),
        unknown_0x1c: String::new(),
        flag_0x20: 0,
        classes: Vec::new(),
    }
}

/// 0x689761: adds the combination of `names` to `record`, each name in
/// its class's group slot (unknown names skipped, the first of a group
/// winning), unless the record has it already.
fn add_combination(
    record: &mut ClassesAllowed,
    names: &[&str],
    classes: &[ClassEntry],
    standard: bool,
    level_limit: i32,
) {
    let Some(mut entry) = Allowed::from_names(names, classes, level_limit) else {
        return;
    };
    entry.standard = standard;
    let slots = entry.classes.each_ref().map(String::as_str);
    if record
        .classes
        .iter()
        .filter_map(helper_data)
        .any(|d| helper_slots(d) == slots)
    {
        return;
    }
    record.classes.push(entry.helper());
}

/// The user files an import changes, read once.
struct Stores<'a> {
    install: &'a Installation,
    lists: Vec<(Category, Vec<Object>, bool)>,
    spells: Option<(SpellsFile, bool)>,
    parts: Option<(PartsFile, bool)>,
    standard_parts: Option<Vec<String>>,
    allowed: Option<(AllowedDb, bool)>,
    classes: Option<Vec<ClassEntry>>,
}

impl<'a> Stores<'a> {
    fn new(install: &'a Installation) -> Self {
        Self {
            install,
            lists: Vec::new(),
            spells: None,
            parts: None,
            standard_parts: None,
            allowed: None,
            classes: None,
        }
    }

    fn read<T>(
        &self,
        relative: &str,
        parse: impl Fn(&[u8]) -> std::result::Result<T, cr_formats::Error>,
    ) -> Result<Option<T>> {
        let Some(path) = self.install.path(relative) else {
            return Ok(None);
        };
        parse(&self.install.read(relative)?)
            .map(Some)
            .map_err(|source| Error::Format { path, source })
    }

    fn list(&mut self, category: Category) -> Result<&mut Vec<Object>> {
        if let Some(i) = self.lists.iter().position(|(c, _, _)| *c == category) {
            self.lists[i].2 = true;
            return Ok(&mut self.lists[i].1);
        }
        let relative = list_file(category);
        let objects = self
            .read(relative, ObjectList::parse)?
            .map_or(Vec::new(), |l| l.objects);
        self.lists.push((category, objects, true));
        Ok(&mut self.lists.last_mut().expect("just pushed").1)
    }

    fn list_ref(&mut self, category: Category) -> Result<&[Object]> {
        let list = self.list(category)?;
        Ok(list)
    }

    fn spells(&mut self) -> Result<&mut SpellsFile> {
        if self.spells.is_none() {
            let file = self.read(SPELLS, SpellsFile::parse)?.unwrap_or(SpellsFile {
                wizard: Vec::new(),
                priest: Vec::new(),
            });
            self.spells = Some((file, false));
        }
        let (file, dirty) = self.spells.as_mut().expect("just set");
        *dirty = true;
        Ok(file)
    }

    fn parts(&mut self) -> Result<&mut PartsFile> {
        if self.parts.is_none() {
            let file = self.read(PARTS, PartsFile::parse)?.unwrap_or(PartsFile {
                arrays: Default::default(),
            });
            self.parts = Some((file, false));
        }
        let (file, dirty) = self.parts.as_mut().expect("just set");
        *dirty = true;
        Ok(file)
    }

    /// The names of every standard part, lowercased.
    fn standard_parts(&mut self) -> Result<&[String]> {
        if self.standard_parts.is_none() {
            let names = self
                .read("Database/Parts.dat", PartsFile::parse)?
                .map(|f| {
                    f.arrays
                        .iter()
                        .flatten()
                        .filter_map(object_name)
                        .map(|n| n.trim().to_lowercase())
                        .collect()
                })
                .unwrap_or_default();
            self.standard_parts = Some(names);
        }
        Ok(self.standard_parts.as_deref().expect("just set"))
    }

    fn allowed(&mut self) -> Result<&mut AllowedDb> {
        if self.allowed.is_none() {
            self.allowed = Some((AllowedDb::load(self.install)?, false));
        }
        let (db, dirty) = self.allowed.as_mut().expect("just set");
        *dirty = true;
        Ok(db)
    }

    fn classes(&mut self) -> Result<Vec<ClassEntry>> {
        if self.classes.is_none() {
            self.classes = Some(class_list(self.install)?);
        }
        Ok(self.classes.clone().expect("just set"))
    }

    /// The standard classes and the user's as this import left them, the
    /// classes it brought in included before they are saved.
    fn classes_after_import(&mut self) -> Result<Vec<ClassEntry>> {
        let mut objects = self
            .read("Database/Class.dat", ObjectList::parse)?
            .map_or(Vec::new(), |l| l.objects);
        objects.extend(self.list_ref(Category::Class)?.iter().cloned());
        Ok(class_entries(&objects))
    }

    /// Checks every changed file can be written, then writes them.
    fn save(self) -> Result<()> {
        let mut files: Vec<(&str, Vec<u8>)> = Vec::new();
        let format = |relative: &str| {
            let path = PathBuf::from(relative);
            move |source| Error::Format { path, source }
        };
        for (category, objects, dirty) in self.lists {
            if dirty {
                let relative = list_file(category);
                let bytes = ObjectList { objects }
                    .to_bytes()
                    .map_err(format(relative))?;
                files.push((relative, bytes));
            }
        }
        if let Some((file, true)) = self.spells {
            files.push((SPELLS, file.to_bytes().map_err(format(SPELLS))?));
        }
        if let Some((file, true)) = self.parts {
            files.push((PARTS, file.to_bytes().map_err(format(PARTS))?));
        }
        if let Some((db, true)) = self.allowed {
            files.push((allowed::FILE, db.to_bytes()?));
        }
        for (relative, bytes) in files {
            if self.install.path(relative).is_some() && self.install.read(relative)? == bytes {
                continue;
            }
            let name = relative.rsplit('/').next().unwrap_or(relative);
            replace_or_create(self.install, relative, name, &bytes)?;
        }
        Ok(())
    }
}

fn list_file(category: Category) -> &'static str {
    LISTS
        .iter()
        .find(|(c, _)| *c == category)
        .map(|(_, f)| *f)
        .expect("a category kept in a list file")
}

impl Installation {
    /// What Export offers: the user's records and characters, by category
    /// and name.
    pub fn exportable(&self) -> Result<Vec<Exportable>> {
        let flavor = flavor_of(self);
        let stores = Stores::new(self);
        let mut out = Vec::new();
        let mut push = |category, name: &str, source| {
            out.push(Exportable {
                category,
                name: name.to_owned(),
                source,
            })
        };
        if let Some(parts) = stores.read(PARTS, PartsFile::parse)? {
            for (t, array) in parts.arrays.iter().enumerate() {
                if !Category::all(flavor).contains(&Category::Part(t)) {
                    continue;
                }
                for (i, o) in array.iter().enumerate() {
                    if let Some(name) = object_name(o) {
                        push(Category::Part(t), name, Source::Record(i));
                    }
                }
            }
        }
        if let Some(spells) = stores.read(SPELLS, SpellsFile::parse)? {
            for (category, list) in [
                (Category::WizardSpell, &spells.wizard),
                (Category::PriestSpell, &spells.priest),
            ] {
                for (i, o) in list.iter().enumerate() {
                    if let Some(name) = object_name(o) {
                        push(category, name, Source::Record(i));
                    }
                }
            }
        }
        for (category, relative) in LISTS {
            if category == Category::Class && flavor != Flavor::Expansion {
                continue;
            }
            if let Some(list) = stores.read(relative, ObjectList::parse)? {
                for (i, o) in list.objects.iter().enumerate() {
                    if let Some(name) = object_name(o) {
                        push(category, name, Source::Record(i));
                    }
                }
            }
        }
        for e in self.characters()?.entries {
            push(Category::Character, &e.name, Source::Character(e.file));
        }
        out.sort_by_cached_key(|e| (e.category.label(), crate::collate::sort_key(&e.name)));
        Ok(out)
    }

    /// Everything a Core Rules 2.0 installation can hand over: its
    /// characters and the user's records, in a 2.0 package. Importing it
    /// into the Expansion converts each item the way the Expansion converts
    /// a `.xpt` package. New content is made in the Expansion only.
    pub fn core20_package(&self) -> Result<Export> {
        if flavor_of(self) == Flavor::Expansion {
            return Err(Error::Invalid(
                "This is an Expansion installation, not a Core Rules 2.0 one".into(),
            ));
        }
        self.export(&self.exportable()?)
    }

    /// The package holding `items`, in the layout of this installation's
    /// program, in the order given.
    pub fn export(&self, items: &[Exportable]) -> Result<Export> {
        let mut file = ExportFile::new(flavor_of(self) == Flavor::Expansion);
        let warnings = self.add_to(&mut file, items)?;
        Ok(Export { file, warnings })
    }

    /// Puts `items` into `file`, a package in the layout of this
    /// installation's program, as Export would: an item takes the place of
    /// the one of its category and name the package holds already, and in
    /// the Expansion a race or class brings its combinations along.
    /// Returns the warnings.
    pub fn add_to(&self, file: &mut ExportFile, items: &[Exportable]) -> Result<Vec<String>> {
        let flavor = flavor_of(self);
        let expansion = flavor == Flavor::Expansion;
        if file.is_expansion() != expansion {
            return Err(Error::Invalid(format!(
                "This is a package for {}; items of {} cannot go into it",
                if file.is_expansion() {
                    "the Expansion"
                } else {
                    "Core Rules 2.0"
                },
                if expansion {
                    "the Expansion"
                } else {
                    "Core Rules 2.0"
                },
            )));
        }
        let mut warnings = Vec::new();
        let stores = Stores::new(self);
        let parts = stores.read(PARTS, PartsFile::parse)?;
        let spells = stores.read(SPELLS, SpellsFile::parse)?;
        let mut lists: Vec<(Category, Vec<Object>)> = Vec::new();
        let allowed = if expansion {
            AllowedDb::load(self)?
        } else {
            AllowedDb::default()
        };
        let missing = |what: &Exportable| {
            Error::Invalid(format!(
                "{} \"{}\" is not among the user's records",
                what.category.label(),
                what.name
            ))
        };
        for item in items {
            let object = match (&item.source, item.category) {
                (Source::Character(name), _) => {
                    let relative = format!("Chars/{name}");
                    let path = self
                        .path(&relative)
                        .ok_or_else(|| Error::Invalid(format!("{relative} not found")))?;
                    let character = Character::parse(&self.read(&relative)?)
                        .map_err(|source| Error::Format { path, source })?;
                    match file
                        .characters
                        .iter()
                        .position(|c| same_name(&c.data.name, &character.data.name))
                    {
                        Some(i) => file.characters[i] = character,
                        None => file.characters.push(character),
                    }
                    continue;
                }
                (Source::Record(i), Category::Part(t)) => parts
                    .as_ref()
                    .and_then(|p| p.arrays.get(t)?.get(*i))
                    .cloned(),
                (Source::Record(i), Category::WizardSpell) => {
                    spells.as_ref().and_then(|s| s.wizard.get(*i)).cloned()
                }
                (Source::Record(i), Category::PriestSpell) => {
                    spells.as_ref().and_then(|s| s.priest.get(*i)).cloned()
                }
                (Source::Record(i), category) => {
                    if !lists.iter().any(|(c, _)| *c == category) {
                        let relative = LISTS
                            .iter()
                            .find(|(c, _)| *c == category)
                            .map(|(_, f)| *f)
                            .ok_or_else(|| missing(item))?;
                        let objects = stores
                            .read(relative, ObjectList::parse)?
                            .map_or(Vec::new(), |l| l.objects);
                        lists.push((category, objects));
                    }
                    lists
                        .iter()
                        .find(|(c, _)| *c == category)
                        .and_then(|(_, l)| l.get(*i))
                        .cloned()
                }
            };
            let object = object
                .filter(|o| object_name(o) == Some(item.name.as_str()))
                .ok_or_else(|| missing(item))?;
            let section = section_mut(file, item.category).ok_or_else(|| missing(item))?;
            match named(section, &item.name) {
                Some(i) => section[i] = object,
                None => section.push(object),
            }
            if expansion {
                match item.category {
                    Category::Race => export_race(file, &allowed, &item.name, &mut warnings),
                    Category::Class => export_class(file, &allowed, &item.name),
                    _ => {}
                }
            }
        }
        if !items.is_empty() {
            drop_map_maker(file, &mut warnings);
        }
        Ok(warnings)
    }

    /// The items of `file` as [`contents`] lists them, with the records
    /// whose names they share.
    pub fn import_preview(&self, file: &ExportFile) -> Result<Vec<Content>> {
        let expansion = flavor_of(self) == Flavor::Expansion;
        let mut stores = Stores::new(self);
        let mut items = contents(file);
        for item in &mut items {
            let custom = match item.category {
                Category::Part(_) => {
                    let lower = item.name.trim().to_lowercase();
                    if expansion && stores.standard_parts()?.contains(&lower) {
                        item.clash = Clash::Standard;
                        continue;
                    }
                    stores
                        .parts()?
                        .arrays
                        .iter()
                        .any(|a| named(a, &item.name).is_some())
                }
                Category::WizardSpell | Category::PriestSpell => {
                    let spells = stores.spells()?;
                    named(&spells.wizard, &item.name).is_some()
                        || named(&spells.priest, &item.name).is_some()
                }
                Category::Character => false,
                category => named(stores.list_ref(category)?, &item.name).is_some(),
            };
            if custom {
                item.clash = Clash::Custom;
            }
        }
        Ok(items)
    }

    /// Imports the items of `file` whose indexes into [`contents`] are in
    /// `chosen` (all with `None`).
    pub fn import(&self, file: &ExportFile, chosen: Option<&[usize]>) -> Result<ImportReport> {
        self.import_named(file, chosen, &HashMap::new())
    }

    /// [`import`](Self::import), with the names `names` gives (by index
    /// into [`contents`]) for the parts a standard part's name hides,
    /// asked in the original's Name Conflict; the others get "*"
    /// appended until no part has their name.
    pub fn import_named(
        &self,
        file: &ExportFile,
        chosen: Option<&[usize]>,
        names: &HashMap<usize, String>,
    ) -> Result<ImportReport> {
        let flavor = flavor_of(self);
        let expansion = flavor == Flavor::Expansion;
        if file.is_expansion() && !expansion {
            return Err(Error::Invalid(
                "This package was made by the Expansion, which Core Rules 2.0 cannot import".into(),
            ));
        }
        let mut report = ImportReport::default();
        let mut stores = Stores::new(self);
        let mut characters = Vec::new();
        // The Expansion turns what Core Rules 2.0 wrote into its own layout.
        let upgrade = expansion && !file.is_expansion();
        let mut image = None;
        let mut classes = Vec::new();
        let mut class_warnings = Vec::new();
        let all = contents(file);
        for (n, item) in all.iter().enumerate() {
            if chosen.is_some_and(|c| !c.contains(&n)) {
                continue;
            }
            match item.category {
                Category::Character => {
                    let character = &file.characters[item.index];
                    characters.push(if upgrade {
                        character.upgraded().map_err(|source| Error::Format {
                            path: "imported character".into(),
                            source,
                        })?
                    } else {
                        character.clone()
                    });
                }
                Category::Class if !expansion => continue,
                category => {
                    let mut object = section(file, category)[item.index].clone();
                    downgrade_patched(&mut object, &item.name, &mut report.warnings);
                    if upgrade {
                        upgrade_part(&mut object, &mut image, self)?;
                    }
                    let name = names.get(&n).map(String::as_str);
                    match import_record(&mut stores, category, &mut object, expansion, name)? {
                        Outcome::Added => {}
                        Outcome::Replaced(category, name) => report.replaced.push((category, name)),
                        Outcome::Renamed(name) => report.renamed.push((item.clone(), name)),
                        Outcome::Skipped => {
                            report.skipped.push(item.clone());
                            continue;
                        }
                    }
                    if expansion && category == Category::Race {
                        import_race(&mut stores, file, &object, &mut report.warnings)?;
                    }
                    if expansion && category == Category::Class {
                        import_class(&mut stores, file, &item.name, &mut class_warnings)?;
                        classes.push(item.name.clone());
                    }
                }
            }
            report.imported.push(item.clone());
        }
        // The combinations of the classes are looked up in the classes
        // there were before the import, as the original does, so those
        // naming a class of the package only come in on a second round,
        // with all of them known; only what still fails is reported. The
        // original needed the package imported twice for this.
        if !classes.is_empty() {
            class_warnings.clear();
            stores.classes = Some(stores.classes_after_import()?);
            for class in &classes {
                import_class(&mut stores, file, class, &mut class_warnings)?;
            }
        }
        report.warnings.append(&mut class_warnings);
        if file.map_maker.count > 0 {
            report.warnings.push(trn!(
                "{n} Map Maker II map was not imported",
                "{n} Map Maker II maps were not imported",
                file.map_maker.count as u64
            ));
        }
        stores.save()?;
        if !characters.is_empty() {
            report.characters = self.add_characters(&characters, expansion)?;
        }
        // Each warning once.
        let mut seen = std::collections::HashSet::new();
        report.warnings.retain(|w| seen.insert(w.clone()));
        Ok(report)
    }

    /// Writes `characters` into `Chars/` and adds them to the list;
    /// returns their file names.
    fn add_characters(&self, characters: &[Character], expansion: bool) -> Result<Vec<String>> {
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
        let format = |path: &str| {
            let path = PathBuf::from(path);
            move |source| Error::Format { path, source }
        };
        let mut index = self.characters()?;
        if index.schema == 0 {
            index.schema = crate::create::INDEX_SCHEMA;
        }
        let kits = if expansion {
            crate::userdb::parts::all_names(self)?
                .get(KITS)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let rules = if expansion {
            None
        } else {
            self.advance_rules().ok()
        };
        let mut written = Vec::new();
        let mut files = Vec::new();
        for character in characters {
            let bytes = character.to_bytes().map_err(format("imported character"))?;
            let mut file = new_file_name(&chars, &index);
            while written
                .iter()
                .any(|w: &String| w.eq_ignore_ascii_case(&file))
            {
                file = next_file_name(&file);
            }
            let entry = if expansion {
                expansion_entry(character, &file, &kits, None)
            } else {
                let levels = rules
                    .as_ref()
                    .and_then(|r| r.levels(character).ok())
                    .map(|l| {
                        l.iter()
                            .map(|c| c.level.to_string())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_else(|| {
                        vec!["1"; character.data.classes.names.len().max(1)].join("/")
                    });
                core20_entry(character, &file, &levels)
            };
            index.entries.push(entry);
            written.push(file.clone());
            files.push((file, bytes));
        }
        let index_bytes = index.to_bytes().map_err(format("Chars/index.chr"))?;
        CharacterIndex::parse(&index_bytes).map_err(format("Chars/index.chr"))?;
        for (file, bytes) in &files {
            write_new(&chars.join(file), bytes)?;
        }
        match self.path("Chars/index.chr") {
            Some(path) => replace(&path, &index_bytes)?,
            None => write_new(&chars.join("index.chr"), &index_bytes)?,
        }
        Ok(written)
    }
}

/// The next eight-digit name after `file` (`12345678.dat`).
fn next_file_name(file: &str) -> String {
    let n: u64 = file
        .trim_end_matches(".dat")
        .parse()
        .map_or(
            10_000_000,
            |n: u64| if n >= 99_999_999 { 10_000_000 } else { n + 1 },
        );
    format!("{n:08}.dat")
}

/// The Expansion's list entry for a character (0x5a08fc). An imported one
/// (the import flag set, `levels` `None`) has no class and level lines;
/// the kit line is only there for a kit the parts know.
pub(crate) fn expansion_entry(
    c: &Character,
    file: &str,
    kits: &[String],
    levels: Option<&str>,
) -> CharacterEntry {
    let d = &c.data;
    let party = d
        .unknown_0x1e68
        .as_ref()
        .map(|p| p[3].clone())
        .unwrap_or_default();
    let mut details = vec![
        if c.skills_and_powers() {
            "Skills & Powers"
        } else {
            "Core Rules"
        }
        .to_owned(),
        format!("{PLAYER_LINE}{}", d.player),
        format!("Party: {party}"),
        format!("Race: {}", d.race),
    ];
    if let Some(levels) = levels {
        details.push(format!("{CLASS_LINE}{}", d.class_title));
        details.push(format!("{LEVEL_LINE}{levels}"));
    }
    if let Some(kit) = kits.iter().find(|k| k.eq_ignore_ascii_case(&d.kit)) {
        details.push(format!("Kit: {kit}"));
    }
    CharacterEntry {
        name: d.name.clone(),
        file: file.into(),
        race: d.race.clone(),
        version: 3,
        extra: Some(EntryExtra {
            unknown: 8,
            unknown_0x2c: party,
        }),
        unknown_0x24: d.unknown_0x1da4,
        // 0x5a08fc sets the flag on a scratch entry, but the copy the index
        // keeps (0x5a1372, 0x5a05db) never takes it, so the original saves
        // whatever its heap held. This is the value the scratch entry had.
        flag_0x28: u8::from(!c.skills_and_powers()),
        details,
    }
}

/// Core Rules 2.0's entry for an imported character, like a new one's.
fn core20_entry(c: &Character, file: &str, levels: &str) -> CharacterEntry {
    let d = &c.data;
    CharacterEntry {
        name: d.name.clone(),
        file: file.into(),
        race: d.race.clone(),
        version: d.unknown_0x1da4,
        extra: None,
        unknown_0x24: d.unknown_0x1da4,
        flag_0x28: 1,
        details: vec![
            "Core Rules".into(),
            format!("Race: {}", d.race),
            format!("{CLASS_LINE}{}", d.class_title),
            format!("{LEVEL_LINE}{levels}"),
        ],
    }
}

/// Turns what the patched Expansion writes (patches 3 to 5) into what the
/// CD's `TSRP3.EXE` reads: classes lose their spell points
/// (`CSpellPointsOb`, a class the CD's program does not have) and kits of
/// version 31 become version 30. The program on the CD rejects such a
/// package as a whole ("Unexpected file format.").
fn downgrade_patched(object: &mut Object, name: &str, warnings: &mut Vec<String>) {
    match object {
        Object::Class(class) => {
            if class.drop_spell_points() > 0 {
                warnings.push(tr!(
                    "The spell points of the class {name} were left out: the Expansion cannot \
                     read them",
                    name
                ));
            }
        }
        Object::Part(part) => {
            if let Some(Some(kit)) = &mut part.data.kit
                && let Object::PartKit(kit) = kit.as_mut()
            {
                kit.downgrade();
            }
        }
        _ => {}
    }
}

/// The Expansion's names for the old class flags of a part, Bard to
/// Thief (0x661697); the eighth flag is not converted.
const OLD_CLASS_NAMES: [(usize, u32); 8] = [
    (0, 0x84c580),
    (1, 0x84c588),
    (2, 0x84c590),
    (3, 0x84c598),
    (4, 0x84c5a0),
    (5, 0x84c5a8),
    (6, 0x84c5b0),
    (8, 0x84c5b8),
];

/// What the Expansion's loader (0x65c3a0) does to a Core Rules 2.0 part,
/// which has no version block: the members newer layouts add get their
/// defaults, and the class flags become names in `allowed_classes`
/// (0x661697).
fn upgrade_part(
    object: &mut Object,
    image: &mut Option<Image>,
    install: &Installation,
) -> Result<()> {
    let Object::Part(part) = object else {
        return Ok(());
    };
    if part.version.is_some() {
        return Ok(());
    }
    part.version = Some(PartVersion {
        marker: VERSION_MARKER,
        string: VERSION_STRING.into(),
    });
    let d = &mut part.data;
    d.unknown_0x52c.get_or_insert(100);
    d.unknown_0x4.get_or_insert([0; 101]);
    d.flag_0x724.get_or_insert(0);
    d.flag_0x4d0.get_or_insert(0);
    d.flag_0x448.get_or_insert(0);
    d.kit.get_or_insert(None);
    let classes = d.allowed_classes.get_or_insert_with(Vec::new);
    for (flag, va) in OLD_CLASS_NAMES {
        if d.old_class_flags[flag] == 0 {
            continue;
        }
        if image.is_none() {
            *image = Some(Image::open(install, Flavor::Expansion)?);
        }
        let name = image
            .as_ref()
            .and_then(|i| i.text(va))
            .ok_or_else(|| Error::Invalid("the program has no class names".into()))?;
        classes.push(name);
        d.old_class_flags[flag] = 0;
    }
    Ok(())
}

/// 0x4a9c8c: a race takes its combinations along, replacing any the
/// export holds for it already.
fn export_race(file: &mut ExportFile, db: &AllowedDb, race: &str, warnings: &mut Vec<String>) {
    let Some(list) = file.allowed.as_mut() else {
        return;
    };
    let Some(record) = db.position(race).map(|i| db.records[i].clone()) else {
        warnings.push(tr!(
            "No race/class combinations found for the race {race}",
            race
        ));
        return;
    };
    list.retain(|o| !matches!(o, Object::ClassesAllowed(r) if r.race == race));
    list.push(Object::ClassesAllowed(record));
}

/// 0x681868: a class takes along, for each race, the combinations that
/// name it.
fn export_class(file: &mut ExportFile, db: &AllowedDb, class: &str) {
    let Some(list) = file.allowed.as_mut() else {
        return;
    };
    for record in &db.records {
        let helpers: Vec<&Object> = record
            .classes
            .iter()
            .filter(|o| helper_data(o).is_some_and(|d| helper_slots(d).contains(&class)))
            .collect();
        if helpers.is_empty() {
            continue;
        }
        let at = match list
            .iter()
            .position(|o| matches!(o, Object::ClassesAllowed(r) if r.race == record.race))
        {
            Some(i) => i,
            None => {
                list.push(Object::ClassesAllowed(empty_record(&record.race)));
                list.len() - 1
            }
        };
        let Object::ClassesAllowed(entry) = &mut list[at] else {
            unreachable!("found as a record");
        };
        for helper in helpers {
            let slots = helper_data(helper).map(helper_slots);
            if !entry
                .classes
                .iter()
                .any(|o| helper_data(o).map(helper_slots) == slots)
            {
                entry.classes.push(helper.clone());
            }
        }
    }
}

/// What importing one record did.
enum Outcome {
    Added,
    /// It replaced the user record of this category and name.
    Replaced(Category, String),
    /// A part renamed to this because a standard part has its name.
    Renamed(String),
    /// Core Rules 2.0 kept the user record of the same name.
    Skipped,
}

/// Adds `object` to its user file: the Expansion (`replace`) replaces the
/// record of the same name and renames a part a standard part's name
/// would hide (to `renamed` when given); Core Rules 2.0 skips a record a
/// user record's name clashes with.
fn import_record(
    stores: &mut Stores,
    category: Category,
    object: &mut Object,
    replace: bool,
    renamed: Option<&str>,
) -> Result<Outcome> {
    let name = object_name(object).unwrap_or_default().to_owned();
    match category {
        Category::Part(t) => {
            let standard = stores.standard_parts()?.to_vec();
            let is_standard = |n: &str| standard.contains(&n.trim().to_lowercase());
            let mut outcome = Outcome::Added;
            if replace && is_standard(&name) {
                let mut new_name = format!("{name}*");
                let parts = stores.parts()?;
                if let Some(renamed) = renamed {
                    renamed.clone_into(&mut new_name);
                }
                while is_standard(&new_name)
                    || parts.arrays.iter().any(|a| named(a, &new_name).is_some())
                {
                    new_name.push('*');
                }
                if let Object::Part(p) = object {
                    p.head.name.clone_from(&new_name);
                }
                outcome = Outcome::Renamed(new_name);
            }
            let new_name = object_name(object).unwrap_or_default().to_owned();
            let parts = stores.parts()?;
            for (u, array) in parts.arrays.iter_mut().enumerate() {
                if let Some(i) = named(array, &new_name) {
                    if !replace {
                        return Ok(Outcome::Skipped);
                    }
                    array.remove(i);
                    outcome = Outcome::Replaced(Category::Part(u), new_name.clone());
                    break;
                }
            }
            parts.arrays[t].push(object.clone());
            Ok(outcome)
        }
        Category::WizardSpell | Category::PriestSpell => {
            let spells = stores.spells()?;
            let mut outcome = Outcome::Added;
            for (c, list) in [
                (Category::WizardSpell, &mut spells.wizard),
                (Category::PriestSpell, &mut spells.priest),
            ] {
                if let Some(i) = named(list, &name) {
                    if !replace {
                        return Ok(Outcome::Skipped);
                    }
                    list.remove(i);
                    outcome = Outcome::Replaced(c, name.clone());
                }
            }
            match category {
                Category::WizardSpell => spells.wizard.push(object.clone()),
                _ => spells.priest.push(object.clone()),
            }
            Ok(outcome)
        }
        category => {
            let list = stores.list(category)?;
            let mut outcome = Outcome::Added;
            if let Some(i) = named(list, &name) {
                if !replace {
                    return Ok(Outcome::Skipped);
                }
                list.remove(i);
                outcome = Outcome::Replaced(category, name);
            }
            list.push(object.clone());
            Ok(outcome)
        }
    }
}

/// 0x49eff1: the combinations of an imported race.
fn import_race(
    stores: &mut Stores,
    file: &ExportFile,
    race: &Object,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let Object::Race(race) = race else {
        return Ok(());
    };
    let classes = stores.classes()?;
    let Some(exported) = &file.allowed else {
        // 0x47c91e: a race without a record gets one from its own lists.
        let db = stores.allowed()?;
        if db.position(&race.name).is_some() {
            return Ok(());
        }
        let mut record = empty_record(&race.name);
        for name in &race.unknown_0x1b8 {
            add_combination(&mut record, &[name], &classes, false, 0);
        }
        for o in &race.multi_classes {
            if let Object::MultiClass(m) = o {
                let names: Vec<&str> = m
                    .classes
                    .iter()
                    .filter(|&&id| id != 0)
                    .filter_map(|&id| {
                        classes
                            .iter()
                            .rev()
                            .find(|c| c.id == id)
                            .map(|c| c.name.as_str())
                    })
                    .collect();
                add_combination(&mut record, &names, &classes, false, 0);
            }
        }
        db.records.push(record);
        return Ok(());
    };
    let db = stores.allowed()?;
    db.records.retain(|r| r.race != race.name);
    let Some(source) = exported.iter().find_map(|o| match o {
        Object::ClassesAllowed(r) if r.race == race.name => Some(r),
        _ => None,
    }) else {
        warnings.push(tr!(
            "The package has no race/class combinations for the race {race}",
            race = race.name
        ));
        return Ok(());
    };
    let mut record = empty_record(&race.name);
    for helper in source.classes.iter().filter_map(helper_data) {
        add_combination(
            &mut record,
            &helper_names(helper),
            &classes,
            helper.flag_0x4 != 0,
            helper.level_limit.unwrap_or(0),
        );
    }
    db.records.push(record);
    Ok(())
}

/// 0x49f33c: the combinations naming an imported class join each race's
/// record.
fn import_class(
    stores: &mut Stores,
    file: &ExportFile,
    class: &str,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let Some(exported) = &file.allowed else {
        return Ok(());
    };
    let classes = stores.classes()?;
    if !classes.iter().any(|c| c.name == class) {
        // The class list was read before this import added the class.
        stores.classes = None;
    }
    let classes = stores.classes()?;
    let db = stores.allowed()?;
    for source in exported.iter().filter_map(|o| match o {
        Object::ClassesAllowed(r) => Some(r),
        _ => None,
    }) {
        for helper in source.classes.iter().filter_map(helper_data) {
            if !helper_slots(helper).contains(&class) {
                continue;
            }
            let (i, _) = db.ensure(&source.race);
            let before = db.records[i].classes.len();
            add_combination(
                &mut db.records[i],
                &helper_names(helper),
                &classes,
                helper.flag_0x4 != 0,
                helper.level_limit.unwrap_or(0),
            );
            if db.records[i].classes.len() == before
                && Allowed::from_names(&helper_names(helper), &classes, 0).is_none()
            {
                let warning = tr!(
                    "No race/class combinations could be added for the race {race}",
                    race = source.race
                );
                // One per race, however many combinations fail.
                if !warnings.contains(&warning) {
                    warnings.push(warning);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_parse_from_words() {
        assert_eq!(Category::parse("race"), Some(Category::Race));
        assert_eq!(Category::parse("Wizard-Spell"), Some(Category::WizardSpell));
        for category in Category::all(Flavor::Expansion) {
            assert_eq!(Category::parse(category.label()), Some(category));
        }
        assert_eq!(Category::parse("weapons"), Some(Category::Part(0)));
        assert_eq!(
            Category::parse("character kits"),
            Some(Category::Part(KITS))
        );
        assert_eq!(Category::parse("classes"), Some(Category::Class));
        assert_eq!(Category::parse("nothing"), None);
    }

    #[test]
    fn next_names_keep_eight_digits() {
        assert_eq!(next_file_name("02464229.dat"), "02464230.dat");
        assert_eq!(next_file_name("99999999.dat"), "10000000.dat");
    }
}
