//! The user databases ("Custom" data): records the user adds next to the
//! standard ones, in `UserDbas/*U.dat`.
//!
//! Every kind of record follows the same rules in both original programs:
//!
//! * lists show the standard records (`Database/X.dat`, never edited) and
//!   the user's (`UserDbas/XU.dat`) merged and sorted by name, with an
//!   All / Standard / Custom filter;
//! * only user records can be edited or deleted; New and Copy work from
//!   any record, and Copy clears the name;
//! * a name is trimmed and must be unique, ignoring case, among the
//!   standard and the user records of the same kind (per part category for
//!   parts), so a user record never shadows a standard one;
//! * the user file holds the user records in the order they were added;
//! * deleting never cascades: characters, encounters and kits keep the
//!   name.
//!
//! A kind is described once by a [`UserKind`]: its file, how a name is
//! read and checked, the defaults of a new record, and one [`FieldDesc`]
//! per control of the original's dialogs, grouped by page (one page per
//! original sub-dialog). [`UserDb`] holds the two lists of one kind and
//! does the rest. Unlike the originals, which write every user file when
//! their menu closes, the app writes the one changed file on each save;
//! the bytes are the ones the original would write for the same records.

pub mod allowed;
pub mod classes;
pub mod monsters;
pub mod parts;
pub mod powers;
pub mod program;
pub mod races;
pub mod slots;
pub mod spells;
pub mod tables;
pub mod traps;

use std::collections::HashMap;
use std::marker::PhantomData;

use cr_formats::Object;
use cr_formats::database::ObjectList;
use cr_i18n::{tr, tr_noop};

pub use crate::sheet::Flavor;
use crate::{Error, Installation, Result};

/// Where a record comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// `Database/`, read only.
    Standard,
    /// `UserDbas/`, the user's own.
    Custom,
}

/// The options combo of every list: All, Standard, Custom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Standard,
    Custom,
}

impl Filter {
    /// The combo's entries, in its order.
    pub const NAMES: [&str; 3] = [tr_noop!("All"), tr_noop!("Standard"), tr_noop!("Custom")];

    pub fn of(index: u32) -> Self {
        match index {
            1 => Self::Standard,
            2 => Self::Custom,
            _ => Self::All,
        }
    }

    pub fn accepts(self, origin: Origin) -> bool {
        match self {
            Self::All => true,
            Self::Standard => origin == Origin::Standard,
            Self::Custom => origin == Origin::Custom,
        }
    }
}

/// Catalog entry keys of standard and user records start with this:
/// `udb:<section>:<standard|custom>:<name>`.
pub const KEY: &str = "udb:";

/// The catalog key of the record `name` of a section.
pub fn key(section: &str, origin: Origin, name: &str) -> String {
    let origin = match origin {
        Origin::Standard => "standard",
        Origin::Custom => "custom",
    };
    format!("{KEY}{section}:{origin}:{name}")
}

/// The section, origin and name of a key made by [`key`].
pub fn parse_key(key: &str) -> Option<(&str, Origin, &str)> {
    let mut parts = key.strip_prefix(KEY)?.splitn(3, ':');
    let section = parts.next()?;
    let origin = match parts.next()? {
        "standard" => Origin::Standard,
        "custom" => Origin::Custom,
        _ => return None,
    };
    Some((section, origin, parts.next()?))
}

/// What lists show for where a record comes from.
pub fn origin_label(origin: Origin) -> &'static str {
    match origin {
        Origin::Standard => tr_noop!("Standard"),
        Origin::Custom => tr_noop!("Custom"),
    }
}

/// Why a name was refused, as the original's name check (0x6842cc and
/// its siblings) decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    Empty,
    /// A standard record of the same kind (and category) has the name.
    UsedByStandard,
    /// Another user record of the same kind (and category) has the name.
    Used,
    /// Longer than the field allows.
    TooLong(usize),
    /// A class name with `/`, the multi-class separator.
    Slash,
    /// A monster type name that starts with `~`.
    Reserved,
}

