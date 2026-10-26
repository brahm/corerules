//! The record editor with no toolkit in it: the original's dialog for a
//! record of a user database, as pages of the fields its kind describes
//! ([`FieldDesc`]), with the questions it asks while a value changes and
//! before saving, and the small dialogs some fields open (see
//! [`super::dialogs`]). An interface draws the fields and the dialog in
//! view and passes back what the user did; saving runs as a job on the
//! installation, away from the window.

use std::sync::Arc;

use cr_formats::monster::MonsterAttack;
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::help::topic;
use cr_store::treasure::{Bundle, TreasureRules};
use cr_store::userdb::classes::{self, levels};
use cr_store::userdb::monsters::{ADD_HEADING, add_problem};
use cr_store::userdb::{
    self as udb, FieldDesc, FieldKind, NameError, Options, Origin, Sources, UserDb, UserKind, Value,
};

use super::Kind;
use super::dialogs::{
    AbilityEdit, AttackEdit, Dialog, DialogInput, FreeSpellsEdit, GroupEdit, NwpsEdit,
};
use crate::bundle::{BundleEditor, BundleInput};
use crate::prompt::{self, Answer, PromptKind};

/// A question or message the record editor waits on.
pub type Prompt = prompt::Prompt<RecordForm>;

/// A page of the editor, one per sub-dialog of the original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Its title, untranslated.
    pub name: &'static str,
    /// Its groups, in the order of their first field.
    pub groups: Vec<Group>,
}

/// A group of fields on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Its heading, untranslated; empty for none.
    pub title: &'static str,
    /// Its fields, by index.
    pub fields: Vec<usize>,
}

/// Whether a field of `kind` has a group of its own, headed with its
/// label: lists and long text.
pub fn own_group(kind: &FieldKind) -> bool {
    matches!(
        kind,
        FieldKind::Dual(_)
            | FieldKind::MultiLine
            | FieldKind::Checked(_)
            | FieldKind::Pairs { .. }
            | FieldKind::Groups(_)
            | FieldKind::Grid { .. }
            | FieldKind::Items
            | FieldKind::Attacks
            | FieldKind::FreeSpells
            | FieldKind::LevelNwps(_)
            | FieldKind::LevelAbilities(_)
            | FieldKind::AutoFill
    )
}

