//! `cr userdb`: list, copy and delete the records of the user databases.

use cr_store::userdb::{
    Filter, Origin, UserDb, UserKind, classes, monsters, origin_label, parts, powers, races,
    spells, traps,
};
use cr_store::{Error, Installation, Result};

pub const KINDS: &str = "spells, powers, traps, monster-types, monsters, races, classes, parts";

pub fn cmd(install: &Installation, args: &[String]) -> Result<()> {
    let (Some(kind), Some(action)) = (args.first(), args.get(1)) else {
        return Err(Error::Invalid(format!(
            "usage: cr userdb INSTALL_DIR KIND list|copy|delete ... (KIND: {KINDS})"
        )));
    };
    let rest = &args[2..];
    match kind.as_str() {
        "spells" => run::<spells::Spells>(install, action, rest),
        "powers" => run::<powers::Powers>(install, action, rest),
        "traps" => run::<traps::Traps>(install, action, rest),
        "monster-types" => run::<monsters::MonsterTypes>(install, action, rest),
        "monsters" => run::<monsters::Monsters>(install, action, rest),
        "races" => run::<races::Races>(install, action, rest),
        "classes" => run::<classes::Classes>(install, action, rest),
        "parts" => run::<parts::Parts>(install, action, rest),
        _ => Err(Error::Invalid(format!(
            "unknown kind {kind}; one of {KINDS}"
        ))),
    }
}

/// The record named `name`, user records first, in any scope.
fn find<K: UserKind>(db: &UserDb<K>, name: &str) -> Option<(Origin, usize)> {
    db.list(Filter::All)
        .into_iter()
        .filter(|e| K::name(e.record) == name)
        .min_by_key(|e| e.origin == Origin::Standard)
        .map(|e| (e.origin, e.index))
}

fn run<K: UserKind>(install: &Installation, action: &str, args: &[String]) -> Result<()> {
    let mut db = UserDb::<K>::load(install)?;
    let invalid = |m: String| Error::Invalid(m);
    match (action, args) {
        ("list", rest) => {
            let filter = match rest.first().map(String::as_str) {
                None | Some("--all") => Filter::All,
                Some("--standard") => Filter::Standard,
                Some("--custom") => Filter::Custom,
                Some(other) => return Err(invalid(format!("unknown option {other}"))),
            };
            for e in db.list(filter) {
                let summary = K::summary(e.record);
                let origin = origin_label(e.origin);
                if summary.is_empty() {
                    outln!("{}\t{origin}", K::name(e.record));
                } else {
                    outln!("{}\t{origin}\t{summary}", K::name(e.record));
                }
            }
            Ok(())
        }
        ("copy", [from, new]) => {
            let (origin, index) =
                find(&db, from).ok_or_else(|| invalid(format!("no {} named {from}", K::NOUN)))?;
            let source = db
                .get(origin, index)
                .cloned()
                .unwrap_or_else(|| unreachable!());
            let mut record = db.copy_of(&source);
            K::set_name(&mut record, new);
            match db.add(record.clone()) {
                Ok(_) => {
                    db.save(install)?;
                    outln!("added {new} to {}", K::FILE);
                    Ok(())
                }
                Err(e) => Err(invalid(db.name_message(&e, &record, true))),
            }
        }
        ("delete", [name]) => {
            let index = db
                .user
                .iter()
                .position(|r| K::name(r) == name)
                .ok_or_else(|| invalid(format!("no custom {} named {name}", K::NOUN)))?;
            if let Some(warning) = K::delete_warning(&db.user[index]) {
                eprintln!("{warning}");
            }
            db.delete(index);
            db.save(install)?;
            outln!("deleted {name} from {}", K::FILE);
            Ok(())
        }
        _ => Err(invalid(
            "usage: list [--all|--standard|--custom] | copy FROM NEW_NAME | delete NAME".into(),
        )),
    }
}