/// A value as an editor control shows it.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Int(i64),
    Bool(bool),
    List(Vec<String>),
    /// Count, sides and bonus.
    Dice([i64; 3]),
    /// A real number (a container's capacity in pounds).
    Real(f64),
    /// Names, each with a number (`CPartIDOb` lists: a skill and its
    /// bonus, a weapon and the level it becomes free at).
    Pairs(Vec<(String, i64)>),
    /// Choose-N-of groups (`CListOfListsOfOptions`): how many to pick and
    /// out of which names.
    Groups(Vec<(i64, Vec<String>)>),
    /// A table of numbers, one row per level.
    Grid(Vec<Vec<i64>>),
}

impl Value {
    pub fn text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Int(n) => n.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::List(l) => l.join(", "),
            Self::Dice([n, s, m]) => dice_text(*n, *s, *m),
            Self::Real(x) => x.to_string(),
            Self::Pairs(p) => p
                .iter()
                .map(|(n, v)| format!("{n} ({v})"))
                .collect::<Vec<_>>()
                .join(", "),
            Self::Groups(g) => g
                .iter()
                .map(|(n, l)| format!("{n} of [{}]", l.join(", ")))
                .collect::<Vec<_>>()
                .join("; "),
            Self::Grid(rows) => rows
                .iter()
                .map(|r| r.iter().map(i64::to_string).collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join(" / "),
        }
    }

    pub fn int(&self) -> i64 {
        match self {
            Self::Int(n) => *n,
            Self::Bool(b) => i64::from(*b),
            Self::Text(s) => s.trim().parse().unwrap_or(0),
            Self::Real(x) => *x as i64,
            _ => 0,
        }
    }

    pub fn real(&self) -> f64 {
        match self {
            Self::Real(x) => *x,
            Self::Text(s) => s.trim().parse().unwrap_or(0.0),
            v => v.int() as f64,
        }
    }

    pub fn pairs(&self) -> Vec<(String, i64)> {
        match self {
            Self::Pairs(p) => p.clone(),
            v => v.list().into_iter().map(|n| (n, 0)).collect(),
        }
    }

    pub fn groups(&self) -> Vec<(i64, Vec<String>)> {
        match self {
            Self::Groups(g) => g.clone(),
            _ => Vec::new(),
        }
    }

    pub fn grid(&self) -> Vec<Vec<i64>> {
        match self {
            Self::Grid(g) => g.clone(),
            _ => Vec::new(),
        }
    }

    pub fn bool(&self) -> bool {
        self.int() != 0
    }

    pub fn list(&self) -> Vec<String> {
        match self {
            Self::List(l) => l.clone(),
            Self::Text(s) if !s.is_empty() => vec![s.clone()],
            Self::Pairs(p) => p.iter().map(|(n, _)| n.clone()).collect(),
            _ => Vec::new(),
        }
    }

    pub fn dice(&self) -> [i64; 3] {
        match self {
            Self::Dice(d) => *d,
            _ => [0; 3],
        }
    }
}

/// `3d6+2`, `1d8`, `2` (no dice).
pub fn dice_text(n: i64, s: i64, m: i64) -> String {
    let base = if n == 0 || s == 0 {
        String::new()
    } else {
        format!("{n}d{s}")
    };
    match (base.is_empty(), m) {
        (true, m) => m.to_string(),
        (false, 0) => base,
        (false, m) => format!("{base}{m:+}"),
    }
}

/// Where a list of choices comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Options {
    Fixed(&'static [&'static str]),
    /// A list read from the installation; see [`Sources`].
    Source(&'static str),
}

