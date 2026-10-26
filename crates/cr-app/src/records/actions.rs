//! What New, Copy, Edit, Delete, New Subrace and Schools… do to a record
//! of the user databases, with no toolkit in them: the editor they open,
//! the question Delete asks, or why nothing opens. They read the
//! installation, away from the window.

use std::sync::Arc;

use cr_i18n::tr;
use cr_store::Installation;
use cr_store::userdb::classes::Classes;
use cr_store::userdb::monsters::{MonsterTypes, Monsters};
use cr_store::userdb::parts::{self, Parts};
use cr_store::userdb::powers::Powers;
use cr_store::userdb::races::Races;
use cr_store::userdb::spells::{self, Spells};
use cr_store::userdb::tables::{self, TableDb};
use cr_store::userdb::templates::Templates;
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{self as udb, Flavor, Origin, UserDb, UserKind};

use super::form::{Only, Opening, RecordForm};
use super::tables::{TableForm, TableSources};
use super::templates::{TemplateData, TemplateForm};
use super::{Kind, kind_of, new_title, static_section};

/// What an action does to a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    New,
    Copy,
    Edit,
    Delete,
    NewSubrace,
    /// Schools… on a spell (0x4feac0).
    Schools,
}

/// What an action on a record leads to.
#[derive(Clone)]
pub enum Opened {
    /// The record's editor.
    Form(Box<RecordForm>),
    /// The treasure table editor.
    Table(Box<TableForm>),
    /// The encounter template editor.
    Template(Box<TemplateForm>),
    /// Nothing opens, for this reason.
    Refused(String),
    /// Delete: ask this (a title and a text) first, or delete at once as
    /// the originals do for some kinds.
    Delete(Option<(String, String)>),
    /// New in the other parts' section asks for the type ("Select the
    /// Type" of IDD 139): the types by section id and translated name,
    /// and the one chosen first.
    ChooseType {
        types: Vec<(&'static str, &'static str)>,
        chosen: usize,
    },
    /// The action does nothing on this record.
    Nothing,
}

/// Acts on the record `name` of `section` that comes from `origin` (New
/// takes neither). `shown` is the key of the entry on the details, whose
/// part type New in the other parts' section starts on, and whose
/// monster type New in the templates' does.
pub fn open(
    install: &Installation,
    section: &str,
    origin: Origin,
    name: &str,
    act: Act,
    shown: Option<&str>,
) -> Result<Opened, String> {
    let (Some(kind), Some(section)) = (kind_of(section), static_section(section)) else {
        return Ok(Opened::Nothing);
    };
    let target = Target {
        section,
        kind,
        origin,
        name,
    };
    let opened = match kind {
        Kind::Spells(scope) if act == Act::Schools => schools(install, &target, scope),
        _ if act == Act::Schools => Ok(Opened::Nothing),
        Kind::Spells(scope) => act_on::<Spells>(install, &target, scope, act),
        Kind::Traps => act_on::<Traps>(install, &target, 0, act),
        Kind::Powers => act_on::<Powers>(install, &target, 0, act),
        Kind::MonsterTypes => act_on::<MonsterTypes>(install, &target, 0, act),
        Kind::Monsters => act_on::<Monsters>(install, &target, 0, act),
        Kind::Races => act_on::<Races>(install, &target, 0, act),
        Kind::Classes => act_on::<Classes>(install, &target, 0, act),
        Kind::Parts(t) => act_on::<Parts>(install, &target, t, act),
        Kind::OtherParts if act == Act::New => {
            let shown = shown.and_then(udb::parse_key).map(|(section, ..)| section);
            Ok(choose_type(cr_store::encounter::flavor_of(install), shown))
        }
        Kind::OtherParts => Ok(Opened::Nothing),
        Kind::Tables => table(install, &target, act),
        // Delete removes a template at once, asking nothing.
        Kind::Templates if act == Act::Delete => act_on::<Templates>(install, &target, 0, act),
        Kind::Templates => template(install, &target, act, shown),
    };
    opened.map_err(|e| e.to_string())
}

