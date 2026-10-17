//! `cr party` and `cr delete`: the Character dialog's Party Management…
//! and Delete.

use cr_store::party::{ALL, party_of};
use cr_store::{Error, Installation, Result};

/// The file of the character `arg` names: its file in `Chars/` or its name.
fn character_file(install: &Installation, arg: &str) -> Result<String> {
    let index = install.characters()?;
    let arg = arg.trim();
    if let Some(e) = index
        .entries
        .iter()
        .find(|e| e.file.eq_ignore_ascii_case(arg))
    {
        return Ok(e.file.clone());
    }
    let lower = arg.to_lowercase();
    let mut found: Vec<_> = index
        .entries
        .iter()
        .filter(|e| e.name.trim().to_lowercase() == lower)
        .collect();
    if found.is_empty() {
        // The start of a name will do when only one name starts so.
        found = index
            .entries
            .iter()
            .filter(|e| e.name.to_lowercase().starts_with(&lower))
            .collect();
    }
    match found.as_slice() {
        [one] => Ok(one.file.clone()),
        [] => Err(Error::Invalid(format!("no character is named {arg}"))),
        _ => Err(Error::Invalid(format!(
            "{arg} names several characters; give the file ({})",
            found
                .iter()
                .map(|e| e.file.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn list(install: &Installation) -> Result<()> {
    let index = install.characters()?;
    let mut parties = install.parties()?;
    parties.push(String::new());
    for party in parties {
        let members: Vec<_> = index
            .entries
            .iter()
            .filter(|e| party_of(e).eq_ignore_ascii_case(&party))
            .collect();
        if members.is_empty() {
            continue;
        }
        println!(
            "{}",
            if party.is_empty() {
                "(no party)"
            } else {
                &party
            }
        );
        for e in members {
            println!("  {}\t{}", e.name, e.file);
        }
    }
    Ok(())
}

pub fn cmd_party(install: &Installation, args: &[String]) -> Result<()> {
    let usage =
        "usage: cr party INSTALL_DIR [PARTY CHARACTER... | --none CHARACTER... | --delete PARTY]";
    let Some(first) = args.first() else {
        return list(install);
    };
    if !install.has_parties() {
        return Err(Error::Invalid("Core Rules 2.0 has no parties".into()));
    }
    if first == "--delete" {
        let [_, party] = args else {
            return Err(Error::Invalid(usage.into()));
        };
        let deleted = install.delete_party(party)?;
        if deleted.is_empty() {
            return Err(Error::Invalid(format!("the party {party} has no members")));
        }
        install.forget_deleted(&deleted)?;
        for d in &deleted {
            println!("deleted {}\t{}", d.name, d.file);
        }
        return Ok(());
    }
    if args.len() < 2 {
        return Err(Error::Invalid(usage.into()));
    }
    let party = if first == "--none" {
        String::new()
    } else if first.trim().eq_ignore_ascii_case(ALL) {
        return Err(Error::Invalid(format!(
            "You may not add to the {ALL} category"
        )));
    } else {
        // An existing party under its own spelling, else a new one.
        match install
            .parties()?
            .into_iter()
            .find(|p| p.eq_ignore_ascii_case(first.trim()))
        {
            Some(p) => p,
            None => {
                if let Some(problem) = install.party_name_problem(first, &[])? {
                    return Err(Error::Invalid(problem));
                }
                first.trim().to_owned()
            }
        }
    };
    for arg in &args[1..] {
        let file = character_file(install, arg)?;
        let new = install.set_party(&file, &party)?;
        println!("{arg}\t{new}");
    }
    Ok(())
}

pub fn cmd_delete(install: &Installation, args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(Error::Invalid(
            "usage: cr delete INSTALL_DIR CHARACTER...".into(),
        ));
    }
    let files = args
        .iter()
        .map(|a| character_file(install, a))
        .collect::<Result<Vec<_>>>()?;
    for file in files {
        let deleted = install.delete_character(&file)?;
        install.forget_deleted(std::slice::from_ref(&deleted))?;
        println!("deleted {}\t{}", deleted.name, deleted.file);
    }
    Ok(())
}