/// The control a field gets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldKind {
    /// One line of text, at most `max` characters (0: no limit).
    Text {
        max: usize,
    },
    /// Several lines (descriptions, notes).
    MultiLine,
    Int {
        min: i64,
        max: i64,
    },
    Check,
    /// One of the options, as its index ([`Value::Int`]).
    Choice(Options),
    /// Text with suggestions; with `free`, any text.
    Combo {
        options: Options,
        free: bool,
    },
    /// The selected names out of the available ones
    /// (Available ⇄ Selected).
    Dual(Options),
    Dice,
    /// A check with a number: the original's "sentinel" controls, whose
    /// check is on when the number is not `none` and whose number is
    /// greyed while it is off (unchecking stores `none`, checking stores
    /// `start`, the value the original's click handler puts in). The
    /// label is the check's. With `inverted`, the check is on when the
    /// number *is* `none` ("Not limited by weight"; a negative `none`
    /// then stands for any negative number).
    OptInt {
        min: i64,
        max: i64,
        none: i64,
        start: i64,
        inverted: bool,
    },
    /// A real number ([`Value::Real`]); `none` as for [`Self::OptInt`]
    /// with `inverted`, under the label `check`.
    Real {
        min: f64,
        max: f64,
        none: Option<(f64, &'static str)>,
    },
    /// Like [`Self::Dual`] behind a check (labelled with the field's
    /// label): the check is on while the list has names, and unchecking
    /// it empties the list.
    Checked(Options),
    /// Names with a number each ([`Value::Pairs`]): the originals'
    /// AddRemoveNumString dialog (`402490`).
    Pairs {
        options: Options,
        number: &'static str,
        min: i64,
        max: i64,
    },
    /// Choose-N-of groups ([`Value::Groups`]): the ListOfListsOfOptions
    /// dialog (`4b2d50`).
    Groups(Options),
    /// One row per level ([`Value::Grid`]), the first row being level
    /// `first`.
    Grid {
        columns: &'static [&'static str],
        first: i64,
        min: i64,
        max: i64,
    },
}

/// One control of an editor: where it goes, how it looks and which part
/// of the record it reads and writes.
pub struct FieldDesc<R> {
    /// The tab, one per sub-dialog of the original ("Main", "Costs").
    pub page: &'static str,
    /// A titled group inside the page; "" for none.
    pub group: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    /// Reads the value; the second argument is [`arg`](Self::arg).
    pub getter: fn(&R, usize) -> Value,
    pub setter: fn(&mut R, usize, &Value, &Sources),
    /// Replaces the getter when the value needs the installation's lists
    /// (multi-class names by class identifier).
    pub source_getter: Option<fn(&R, &Sources) -> Value>,
    /// Passed to the getter and setter: the index of one element of an
    /// array member (a size flag, a terrain flag).
    pub arg: usize,
    /// Re-evaluated whenever a value changes (XP Value needs Magic).
    pub enabled: Option<fn(&R) -> bool>,
    /// Like `enabled`, for a rule that needs [`arg`](Self::arg).
    pub enabled_at: Option<fn(&R, usize) -> bool>,
    /// Evaluated when the editor opens (subrace pages, priest spheres).
    pub visible: Option<fn(&R) -> bool>,
}

impl<R> FieldDesc<R> {
    pub fn new(
        page: &'static str,
        label: &'static str,
        kind: FieldKind,
        getter: fn(&R, usize) -> Value,
        setter: fn(&mut R, usize, &Value, &Sources),
    ) -> Self {
        Self {
            page,
            group: "",
            label,
            kind,
            getter,
            setter,
            source_getter: None,
            arg: 0,
            enabled: None,
            enabled_at: None,
            visible: None,
        }
    }

    pub fn arg(mut self, arg: usize) -> Self {
        self.arg = arg;
        self
    }

    pub fn get(&self, record: &R) -> Value {
        (self.getter)(record, self.arg)
    }

    /// The value as the editor shows it, with the installation's lists.
    pub fn get_in(&self, record: &R, sources: &Sources) -> Value {
        match self.source_getter {
            Some(getter) => getter(record, sources),
            None => self.get(record),
        }
    }

    pub fn with_sources(mut self, getter: fn(&R, &Sources) -> Value) -> Self {
        self.source_getter = Some(getter);
        self
    }

    pub fn set(&self, record: &mut R, value: &Value, sources: &Sources) {
        (self.setter)(record, self.arg, value, sources)
    }

    pub fn group(mut self, group: &'static str) -> Self {
        self.group = group;
        self
    }

    pub fn enabled(mut self, rule: fn(&R) -> bool) -> Self {
        self.enabled = Some(rule);
        self
    }

    pub fn enabled_at(mut self, rule: fn(&R, usize) -> bool) -> Self {
        self.enabled_at = Some(rule);
        self
    }

    pub fn visible(mut self, rule: fn(&R) -> bool) -> Self {
        self.visible = Some(rule);
        self
    }

    pub fn is_enabled(&self, record: &R) -> bool {
        self.enabled.is_none_or(|rule| rule(record))
            && self.enabled_at.is_none_or(|rule| rule(record, self.arg))
    }