/// The editor's pages for `fields`: the pages in the order of their
/// first field, each with its groups. A field of a group of its own (see
/// [`own_group`]) is headed with its label, unless it names a group or
/// its label is the page's.
pub fn layout<R>(fields: &[FieldDesc<R>]) -> Vec<Page> {
    let described: Vec<(&'static str, &'static str, &'static str, FieldKind)> = fields
        .iter()
        .map(|f| (f.page, f.group, f.label, f.kind))
        .collect();
    layout_of(&described)
}

fn layout_of(fields: &[(&'static str, &'static str, &'static str, FieldKind)]) -> Vec<Page> {
    // Each page with the names its groups go by.
    let mut pages: Vec<(Page, Vec<&'static str>)> = Vec::new();
    for (index, &(page, group, label, kind)) in fields.iter().enumerate() {
        let at = match pages.iter().position(|(p, _)| p.name == page) {
            Some(at) => at,
            None => {
                pages.push((
                    Page {
                        name: page,
                        groups: Vec::new(),
                    },
                    Vec::new(),
                ));
                pages.len() - 1
            }
        };
        let (page, keys) = &mut pages[at];
        let own = own_group(&kind);
        let key = if own && group.is_empty() {
            label
        } else {
            group
        };
        let slot = if own {
            None
        } else {
            keys.iter().position(|k| *k == key)
        };
        match slot {
            Some(slot) => page.groups[slot].fields.push(index),
            None => {
                let title = if !key.is_empty() && !(key == page.name && own) {
                    key
                } else {
                    ""
                };
                page.groups.push(Group {
                    title,
                    fields: vec![index],
                });
                keys.push(key);
            }
        }
    }
    pages.into_iter().map(|(page, _)| page).collect()
}

/// A field as the editor shows it now.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Its label, untranslated.
    pub label: &'static str,
    pub kind: FieldKind,
    pub value: Value,
    pub visible: bool,
    /// Whether it may be changed: its rule allows it, and an editor with
    /// only some fields open has it open.
    pub enabled: bool,
    /// The label of the button that offers a suggested value, untranslated.
    pub suggest: Option<&'static str>,
    /// The check of a number or a list behind one: on while a
    /// [`FieldKind::OptInt`]'s number counts (off when `inverted`), while
    /// a [`FieldKind::Real`] is not limited, and while a
    /// [`FieldKind::Checked`] list is in use.
    pub checked: bool,
}

impl Field {
    /// Whether the number of an [`FieldKind::OptInt`] or
    /// [`FieldKind::Real`], or the list of a [`FieldKind::Checked`], may
    /// be changed.
    pub fn number_open(&self) -> bool {
        self.enabled
            && match self.kind {
                FieldKind::OptInt { inverted, .. } => self.checked != inverted,
                FieldKind::Real { none: Some(_), .. } => !self.checked,
                FieldKind::Checked(_) => self.checked,
                _ => true,
            }
    }
}

/// Whether an [`FieldKind::OptInt`] holding `value` has its check on.
fn opt_int_checked(value: i64, none: i64, inverted: bool) -> bool {
    // A "not limited" check is on for any negative number when the
    // limit-free value is negative (the Container page tests `< 0`).
    let numbered = value != none && !(inverted && none < 0 && value < 0);
    numbered != inverted
}

/// Whether a [`FieldKind::Real`] holding `value` is not limited.
fn real_unlimited(value: f64, none: f64) -> bool {
    if none < 0.0 {
        value < 0.0
    } else {
        value == none
    }
}

/// What the editor knows of a field without the record.
#[derive(Debug, Clone)]
struct Info {
    label: &'static str,
    kind: FieldKind,
    suggest: Option<&'static str>,
}

/// Saving, run on the installation away from the window; what it gives
/// goes to [`RecordForm::saved`].
pub type SaveJob = Arc<dyn Fn(&Installation) -> Result<Saved, String> + Send + Sync>;

/// What saving did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    /// The record was saved under `name`; `key` is its entry's.
    Done { name: String, key: String },
    /// Its name was refused: the message's title and text.
    Refused { title: String, text: String },
}

/// What the app should do for the editor.
#[derive(Clone)]
pub enum RecordEffect {
    /// Run the job and pass what it gives to [`RecordForm::saved`].
    Save(SaveJob),
    /// The record was saved: leave the editor for the entry with `key` of
    /// `section`, saying `said`, then tell `told` (a title and a text) if
    /// there is something to tell.
    Done {
        section: &'static str,
        key: String,
        name: String,
        said: String,
        told: Option<(String, String)>,
    },
}

/// One record of some kind, with its fields, as the editor holds it.
trait Record: Send + Sync {
    fn boxed(&self) -> Box<dyn Record>;
    fn value(&self, field: usize) -> Value;
    fn visible(&self, field: usize) -> bool;
    fn enabled(&self, field: usize) -> bool;
    /// The value the field's suggestion offers, and the question asked
    /// with it.
    fn suggestion(&self, field: usize) -> Option<(i64, String)>;
    /// Sets the field's value; the question the original asks about the
    /// change, when `asking` and it asks one.
    fn set(&mut self, field: usize, value: &Value, asking: bool) -> Option<&'static str>;
    /// Puts back the field's stored value, after No to its question.
    fn put_back(&mut self, field: usize);
    fn changed(&self) -> bool;
    fn name(&self) -> String;
    fn save_warning(&self) -> Option<String>;
    fn job(&self, section: &'static str, editing: Option<usize>, detach: bool) -> SaveJob;
    /// The job of an editor with only some fields open, which saves them
    /// in the record where it is, at `origin`.
    fn job_only(&self, section: &'static str, origin: Origin) -> Option<SaveJob>;
}

impl Clone for Box<dyn Record> {
    fn clone(&self) -> Self {
        self.boxed()
    }
}

/// Saves the open fields of a record where it is.
type SaveOnly<R> = fn(&Installation, Origin, &R) -> cr_store::Result<()>;

struct Of<K: UserKind> {
    fields: Arc<Vec<FieldDesc<K::Rec>>>,
    sources: Arc<Sources>,
    record: K::Rec,
    /// The record as the editor opened it.
    kept: K::Rec,
    /// The record as stored, when it is changed rather than added.
    stored: Option<K::Rec>,
    save_only: Option<SaveOnly<K::Rec>>,
}

impl<K: UserKind> Record for Of<K>
where
    K::Rec: Send + Sync,
{
    fn boxed(&self) -> Box<dyn Record> {
        Box::new(Of::<K> {
            fields: self.fields.clone(),
            sources: self.sources.clone(),
            record: self.record.clone(),
            kept: self.kept.clone(),
            stored: self.stored.clone(),
            save_only: self.save_only,
        })
    }

    fn value(&self, field: usize) -> Value {
        self.fields[field].get_in(&self.record, &self.sources)
    }

    fn visible(&self, field: usize) -> bool {
        self.fields[field].is_visible(&self.record)
    }

    fn enabled(&self, field: usize) -> bool {
        self.fields[field].is_enabled(&self.record)
    }

    fn suggestion(&self, field: usize) -> Option<(i64, String)> {
        let suggestion = self.fields[field].suggest.as_ref()?;
        let value = (suggestion.value)(&self.record);
        Some((value, (suggestion.question)(value)))
    }

    fn set(&mut self, field: usize, value: &Value, asking: bool) -> Option<&'static str> {
        self.fields[field].set(&mut self.record, value, &self.sources);
        let stored = self.stored.as_ref().filter(|_| asking)?;
        K::change_question(stored, &self.record)
    }

    fn put_back(&mut self, field: usize) {
        if let Some(stored) = &self.stored {
            let value = self.fields[field].get(stored);
            self.fields[field].set(&mut self.record, &value, &self.sources);
        }
    }

    fn changed(&self) -> bool {
        self.record != self.kept
    }

    fn name(&self) -> String {
        K::name(&self.record).to_owned()
    }

    fn save_warning(&self) -> Option<String> {
        K::save_warning(&self.record)
    }

    fn job(&self, section: &'static str, editing: Option<usize>, detach: bool) -> SaveJob {
        let record = self.record.clone();
        Arc::new(move |install| {
            let mut db = UserDb::<K>::load(install).map_err(|e| e.to_string())?;
            // What the record was, for what a change takes along.
            let before = editing.and_then(|i| db.user.get(i).cloned());
            let stored = match editing {
                Some(i) if i < db.user.len() => db.replace(i, record.clone()),
                _ => db.add(record.clone()),
            };
            match stored {
                Ok(i) => {
                    db.save(install).map_err(|e| e.to_string())?;
                    if let Some(before) = before.filter(|_| detach) {
                        K::detach(install, &before).map_err(|e| e.to_string())?;
                    }
                    let name = K::name(&db.user[i]).to_owned();
                    Ok(Saved::Done {
                        key: udb::key(section, Origin::Custom, &name),
                        name,
                    })
                }
                Err(error) => Ok(Saved::Refused {
                    title: if error == NameError::Slash {
                        tr(classes::SLASH_TITLE)
                    } else {
                        tr("Not Saved")
                    }
                    .to_owned(),
                    text: K::name_message(&error, &record, editing.is_none()),
                }),
            }
        })
    }

    fn job_only(&self, section: &'static str, origin: Origin) -> Option<SaveJob> {
        let save = self.save_only?;
        let record = self.record.clone();
        Some(Arc::new(move |install| {
            save(install, origin, &record).map_err(|e| e.to_string())?;
            let name = K::name(&record).to_owned();
            Ok(Saved::Done {
                key: udb::key(section, origin, &name),
                name,
            })
        }))
    }
}

/// What opens a record editor: the record and where it comes from.
pub struct Opening<K: UserKind> {
    pub section: &'static str,
    pub kind: Kind,
    pub title: String,
    pub record: K::Rec,
    /// Its index in the user list, when it is changed rather than added.
    pub editing: Option<usize>,
}

/// An editor with only some fields open, saving them its own way:
/// Schools… on a spell.
pub struct Only<R> {
    pub labels: &'static [&'static str],
    /// Where the record is.
    pub origin: Origin,
    pub save: SaveOnly<R>,
}

/// The record editor.
#[derive(Clone)]
pub struct RecordForm {
    section: &'static str,
    kind: Kind,
    title: String,
    sources: Arc<Sources>,
    fields: Arc<Vec<Info>>,
    pages: Arc<Vec<Page>>,
    record: Box<dyn Record>,
    editing: Option<usize>,
    /// The labels of the only fields open, and where the record is.
    only: Option<(&'static [&'static str], Origin)>,
    /// A change the original asks about was accepted: saving takes the
    /// stored record out of the other files.
    detach: bool,
    /// The [`FieldKind::Checked`] fields whose check the user turned on,
    /// which stay on before a name is chosen.
    wanted: Vec<usize>,
    /// The page shown, by index.
    pub page: usize,
    /// The small dialog in view.
    pub dialog: Option<Dialog>,
    /// Counts the dialogs opened and the times one stayed open after OK,
    /// for an app whose dialog closes on OK to open it again.
    pub dialog_serial: u64,
    /// Editing the items of an [`FieldKind::Items`] field.
    pub items: Option<BundleEditor>,
    /// What the Add box of each list field has chosen, until it is added.
    adding: Vec<(usize, String)>,
    /// The first row each [`FieldKind::Grid`] field shows.
    grid_rows: Vec<(usize, usize)>,
    prompt: Option<Prompt>,
    serial: u64,
    effects: Vec<RecordEffect>,
    /// Why the record could not be saved.
    pub error: Option<String>,
    /// Save is under way.
    pub saving: bool,
}

/// The rows a [`FieldKind::Grid`] field shows at a time.
pub const GRID_ROWS: usize = 10;

/// A change in the editor, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordInput {
    /// Show the page with this index.
    Page(usize),
    /// A field's new value.
    Set(usize, Value),
    /// A choice of a [`FieldKind::Choice`] or [`FieldKind::Combo`] field,
    /// by its index among [`RecordForm::choices`].
    Choose(usize, usize),
    /// The check of a field's number or list (see [`Field::checked`]).
    Check(usize, bool),
    /// What the Add box of a list field chose.
    Adding(usize, String),
    /// Add on a list field: the name its box chose.
    Add(usize),
    /// Remove the name with this index from a list field.
    Remove(usize, usize),
    /// The number of a list field's name, by its index.
    Number(usize, usize, i64),
    /// The suggestion of a number field.
    Suggest(usize),
    /// The rows a grid field shows, from this one.
    GridRows(usize, usize),
    /// A cell of a grid field: its row and column, and the number.
    Cell(usize, usize, usize, i64),
    /// Open a field's dialog on its entry with this index, or on a new
    /// one: a group, a level, an attack.
    Open(usize, Option<usize>),
    /// Remove a field's entry with this index: a group, a level, an
    /// attack.
    RemoveEntry(usize, usize),
    /// Add the attack type with this name to an attacks field.
    AddAttack(usize, String),
    /// Auto Fill from the class with this name.
    AutoFill(usize, String),
    /// A change in the dialog in view.
    Dialog(DialogInput),
    /// OK (true) or Cancel on the dialog in view.
    CloseDialog(bool),
    /// An edit of the items.
    Items(BundleInput),
    /// The answer to the items' dialog with this number.
    ItemsAnswer(u64, bool),
    Save,
}

