//! A record of the user databases as the Mac app edits it in a sheet: the
//! editor of `cr_app::records::form`, each page described as a form
//! ([`crate::form`]) with every field drawn as its kind asks, as the
//! Windows app's record.rs does, with the dialogs some fields open (a
//! group to choose from, a class level's free spells, NWPs or ability, a
//! monster attack, the items), its questions as prompts, Choose Picture,
//! and Save, which writes the record.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::bundle::BundleInput;
use cr_app::prompt::Answer;
use cr_app::records::actions::{self, Act, Opened};
use cr_app::records::allowed::{self, AllowedEffect, AllowedForm, AllowedInput};
use cr_app::records::dialogs::{self, AttackInput, Dialog, DialogInput, FreeSpellsEdit};
use cr_app::records::form::{Field, GRID_ROWS, RecordEffect, RecordForm, RecordInput};
use cr_app::records::tables::{self, TableEffect, TableForm, TableInput};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::pictures::Kind;
use cr_store::userdb::classes::levels::{FREQUENCIES, frequency_shown, frequency_stored};
use cr_store::userdb::monsters::{AREAS, SAVE_RESULTS, SAVES, TO_HIT, area_labels};
use cr_store::userdb::{FieldKind, Origin, Value};

use crate::form::{
    Acts, Builder, FormButton, FormDialog, FormGridRow, FormNumber, FormPage, FormRow, FormValue,
};
use crate::prompt::PromptInfo;
use crate::{CoreError, png};

/// A control of the editor.
#[derive(Clone)]
pub(crate) enum In {
    Record(RecordInput),
    /// Choose… on the picture or icon field with this index.
    Picture(usize),
    Table(TableInput),
    Allowed(AllowedInput),
}

/// What the sheet edits: a record, a treasure table, or a race's
/// Allowed Classes, with the key of the race.
pub(crate) enum Editor {
    Record(Box<RecordForm>),
    Table(Box<TableForm>),
    Allowed(Box<AllowedForm>, String),
}

fn rec(input: RecordInput) -> In {
    In::Record(input)
}

fn change(change: DialogInput) -> In {
    rec(RecordInput::Dialog(change))
}

fn attack(change: AttackInput) -> In {
    self::change(DialogInput::Attack(change))
}

/// The record saved: the entry to show, and what to say.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RecordDone {
    /// The catalog section of the record, and its entry's key.
    pub section: String,
    pub key: String,
    pub name: String,
    pub said: String,
    /// What there is to tell besides, as a title and a text.
    pub told: Option<Vec<String>>,
}

/// A picture Choose Picture offers.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PictureChoice {
    /// The heading of its group.
    pub group: String,
    /// What the tile is called under it.
    pub name: String,
    /// What the record keeps when it is chosen.
    pub stored: String,
    /// What the search looks in, in lower case.
    pub searched: String,
    /// Its file, or the PNG of an icon cut from a strip.
    pub path: Option<String>,
    pub png: Option<Vec<u8>>,
}

/// A record's editor, open in its sheet.
#[derive(uniffi::Object)]
pub struct RecordSession {
    install: Arc<Installation>,
    editor: Mutex<Editor>,
    acts: Mutex<Acts<In>>,
    /// The field Choose… asked a picture for, until the app takes it.
    picking: Mutex<Option<u32>>,
    done: Mutex<Option<RecordDone>>,
}

impl RecordSession {
    pub(crate) fn new(install: Arc<Installation>, editor: Editor) -> Arc<Self> {
        Arc::new(Self {
            install,
            editor: Mutex::new(editor),
            acts: Mutex::default(),
            picking: Mutex::default(),
            done: Mutex::default(),
        })
    }

    fn editor(&self) -> MutexGuard<'_, Editor> {
        self.editor.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn done(&self, done: RecordDone) {
        *self.done.lock().unwrap_or_else(|e| e.into_inner()) = Some(done);
    }

    /// Does what the editor asks: saves, and keeps what saving did.
    fn effects(&self, editor: &mut Editor) {
        match editor {
            Editor::Record(form) => self.record_effects(form),
            Editor::Table(form) => self.table_effects(form),
            Editor::Allowed(form, key) => self.allowed_effects(form, key),
        }
    }

    fn table_effects(&self, form: &mut TableForm) {
        loop {
            let effects = form.take_effects();
            if effects.is_empty() {
                return;
            }
            for effect in effects {
                match effect {
                    TableEffect::Save { editing, table } => {
                        let saved = tables::save(&self.install, editing, *table);
                        form.saved(saved);
                    }
                    TableEffect::Done { key, said } => self.done(RecordDone {
                        section: tables::SECTION.to_owned(),
                        key,
                        name: form.table.name.clone(),
                        said,
                        told: None,
                    }),
                }
            }
        }
    }

    fn allowed_effects(&self, form: &mut AllowedForm, key: &str) {
        loop {
            let effects = form.take_effects();
            if effects.is_empty() {
                return;
            }
            for effect in effects {
                match effect {
                    AllowedEffect::Save { race, entries } => {
                        let saved = allowed::save(&self.install, &race, &entries);
                        form.saved(saved);
                    }
                    AllowedEffect::Done { said } => self.done(RecordDone {
                        section: "races".to_owned(),
                        key: key.to_owned(),
                        name: form.race.clone(),
                        said,
                        told: None,
                    }),
                }
            }
        }
    }