    pub fn is_visible(&self, record: &R) -> bool {
        self.visible.is_none_or(|rule| rule(record))
    }
}

/// Field descriptors for plain struct fields of the record type `Rec`,
/// which the calling scope names (`type Rec = Spell;`): `text!(page, label, max,
/// field)`, `int!(page, label, min, max, field)`, `check!(page, label,
/// field)` and `multiline!(page, label, field)`. The field may be a path
/// (`head.name`, `limits[0][1]`).
#[macro_export]
macro_rules! udb_text {
    ($page:expr, $label:expr, $max:expr, $($f:tt)+) => {
        $crate::userdb::FieldDesc::new(
            $page,
            $label,
            $crate::userdb::FieldKind::Text { max: $max },
            |r: &Rec, _| $crate::userdb::Value::Text(r.$($f)+.clone()),
            |r, _, v, _| r.$($f)+ = v.text(),
        )
    };
}

#[macro_export]
macro_rules! udb_multiline {
    ($page:expr, $label:expr, $($f:tt)+) => {
        $crate::userdb::FieldDesc::new(
            $page,
            $label,
            $crate::userdb::FieldKind::MultiLine,
            |r: &Rec, _| $crate::userdb::Value::Text(r.$($f)+.clone()),
            |r, _, v, _| r.$($f)+ = v.text(),
        )
    };
}

#[macro_export]
macro_rules! udb_int {
    ($page:expr, $label:expr, $min:expr, $max:expr, $($f:tt)+) => {
        $crate::userdb::FieldDesc::new(
            $page,
            $label,
            $crate::userdb::FieldKind::Int { min: $min, max: $max },
            |r: &Rec, _| $crate::userdb::Value::Int(r.$($f)+ as i64),
            |r, _, v, _| r.$($f)+ = v.int() as _,
        )
    };
}

#[macro_export]
macro_rules! udb_check {
    ($page:expr, $label:expr, $($f:tt)+) => {
        $crate::userdb::FieldDesc::new(
            $page,
            $label,
            $crate::userdb::FieldKind::Check,
            |r: &Rec, _| $crate::userdb::Value::Bool(r.$($f)+ != 0),
            |r, _, v, _| r.$($f)+ = v.bool() as _,
        )
    };
}

#[macro_export]
macro_rules! udb_list {
    ($page:expr, $label:expr, $options:expr, $($f:tt)+) => {
        $crate::userdb::FieldDesc::new(
            $page,
            $label,
            $crate::userdb::FieldKind::Dual($options),
            |r: &Rec, _| $crate::userdb::Value::List(r.$($f)+.clone()),
            |r, _, v, _| r.$($f)+ = v.list(),
        )
    };
}

#[macro_export]
macro_rules! udb_combo {
    ($page:expr, $label:expr, $options:expr, $free:expr, $($f:tt)+) => {
        $crate::userdb::FieldDesc::new(
            $page,
            $label,
            $crate::userdb::FieldKind::Combo { options: $options, free: $free },
            |r: &Rec, _| $crate::userdb::Value::Text(r.$($f)+.clone()),
            |r, _, v, _| r.$($f)+ = v.text(),
        )
    };
}

pub use crate::{udb_check, udb_combo, udb_int, udb_list, udb_multiline, udb_text};

/// What a new record starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewContext {
    pub flavor: Flavor,
    /// The list's current scope: spell type (0 wizard, 1 priest), part
    /// type.
    pub scope: usize,
    /// The record the new one belongs to: the monster type selected in the
    /// Monsters book, the parent race of a subrace.
    pub parent: String,
}

