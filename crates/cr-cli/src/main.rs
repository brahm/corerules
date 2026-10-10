//! `cr`: inspect an AD&D Core Rules 2.0 installation from the command line.

use std::process::ExitCode;

use cr_formats::character::Character;
use cr_formats::database::survey;
use cr_store::Installation;
use cr_store::edit::CharacterDetails;
use cr_store::lists::{CharacterLists, Item};

const USAGE: &str = "usage:
  cr survey [INSTALL_DIR]   list database files, record counts and classes
  cr chars [INSTALL_DIR]    list characters
  cr edit INSTALL_DIR FILE FIELD=VALUE...
                            change a character in Chars/FILE; fields: name,
                            player, social_class, age, height, weight,
                            str, dex, con, int, wis, cha (score and both
                            subabilities), exceptional; LIST+=NAME and
                            LIST-=NAME for the lists weapon, nwp, language,
                            wizard, priest (spells known); equipment animal,
                            food, item (carried), readied, stored, worn,
                            service, transport; book+=SPELL:PAGES (first
                            spellbook); memorized+=CLASS:LEVEL:SPELL and
                            priest-memorized+=...; coins cp= sp= ep= gp= pp=;
                            class+=NAME (humans: dual-class)

Without INSTALL_DIR, the installation in the Wine prefix is used.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(command), dir) = (args.first(), args.get(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let install = match dir {
        Some(dir) => Installation::open(dir),
        None => Installation::discover()
            .ok_or_else(|| cr_store::Error::NotAnInstallation(cr_store::DEFAULT_WINE_PATH.into())),
    };
    let result = install.and_then(|install| match command.as_str() {
        "survey" => cmd_survey(&install),
        "chars" => cmd_chars(&install),
        "edit" => cmd_edit(&install, &args[2..]),
        _ => {
            eprintln!("{USAGE}");
            Ok(())
        }
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cr: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_survey(install: &Installation) -> cr_store::Result<()> {
    for folder in ["Database", "UserDbas"] {
        println!("{folder}/");
        for path in install.data_files(folder)? {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let data = std::fs::read(&path).map_err(|source| cr_store::Error::Io {
                path: path.clone(),
                source,
            })?;
            match survey(&data) {
                Ok(s) => {
                    let class = s
                        .class
                        .map(|c| format!("{} (schema {})", c.name, c.schema))
                        .unwrap_or_default();
                    println!("  {name:<14} {:>6} records  {class}", s.records);
                }
                Err(e) => println!("  {name:<14} not a record list: {e}"),
            }
        }
    }
    Ok(())
}

fn cmd_chars(install: &Installation) -> cr_store::Result<()> {
    let index = install.characters()?;
    for entry in &index.entries {
        let character = install
            .read(&format!("Chars/{}", entry.file))
            .ok()
            .and_then(|data| Character::parse(&data).ok());
        let version = character
            .as_ref()
            .map(|c| c.version.as_str())
            .unwrap_or("unreadable");
        println!(
            "{:<24} {:<10} {}  [{}, {}]",
            entry.name,
            entry.race,
            entry.details.get(4..).unwrap_or_default().join(", "),
            entry.file,
            version
        );
    }
    Ok(())
}

fn cmd_edit(install: &Installation, args: &[String]) -> cr_store::Result<()> {
    let invalid = |message: String| cr_store::Error::Invalid(message);
    let (file, changes) = args
        .split_first()
        .ok_or_else(|| invalid("edit needs a character file".into()))?;
    let character = install.character(file)?;
    let mut details = CharacterDetails::of(&character);
    let mut lists = CharacterLists::of(&character);
    for change in changes {
        if let Some((list, value, add)) = list_change(change) {
            const BUNDLES: [&str; 9] = [
                "animal",
                "food",
                "item",
                "readied",
                "stored",
                "worn",
                "",
                "service",
                "transport",
            ];
            if let Some(b) = BUNDLES.iter().position(|n| !n.is_empty() && *n == list) {
                let items = &mut lists.equipment[b];
                if add {
                    items.push(Item::new(value));
                } else {
                    let i = items
                        .iter()
                        .position(|i| i.name == value)
                        .ok_or_else(|| invalid(format!("{value} is not in {list}")))?;
                    items.remove(i);
                }
                continue;
            }
            if list == "class" && add {
                // class+=NAME adds a class where the race may dual-class.
                lists.new_classes.push(value.into());
                continue;
            }
            if list == "book" {
                // book+=SPELL:PAGES on the first spellbook.
                let book = lists
                    .spellbooks
                    .first_mut()
                    .ok_or_else(|| invalid("the character has no spellbook".into()))?;
                if add {
                    let (spell, pages) = value
                        .split_once(':')
                        .ok_or_else(|| invalid("book+= takes SPELL:PAGES".into()))?;
                    let pages = pages
                        .parse()
                        .map_err(|_| invalid(format!("{pages} is not a number")))?;
                    book.spells.push((spell.into(), pages, true));
                } else {
                    book.spells.retain(|s| s.0 != value);
                }
                continue;
            }
            if list == "memorized" {
                // memorized+=CLASS:LEVEL:SPELL, class 0 for the first class;
                // wizard spells (priest spells with priest-memorized).
                continue_memorized(&mut lists, value, add, true).map_err(invalid)?;
                continue;
            }
            if list == "priest-memorized" {
                continue_memorized(&mut lists, value, add, false).map_err(invalid)?;
                continue;
            }
            let list = match list {
                "weapon" => &mut lists.weapon_proficiencies,
                "nwp" => &mut lists.nonweapon_proficiencies,
                "language" => &mut lists.languages,
                "wizard" => &mut lists.wizard_spells,
                "priest" => &mut lists.priest_spells,
                _ => return Err(invalid(format!("unknown list {list}"))),
            };
            if add {
                list.push(value.into());
            } else {
                let i = list
                    .iter()
                    .position(|n| n == value)
                    .ok_or_else(|| invalid(format!("{value} is not in the list")))?;
                list.remove(i);
            }
            continue;
        }
        let (field, value) = change
            .split_once('=')
            .ok_or_else(|| invalid(format!("{change}: expected FIELD=VALUE")))?;
        let number = || {
            value
                .parse::<u32>()
                .map_err(|_| invalid(format!("{field}: {value} is not a number")))
        };
        const ABILITIES: [&str; 6] = ["str", "dex", "con", "int", "wis", "cha"];
        if let Some(c) = ["cp", "sp", "ep", "gp", "pp"]
            .iter()
            .position(|c| *c == field)
        {
            lists.coins[c] = number()?;
            continue;
        }
        match field {
            "name" => details.name = value.into(),
            "player" => details.player = value.into(),
            "social_class" => details.social_class = value.into(),
            "age" => details.age = number()?,
            "height" => details.height = number()?,
            "weight" => details.weight = number()?,
            "exceptional" => details.exceptional_strength = number()?,
            _ => match ABILITIES.iter().position(|a| *a == field) {
                Some(i) => details.abilities[i] = [number()?; 3],
                None => return Err(invalid(format!("unknown field {field}"))),
            },
        }
    }
    install.save_character(file, &details, Some(&lists))
}

/// Adds or removes a memorized spell given as `CLASS:LEVEL:SPELL`.
fn continue_memorized(
    lists: &mut CharacterLists,
    value: &str,
    add: bool,
    wizard: bool,
) -> Result<(), String> {
    let mut parts = value.splitn(3, ':');
    let (Some(class), Some(level), Some(spell)) = (parts.next(), parts.next(), parts.next()) else {
        return Err("memorized takes CLASS:LEVEL:SPELL".into());
    };
    let class: usize = class
        .parse()
        .map_err(|_| format!("{class} is not a number"))?;
    let level: usize = level
        .parse()
        .map_err(|_| format!("{level} is not a number"))?;
    let slot = lists
        .memorized
        .get_mut(class)
        .and_then(|c| c.get_mut(level))
        .ok_or("class 0-4, level 0-10")?;
    let list = if wizard { &mut slot.0 } else { &mut slot.1 };
    if add {
        list.push(spell.into());
    } else if let Some(i) = list.iter().position(|s| s == spell) {
        list.remove(i);
    }
    Ok(())
}

/// `LIST+=NAME` or `LIST-=NAME`: the list, the name and whether to add.
fn list_change(change: &str) -> Option<(&str, &str, bool)> {
    let (left, value) = change.split_once('=')?;
    if let Some(list) = left.strip_suffix('+') {
        Some((list, value, true))
    } else {
        left.strip_suffix('-').map(|list| (list, value, false))
    }
}