impl RecordForm {
    /// The editor of `opening`'s record of kind `K`, with only `only`'s
    /// fields open if given; it reads the lists the fields offer and, for
    /// items, the treasure tables.
    pub fn open<K: UserKind>(
        install: &Installation,
        opening: Opening<K>,
        only: Option<Only<K::Rec>>,
    ) -> cr_store::Result<Self>
    where
        K::Rec: Send + Sync,
    {
        let flavor = cr_store::encounter::flavor_of(install);
        let fields = K::fields(flavor);
        let sources = Arc::new(Sources::load(install, &fields)?);
        let rules = if fields.iter().any(|f| f.kind == FieldKind::Items) {
            Some(Arc::new(install.treasure_rules()?))
        } else {
            None
        };
        Ok(Self::with::<K>(opening, only, fields, sources, rules))
    }

    /// The editor over fields and lists already read.
    pub fn with<K: UserKind>(
        opening: Opening<K>,
        only: Option<Only<K::Rec>>,
        fields: Vec<FieldDesc<K::Rec>>,
        sources: Arc<Sources>,
        rules: Option<Arc<TreasureRules>>,
    ) -> Self
    where
        K::Rec: Send + Sync,
    {
        let infos: Vec<Info> = fields
            .iter()
            .map(|f| Info {
                label: f.label,
                kind: f.kind,
                suggest: f.suggest.as_ref().map(|s| s.label),
            })
            .collect();
        let pages = layout(&fields);
        let Opening {
            section,
            kind,
            title,
            record,
            editing,
        } = opening;
        let stored = editing.map(|_| record.clone());
        let record = Of::<K> {
            fields: Arc::new(fields),
            sources: sources.clone(),
            kept: record.clone(),
            record,
            stored,
            save_only: only.as_ref().map(|o| o.save),
        };
        let mut form = Self {
            section,
            kind,
            title,
            sources,
            fields: Arc::new(infos),
            pages: Arc::new(pages),
            record: Box::new(record),
            editing,
            only: only.map(|o| (o.labels, o.origin)),
            detach: false,
            wanted: Vec::new(),
            page: 0,
            dialog: None,
            dialog_serial: 0,
            items: rules.map(BundleEditor::new),
            adding: Vec::new(),
            grid_rows: Vec::new(),
            prompt: None,
            serial: 0,
            effects: Vec::new(),
            error: None,
            saving: false,
        };
        form.page = form.first_page();
        form
    }