/// The record an action is about.
struct Target<'a> {
    section: &'static str,
    kind: Kind,
    origin: Origin,
    name: &'a str,
}

/// The index of the record `name` of `scope` in its own list.
fn position<K: UserKind>(
    db: &UserDb<K>,
    origin: Origin,
    scope: usize,
    name: &str,
) -> Option<usize> {
    let list = match origin {
        Origin::Standard => &db.standard,
        Origin::Custom => &db.user,
    };
    list.iter()
        .position(|r| K::scope(r) == scope && K::name(r) == name)
}

fn act_on<K: UserKind>(
    install: &Installation,
    target: &Target,
    scope: usize,
    act: Act,
) -> cr_store::Result<Opened>
where
    K::Rec: Send + Sync,
{
    let db = UserDb::<K>::load(install)?;
    if K::NOUN == "Class" && db.flavor == Flavor::Core20 {
        return Ok(Opened::Refused(
            tr("Core Rules 2.0 has no custom classes").to_owned(),
        ));
    }
    let found = position(&db, target.origin, scope, target.name);
    let record = found.and_then(|i| db.get(target.origin, i)).cloned();
    let editor = |title: String, record: K::Rec, editing: Option<usize>| {
        let opening = Opening::<K> {
            section: target.section,
            kind: target.kind,
            title,
            record,
            editing,
        };
        RecordForm::open::<K>(install, opening, None).map(|f| Opened::Form(Box::new(f)))
    };
    match act {
        Act::New => editor(
            new_title(target.kind).to_owned(),
            db.new_record(scope, ""),
            None,
        ),
        Act::NewSubrace => {
            // A subrace's subrace belongs to its race.
            let summary = record.as_ref().map(K::summary).unwrap_or_default();
            let parent = summary
                .strip_prefix("Subrace of ")
                .unwrap_or(target.name)
                .to_owned();
            editor(
                tr!("New Subrace of {race}", race = parent.clone()),
                db.new_record(scope, &parent),
                None,
            )
        }
        Act::Copy => {
            let Some(record) = record else {
                return Ok(Opened::Nothing);
            };
            if let Some(refusal) = K::copy_refusal(&record) {
                return Ok(Opened::Refused(refusal));
            }
            editor(
                tr!("Copy of {name}", name = target.name.to_owned()),
                db.copy_of(&record),
                None,
            )
        }
        Act::Edit => {
            let (Some(record), Some(index)) = (record, found) else {
                return Ok(Opened::Nothing);
            };
            if target.origin == Origin::Standard {
                return Ok(Opened::Refused(
                    tr("Standard records cannot be edited; copy them instead").to_owned(),
                ));
            }
            editor(target.name.to_owned(), record, Some(index))
        }
        Act::Delete => {
            let Some(record) = record else {
                return Ok(Opened::Nothing);
            };
            if target.origin == Origin::Standard {
                return Ok(Opened::Refused(
                    tr("Standard records cannot be deleted").to_owned(),
                ));
            }
            Ok(Opened::Delete(K::delete_warning(&record).map(|warning| {
                (
                    tr!("Delete {name}?", name = target.name.to_owned()),
                    warning,
                )
            })))
        }
        Act::Schools => Ok(Opened::Nothing),
    }
}

/// Schools… on a spell: its editor, standard or custom, with only
/// Schools and Spheres open (0x6a40ae); Save writes them into the spell
/// where it is.
fn schools(install: &Installation, target: &Target, scope: usize) -> cr_store::Result<Opened> {
    let db = UserDb::<Spells>::load(install)?;
    let Some(record) = position(&db, target.origin, scope, target.name)
        .and_then(|i| db.get(target.origin, i))
        .cloned()
    else {
        return Ok(Opened::Refused(tr("Spell not found: Error").to_owned()));
    };
    let opening = Opening::<Spells> {
        section: target.section,
        kind: target.kind,
        title: tr!("Schools of {spell}", spell = target.name.to_owned()),
        record,
        editing: None,
    };
    let only = Only {
        labels: &spells::SCHOOL_FIELDS,
        origin: target.origin,
        save: spells::set_schools,
    };
    Ok(Opened::Form(Box::new(RecordForm::open::<Spells>(
        install,
        opening,
        Some(only),
    )?)))
}