    fn record_effects(&self, form: &mut RecordForm) {
        loop {
            let effects = form.take_effects();
            if effects.is_empty() {
                return;
            }
            for effect in effects {
                match effect {
                    RecordEffect::Save(job) => {
                        let saved = job(&self.install);
                        form.saved(saved);
                    }
                    RecordEffect::Done {
                        section,
                        key,
                        name,
                        said,
                        told,
                    } => self.done(RecordDone {
                        section: section.to_owned(),
                        key,
                        name,
                        said,
                        told: told.map(|(title, text)| vec![title, text]),
                    }),
                }
            }
        }
    }

    fn input(&self, input: In) {
        let mut editor = self.editor();
        match (&mut *editor, input) {
            (Editor::Record(form), In::Record(input)) => form.input(input),
            (Editor::Record(_), In::Picture(field)) => {
                *self.picking.lock().unwrap_or_else(|e| e.into_inner()) = Some(field as u32);
            }
            (Editor::Table(form), In::Table(input)) => form.input(input),
            (Editor::Allowed(form, _), In::Allowed(input)) => form.input(input),
            _ => return,
        }
        self.effects(&mut editor);
    }

    /// The kind of field `field` of the record edited.
    fn field_kind(&self, field: u32) -> Option<FieldKind> {
        match &*self.editor() {
            Editor::Record(form) => Some(form.field(field as usize).kind),
            _ => None,
        }
    }

    /// The pages shown, by index.
    fn shown(form: &RecordForm) -> Vec<usize> {
        (0..form.pages().len())
            .filter(|&p| form.page_visible(p))
            .collect()
    }
}

#[uniffi::export]
impl RecordSession {
    /// The record's name, or what is being made.
    pub fn title(&self) -> String {
        match &*self.editor() {
            Editor::Record(form) => form.title().to_owned(),
            Editor::Table(form) => form.title().to_owned(),
            Editor::Allowed(form, _) => form.title(),
        }
    }

    /// The program help's topic.
    pub fn help(&self) -> u32 {
        match &*self.editor() {
            Editor::Record(form) => form.help(),
            Editor::Table(form) => form.help(),
            Editor::Allowed(form, _) => form.help(),
        }
    }

    /// The pages to choose from.
    pub fn tabs(&self) -> Vec<String> {
        let Editor::Record(form) = &*self.editor() else {
            return Vec::new();
        };
        Self::shown(form)
            .into_iter()
            .map(|p| tr(form.pages()[p].name).to_owned())
            .collect()
    }

    /// The page shown, by its place among [`Self::tabs`].
    pub fn tab(&self) -> u32 {
        let Editor::Record(form) = &*self.editor() else {
            return 0;
        };
        Self::shown(form)
            .iter()
            .position(|&p| p == form.page)
            .unwrap_or(0) as u32
    }

    pub fn set_tab(&self, index: u32) {
        let page = match &*self.editor() {
            Editor::Record(form) => Self::shown(form).get(index as usize).copied(),
            _ => None,
        };
        if let Some(page) = page {
            self.input(rec(RecordInput::Page(page)));
        }
    }

    /// The page shown as a form, with the dialog over it.
    pub fn form_page(&self) -> FormPage {
        let editor = self.editor();
        let mut builder = Builder::new();
        let dialog = match &*editor {
            Editor::Record(form) => {
                describe(&mut builder, form, &self.install);
                match (&form.dialog, &form.items) {
                    (Some(dialog), _) => Some(dialog_of(&mut builder, form, dialog)),
                    (None, Some(items)) => crate::bundle::dialog(
                        &mut builder,
                        items,
                        |i| rec(RecordInput::Items(i)),
                        |serial, yes| rec(RecordInput::ItemsAnswer(serial, yes)),
                    ),
                    (None, None) => None,
                }
            }
            Editor::Table(form) => crate::treasure_tables::describe(&mut builder, form),
            Editor::Allowed(form, _) => {
                crate::allowed::describe(&mut builder, form);
                None
            }
        };
        let (groups, acts) = builder.finish();
        *self.acts.lock().unwrap_or_else(|e| e.into_inner()) = acts;
        FormPage { groups, dialog }
    }

    /// Control `id` of the form last described changed to `value`.
    pub fn act(&self, id: u32, value: FormValue) {
        let input = self
            .acts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .input(id, &value);
        if let Some(input) = input {
            self.input(input);
        }
    }

    /// The question or message the editor waits on.
    pub fn prompt(&self) -> Option<PromptInfo> {
        match &*self.editor() {
            Editor::Record(form) => form.prompt().map(PromptInfo::of),
            Editor::Table(form) => form.prompt().map(PromptInfo::of),
            Editor::Allowed(form, _) => form.prompt().map(PromptInfo::of),
        }
    }

    pub fn answer(&self, yes: bool) {
        let answer = if yes { Answer::Yes } else { Answer::No };
        let mut editor = self.editor();
        match &mut *editor {
            Editor::Record(form) => form.answer(answer),
            Editor::Table(form) => form.answer(answer),
            Editor::Allowed(form, _) => form.answer(answer),
        }
        self.effects(&mut editor);
    }

    /// Why the record could not be saved.
    pub fn error(&self) -> Option<String> {
        match &*self.editor() {
            Editor::Record(form) => form.error.clone(),
            Editor::Table(form) => form.error.clone(),
            Editor::Allowed(form, _) => form.error.clone(),
        }
    }

    pub fn can_save(&self) -> bool {
        match &*self.editor() {
            Editor::Record(form) => form.can_save(),
            Editor::Table(form) => form.can_save(),
            Editor::Allowed(form, _) => form.can_save(),
        }
    }