/// One kind of record with a user file.
pub trait UserKind: 'static {
    type Rec: Clone + std::fmt::Debug + PartialEq + 'static;
    /// "Spell", used in messages.
    const NOUN: &'static str;
    /// `UserDbas/SpellsU.dat`.
    const FILE: &'static str;
    /// Longest name the name field takes (DDV_MaxChars).
    const MAX_NAME: usize = 50;

    fn name(record: &Self::Rec) -> &str;
    fn set_name(record: &mut Self::Rec, name: &str);

    /// The group names must be unique in (spell type, part category).
    fn scope(_record: &Self::Rec) -> usize {
        0
    }

    /// The standard records, `Database/X.dat`.
    fn standard(install: &Installation) -> Result<Vec<Self::Rec>>;
    /// The user records, from the bytes of the user file.
    fn parse_user(data: &[u8]) -> std::result::Result<Vec<Self::Rec>, cr_formats::Error>;
    /// The user file for `records`, in their order.
    fn user_bytes(records: &[Self::Rec]) -> std::result::Result<Vec<u8>, cr_formats::Error>;

    /// A new record, as the original's constructor and New handler leave
    /// it.
    fn new_record(context: &NewContext) -> Self::Rec;
    /// What Copy changes besides clearing the name.
    fn copied(record: &mut Self::Rec) {
        let _ = record;
    }

    /// Fills in what the original sets on OK before storing the record.
    fn prepare(record: &mut Self::Rec) {
        let _ = record;
    }

    /// The editor's controls for records of `flavor`.
    fn fields(flavor: Flavor) -> Vec<FieldDesc<Self::Rec>>;

    /// Checks beyond emptiness, length and uniqueness.
    fn check_name(_name: &str) -> Option<NameError> {
        None
    }

    /// The message the original shows for `error`; `adding` is false when
    /// an existing record is being changed.
    fn name_message(error: &NameError, record: &Self::Rec, adding: bool) -> String {
        let _ = (record, adding);
        tr!(
            "{noun} not added: Error. {reason}",
            noun = tr(Self::NOUN),
            reason = default_reason(error)
        )
    }

    /// The warning shown before a delete, when the original asks.
    fn delete_warning(_record: &Self::Rec) -> Option<String> {
        None
    }

    /// One line under the name in lists.
    fn summary(_record: &Self::Rec) -> String {
        String::new()
    }
}

/// The reasons the original gives (§1.3 of the spec).
pub fn default_reason(error: &NameError) -> String {
    match error {
        NameError::Empty => tr("A name must be supplied.").into(),
        NameError::UsedByStandard | NameError::Used => tr("Name already used.").into(),
        NameError::TooLong(max) => tr!("The name can have at most {max} characters.", max),
        NameError::Slash => tr(
            "The slash character is not allowed as a part of the class name.  \
             Please choose a different character.",
        )
        .into(),
        NameError::Reserved => tr(
            "Monster Type names beginning with '~' are reserved.\n Please enter a different name",
        )
        .into(),
    }
}

/// The standard and user records of one kind.
#[derive(Debug, Clone, PartialEq)]
pub struct UserDb<K: UserKind> {
    pub flavor: Flavor,
    pub standard: Vec<K::Rec>,
    /// In file order.
    pub user: Vec<K::Rec>,
    kind: PhantomData<K>,
}

/// One record of a merged list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListEntry<'a, R> {
    pub origin: Origin,
    /// Index in its own list (standard or user).
    pub index: usize,
    pub record: &'a R,
}

impl<K: UserKind> UserDb<K> {
    pub fn new(flavor: Flavor, standard: Vec<K::Rec>, user: Vec<K::Rec>) -> Self {
        Self {
            flavor,
            standard,
            user,
            kind: PhantomData,
        }
    }

    /// Reads both lists; a missing user file is an empty list.
    pub fn load(install: &Installation) -> Result<Self> {
        let standard = K::standard(install)?;
        let user = match install.path(K::FILE) {
            Some(path) => K::parse_user(&install.read(K::FILE)?)
                .map_err(|source| Error::Format { path, source })?,
            None => Vec::new(),
        };
        Ok(Self::new(
            crate::encounter::flavor_of(install),
            standard,
            user,
        ))
    }