/// The part types New in the other parts' section offers, by section id
/// and translated name, starting on the shown part's.
pub fn choose_type(flavor: Flavor, shown: Option<&str>) -> Opened {
    let offered = parts::editable_types(flavor);
    let types: Vec<(&'static str, &'static str)> = parts::OTHER_TYPES
        .iter()
        .filter(|(_, t, _)| offered.iter().any(|(o, _)| o == t))
        .map(|&(id, _, name)| (id, tr(name)))
        .collect();
    let chosen = types
        .iter()
        .position(|(id, _)| shown == Some(*id))
        .unwrap_or(0);
    Opened::ChooseType { types, chosen }
}

/// New, Copy and Edit on an encounter template: its editor.
fn template(
    install: &Installation,
    target: &Target,
    act: Act,
    shown: Option<&str>,
) -> cr_store::Result<Opened> {
    let db = UserDb::<Templates>::load(install)?;
    let found = position(&db, target.origin, 0, target.name);
    let record = found.and_then(|i| db.get(target.origin, i)).cloned();
    let editor = |record, editing, title| -> cr_store::Result<Opened> {
        let data = Arc::new(TemplateData::load(install)?);
        Ok(Opened::Template(Box::new(TemplateForm::new(
            record, editing, title, data,
        ))))
    };
    match (act, record, found) {
        (Act::New, ..) => {
            // The original starts from the monster type selected in its
            // list; here, the shown template's.
            let kind = shown
                .and_then(udb::parse_key)
                .filter(|(section, ..)| *section == target.section)
                .and_then(|(_, origin, name)| {
                    position(&db, origin, 0, name).and_then(|i| db.get(origin, i))
                })
                .map(|t| t.monster_type.clone())
                .unwrap_or_default();
            editor(
                db.new_record(0, &kind),
                None,
                new_title(target.kind).to_owned(),
            )
        }
        (Act::Copy, Some(record), _) => editor(
            db.copy_of(&record),
            None,
            tr!("Copy of {name}", name = target.name.to_owned()),
        ),
        (Act::Edit, Some(_), _) if target.origin == Origin::Standard => Ok(Opened::Refused(
            tr("Standard records cannot be edited; copy them instead").to_owned(),
        )),
        (Act::Edit, Some(record), Some(index)) => {
            editor(record, Some(index), target.name.to_owned())
        }
        _ => Ok(Opened::Nothing),
    }
}

fn table(install: &Installation, target: &Target, act: Act) -> cr_store::Result<Opened> {
    let db = TableDb::load(install)?;
    let index = db.position(target.name);
    let editor = |table, editing, title: Option<String>| -> cr_store::Result<Opened> {
        let sources = Arc::new(TableSources::load(install, &db)?);
        let mut form = TableForm::new(table, editing, sources);
        if let Some(title) = title {
            form.set_title(title);
        }
        Ok(Opened::Table(Box::new(form)))
    };
    match (act, index) {
        (Act::New, _) => editor(TableDb::new_table(), None, None),
        (Act::Copy, Some(i)) => editor(
            TableDb::copy_of(&db.tables[i]),
            None,
            Some(tr!("Copy of {name}", name = target.name.to_owned())),
        ),
        // Every table is edited, the standard ones in the user's copy.
        (Act::Edit, Some(i)) => editor(db.tables[i].clone(), Some(i), None),
        (Act::Delete, Some(i)) if tables::is_standard(&db.tables[i]) => Ok(Opened::Refused(
            tr("Standard tables can be restored, not deleted").to_owned(),
        )),
        (Act::Delete, Some(_)) => Ok(Opened::Delete(Some(super::tables::delete_question()))),
        _ => Ok(Opened::Nothing),
    }
}