    /// The page the editor opens on: the first with an open field when
    /// only some are, else the first with something shown (a nonweapon
    /// proficiency has nothing on the Part page).
    fn first_page(&self) -> usize {
        let open = self.only.and_then(|(labels, _)| {
            self.pages.iter().position(|p| {
                p.groups
                    .iter()
                    .flat_map(|g| &g.fields)
                    .any(|&f| labels.contains(&self.fields[f].label))
            })
        });
        open.or_else(|| (0..self.pages.len()).find(|&p| self.page_visible(p)))
            .unwrap_or(0)
    }

    /// The catalog section of the record.
    pub fn section(&self) -> &'static str {
        self.section
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The editor's title: the record's name, or what is being made.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The help topic of the editor.
    pub fn help(&self) -> u32 {
        topic::of_editor(self.section)
    }

    /// The lists the fields offer.
    pub fn sources(&self) -> &Sources {
        &self.sources
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// Whether the page with index `page` shows any field.
    pub fn page_visible(&self, page: usize) -> bool {
        self.pages.get(page).is_some_and(|p| {
            p.groups
                .iter()
                .any(|g| g.fields.iter().any(|&f| self.record.visible(f)))
        })
    }

    /// Whether a group shows any field.
    pub fn group_visible(&self, group: &Group) -> bool {
        group.fields.iter().any(|&f| self.record.visible(f))
    }

    /// The field with index `field` as it is now.
    pub fn field(&self, field: usize) -> Field {
        let info = &self.fields[field];
        let value = self.record.value(field);
        let checked = match info.kind {
            FieldKind::OptInt { none, inverted, .. } => {
                opt_int_checked(value.int(), none, inverted)
            }
            FieldKind::Real {
                none: Some((none, _)),
                ..
            } => real_unlimited(value.real(), none),
            FieldKind::Checked(_) => !value.list().is_empty() || self.wanted.contains(&field),
            _ => false,
        };
        let open = self
            .only
            .is_none_or(|(labels, _)| labels.contains(&info.label));
        Field {
            label: info.label,
            kind: info.kind,
            visible: self.record.visible(field),
            enabled: open && self.record.enabled(field),
            suggest: info.suggest,
            checked,
            value,
        }
    }

    /// The choices of `options`.
    pub fn options(&self, options: Options) -> Vec<String> {
        self.sources.options(options)
    }

    /// What a [`FieldKind::Choice`] or [`FieldKind::Combo`] field offers,
    /// translated where they are interface words, and the one chosen. A
    /// text missing from the list is kept as a choice, first.
    pub fn choices(&self, field: usize) -> (Vec<String>, Option<usize>) {
        let (choices, _) = self.choice_values(field);
        let translate = matches!(
            self.fields[field].kind,
            FieldKind::Choice(Options::Fixed(_))
        );
        let selected = match self.record.value(field) {
            Value::Text(t) => choices.iter().position(|c| *c == t),
            v => usize::try_from(v.int()).ok().filter(|&i| i < choices.len()),
        };
        let shown = choices
            .into_iter()
            .map(|c| if translate { tr(&c).to_owned() } else { c })
            .collect();
        (shown, selected)
    }

    /// The choices of a choice field, as stored, and whether the value is
    /// their text rather than their index.
    fn choice_values(&self, field: usize) -> (Vec<String>, bool) {
        let options = match self.fields[field].kind {
            FieldKind::Choice(options) | FieldKind::Combo { options, .. } => options,
            _ => return (Vec::new(), false),
        };
        let mut choices = self.sources.options(options);
        let value = self.record.value(field);
        let as_text = matches!(value, Value::Text(_));
        if as_text {
            let current = value.text();
            if !choices.contains(&current) {
                choices.insert(0, current);
            }
        }
        (choices, as_text)
    }

    /// The names a list field may add: its options not yet chosen.
    pub fn list_choices(&self, field: usize) -> Vec<String> {
        let options = match self.fields[field].kind {
            FieldKind::Dual(options)
            | FieldKind::Checked(options)
            | FieldKind::Pairs { options, .. } => options,
            _ => return Vec::new(),
        };
        let chosen = self.record.value(field).pairs();
        self.sources
            .options(options)
            .into_iter()
            .filter(|n| !chosen.iter().any(|(c, _)| c == n))
            .collect()
    }

    /// What the Add box of a list field has chosen.
    pub fn adding(&self, field: usize) -> Option<&str> {
        self.adding
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, n)| n.as_str())
    }

    /// The first row a grid field shows.
    pub fn grid_first(&self, field: usize) -> usize {
        self.grid_rows
            .iter()
            .find(|(f, _)| *f == field)
            .map_or(0, |(_, r)| *r)
    }

    /// The attack types an attacks field may add, by name, sorted.
    pub fn attack_types(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .sources
            .attacks
            .iter()
            .map(|a| a.name.clone())
            .collect();
        names.sort_by_cached_key(|n| cr_store::collate::sort_key(n));
        names.dedup();
        names
    }

    /// The classes Auto Fill copies from.
    pub fn auto_fill_classes(&self) -> Vec<String> {
        self.sources.list("class-records").to_vec()
    }

    /// The spells free spells may give: name, whether a priest spell, and
    /// level, sorted by name.
    pub fn spells(&self) -> Vec<(String, bool, i32)> {
        let mut spells: Vec<(String, bool, i32)> = self
            .sources
            .spells
            .iter()
            .map(|s| (s.name.clone(), s.priest != 0, s.level))
            .collect();
        spells.sort_by_cached_key(|(n, _, _)| cr_store::collate::sort_key(n));
        spells
    }

    /// The entries of a field that lists them with Edit and Remove (a
    /// group, a level, an attack), as a title and a line under it.
    pub fn entries(&self, field: usize) -> Vec<(String, String)> {
        let value = self.record.value(field);
        match self.fields[field].kind {
            FieldKind::Groups(_) => value
                .groups()
                .into_iter()
                .map(|(count, names)| (tr!("Choose {count} of", count), names.join(", ")))
                .collect(),
            FieldKind::FreeSpells => match value {
                Value::FreeSpells(class, list) => list
                    .iter()
                    .map(|l| {
                        (
                            tr!("Level {level}", level = l.level),
                            levels::free_spell_lines(&class, l).join("\n"),
                        )
                    })
                    .collect(),
                _ => Vec::new(),
            },
            FieldKind::LevelNwps(_) => value
                .level_nwps()
                .iter()
                .map(|l| {
                    (
                        tr!("Level {level}", level = l.level),
                        levels::level_nwps_summary(l),
                    )
                })
                .collect(),
            FieldKind::LevelAbilities(_) => value
                .pairs()
                .into_iter()
                .map(|(name, level)| (tr!("Level {level}", level), name))
                .collect(),
            FieldKind::Attacks => value
                .attacks()
                .iter()
                .map(|a| {
                    (
                        a.name.clone(),
                        cr_store::userdb::monsters::attack_summary(a),
                    )
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The names the dialog's list `list` may add: its field's options not
    /// yet chosen, or the spells free spells' Add offers.
    pub fn dialog_choices(&self, list: usize) -> Vec<String> {
        let Some(dialog) = &self.dialog else {
            return Vec::new();
        };
        let options = match self.fields[dialog.field()].kind {
            FieldKind::Groups(options)
            | FieldKind::LevelNwps(options)
            | FieldKind::LevelAbilities(options) => self.sources.options(options),
            _ => Vec::new(),
        };
        let chosen = match dialog {
            Dialog::Group(d) => d.names.clone(),
            Dialog::Nwps(d) => d.chosen(list),
            Dialog::FreeSpells(d) => return d.choices(&self.spells()),
            Dialog::Ability(_) | Dialog::Attack(_) => Vec::new(),
        };
        options
            .into_iter()
            .filter(|n| !chosen.contains(n))
            .collect()
    }

    /// The items of an items field.
    pub fn bundle(&self) -> Option<Bundle> {
        let field = self
            .fields
            .iter()
            .position(|f| f.kind == FieldKind::Items)?;
        Some(self.record.value(field).items())
    }

    /// Whether anything was changed.
    pub fn changed(&self) -> bool {
        self.record.changed()
    }

    /// Whether Save may be pressed.
    pub fn can_save(&self) -> bool {
        !self.saving && self.prompt.is_none() && self.dialog.is_none()
    }

    /// The question or message in view.
    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    /// Which prompt is in view, so that an answer to one that is gone is
    /// not taken for the next.
    pub fn prompt_serial(&self) -> u64 {
        self.serial
    }

    /// What the app should do now, once.
    pub fn take_effects(&mut self) -> Vec<RecordEffect> {
        std::mem::take(&mut self.effects)
    }

    fn show(&mut self, prompt: Prompt) {
        self.prompt = Some(prompt);
        self.serial += 1;
    }

    /// Answers the prompt in view.
    pub fn answer(&mut self, answer: Answer) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        if let Some(then) = prompt.then(answer) {
            then(self);
        }
    }

    /// A Yes/No question whose Yes is the default.
    fn ask_yes(&mut self, title: &str, text: String, yes: prompt::Then<Self>) {
        let mut prompt = Prompt::new(PromptKind::Question { yes_default: true }, title, text);
        prompt.yes = Some(yes);
        self.show(prompt);
    }

    /// Sets a field's value and asks the original's question about the
    /// change, if it asks one: No puts back the stored value.
    fn set(&mut self, field: usize, value: Value) {
        if let Some(question) = self.record.set(field, &value, !self.detach) {
            self.show(Prompt::ask(
                "",
                tr(question),
                Arc::new(|form: &mut Self| form.detach = true),
                Some(Arc::new(move |form: &mut Self| form.record.put_back(field))),
            ));
        }
    }

    /// Makes a change, or presses a button.
    pub fn input(&mut self, input: RecordInput) {
        if self.prompt.is_some() || self.saving {
            return;
        }
        match input {
            RecordInput::Page(page) => {
                if page < self.pages.len() {
                    self.page = page;
                }
            }
            RecordInput::Set(field, value) => self.set(field, value),
            RecordInput::Choose(field, index) => {
                let (choices, as_text) = self.choice_values(field);
                if let Some(choice) = choices.get(index) {
                    let value = if as_text {
                        Value::Text(choice.clone())
                    } else {
                        Value::Int(index as i64)
                    };
                    self.set(field, value);
                }
            }
            RecordInput::Check(field, on) => self.check(field, on),
            RecordInput::Adding(field, name) => {
                self.adding.retain(|(f, _)| *f != field);
                self.adding.push((field, name));
            }
            RecordInput::Add(field) => {
                let Some(name) = self.adding(field).map(str::to_owned) else {
                    return;
                };
                self.adding.retain(|(f, _)| *f != field);
                let mut list = self.record.value(field).pairs();
                if list.iter().any(|(n, _)| *n == name) {
                    return;
                }
                let start = match self.fields[field].kind {
                    FieldKind::Pairs { min, max, .. } => min.max(0).min(max),
                    _ => 0,
                };
                list.push((name, start));
                self.set_list(field, list);
            }
            RecordInput::Remove(field, index) => {
                let mut list = self.record.value(field).pairs();
                if index < list.len() {
                    list.remove(index);
                    self.set_list(field, list);
                }
            }
            RecordInput::Number(field, index, number) => {
                let mut list = self.record.value(field).pairs();
                if let FieldKind::Pairs { min, max, .. } = self.fields[field].kind
                    && let Some(entry) = list.get_mut(index)
                {
                    entry.1 = number.clamp(min, max);
                    self.set_list(field, list);
                }
            }
            RecordInput::Suggest(field) => {
                if let Some((value, question)) = self.record.suggestion(field) {
                    self.ask_yes(
                        "",
                        question,
                        Arc::new(move |form: &mut Self| form.set(field, Value::Int(value))),
                    );
                }
            }
            RecordInput::GridRows(field, first) => {
                self.grid_rows.retain(|(f, _)| *f != field);
                self.grid_rows.push((field, first));
            }
            RecordInput::Cell(field, row, column, number) => {
                if let FieldKind::Grid { min, max, .. } = self.fields[field].kind {
                    let mut grid = self.record.value(field).grid();
                    if let Some(cell) = grid.get_mut(row).and_then(|r| r.get_mut(column)) {
                        *cell = number.clamp(min, max);
                        self.set(field, Value::Grid(grid));
                    }
                }
            }
            RecordInput::Open(field, at) => self.open_dialog(field, at),
            RecordInput::RemoveEntry(field, index) => self.remove_entry(field, index),
            RecordInput::AddAttack(field, name) => {
                let mut attacks = self.record.value(field).attacks();
                if let Some(problem) = add_problem(&attacks, &name) {
                    self.show(Prompt::message(tr(ADD_HEADING), problem));
                    return;
                }
                if let Some(attack) = self.sources.attacks.iter().find(|a| a.name == name) {
                    attacks.push(attack.clone());
                    self.set(field, Value::Attacks(attacks));
                }
            }
            RecordInput::AutoFill(field, class) => {
                // The original breaks the first line where its message box
                // was narrow.
                let question = levels::auto_fill_question(&class).replacen('\n', " ", 1);
                self.show(Prompt::ask(
                    tr(levels::AUTO_FILL_HEADING),
                    question,
                    Arc::new(move |form: &mut Self| form.set(field, Value::Text(class.clone()))),
                    None,
                ));
            }
            RecordInput::Dialog(change) => {
                if let Some(dialog) = &mut self.dialog {
                    dialog.input(change);
                }
            }
            RecordInput::CloseDialog(ok) => self.close_dialog(ok),
            RecordInput::Items(change) => self.edit_items(|items, bundle| {
                items.input(bundle, change);
            }),
            RecordInput::ItemsAnswer(serial, yes) => self.edit_items(|items, bundle| {
                items.answer(bundle, serial, yes);
            }),
            RecordInput::Save => self.save(),
        }
    }

    fn set_list(&mut self, field: usize, list: Vec<(String, i64)>) {
        let value = match self.fields[field].kind {
            FieldKind::Pairs { .. } => Value::Pairs(list),
            _ => Value::List(list.into_iter().map(|(n, _)| n).collect()),
        };
        self.set(field, value);
    }

    /// The check of a number or a list: an [`FieldKind::OptInt`] stores
    /// its start or its "none", a [`FieldKind::Real`] its "none" or 0,
    /// and turning a [`FieldKind::Checked`] off empties its list.
    fn check(&mut self, field: usize, on: bool) {
        match self.fields[field].kind {
            FieldKind::OptInt {
                none,
                start,
                inverted,
                ..
            } => {
                let numbered = on != inverted;
                self.set(field, Value::Int(if numbered { start } else { none }));
            }
            FieldKind::Real {
                none: Some((none, _)),
                ..
            } => self.set(field, Value::Real(if on { none } else { 0.0 })),
            FieldKind::Checked(_) => {
                self.wanted.retain(|f| *f != field);
                if on {
                    self.wanted.push(field);
                } else {
                    self.set(field, Value::List(Vec::new()));
                }
            }
            _ => {}
        }
    }

    /// Opens the dialog of a field on its entry `at`, or on a new one.
    fn open_dialog(&mut self, field: usize, at: Option<usize>) {
        let value = self.record.value(field);
        let dialog = match self.fields[field].kind {
            FieldKind::Groups(_) => {
                let (count, names) = at
                    .and_then(|i| value.groups().get(i).cloned())
                    .unwrap_or((1, Vec::new()));
                Dialog::Group(GroupEdit::new(field, at, count, names))
            }
            FieldKind::FreeSpells => {
                let Value::FreeSpells(_, list) = value else {
                    return;
                };
                Dialog::FreeSpells(FreeSpellsEdit::new(field, at, &list))
            }
            FieldKind::LevelNwps(_) => Dialog::Nwps(NwpsEdit::new(field, at, &value.level_nwps())),
            FieldKind::LevelAbilities(options) => Dialog::Ability(AbilityEdit::new(
                field,
                at,
                &value.pairs(),
                &self.sources.options(options),
            )),
            FieldKind::Attacks => {
                let attack = match at {
                    Some(i) => match value.attacks().get(i) {
                        Some(a) => a.clone(),
                        None => return,
                    },
                    None => MonsterAttack::default(),
                };
                Dialog::Attack(AttackEdit::new(field, at, attack))
            }
            _ => return,
        };
        self.dialog = Some(dialog);
        self.dialog_serial += 1;
    }

    fn remove_entry(&mut self, field: usize, index: usize) {
        let value = self.record.value(field);
        let value = match (self.fields[field].kind, value) {
            (FieldKind::Groups(_), v) => {
                let mut groups = v.groups();
                if index >= groups.len() {
                    return;
                }
                groups.remove(index);
                Value::Groups(groups)
            }
            (FieldKind::FreeSpells, Value::FreeSpells(class, mut list)) => {
                if index >= list.len() {
                    return;
                }
                list.remove(index);
                Value::FreeSpells(class, list)
            }
            (FieldKind::LevelNwps(_), v) => {
                let mut list = v.level_nwps();
                if index >= list.len() {
                    return;
                }
                list.remove(index);
                Value::LevelNwps(list)
            }
            (FieldKind::LevelAbilities(_), v) => {
                let mut list = v.pairs();
                if index >= list.len() {
                    return;
                }
                list.remove(index);
                Value::Pairs(list)
            }
            // Removed without asking, as the original does.
            (FieldKind::Attacks, v) => {
                let mut list = v.attacks();
                if index >= list.len() {
                    return;
                }
                list.remove(index);
                Value::Attacks(list)
            }
            _ => return,
        };
        self.set(field, value);
    }

    /// OK or Cancel on the dialog in view: OK puts what it holds in its
    /// field, unless the dialog has a problem to say, when it stays open.
    fn close_dialog(&mut self, ok: bool) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        if !ok {
            return;
        }
        let field = dialog.field();
        let value = self.record.value(field);
        match dialog.finish(value) {
            Ok(value) => self.set(field, value),
            Err(dialog) => {
                self.dialog = Some(*dialog);
                self.dialog_serial += 1;
            }
        }
    }

    /// Edits the items of the items field.
    fn edit_items(&mut self, edit: impl FnOnce(&mut BundleEditor, &mut Bundle)) {
        let Some(field) = self.fields.iter().position(|f| f.kind == FieldKind::Items) else {
            return;
        };
        let Some(items) = &mut self.items else {
            return;
        };
        let before = self.record.value(field).items();
        let mut bundle = before.clone();
        edit(items, &mut bundle);
        if bundle != before {
            self.set(field, Value::Items(bundle));
        }
    }

    /// Save: asks first when the record would be saved with something
    /// missing.
    fn save(&mut self) {
        if self.dialog.is_some() {
            return;
        }
        match self.record.save_warning().filter(|_| self.only.is_none()) {
            Some(warning) => self.show(Prompt::ask(
                tr("Save Anyway?"),
                warning,
                Arc::new(Self::store),
                None,
            )),
            None => self.store(),
        }
    }

    fn store(&mut self) {
        let job = match self.only {
            Some((_, origin)) => self.record.job_only(self.section, origin),
            None => Some(self.record.job(self.section, self.editing, self.detach)),
        };
        if let Some(job) = job {
            self.saving = true;
            self.error = None;
            self.effects.push(RecordEffect::Save(job));
        }
    }

    /// What saving did: done, the editor leaves for the entry; a refused
    /// name is said and the editor stays.
    pub fn saved(&mut self, result: Result<Saved, String>) {
        self.saving = false;
        match result {
            Ok(Saved::Done { name, key }) => {
                let told =
                    (self.editing.is_none() && self.only.is_none() && self.kind == Kind::Classes)
                        .then(|| (tr("Class Added").to_owned(), classes::added_message(&name)));
                self.effects.push(RecordEffect::Done {
                    section: self.section,
                    said: tr!("“{name}” saved", name = name.clone()),
                    key,
                    name,
                    told,
                });
            }
            Ok(Saved::Refused { title, text }) => self.show(Prompt::message(&title, text)),
            Err(why) => self.error = Some(why),
        }
    }

    /// The record's name as it is now.
    pub fn name(&self) -> String {
        self.record.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_store::userdb::udb_text;

    #[derive(Debug, Clone, PartialEq, Default)]
    struct Rec {
        name: String,
        group: i64,
        notes: Vec<String>,
    }

    fn fields() -> Vec<FieldDesc<Rec>> {
        vec![
            udb_text!("Main", "Name", 20, name),
            FieldDesc::new(
                "Main",
                "Group",
                FieldKind::Int { min: 0, max: 3 },
                |r: &Rec, _| Value::Int(r.group),
                |r, _, v, _| r.group = v.int(),
            ),
            FieldDesc::new(
                "Main",
                "Notes",
                FieldKind::Dual(Options::Fixed(&["A", "B", "C"])),
                |r: &Rec, _| Value::List(r.notes.clone()),
                |r, _, v, _| r.notes = v.list(),
            ),
            FieldDesc::new(
                "More",
                "Other",
                FieldKind::Text { max: 0 },
                |_: &Rec, _| Value::Text(String::new()),
                |_, _, _, _| {},
            )
            .group("Extra"),
        ]
    }

    #[test]
    fn layout_groups_fields_by_page_and_gives_lists_their_own() {
        let pages = layout(&fields());
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].name, "Main");
        assert_eq!(
            pages[0].groups,
            vec![
                Group {
                    title: "",
                    fields: vec![0, 1]
                },
                Group {
                    title: "Notes",
                    fields: vec![2]
                }
            ]
        );
        assert_eq!(
            pages[1].groups,
            vec![Group {
                title: "Extra",
                fields: vec![3]
            }]
        );
    }

    #[test]
    fn opt_int_and_real_checks_read_the_none_values() {
        assert!(opt_int_checked(5, 0, false));
        assert!(!opt_int_checked(0, 0, false));
        // "Not limited": on at the none value, and below 0 when it is.
        assert!(opt_int_checked(-1, -1, true));
        assert!(opt_int_checked(-7, -1, true));
        assert!(!opt_int_checked(30, -1, true));
        assert!(real_unlimited(-3.0, -1.0));
        assert!(!real_unlimited(2.5, -1.0));
        assert!(real_unlimited(0.0, 0.0));
    }
}