    /// Both lists merged and sorted by name, ignoring case, keeping only
    /// what `filter` accepts.
    pub fn list(&self, filter: Filter) -> Vec<ListEntry<'_, K::Rec>> {
        let mut entries: Vec<ListEntry<'_, K::Rec>> = [
            (Origin::Standard, &self.standard),
            (Origin::Custom, &self.user),
        ]
        .into_iter()
        .filter(|(origin, _)| filter.accepts(*origin))
        .flat_map(|(origin, records)| {
            records
                .iter()
                .enumerate()
                .map(move |(index, record)| ListEntry {
                    origin,
                    index,
                    record,
                })
        })
        .collect();
        entries.sort_by_cached_key(|e| crate::collate::sort_key(K::name(e.record)));
        entries
    }

    /// The record named `name` in `scope`, user records first.
    pub fn find(&self, scope: usize, name: &str) -> Option<(Origin, usize)> {
        let matches = |r: &K::Rec| K::scope(r) == scope && K::name(r) == name;
        if let Some(i) = self.user.iter().position(matches) {
            return Some((Origin::Custom, i));
        }
        self.standard
            .iter()
            .position(matches)
            .map(|i| (Origin::Standard, i))
    }

    pub fn get(&self, origin: Origin, index: usize) -> Option<&K::Rec> {
        match origin {
            Origin::Standard => self.standard.get(index),
            Origin::Custom => self.user.get(index),
        }
    }

    /// Checks the name of `record` as the original does before adding it
    /// or, with `editing`, before replacing user record `editing`.
    pub fn validate_name(
        &self,
        record: &K::Rec,
        editing: Option<usize>,
    ) -> std::result::Result<(), NameError> {
        let name = K::name(record).trim();
        if name.is_empty() {
            return Err(NameError::Empty);
        }
        if name.chars().count() > K::MAX_NAME {
            return Err(NameError::TooLong(K::MAX_NAME));
        }
        if let Some(error) = K::check_name(name) {
            return Err(error);
        }
        let scope = K::scope(record);
        let clash =
            |r: &K::Rec| K::scope(r) == scope && K::name(r).trim().eq_ignore_ascii_case(name);
        // A user record that came in with a standard record's name (an
        // import does not look at standard names) can still be edited as
        // long as it keeps that name; the original would refuse it.
        let kept = editing
            .and_then(|i| self.user.get(i))
            .is_some_and(|old| K::name(old).trim().eq_ignore_ascii_case(name));
        if !kept && self.standard.iter().any(clash) {
            return Err(NameError::UsedByStandard);
        }
        if self
            .user
            .iter()
            .enumerate()
            .any(|(i, r)| Some(i) != editing && clash(r))
        {
            return Err(NameError::Used);
        }
        Ok(())
    }

    /// The message for a refused name, as the original words it.
    pub fn name_message(&self, error: &NameError, record: &K::Rec, adding: bool) -> String {
        K::name_message(error, record, adding)
    }

    /// Appends `record` (its name trimmed) to the user list; returns its
    /// index.
    pub fn add(&mut self, mut record: K::Rec) -> std::result::Result<usize, NameError> {
        let name = K::name(&record).trim().to_owned();
        K::set_name(&mut record, &name);
        self.validate_name(&record, None)?;
        K::prepare(&mut record);
        self.user.push(record);
        Ok(self.user.len() - 1)
    }

    /// Replaces user record `index`, keeping its place in the file.
    pub fn replace(
        &mut self,
        index: usize,
        mut record: K::Rec,
    ) -> std::result::Result<(), NameError> {
        let name = K::name(&record).trim().to_owned();
        K::set_name(&mut record, &name);
        self.validate_name(&record, Some(index))?;
        K::prepare(&mut record);
        self.user[index] = record;
        Ok(())
    }

    /// Removes user record `index`; nothing that refers to it changes.
    pub fn delete(&mut self, index: usize) -> K::Rec {
        self.user.remove(index)
    }

    /// A new record for the editor.
    pub fn new_record(&self, scope: usize, parent: &str) -> K::Rec {
        K::new_record(&NewContext {
            flavor: self.flavor,
            scope,
            parent: parent.to_owned(),
        })
    }

    /// What Copy opens the editor on: a clone with the name cleared.
    pub fn copy_of(&self, record: &K::Rec) -> K::Rec {
        let mut copy = record.clone();
        K::set_name(&mut copy, "");
        K::copied(&mut copy);
        copy
    }

    pub fn user_bytes(&self) -> Result<Vec<u8>> {
        K::user_bytes(&self.user).map_err(|source| Error::Format {
            path: K::FILE.into(),
            source,
        })
    }

    /// Writes the user file, keeping the old one as `.bak`.
    pub fn save(&self, install: &Installation) -> Result<()> {
        let bytes = self.user_bytes()?;
        let file = K::FILE.rsplit('/').next().unwrap_or(K::FILE);
        crate::encounter::replace_or_create(install, K::FILE, file, &bytes)
    }
}

