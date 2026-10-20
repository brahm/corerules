//! Custom Spells (`UserDbas/SpellsU.dat`): IDD 208 "Spells" and the IDD
//! 169 editor. The file holds the wizard array, then the priest array; a
//! name must be unique within its spell type.

use cr_formats::Object;
use cr_formats::spells::{Spell, SpellsFile};
use cr_i18n::{tr, tr_noop};

use super::{
    FieldDesc, FieldKind, Flavor, NameError, NewContext, Options, UserKind, Value, default_reason,
    udb_check, udb_combo, udb_list, udb_multiline, udb_text,
};
use crate::{Error, Installation, Result};

pub struct Spells;

/// The spell types, the list's radio buttons, by scope.
pub const SPELL_TYPES: [&str; 2] = [tr_noop!("Wizard"), tr_noop!("Priest")];

/// The Components combo (ctl 1333).
pub const COMPONENTS: &[&str] = &["V", "S", "M", "V, S, M", "V, S", "V, M", "S, M"];

fn is_priest(s: &Spell) -> bool {
    s.priest != 0
}

impl UserKind for Spells {
    type Rec = Spell;
    const NOUN: &'static str = tr_noop!("Spell");
    const FILE: &'static str = "UserDbas/SpellsU.dat";

    fn name(r: &Spell) -> &str {
        &r.name
    }

    fn set_name(r: &mut Spell, name: &str) {
        name.clone_into(&mut r.name);
    }

    fn scope(r: &Spell) -> usize {
        usize::from(is_priest(r))
    }

    fn standard(install: &Installation) -> Result<Vec<Spell>> {
        let relative = "Database/Spells.dat";
        let Some(path) = install.path(relative) else {
            return Ok(Vec::new());
        };
        let file = SpellsFile::parse(&install.read(relative)?)
            .map_err(|source| Error::Format { path, source })?;
        Ok(spells_of(file))
    }

    fn parse_user(data: &[u8]) -> std::result::Result<Vec<Spell>, cr_formats::Error> {
        SpellsFile::parse(data).map(spells_of)
    }

    fn user_bytes(records: &[Spell]) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        let object = |s: &Spell| Object::Spell(Box::new(s.clone()));
        SpellsFile {
            wizard: records
                .iter()
                .filter(|s| !is_priest(s))
                .map(object)
                .collect(),
            priest: records
                .iter()
                .filter(|s| is_priest(s))
                .map(object)
                .collect(),
        }
        .to_bytes()
    }

    fn new_record(context: &NewContext) -> Spell {
        let priest = context.scope == 1;
        Spell {
            name: String::new(),
            unknown_0x60: 0,
            unknown_0x64: 0,
            wizard: i32::from(!priest),
            priest: i32::from(priest),
            level: 1,
            area_of_effect: String::new(),
            casting_time: String::new(),
            components: String::new(),
            unknown_0x44: String::new(),
            duration: String::new(),
            unknown_0x4c: String::new(),
            range: String::new(),
            saving_throw: String::new(),
            unknown_0x58: String::new(),
            unknown_0x5c: String::new(),
            id: 0,
            // The Description dialog's text for a new spell.
            unknown_0xc: "Description".into(),
            schools: Vec::new(),
            spheres: Vec::new(),
        }
    }

    fn copied(r: &mut Spell) {
        r.id = 0;
    }

    fn fields(_flavor: Flavor) -> Vec<FieldDesc<Spell>> {
        type Rec = Spell;
        const MAIN: &str = tr_noop!("Spell");
        vec![
            udb_text!(MAIN, tr_noop!("Name"), 50, name),
            FieldDesc::new(
                MAIN,
                tr_noop!("Level"),
                FieldKind::Int { min: 0, max: 9 },
                |r: &Spell, _| Value::Int(r.level.into()),
                |r, _, v, _| r.level = v.int() as i32,
            ),
            udb_text!(MAIN, tr_noop!("Range"), 0, range).group(tr_noop!("Statistics")),
            udb_text!(MAIN, tr_noop!("Duration"), 0, duration).group(tr_noop!("Statistics")),
            udb_text!(MAIN, tr_noop!("Area of Effect"), 0, area_of_effect)
                .group(tr_noop!("Statistics")),
            udb_combo!(
                MAIN,
                tr_noop!("Components"),
                Options::Fixed(COMPONENTS),
                true,
                components
            )
            .group(tr_noop!("Statistics")),
            udb_text!(MAIN, tr_noop!("Casting Time"), 0, casting_time)
                .group(tr_noop!("Statistics")),
            udb_text!(MAIN, tr_noop!("Saving Throw"), 0, saving_throw)
                .group(tr_noop!("Statistics")),
            udb_text!(MAIN, tr_noop!("Subtlety"), 0, unknown_0x5c)
                .group(tr_noop!("Combat & Tactics")),
            udb_text!(MAIN, tr_noop!("Sensory"), 0, unknown_0x58)
                .group(tr_noop!("Combat & Tactics")),
            udb_text!(MAIN, tr_noop!("Knockdown"), 0, unknown_0x4c)
                .group(tr_noop!("Combat & Tactics")),
            udb_text!(MAIN, tr_noop!("Critical"), 0, unknown_0x44)
                .group(tr_noop!("Combat & Tactics")),
            udb_check!(MAIN, tr_noop!("Reversible"), unknown_0x64).group(tr_noop!("Options")),
            udb_check!(MAIN, tr_noop!("Never Ban Cantrip"), unknown_0x60)
                .group(tr_noop!("Options"))
                .enabled(|r| r.level == 0 && !is_priest(r)),
            udb_list!(
                tr_noop!("Schools"),
                tr_noop!("Schools"),
                Options::Source("Schools Of Magic"),
                schools
            ),
            udb_list!(
                tr_noop!("Spheres"),
                tr_noop!("Spheres"),
                Options::Source("Spheres"),
                spheres
            )
            .visible(is_priest),
            udb_multiline!(
                tr_noop!("Description"),
                tr_noop!("Description"),
                unknown_0xc
            ),
        ]
    }

    fn name_message(error: &NameError, _r: &Spell, adding: bool) -> String {
        let reason = default_reason(error);
        if adding {
            tr!("Spell not added: Error. {reason}", reason)
        } else {
            tr!("Spell not Modified. Error. {reason}", reason)
        }
    }

    fn summary(r: &Spell) -> String {
        if Self::scope(r) == 1 {
            tr!("Priest level {level}", level = r.level)
        } else {
            tr!("Wizard level {level}", level = r.level)
        }
    }
}

