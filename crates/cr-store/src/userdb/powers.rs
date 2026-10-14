//! Custom Powers (`UserDbas/PsionicU.dat`): IDD 344 "Psionic Powers List"
//! and IDD 346 "Psionic Power Information".

use cr_formats::Object;
use cr_formats::psionic::PsionicPower;

use super::{
    FieldDesc, FieldKind, Flavor, NameError, NewContext, Options, UserKind, Value, default_reason,
    udb_check, udb_int, udb_list, udb_multiline, udb_text,
};
use crate::{Installation, Result};

pub struct Powers;

/// The Discipline combo (ctl 2256), by `unknown_0x28`.
pub const DISCIPLINES: &[&str] = &[
    "Clairsentient",
    "Psychokinetic",
    "Psychometabolic",
    "Psychoportive",
    "Telepathic",
];

/// The Science/Devotion radio (ctl 2261), by `unknown_0x2c`.
pub const POWER_TYPES: &[&str] = &["Science", "Devotion"];

impl UserKind for Powers {
    type Rec = PsionicPower;
    const NOUN: &'static str = "Power";
    const FILE: &'static str = "UserDbas/PsionicU.dat";

    fn name(r: &PsionicPower) -> &str {
        &r.name
    }

    fn set_name(r: &mut PsionicPower, name: &str) {
        name.clone_into(&mut r.name);
    }

    fn standard(install: &Installation) -> Result<Vec<PsionicPower>> {
        super::standard_list(install, "Psionic", pick)
    }

    fn parse_user(data: &[u8]) -> std::result::Result<Vec<PsionicPower>, cr_formats::Error> {
        super::parse_list(data).map(|l| l.into_iter().filter_map(pick).collect())
    }

    fn user_bytes(records: &[PsionicPower]) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        super::list_bytes(
            records
                .iter()
                .map(|p| Object::PsionicPower(Box::new(p.clone())))
                .collect(),
        )
    }

    fn new_record(_context: &NewContext) -> PsionicPower {
        PsionicPower {
            name: String::new(),
            // The help id of a new power.
            unknown_0x24: u32::MAX,
            unknown_0x28: 0,
            unknown_0x2c: 0,
            // MAC.
            unknown_0x30: 10,
            unknown_0x34: 0,
            unknown_0x38: 0,
            cost: String::new(),
            unknown_0x48: 0,
            unknown_0x44: 0,
            range: String::new(),
            area: String::new(),
            description: String::new(),
            unknown_0x40: 0,
            unknown_0x1c: 0,
            prerequisites: Vec::new(),
        }
    }

    fn fields(_flavor: Flavor) -> Vec<FieldDesc<PsionicPower>> {
        type Rec = PsionicPower;
        const MAIN: &str = "Power";
        vec![
            udb_text!(MAIN, "Name", 50, name),
            FieldDesc::new(
                MAIN,
                "Discipline",
                FieldKind::Choice(Options::Fixed(DISCIPLINES)),
                |r: &PsionicPower, _| Value::Int(r.unknown_0x28.into()),
                |r, _, v, _| r.unknown_0x28 = v.int() as u32,
            ),
            FieldDesc::new(
                MAIN,
                "Type",
                FieldKind::Choice(Options::Fixed(POWER_TYPES)),
                |r: &PsionicPower, _| Value::Int(r.unknown_0x2c.min(1).into()),
                |r, _, v, _| r.unknown_0x2c = v.int() as u32,
            ),
            udb_int!(MAIN, "MAC", -10, 10, unknown_0x30).group("MAC"),
            udb_check!(MAIN, "Base", unknown_0x34).group("MAC"),
            udb_check!(MAIN, "Varies", unknown_0x38).group("MAC"),
            udb_text!(MAIN, "PSP Cost", 50, cost).group("Cost and Reach"),
            udb_int!(MAIN, "Minimum PSP Cost to Use Once", 0, 10000, unknown_0x48)
                .group("Cost and Reach"),
            udb_text!(MAIN, "Range", 50, range).group("Cost and Reach"),
            udb_text!(MAIN, "Area of Effect", 50, area).group("Cost and Reach"),
            udb_int!(MAIN, "Minimum Level", 0, 10000, unknown_0x44).group("Cost and Reach"),
            udb_list!(
                "Prerequisites",
                "Prerequisites",
                Options::Source("powers"),
                prerequisites
            ),
            udb_multiline!("Description", "Description", description),
        ]
    }

    fn name_message(error: &NameError, r: &PsionicPower, _adding: bool) -> String {
        let reason = match error {
            NameError::Used | NameError::UsedByStandard => format!(
                "Name already used in catagory {}.",
                DISCIPLINES
                    .get(r.unknown_0x28 as usize)
                    .copied()
                    .unwrap_or_default()
            ),
            e => default_reason(e),
        };
        format!("Power not added: Error. {reason}")
    }

    fn summary(r: &PsionicPower) -> String {
        let discipline = DISCIPLINES
            .get(r.unknown_0x28 as usize)
            .copied()
            .unwrap_or("");
        let kind = POWER_TYPES[r.unknown_0x2c.min(1) as usize];
        format!("{discipline} {}", kind.to_lowercase())
    }
}

fn pick(o: Object) -> Option<PsionicPower> {
    match o {
        Object::PsionicPower(p) => Some(*p),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::UserDb;
    use crate::userdb::testing::TempInstall;

    #[test]
    fn new_power_defaults_and_messages() {
        let t = TempInstall::new("powers", &[("UserDbas/PsionicU.dat", vec![0, 0])]);
        let mut db = UserDb::<Powers>::load(&t.install).unwrap();
        let mut p = db.new_record(0, "");
        assert_eq!((p.unknown_0x24, p.unknown_0x30), (u32::MAX, 10));
        p.name = "Mind bar".into();
        db.add(p.clone()).unwrap();
        p.unknown_0x28 = 4;
        assert_eq!(
            Powers::name_message(&db.validate_name(&p, None).unwrap_err(), &p, true),
            "Power not added: Error. Name already used in catagory Telepathic."
        );
        db.save(&t.install).unwrap();
        assert_eq!(UserDb::<Powers>::load(&t.install).unwrap().user.len(), 1);
    }
}