/// Lists of names an editor offers, read from the installation: schools,
/// spheres, powers, attack types, monster types and the like.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    lists: HashMap<String, Vec<String>>,
    /// The attack types (`Attack.dat` then `AttackU.dat`), for the monster
    /// Attacks page.
    pub attacks: Vec<cr_formats::monster::MonsterAttack>,
    /// Class names, identifiers and groups, for multi-classes.
    pub classes: Vec<classes::ClassEntry>,
    /// Every spell (standard then user), for lists that keep copies of
    /// them (a kit's bonus spells).
    pub spells: Vec<cr_formats::spells::Spell>,
}

impl Sources {
    pub fn list(&self, name: &str) -> &[String] {
        self.lists.get(name).map_or(&[], Vec::as_slice)
    }

    pub fn set(&mut self, name: &str, items: Vec<String>) {
        self.lists.insert(name.to_owned(), items);
    }

    /// The choices of `options`.
    pub fn options(&self, options: Options) -> Vec<String> {
        match options {
            Options::Fixed(items) => items.iter().map(|s| (*s).to_owned()).collect(),
            Options::Source(name) => self.list(name).to_vec(),
        }
    }

    /// Loads every list the fields name.
    pub fn load<R>(install: &Installation, fields: &[FieldDesc<R>]) -> Result<Self> {
        let mut names: Vec<&str> = fields
            .iter()
            .filter_map(|f| match f.kind {
                FieldKind::Choice(Options::Source(n))
                | FieldKind::Combo {
                    options: Options::Source(n),
                    ..
                }
                | FieldKind::Dual(Options::Source(n))
                | FieldKind::Checked(Options::Source(n))
                | FieldKind::Groups(Options::Source(n))
                | FieldKind::Pairs {
                    options: Options::Source(n),
                    ..
                } => Some(n),
                _ => None,
            })
            .collect();
        names.sort_unstable();
        names.dedup();
        let mut sources = Self::default();
        let mut parts = None;
        for name in names {
            let items = match name {
                "powers" => names_of(&install.list_objects("Psionic")?, |o| match o {
                    Object::PsionicPower(p) => Some(p.name.clone()),
                    _ => None,
                }),
                "attacks" => {
                    sources.attacks = attack_types(install)?;
                    sources.attacks.iter().map(|a| a.name.clone()).collect()
                }
                "monster-types" => names_of(&install.list_objects("MonType")?, |o| match o {
                    Object::MonsterType(t) => Some(t.name.clone()),
                    _ => None,
                }),
                "classes" => {
                    sources.classes = classes::class_list(install)?;
                    sources.classes.iter().map(|c| c.name.clone()).collect()
                }
                "multiclasses" => {
                    sources.classes = classes::class_list(install)?;
                    classes::multiclass_names(&sources.classes)
                }
                "main-races" => names_of(&install.list_objects("Race")?, |o| match o {
                    Object::Race(r) if r.unknown_0x184 == 0 => Some(r.name.clone()),
                    _ => None,
                }),
                "races" => names_of(&install.list_objects("Race")?, |o| match o {
                    Object::Race(r) => Some(r.name.clone()),
                    _ => None,
                }),
                "treasure-tables" => tables::TableDb::load(install)?
                    .tables
                    .iter()
                    .map(|t| t.name.clone())
                    .collect(),
                "weapon-groups" => parts::weapon_groups(install)?,
                "standard-classes" => {
                    sources.classes = classes::class_list(install)?;
                    sources
                        .classes
                        .iter()
                        .filter(|c| c.id != 0)
                        .map(|c| c.name.clone())
                        .collect()
                }
                "priest-spells" => {
                    let mut names: Vec<String> = spells::all_spells(install)?
                        .into_iter()
                        .filter(|s| s.priest != 0)
                        .map(|s| s.name)
                        .collect();
                    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
                    names.dedup();
                    names
                }
                "weapon-proficiencies" => parts::weapon_proficiencies(install)?,
                "rogue-skills" => parts::rogue_skills(install)?,
                "class-combinations" => {
                    sources.classes = classes::class_list(install)?;
                    let mut names: Vec<String> =
                        sources.classes.iter().map(|c| c.name.clone()).collect();
                    names.extend(classes::multiclass_names(&sources.classes));
                    names
                }
                "spells" => {
                    sources.spells = spells::all_spells(install)?;
                    let mut names: Vec<String> =
                        sources.spells.iter().map(|s| s.name.clone()).collect();
                    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
                    names.dedup();
                    names
                }
                n if program::is_program_list(n) => {
                    program::program_list(install, crate::encounter::flavor_of(install), n)?
                }
                n if n.starts_with("monster-") => monsters::distinct(install, n)?,
                n => match parts::source_type(n) {
                    Some(t) => {
                        if parts.is_none() {
                            parts = Some(parts::all_names(install)?);
                        }
                        parts.as_ref().map(|p| p[t].clone()).unwrap_or_default()
                    }
                    None => Vec::new(),
                },
            };
            sources.set(name, items);
        }
        Ok(sources)
    }
}