/// Deletes the custom record `name` of `section`, with what it takes
/// along; what to say.
pub fn delete(install: &Installation, section: &str, name: &str) -> Result<String, String> {
    let deleted = match kind_of(section) {
        Some(Kind::Spells(scope)) => delete_of::<Spells>(install, scope, name),
        Some(Kind::Traps) => delete_of::<Traps>(install, 0, name),
        Some(Kind::Powers) => delete_of::<Powers>(install, 0, name),
        Some(Kind::MonsterTypes) => delete_of::<MonsterTypes>(install, 0, name),
        Some(Kind::Monsters) => delete_of::<Monsters>(install, 0, name),
        Some(Kind::Races) => delete_of::<Races>(install, 0, name),
        Some(Kind::Classes) => delete_of::<Classes>(install, 0, name),
        Some(Kind::Parts(t)) => delete_of::<Parts>(install, t, name),
        Some(Kind::Templates) => delete_of::<Templates>(install, 0, name),
        Some(Kind::Tables) => delete_table(install, name),
        Some(Kind::OtherParts) | None => Ok(()),
    };
    deleted.map_err(|e| e.to_string())?;
    Ok(tr!("“{name}” deleted", name = name.to_owned()))
}

fn delete_of<K: UserKind>(
    install: &Installation,
    scope: usize,
    name: &str,
) -> cr_store::Result<()> {
    let mut db = UserDb::<K>::load(install)?;
    let Some(i) = position(&db, Origin::Custom, scope, name) else {
        return Ok(());
    };
    let record = db.user[i].clone();
    db.delete(i);
    db.save(install)?;
    K::detach(install, &record)
}

fn delete_table(install: &Installation, name: &str) -> cr_store::Result<()> {
    let mut db = TableDb::load(install)?;
    if let Some(i) = db.position(name) {
        db.delete(i).map_err(cr_store::Error::Invalid)?;
    }
    db.save(install)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::form::RecordInput;

    /// The installation `CORERULES_DIR` names, if it is set.
    fn install() -> Option<Installation> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        Installation::open(std::path::Path::new(&dir)).ok()
    }

    /// Every section's New and, on its first standard record, Copy open
    /// an editor whose fields all read, on every page.
    #[test]
    fn every_editor_opens_on_the_shipped_data() {
        let Some(install) = install() else {
            return;
        };
        let catalog = install.catalog();
        for section in super::super::SECTIONS {
            let first = catalog
                .iter()
                .find(|s| s.id == section)
                .and_then(|s| s.entries.first())
                .and_then(|e| cr_store::userdb::parse_key(&e.key))
                .map(|(_, origin, name)| (origin, name.to_owned()));
            let mut acts = vec![(Act::New, Origin::Standard, String::new())];
            if let Some((origin, name)) = first {
                acts.push((Act::Copy, origin, name));
            }
            for (act, origin, name) in acts {
                let opened = open(&install, section, origin, &name, act, None)
                    .unwrap_or_else(|e| panic!("{section} {act:?}: {e}"));
                match opened {
                    Opened::Form(mut form) => {
                        for page in 0..form.pages().len() {
                            form.input(RecordInput::Page(page));
                            for group in form.pages()[page].groups.clone() {
                                for field in group.fields {
                                    let f = form.field(field);
                                    let _ = (form.choices(field), form.entries(field));
                                    let _ = (form.list_choices(field), f.number_open());
                                }
                            }
                        }
                        assert!(!form.changed(), "{section}: changed on opening");
                    }
                    Opened::Table(form) => {
                        assert!(!form.sources().items.is_empty());
                        let _ = form.rows();
                    }
                    Opened::Template(form) => {
                        assert!(!form.data().monsters.is_empty());
                        assert!(!form.changed(), "{section}: changed on opening");
                    }
                    Opened::Refused(_) | Opened::Nothing => {}
                    Opened::Delete(_) | Opened::ChooseType { .. } => {
                        panic!("{section} {act:?} did not open an editor")
                    }
                }
            }
        }
    }
}