    pub fn saving(&self) -> bool {
        match &*self.editor() {
            Editor::Record(form) => form.saving,
            Editor::Table(form) => form.saving,
            Editor::Allowed(form, _) => form.saving,
        }
    }

    /// Whether anything was changed.
    pub fn changed(&self) -> bool {
        match &*self.editor() {
            Editor::Record(form) => form.changed(),
            Editor::Table(form) => form.changed(),
            Editor::Allowed(form, _) => form.changed(),
        }
    }

    /// Save: writes the record, or asks first.
    pub fn save(&self) {
        let save = match &*self.editor() {
            Editor::Record(_) => rec(RecordInput::Save),
            Editor::Table(_) => In::Table(TableInput::Save),
            Editor::Allowed(..) => In::Allowed(AllowedInput::Save),
        };
        self.input(save);
    }

    /// The record saved, once.
    pub fn take_done(&self) -> Option<RecordDone> {
        self.done.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// The field Choose… asked a picture for, once.
    pub fn take_picking(&self) -> Option<u32> {
        self.picking
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// The pictures, or the icons, field `field` may have.
    pub fn picture_choices(&self, field: u32) -> Result<Vec<PictureChoice>, CoreError> {
        match self.field_kind(field) {
            Some(FieldKind::Icon) => icons(&self.install),
            Some(FieldKind::Picture(kind)) => pictures(&self.install, kind),
            _ => Ok(Vec::new()),
        }
    }

    /// Makes a picture for field `field` from the user's picture at
    /// `path`, and keeps it in the record.
    pub fn add_picture(&self, field: u32, path: String) -> Result<(), CoreError> {
        let path = std::path::Path::new(&path);
        let stored = match self.field_kind(field) {
            Some(FieldKind::Icon) => self.install.add_icon(path)?,
            Some(FieldKind::Picture(kind)) => {
                let made = self.install.add_picture(kind, path)?;
                cr_store::pictures::stored_path(self.install.root(), &made).unwrap_or_default()
            }
            _ => return Ok(()),
        };
        self.set_picture(field, stored);
        Ok(())
    }

    /// Keeps the picture chosen, as the record keeps it.
    pub fn set_picture(&self, field: u32, stored: String) {
        self.input(rec(RecordInput::Set(field as usize, Value::Text(stored))));
    }
}

/// The pictures of `kind`: the user's first, then the shipped ones.
fn pictures(install: &Installation, kind: Kind) -> Result<Vec<PictureChoice>, CoreError> {
    let choices = install.picture_choices(kind, "")?;
    let groups = [tr("Your Pictures"), tr("Monster Pictures")];
    let (first, rest): (Vec<_>, Vec<_>) = choices.into_iter().partition(|c| c.first);
    Ok(first
        .into_iter()
        .map(|c| (c, groups[0]))
        .chain(rest.into_iter().map(|c| (c, groups[1])))
        .map(|(c, group)| PictureChoice {
            group: group.to_owned(),
            stored: cr_store::pictures::stored_path(install.root(), &c.path).unwrap_or_default(),
            searched: c.file.to_lowercase(),
            name: c.file,
            path: Some(c.path.display().to_string()),
            png: None,
        })
        .collect())
}

/// The icons an item may have, by the groups of their items.
fn icons(install: &Installation) -> Result<Vec<PictureChoice>, CoreError> {
    let choices = install.icon_choices()?;
    // The strips the icons are cut from, each decoded once.
    let mut strips: Vec<(std::path::PathBuf, Option<cr_formats::bmp::Image>)> = Vec::new();
    Ok(choices
        .into_iter()
        .map(|c| {
            let name = cr_app::pictures::icon_name(&c.stored);
            let (path, image) = match c.icon.left {
                None => (Some(c.icon.path.display().to_string()), None),
                Some(left) => {
                    let at = match strips.iter().position(|(p, _)| *p == c.icon.path) {
                        Some(at) => at,
                        None => {
                            let strip = cr_app::pictures::load(&c.icon.path);
                            strips.push((c.icon.path.clone(), strip));
                            strips.len() - 1
                        }
                    };
                    let image = strips[at]
                        .1
                        .as_ref()
                        .and_then(|strip| cr_app::pictures::icon_in(strip, left))
                        .and_then(|i| png(&i));
                    (None, image)
                }
            };
            let group = cr_app::pictures::icon_group_title(c.group);
            PictureChoice {
                group: group.to_owned(),
                name: c.names.first().cloned().unwrap_or_else(|| name.clone()),
                searched: std::iter::once(name)
                    .chain(c.names.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .to_lowercase(),
                stored: c.stored,
                path,
                png: image,
            }
        })
        .collect())
}

/// The editor's page.
fn describe(form: &mut Builder<In>, f: &RecordForm, install: &Installation) {
    let Some(page) = f.pages().get(f.page) else {
        return;
    };
    for group in &page.groups {
        if !f.group_visible(group) {
            continue;
        }
        form.group(tr(group.title), "");
        for &index in &group.fields {
            field(form, f, index, install);
        }
    }
}

/// A row with a check box: the check of a number or a list.
fn check(form: &mut Builder<In>, title: &str, on: bool, enabled: bool, index: usize) {
    let id = form.on_toggle(move |on| rec(RecordInput::Check(index, on)));
    form.row(FormRow::Check {
        id,
        title: title.to_owned(),
        subtitle: String::new(),
        on,
        enabled,
    });
}

/// One field, as its kind draws it.
fn field(form: &mut Builder<In>, f: &RecordForm, index: usize, install: &Installation) {
    let field = f.field(index);
    if !field.visible {
        return;
    }
    let label = tr(field.label);
    let enabled = field.enabled;
    let set = move |value: Value| rec(RecordInput::Set(index, value));
    match field.kind {
        FieldKind::Text { .. } => {
            if enabled {
                form.text(label, &field.value.text(), false, Vec::new(), move |t| {
                    set(Value::Text(t))
                });
            } else {
                form.value(label, "", &field.value.text());
            }
        }
        FieldKind::MultiLine => {
            if enabled {
                form.text(label, &field.value.text(), true, Vec::new(), move |t| {
                    set(Value::Text(t))
                });
            } else {
                form.note(&field.value.text());
            }
        }
        FieldKind::Int { min, max } => {
            let buttons = field
                .suggest
                .map(|s| vec![form.button(tr(s), "", enabled, rec(RecordInput::Suggest(index)))])
                .unwrap_or_default();
            form.number(
                (label, ""),
                field.value.int(),
                (min, max),
                buttons,
                move |n| set(Value::Int(n)),
            )
            .enabled(enabled);
        }
        FieldKind::Check => {
            let id = form.on_toggle(move |on| set(Value::Bool(on)));
            form.row(FormRow::Check {
                id,
                title: label.to_owned(),
                subtitle: String::new(),
                on: field.value.bool(),
                enabled,
            });
        }
        FieldKind::Choice(_) | FieldKind::Combo { free: false, .. } => {
            let (choices, selected) = f.choices(index);
            if enabled {
                form.choice((label, ""), choices, selected, Vec::new(), move |i| {
                    rec(RecordInput::Choose(index, i))
                });
            } else {
                let shown = selected.and_then(|s| choices.get(s)).cloned();
                form.value(label, "", &shown.unwrap_or_default());
            }
        }
        FieldKind::Combo {
            options,
            free: true,
        } => {
            if !enabled {
                form.value(label, "", &field.value.text());
                return;
            }
            form.text(label, &field.value.text(), false, Vec::new(), move |t| {
                set(Value::Text(t))
            });
            let choices = f.options(options);
            if !choices.is_empty() {
                let chosen = choices.clone();
                let id = form.on_index(move |i| set(Value::Text(chosen[i].clone())));
                form.row(FormRow::Add {
                    id,
                    label: tr!("Choose {field}", field = label),
                    options: choices,
                });
            }
        }
        FieldKind::Dual(_) | FieldKind::Pairs { .. } => names(form, f, index, &field),
        FieldKind::Checked(_) => {
            check(form, label, field.checked, enabled, index);
            if field.checked {
                names(form, f, index, &field);
            }
        }
        FieldKind::OptInt { min, max, .. } => {
            check(form, label, field.checked, enabled, index);
            form.number(
                ("", ""),
                field.value.int(),
                (min, max),
                Vec::new(),
                move |n| set(Value::Int(n)),
            )
            .enabled(field.number_open());
        }
        FieldKind::Real { min, max, none } => {
            if let Some((_, title)) = none {
                check(form, tr(title), field.checked, enabled, index);
            }
            let value = field.value.text();
            if field.number_open() {
                let id = form.on_real(move |v| set(Value::Real(v.clamp(min, max))));
                form.row(FormRow::Text {
                    id,
                    title: label.to_owned(),
                    value,
                    multiline: false,
                    buttons: Vec::new(),
                });
            } else if none.is_none() {
                form.value(label, "", &value);
            }
        }
        FieldKind::Groups(_) => entries(form, f, index, enabled, tr("Add a Group…")),
        FieldKind::FreeSpells => entries(form, f, index, enabled, tr("Add Free Spells…")),
        FieldKind::LevelNwps(_) => entries(form, f, index, enabled, tr("Add NWPs…")),
        FieldKind::LevelAbilities(_) => {
            entries(form, f, index, enabled, tr("Add a Class Ability…"))
        }
        FieldKind::Attacks => {
            let new = form.button(tr("New…"), "", enabled, rec(RecordInput::Open(index, None)));
            form.heading_button(new);
            entry_rows(form, f, index, enabled);
            if enabled {
                let types = f.attack_types();
                let chosen = f.adding(index).map(str::to_owned);
                let selected = chosen
                    .as_ref()
                    .and_then(|c| types.iter().position(|t| t == c));
                let add = form.button(
                    tr("_Add"),
                    "",
                    chosen.is_some(),
                    rec(RecordInput::AddAttack(index, chosen.unwrap_or_default())),
                );
                let names = types.clone();
                form.choice((tr("Attacks"), ""), types, selected, vec![add], move |i| {
                    rec(RecordInput::Adding(index, names[i].clone()))
                });
            }
        }
        FieldKind::Items => {
            let add = form.button(
                tr("_Add Item…"),
                "",
                enabled,
                rec(RecordInput::Items(BundleInput::Add)),
            );
            form.heading_button(add);
            crate::bundle::rows(
                form,
                &cr_app::bundle::rows(&field.value.items()),
                enabled,
                |i| rec(RecordInput::Items(i)),
            );
        }
        FieldKind::Grid {
            columns,
            first,
            min,
            max,
        } => grid(form, f, index, &field, (columns, first, min, max)),
        FieldKind::AutoFill => {
            let classes = f.auto_fill_classes();
            let chosen = f.adding(index).map(str::to_owned);
            let selected = chosen
                .as_ref()
                .and_then(|c| classes.iter().position(|n| n == c));
            let copy = form.button(
                tr("Copy"),
                "",
                enabled && chosen.is_some(),
                rec(RecordInput::AutoFill(index, chosen.unwrap_or_default())),
            );
            let names = classes.clone();
            form.choice(
                (
                    tr("Copy from another class…"),
                    tr("Fills this page with what another class has on it"),
                ),
                classes,
                selected,
                vec![copy],
                move |i| rec(RecordInput::Adding(index, names[i].clone())),
            );
        }
        FieldKind::Dice => {
            let values = field.value.dice();
            let numbers = dice(
                form,
                label,
                values,
                [(0, 1000), (0, 1000), (-1000, 100_000)],
                {
                    move |k, v| {
                        let mut d = values;
                        d[k] = v;
                        set(Value::Dice(d))
                    }
                },
            );
            form.row(FormRow::Numbers {
                title: label.to_owned(),
                numbers,
                separators: vec!["d".to_owned(), "+".to_owned()],
                enabled,
            });
        }
        FieldKind::Picture(_) | FieldKind::Icon => {
            let stored = field.value.text();
            let icon = field.kind == FieldKind::Icon;
            let image = match field.kind {
                FieldKind::Picture(kind) => install
                    .picture_path(&stored, kind)
                    .and_then(|p| std::fs::read(p).ok()),
                _ => install
                    .icon(stored.trim())
                    .and_then(|i| cr_app::pictures::icon(&i))
                    .and_then(|i| png(&i)),
            };
            let subtitle = cr_app::pictures::picture_line(&stored, icon, image.is_some());
            let clear = (!cr_store::pictures::split_stored(&stored)
                .1
                .trim()
                .is_empty())
            .then(|| {
                form.button(
                    tr("No Picture"),
                    "xmark.circle",
                    enabled,
                    set(Value::Text(String::new())),
                )
            });
            let choose = form.button(tr("_Choose…"), "", enabled, In::Picture(index));
            form.row(FormRow::Picture {
                title: label.to_owned(),
                subtitle,
                png: image,
                icon,
                choose,
                clear,
            });
        }
    }
}

/// Three numbers for dice: count, sides and bonus.
pub(crate) fn dice(
    form: &mut Builder<In>,
    label: &str,
    values: [i64; 3],
    ranges: [(i64, i64); 3],
    make: impl Fn(usize, i64) -> In + Clone + Send + Sync + 'static,
) -> Vec<FormNumber> {
    let parts = [tr("Dice"), tr("Sides"), tr("Bonus")];
    (0..3)
        .map(|k| {
            let make = make.clone();
            FormNumber {
                id: form.on_number(ranges[k], move |v| make(k, v)),
                label: format!("{label}: {}", parts[k]),
                value: values[k],
                min: ranges[k].0,
                max: ranges[k].1,
            }
        })
        .collect()
}

/// The names a list field has chosen, each with its number for pairs and
/// Remove, and the menu of those it may add.
fn names(form: &mut Builder<In>, f: &RecordForm, index: usize, field: &Field) {
    let enabled = field.number_open();
    let number = match field.kind {
        FieldKind::Pairs {
            number, min, max, ..
        } => Some((tr(number), min, max)),
        _ => None,
    };
    let list = field.value.pairs();
    if list.is_empty() {
        form.none();
    }
    for (i, (name, n)) in list.iter().enumerate() {
        let (number, subtitle) = match number {
            Some((label, min, max)) if enabled => (
                Some(FormNumber {
                    id: form.on_number((min, max), move |v| rec(RecordInput::Number(index, i, v))),
                    label: label.to_owned(),
                    value: *n,
                    min,
                    max,
                }),
                String::new(),
            ),
            Some((label, ..)) => (None, format!("{label}: {n}")),
            None => (None, String::new()),
        };
        let remove = enabled.then(|| form.on_press(rec(RecordInput::Remove(index, i))));
        form.row(FormRow::Item {
            title: name.clone(),
            subtitle,
            number,
            remove,
            icon: None,
        });
    }
    if enabled {
        let choices = f.list_choices(index);
        let chosen = f.adding(index);
        let selected = chosen.and_then(|c| choices.iter().position(|n| n == c));
        let add = form.button(
            tr("_Add"),
            "",
            selected.is_some(),
            rec(RecordInput::Add(index)),
        );
        let names = choices.clone();
        form.choice(
            (&tr!("Add to {list}", list = tr(field.label)), ""),
            choices,
            selected,
            vec![add],
            move |i| rec(RecordInput::Adding(index, names[i].clone())),
        );
    }
}

/// The entries of a field listed with Edit and Remove (groups, levels,
/// attacks).
fn entry_rows(form: &mut Builder<In>, f: &RecordForm, index: usize, enabled: bool) {
    let entries = f.entries(index);
    if entries.is_empty() {
        form.none();
    }
    for (i, (title, subtitle)) in entries.into_iter().enumerate() {
        let buttons = if enabled {
            vec![
                form.button(
                    tr("Edit"),
                    "pencil",
                    true,
                    rec(RecordInput::Open(index, Some(i))),
                ),
                form.button(
                    tr("Remove"),
                    "minus.circle",
                    true,
                    rec(RecordInput::RemoveEntry(index, i)),
                ),
            ]
        } else {
            Vec::new()
        };
        form.row(FormRow::Tree {
            title,
            subtitle,
            depth: 0,
            dimmed: false,
            buttons,
        });
    }
}

/// The entries, and the button that adds one beside the heading.
fn entries(form: &mut Builder<In>, f: &RecordForm, index: usize, enabled: bool, add: &str) {
    let add = form.button(add, "", enabled, rec(RecordInput::Open(index, None)));
    form.heading_button(add);
    entry_rows(form, f, index, enabled);
}

/// A table of numbers, one row per level, shown ten levels at a time.
fn grid(
    form: &mut Builder<In>,
    f: &RecordForm,
    index: usize,
    field: &Field,
    (columns, first, min, max): (&'static [&'static str], i64, i64, i64),
) {
    let rows = field.value.grid();
    if rows.is_empty() {
        form.none();
        return;
    }
    let start = f.grid_first(index).min(rows.len() - 1) / GRID_ROWS * GRID_ROWS;
    if rows.len() > GRID_ROWS {
        let ranges: Vec<String> = (0..rows.len())
            .step_by(GRID_ROWS)
            .map(|s| {
                let last = (s + GRID_ROWS).min(rows.len()) - 1;
                tr!(
                    "Levels {from} to {to}",
                    from = first + s as i64,
                    to = first + last as i64
                )
            })
            .collect();
        form.choice(
            (tr("Levels"), ""),
            ranges,
            Some(start / GRID_ROWS),
            Vec::new(),
            move |i| rec(RecordInput::GridRows(index, i * GRID_ROWS)),
        );
    }
    let shown: Vec<FormGridRow> = rows[start..(start + GRID_ROWS).min(rows.len())]
        .iter()
        .enumerate()
        .map(|(r, values)| {
            let row = start + r;
            let level = first + row as i64;
            FormGridRow {
                title: level.to_string(),
                cells: values
                    .iter()
                    .enumerate()
                    .take(columns.len())
                    .map(|(c, &value)| FormNumber {
                        id: form.on_number((min, max), move |v| {
                            rec(RecordInput::Cell(index, row, c, v))
                        }),
                        label: format!("{} {level}: {}", tr("Level"), tr(columns[c])),
                        value,
                        min,
                        max,
                    })
                    .collect(),
            }
        })
        .collect();
    form.row(FormRow::Grid {
        columns: std::iter::once(tr("Level").to_owned())
            .chain(columns.iter().map(|c| tr(c).to_owned()))
            .collect(),
        rows: shown,
        enabled: field.enabled,
    });
}

/// A dialog's list of names, each with its number if it has one and
/// Remove, and the menu of those to add.
fn dialog_list(
    form: &mut Builder<In>,
    f: &RecordForm,
    list: usize,
    add: &str,
    rows: &[(String, Option<i64>)],
    number: Option<(&str, i64, i64)>,
    adding: Option<&str>,
) {
    if rows.is_empty() {
        form.none();
    }
    for (i, (name, n)) in rows.iter().enumerate() {
        let number = match (number, n) {
            (Some((label, min, max)), Some(n)) => Some(FormNumber {
                id: form.on_number((min, max), move |v| change(DialogInput::Number(list, i, v))),
                label: label.to_owned(),
                value: *n,
                min,
                max,
            }),
            _ => None,
        };
        let remove = form.on_press(change(DialogInput::Remove(list, i)));
        form.row(FormRow::Item {
            title: name.clone(),
            subtitle: String::new(),
            number,
            remove: Some(remove),
            icon: None,
        });
    }
    let choices = f.dialog_choices(list);
    let selected = adding.and_then(|a| choices.iter().position(|c| c == a));
    let button = form.button(
        tr("_Add"),
        "",
        selected.is_some(),
        change(DialogInput::Add(list)),
    );
    let names = choices.clone();
    form.choice((add, ""), choices, selected, vec![button], move |i| {
        change(DialogInput::Adding(list, names[i].clone()))
    });
}

fn level(form: &mut Builder<In>, level: i64) {
    form.number(
        (tr("Level"), ""),
        level,
        (1, dialogs::MOST_LEVEL),
        Vec::new(),
        |v| change(DialogInput::Level(v)),
    );
}

/// A choice of a dialog among untranslated `items`.
fn pick(
    form: &mut Builder<In>,
    title: &str,
    items: &[&str],
    selected: usize,
    make: fn(usize) -> AttackInput,
) {
    form.choice(
        (title, ""),
        items.iter().map(|i| tr(i).to_owned()).collect(),
        Some(selected),
        Vec::new(),
        move |i| attack(make(i)),
    );
}

/// A whole number of the Monster Attack dialog.
fn attack_number(
    form: &mut Builder<In>,
    title: &str,
    value: i32,
    min: i64,
    make: fn(i32) -> AttackInput,
) {
    form.number(
        (title, ""),
        value.into(),
        (min, dialogs::MOST_ATTACK_NUMBER.into()),
        Vec::new(),
        move |v| attack(make(v as i32)),
    );
}

/// The dialog a field opened; its controls are numbered with the page's.
fn dialog_of(form: &mut Builder<In>, f: &RecordForm, dialog: &Dialog) -> FormDialog {
    let page = form.take_groups();
    let title = match dialog {
        Dialog::Group(d) => {
            form.group("", "").number(
                (tr("Number to choose"), ""),
                d.count,
                (1, dialogs::MOST_COUNT),
                Vec::new(),
                |v| change(DialogInput::Count(v)),
            );
            let rows: Vec<(String, Option<i64>)> =
                d.names.iter().map(|n| (n.clone(), None)).collect();
            form.group(tr("Options"), "");
            dialog_list(
                form,
                f,
                0,
                tr("Add an Option…"),
                &rows,
                None,
                d.adding.as_deref(),
            );
            tr(f.field(d.field).label).to_owned()
        }
        Dialog::FreeSpells(d) => {
            let l = &d.level;
            form.group("", "");
            level(form, l.level.into());
            form.number(
                (tr("The spells may be cast this often"), ""),
                l.uses.into(),
                (0, dialogs::MOST_USES),
                Vec::new(),
                |v| change(DialogInput::Uses(v)),
            )
            .text(
                tr("Frequency"),
                &frequency_shown(&l.frequency),
                false,
                Vec::new(),
                |t| change(DialogInput::Frequency(frequency_stored(&t))),
            );
            // Stored as the original writes them.
            let id = form.on_index(|i| {
                change(DialogInput::Frequency(
                    FREQUENCIES.get(i).copied().unwrap_or_default().to_owned(),
                ))
            });
            form.row(FormRow::Add {
                id,
                label: tr("Choose").to_owned(),
                options: FREQUENCIES.iter().map(|f| tr(f).to_owned()).collect(),
            });
            let pick_one = form.on_toggle(|on| change(DialogInput::PickOne(on)));
            form.row(FormRow::Check {
                id: pick_one,
                title: tr("Must select one spell from the list").to_owned(),
                subtitle: String::new(),
                on: l.pick_one,
                enabled: true,
            });
            form.group(tr("Spells"), "");
            if l.spells.is_empty() {
                form.none();
            }
            for (i, spell) in l.spells.iter().enumerate() {
                let number = FormNumber {
                    id: form.on_number((0, dialogs::MOST_USES), move |v| {
                        change(DialogInput::Number(0, i, v))
                    }),
                    label: tr("Uses").to_owned(),
                    value: spell.uses.into(),
                    min: 0,
                    max: dialogs::MOST_USES,
                };
                let remove = form.on_press(change(DialogInput::Remove(0, i)));
                form.row(FormRow::Item {
                    title: spell.name.clone(),
                    subtitle: FreeSpellsEdit::spell_kind(spell).to_owned(),
                    number: Some(number),
                    remove: Some(remove),
                    icon: None,
                });
            }
            form.choice(
                (tr("Spell type"), ""),
                vec![tr("Priest").to_owned(), tr("Wizard").to_owned()],
                Some(usize::from(d.wizard)),
                Vec::new(),
                |i| change(DialogInput::Wizard(i == 1)),
            )
            .number(
                (tr("Spell level (0 for all)"), ""),
                d.spell_level,
                (0, dialogs::MOST_SPELL_LEVEL),
                Vec::new(),
                |v| change(DialogInput::SpellLevel(v)),
            );
            let choices = f.dialog_choices(0);
            let selected = d
                .adding
                .as_ref()
                .and_then(|a| choices.iter().position(|c| c == a));
            let add = form.button(
                tr("_Add"),
                "",
                selected.is_some(),
                change(DialogInput::Add(0)),
            );
            let names = choices.clone();
            form.choice(
                (tr("Add a Spell…"), ""),
                choices,
                selected,
                vec![add],
                move |i| change(DialogInput::Adding(0, names[i].clone())),
            );
            tr("Free Spells").to_owned()
        }
        Dialog::Nwps(d) => {
            form.group("", "");
            level(form, d.level.level.into());
            let granted: Vec<(String, Option<i64>)> =
                d.level.granted.iter().map(|n| (n.clone(), None)).collect();
            form.group(tr("Bonus NWPs granted at this level"), "");
            dialog_list(
                form,
                f,
                0,
                tr("Add a Proficiency…"),
                &granted,
                None,
                d.adding[0].as_deref(),
            );
            let pick_one = form.on_toggle(|on| change(DialogInput::PickOne(on)));
            form.row(FormRow::Check {
                id: pick_one,
                title: tr("May only pick one from the list above").to_owned(),
                subtitle: String::new(),
                on: d.level.pick_one,
                enabled: true,
            });
            let modified: Vec<(String, Option<i64>)> = d
                .level
                .modifications
                .iter()
                .map(|(n, m)| (n.clone(), Some(i64::from(*m))))
                .collect();
            form.group(tr("NWP score modifications"), "");
            dialog_list(
                form,
                f,
                1,
                tr("Modify a Proficiency…"),
                &modified,
                Some((tr("Modification"), 0, dialogs::MOST_MODIFICATION)),
                d.adding[1].as_deref(),
            );
            tr("NWPs").to_owned()
        }
        Dialog::Ability(d) => {
            form.group("", "").note(&d.prompt());
            level(form, d.level);
            form.choice(
                (tr("Class Ability"), ""),
                d.choices.clone(),
                d.chosen(),
                Vec::new(),
                |i| change(DialogInput::Ability(i)),
            );
            tr("Class Abilities").to_owned()
        }
        Dialog::Attack(d) => {
            let a = &d.attack;
            let most = i64::from(dialogs::MOST_ATTACK_NUMBER);
            form.group("", "");
            if let Some(problem) = d.problem {
                form.banner(problem, true);
            }
            form.text(tr("Name"), &a.name, false, Vec::new(), |t| {
                attack(AttackInput::Name(t))
            });
            attack_number(form, tr("Help ID"), a.help_id, 0, AttackInput::HelpId);
            let numbers = dice(
                form,
                tr("Damage"),
                [a.damage_dice, a.damage_sides, a.damage_bonus].map(i64::from),
                [(0, most); 3],
                |k, v| attack(AttackInput::Damage(k, v as i32)),
            );
            form.row(FormRow::Numbers {
                title: tr("Damage").to_owned(),
                numbers,
                separators: vec!["d".to_owned(), "+".to_owned()],
                enabled: true,
            });
            let to_hit: Vec<&str> = TO_HIT.iter().map(|(_, l)| *l).collect();
            pick(
                form,
                tr("Must make a to hit roll against"),
                &to_hit,
                d.to_hit(),
                AttackInput::ToHit,
            );
            form.text(
                tr("Special Effect"),
                &a.special_effect,
                false,
                Vec::new(),
                |t| attack(AttackInput::Effect(t)),
            )
            .number(
                (
                    tr("Bonus Hit Dice (used to calculate modified HD for experience)"),
                    "",
                ),
                a.bonus_hit_dice.into(),
                (-most, most),
                Vec::new(),
                |v| attack(AttackInput::HitDice(v as i32)),
            );
            form.group(tr("Area of Effect"), "");
            pick(form, tr("Type"), &AREAS, d.area(), AttackInput::Area);
            for (k, (label, value)) in area_labels(d.area() as i32)
                .iter()
                .zip(d.measures())
                .enumerate()
            {
                let Some(label) = label else {
                    continue;
                };
                let label = tr(label).trim_end_matches(':').to_owned();
                form.number(
                    (&tr!("{measure} (feet)", measure = label), ""),
                    value.into(),
                    (0, most),
                    Vec::new(),
                    move |v| attack(AttackInput::Measure(k, v as i32)),
                );
            }
            form.group(tr("Saving Throws"), "");
            pick(
                form,
                tr("Save Against"),
                &SAVES,
                d.save(),
                AttackInput::Save,
            );
            attack_number(
                form,
                tr("Modified by"),
                a.save_modifier,
                0,
                AttackInput::Modifier,
            );
            pick(
                form,
                tr("To gain this"),
                &SAVE_RESULTS,
                d.result(),
                AttackInput::Result,
            );
            tr("Monster Attack").to_owned()
        }
    };
    let close = form.button(
        tr("_Cancel"),
        "",
        true,
        rec(RecordInput::CloseDialog(false)),
    );
    let accept: FormButton = form.button(tr("_OK"), "", true, rec(RecordInput::CloseDialog(true)));
    let groups = form.take_groups();
    form.put_groups(page);
    FormDialog {
        title,
        groups,
        close,
        accept: Some(accept),
    }
}

/// What an action does to a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RecordAct {
    New,
    Copy,
    Edit,
    Delete,
    NewSubrace,
    /// Schools… on a spell.
    Schools,
}

impl RecordAct {
    fn of(self) -> Act {
        match self {
            Self::New => Act::New,
            Self::Copy => Act::Copy,
            Self::Edit => Act::Edit,
            Self::Delete => Act::Delete,
            Self::NewSubrace => Act::NewSubrace,
            Self::Schools => Act::Schools,
        }
    }
}

/// A part type New in the other parts' section offers.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PartType {
    /// The section of its parts.
    pub section: String,
    pub name: String,
}