/// Sorted, deduplicated names of `objects`.
fn names_of(objects: &[Object], name: impl Fn(&Object) -> Option<String>) -> Vec<String> {
    let mut names: Vec<String> = objects.iter().filter_map(name).collect();
    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
    names.dedup();
    names
}

/// The attack types, `Database/Attack.dat` then `UserDbas/AttackU.dat`.
pub fn attack_types(install: &Installation) -> Result<Vec<cr_formats::monster::MonsterAttack>> {
    Ok(install
        .list_objects("Attack")?
        .into_iter()
        .filter_map(|o| match o {
            Object::MonsterAttack(a) => Some(*a),
            _ => None,
        })
        .collect())
}

/// The objects of a `CObList` file, for kinds stored that way.
pub(crate) fn parse_list(data: &[u8]) -> std::result::Result<Vec<Object>, cr_formats::Error> {
    ObjectList::parse(data).map(|l| l.objects)
}

pub(crate) fn list_bytes(objects: Vec<Object>) -> std::result::Result<Vec<u8>, cr_formats::Error> {
    ObjectList { objects }.to_bytes()
}

/// The standard records of a `CObList` database file, `Database/<name>.dat`.
pub(crate) fn standard_list<R>(
    install: &Installation,
    name: &str,
    pick: impl Fn(Object) -> Option<R>,
) -> Result<Vec<R>> {
    let relative = format!("Database/{name}.dat");
    let Some(path) = install.path(&relative) else {
        return Ok(Vec::new());
    };
    let objects =
        parse_list(&install.read(&relative)?).map_err(|source| Error::Format { path, source })?;
    Ok(objects.into_iter().filter_map(pick).collect())
}

#[cfg(test)]
pub(crate) mod testing {
    //! A throwaway installation folder for the tests of each kind.

    use std::fs;
    use std::path::PathBuf;

    use crate::Installation;

    pub struct TempInstall {
        pub root: PathBuf,
        pub install: Installation,
    }

    impl TempInstall {
        /// An installation with `files` (relative path, bytes).
        pub fn new(name: &str, files: &[(&str, Vec<u8>)]) -> Self {
            let root =
                std::env::temp_dir().join(format!("cr-userdb-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("Database")).unwrap();
            fs::create_dir_all(root.join("UserDbas")).unwrap();
            for (path, bytes) in files {
                fs::write(root.join(path), bytes).unwrap();
            }
            let install = Installation::open(&root).unwrap();
            Self { root, install }
        }
    }

    impl Drop for TempInstall {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dice_text_reads_like_the_originals() {
        assert_eq!(dice_text(3, 6, 0), "3d6");
        assert_eq!(dice_text(1, 4, 2), "1d4+2");
        assert_eq!(dice_text(0, 0, 1), "1");
        assert_eq!(dice_text(2, 8, -1), "2d8-1");
    }

    #[test]
    fn keys_round_trip() {
        let k = key("weapons", Origin::Custom, "Sword: long");
        assert_eq!(
            parse_key(&k),
            Some(("weapons", Origin::Custom, "Sword: long"))
        );
        assert_eq!(parse_key("table:x"), None);
    }

    #[test]
    fn filter_accepts_by_origin() {
        assert!(Filter::All.accepts(Origin::Standard));
        assert!(Filter::of(1).accepts(Origin::Standard));
        assert!(!Filter::of(1).accepts(Origin::Custom));
        assert!(Filter::of(2).accepts(Origin::Custom));
    }
}