/// Every spell, standard (`Database/Spells.dat`) then user, in file order.
pub fn all_spells(install: &Installation) -> Result<Vec<Spell>> {
    let mut spells = Spells::standard(install)?;
    if let Some(path) = install.path(Spells::FILE) {
        let file = SpellsFile::parse(&install.read(Spells::FILE)?)
            .map_err(|source| Error::Format { path, source })?;
        spells.extend(spells_of(file));
    }
    Ok(spells)
}

fn spells_of(file: SpellsFile) -> Vec<Spell> {
    file.wizard
        .into_iter()
        .chain(file.priest)
        .filter_map(|o| match o {
            Object::Spell(s) => Some(*s),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::testing::TempInstall;
    use crate::userdb::{Filter, Origin, UserDb};

    fn standard() -> Vec<u8> {
        let mut sleep = Spells::new_record(&NewContext {
            flavor: Flavor::Expansion,
            scope: 0,
            parent: String::new(),
        });
        sleep.name = "Sleep".into();
        sleep.id = 7;
        let mut bless = sleep.clone();
        bless.name = "Bless".into();
        bless.wizard = 0;
        bless.priest = 1;
        Spells::user_bytes(&[sleep, bless]).unwrap()
    }

    #[test]
    fn spells_are_unique_per_type_and_written_by_type() {
        let t = TempInstall::new(
            "spells",
            &[
                ("Database/Spells.dat", standard()),
                ("UserDbas/SpellsU.dat", vec![0; 4]),
            ],
        );
        let mut db = UserDb::<Spells>::load(&t.install).unwrap();
        assert_eq!(db.list(Filter::All).len(), 2);
        assert!(db.list(Filter::Custom).is_empty());

        let mut spell = db.new_record(0, "");
        assert_eq!(db.add(spell.clone()), Err(NameError::Empty));
        spell.name = " sleep ".into();
        assert_eq!(db.add(spell.clone()), Err(NameError::UsedByStandard));
        assert_eq!(
            Spells::name_message(&NameError::UsedByStandard, &spell, true),
            "Spell not added: Error. Name already used."
        );
        // A priest spell may share a wizard spell's name.
        let mut priest = db.new_record(1, "");
        priest.name = "Sleep".into();
        assert_eq!(db.add(priest), Ok(0));
        spell.name = "  Udb bolt ".into();
        assert_eq!(db.add(spell.clone()), Ok(1));
        assert_eq!(db.user[1].name, "Udb bolt");
        spell.name = "UDB BOLT".into();
        assert_eq!(db.add(spell.clone()), Err(NameError::Used));
        // Editing may keep its own name.
        let mut edited = db.user[1].clone();
        edited.level = 3;
        assert_eq!(db.replace(1, edited), Ok(1));

        db.save(&t.install).unwrap();
        let reread = UserDb::<Spells>::load(&t.install).unwrap();
        // Wizard spells come first in the file.
        assert_eq!(reread.user[0].name, "Udb bolt");
        assert_eq!(reread.user[0].level, 3);
        assert_eq!(reread.user[1].name, "Sleep");
        assert!(t.root.join("UserDbas/SpellsU.dat.bak").exists());
        let names: Vec<_> = reread
            .list(Filter::All)
            .iter()
            .map(|e| (e.record.name.as_str(), e.origin))
            .collect();
        assert_eq!(names[0], ("Bless", Origin::Standard));

        let copy = db.copy_of(&db.standard[0]);
        assert_eq!((copy.name.as_str(), copy.id), ("", 0));
        db.delete(0);
        assert_eq!(db.user.len(), 1);

        // An imported user spell with a standard spell's name can be
        // edited if it keeps the name, but not renamed to another one.
        let mut imported = db.new_record(0, "");
        imported.name = "Sleep".into();
        db.user.push(imported);
        let mut edited = db.user[1].clone();
        edited.level = 2;
        assert_eq!(db.replace(1, edited.clone()), Ok(1));
        edited.name = "Bless".into();
        edited.wizard = 0;
        edited.priest = 1;
        assert_eq!(db.replace(1, edited), Err(NameError::UsedByStandard));
    }

    #[test]
    fn never_ban_cantrip_needs_a_wizard_cantrip() {
        let fields = Spells::fields(Flavor::Expansion);
        let cantrip = fields
            .iter()
            .find(|f| f.label == "Never Ban Cantrip")
            .unwrap();
        let mut s = Spells::new_record(&NewContext {
            flavor: Flavor::Expansion,
            scope: 0,
            parent: String::new(),
        });
        assert!(!cantrip.is_enabled(&s));
        s.level = 0;
        assert!(cantrip.is_enabled(&s));
        let spheres = fields.iter().find(|f| f.label == "Spheres").unwrap();
        assert!(!spheres.is_visible(&s));
    }
}