/// What an action on a record leads to.
#[derive(uniffi::Enum)]
pub enum RecordOpened {
    /// The record's editor.
    Editor { session: Arc<RecordSession> },
    /// Nothing opens, for this reason.
    Refused { message: String },
    /// Delete asks this first.
    AskDelete { title: String, text: String },
    /// Delete deletes at once, as the originals do for some kinds.
    DeleteNow,
    /// New in the other parts' section asks for the type first.
    ChooseType { types: Vec<PartType>, chosen: u32 },
    /// The action does nothing on this record.
    Nothing,
}

/// Acts on the record `name` of `section`, custom or standard; `shown`
/// is the key of the entry in view.
pub(crate) fn act_on(
    install: &Arc<Installation>,
    section: &str,
    custom: bool,
    name: &str,
    act: RecordAct,
    shown: Option<&str>,
) -> Result<RecordOpened, CoreError> {
    let origin = if custom {
        Origin::Custom
    } else {
        Origin::Standard
    };
    let opened =
        actions::open(install, section, origin, name, act.of(), shown).map_err(CoreError::new)?;
    Ok(match opened {
        Opened::Form(form) => RecordOpened::Editor {
            session: RecordSession::new(install.clone(), Editor::Record(form)),
        },
        Opened::Table(form) => RecordOpened::Editor {
            session: RecordSession::new(install.clone(), Editor::Table(form)),
        },
        Opened::Refused(message) => RecordOpened::Refused { message },
        Opened::Delete(Some((title, text))) => RecordOpened::AskDelete { title, text },
        Opened::Delete(None) => RecordOpened::DeleteNow,
        Opened::ChooseType { types, chosen } => RecordOpened::ChooseType {
            types: types
                .into_iter()
                .map(|(section, name)| PartType {
                    section: section.to_owned(),
                    name: name.to_owned(),
                })
                .collect(),
            chosen: chosen as u32,
        },
        // The templates' editor comes next.
        Opened::Template(_) | Opened::Nothing => RecordOpened::Nothing,
    })
}

/// Whether the Mac app has the editor of `section`'s records yet.
pub(crate) fn edited_here(section: &str) -> bool {
    !matches!(
        cr_app::records::kind_of(section),
        None | Some(cr_app::records::Kind::Templates)
    )
}
