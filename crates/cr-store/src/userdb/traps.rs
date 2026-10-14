//! Traps (`UserDbas/TrapsU.dat`): IDD 295 "Traps" and IDD 296 "Trap
//! Detail", seven strings of at most 50 characters.

use cr_formats::Object;
use cr_formats::traps::Trap;

use super::{FieldDesc, Flavor, NewContext, UserKind, udb_text};
use crate::{Installation, Result};

pub struct Traps;

impl UserKind for Traps {
    type Rec = Trap;
    const NOUN: &'static str = "Trap";
    const FILE: &'static str = "UserDbas/TrapsU.dat";

    fn name(r: &Trap) -> &str {
        &r.name
    }

    fn set_name(r: &mut Trap, name: &str) {
        name.clone_into(&mut r.name);
    }

    fn standard(install: &Installation) -> Result<Vec<Trap>> {
        super::standard_list(install, "Traps", pick)
    }

    fn parse_user(data: &[u8]) -> std::result::Result<Vec<Trap>, cr_formats::Error> {
        super::parse_list(data).map(|l| l.into_iter().filter_map(pick).collect())
    }

    fn user_bytes(records: &[Trap]) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        super::list_bytes(
            records
                .iter()
                .map(|t| Object::Trap(Box::new(t.clone())))
                .collect(),
        )
    }

    fn new_record(_context: &NewContext) -> Trap {
        Trap {
            name: String::new(),
            damage: String::new(),
            unknown_0x14: String::new(),
            unknown_0x10: String::new(),
            unknown_0xc: String::new(),
            unknown_0x8: String::new(),
            unknown_0x4: String::new(),
        }
    }

    /// The controls of IDD 296 (the member each one writes, from the
    /// golden `TrapsU_01_udbpit.dat`).
    fn fields(_flavor: Flavor) -> Vec<FieldDesc<Trap>> {
        type Rec = Trap;
        const PAGE: &str = "Trap";
        vec![
            udb_text!(PAGE, "Trap Name", 50, name),
            udb_text!(PAGE, "Location", 50, unknown_0x4),
            udb_text!(PAGE, "Damage", 50, damage),
            udb_text!(PAGE, "Detection Modifier", 50, unknown_0x14),
            udb_text!(PAGE, "Number of Characters Trapped", 50, unknown_0x10),
            udb_text!(PAGE, "Saving Throw", 50, unknown_0xc),
            udb_text!(PAGE, "Special Effects", 50, unknown_0x8),
        ]
    }

    fn summary(r: &Trap) -> String {
        r.damage.clone()
    }
}

fn pick(o: Object) -> Option<Trap> {
    match o {
        Object::Trap(t) => Some(*t),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::testing::TempInstall;
    use crate::userdb::{NameError, UserDb};

    #[test]
    fn traps_are_appended_and_deleted() {
        let mut deadfall = Traps::new_record(&NewContext {
            flavor: Flavor::Expansion,
            scope: 0,
            parent: String::new(),
        });
        deadfall.name = "Deadfall".into();
        let t = TempInstall::new(
            "traps",
            &[
                (
                    "Database/Traps.dat",
                    Traps::user_bytes(std::slice::from_ref(&deadfall)).unwrap(),
                ),
                ("UserDbas/TrapsU.dat", vec![0, 0]),
            ],
        );
        let mut db = UserDb::<Traps>::load(&t.install).unwrap();
        let fields = Traps::fields(Flavor::Expansion);
        let mut pit = db.new_record(0, "");
        for (field, value) in fields.iter().zip(["Pit", "Corridor", "2d6"]) {
            field.set(
                &mut pit,
                &super::super::Value::Text(value.into()),
                &Default::default(),
            );
        }
        assert_eq!(
            (pit.unknown_0x4.as_str(), pit.damage.as_str()),
            ("Corridor", "2d6")
        );
        assert_eq!(db.add(pit.clone()), Ok(0));
        let mut again = pit.clone();
        again.name = "deadfall".into();
        assert_eq!(db.add(again), Err(NameError::UsedByStandard));
        db.save(&t.install).unwrap();
        assert_eq!(UserDb::<Traps>::load(&t.install).unwrap().user, vec![pit]);
        db.delete(0);
        db.save(&t.install).unwrap();
        assert_eq!(
            std::fs::read(t.root.join("UserDbas/TrapsU.dat")).unwrap(),
            [0, 0]
        );
    }
}
