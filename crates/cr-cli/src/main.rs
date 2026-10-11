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
                            class+=NAME (humans: dual-class);
                            xp=CLASS:AMOUNT (CLASS 0-4, all or split;
                            AMOUNT may be negative; prime requisite bonus
                            applied); limits=0|1|2 (Expansion level
                            limits: none, racial, with prime bonuses)
  cr sheet INSTALL_DIR FILE [--html|--txt]
                            print the character sheet of Chars/FILE (or of
                            the character file at path FILE) as the export
                            writes it: TXT (default with the Expansion) or
                            HTML (Core Rules 2.0 exports HTML only)
  cr treasure INSTALL_DIR [--seed N] [--electrum] [--accumulate] LETTERS...
                            roll treasure types as the Treasure roller does
                            (A B B Q, or Bx2) and print the bundle and its
                            Calculate summary; a lone / starts another roll
                            from the same dice, and --electrum and
                            --accumulate (add to the bundle before) apply
                            to the roll they are given in
  cr treasure INSTALL_DIR [--seed N] [--count N] --table NAME
                            roll once (or N times) on one treasure table
  cr treasure INSTALL_DIR --saved [NAME]
                            list the saved treasure bundles, or print one

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
        "sheet" => cmd_sheet(&install, &args[2..]),
        "treasure" => cmd_treasure(&install, &args[2..]),
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

fn cmd_sheet(install: &Installation, args: &[String]) -> cr_store::Result<()> {
    use cr_store::sheet::{Flavor, Timestamp};
    let invalid = |message: String| cr_store::Error::Invalid(message);
    let mut file = None;
    let mut html = None;
    for arg in args {
        match arg.as_str() {
            "--html" => html = Some(true),
            "--txt" => html = Some(false),
            _ if file.is_none() => file = Some(arg),
            _ => return Err(invalid(format!("unexpected argument {arg}"))),
        }
    }
    let file = file.ok_or_else(|| invalid("sheet needs a character file".into()))?;
    let path = std::path::Path::new(file);
    let character = if path.is_file() {
        let data = std::fs::read(path).map_err(|source| cr_store::Error::Io {
            path: path.into(),
            source,
        })?;
        cr_formats::character::Character::parse(&data).map_err(|source| {
            cr_store::Error::Format {
                path: path.into(),
                source,
            }
        })?
    } else {
        install.character(file)?
    };
    let sheet = install.sheet(&character)?;
    let html = html.unwrap_or(sheet.flavor == Flavor::Core20);
    if !html && sheet.flavor == Flavor::Core20 {
        return Err(invalid("Core Rules 2.0 exports HTML only".into()));
    }
    let date = Timestamp::now_utc().format(sheet.flavor);
    let text = if html {
        sheet.to_html(&date)
    } else {
        sheet.to_txt(&date)
    };
    print!("{text}");
    Ok(())
}

/// One roll of `cr treasure`: letters, Accumulate Rolls, the coin.
#[derive(Default)]
struct TreasureRoll {
    letters: String,
    accumulate: bool,
    coin: cr_store::treasure::Coin,
}

fn cmd_treasure(install: &Installation, args: &[String]) -> cr_store::Result<()> {
    use cr_store::treasure::{Bundle, Coin, Rand, TreasureRules, parse_letters};
    let invalid = |message: String| cr_store::Error::Invalid(message);
    let mut seed = None;
    let mut table = None;
    let mut count = 1u32;
    let mut saved = None;
    let mut rolls = vec![TreasureRoll::default()];
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| invalid(format!("{name} needs a value")))
        };
        let number = |text: &String| {
            text.parse::<u32>()
                .map_err(|_| invalid(format!("{text} is not a number")))
        };
        let roll = rolls.last_mut().unwrap();
        match arg.as_str() {
            "--seed" => seed = Some(number(value("--seed")?)?),
            "--count" => count = number(value("--count")?)?,
            "--table" => table = Some(value("--table")?.clone()),
            "--electrum" => roll.coin = Coin::Electrum,
            "--accumulate" => roll.accumulate = true,
            "--saved" => saved = Some(None),
            "/" => rolls.push(TreasureRoll::default()),
            _ if saved == Some(None) => saved = Some(Some(arg.clone())),
            _ => {
                roll.letters.push(' ');
                roll.letters.push_str(arg);
            }
        }
    }

    let print = |bundle: &Bundle, rules: &TreasureRules| {
        print!("{}", bundle.to_text());
        println!();
        for line in rules.summary(bundle).lines() {
            println!("{line}");
        }
    };

    if let Some(name) = saved {
        let list = install.saved_treasures()?;
        let Some(name) = name else {
            let mut names: Vec<&str> = list.iter().map(|t| t.name.as_str()).collect();
            names.sort_unstable();
            for name in names {
                println!("{name}");
            }
            return Ok(());
        };
        let treasure = list
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| invalid(format!("no saved treasure named {name}")))?;
        print(&treasure.bundle, &install.treasure_rules()?);
        return Ok(());
    }

    let rules = install.treasure_rules()?;
    let seed = seed.unwrap_or_else(Rand::clock_seed);
    eprintln!("seed {seed}");
    let mut rand = Rand::seeded(seed);
    if let Some(table) = table {
        for _ in 0..count {
            let item = rules
                .roll_table(&mut rand, &table)
                .ok_or_else(|| invalid(format!("no treasure table named {table}")))?;
            print!("{}", Bundle { items: vec![item] }.to_text());
        }
        return Ok(());
    }
    if rolls.iter().all(|r| r.letters.trim().is_empty()) {
        return Err(invalid("treasure needs letters, --table or --saved".into()));
    }
    let mut bundle = Bundle::default();
    for (i, roll) in rolls.iter().enumerate() {
        let counts = parse_letters(&roll.letters).map_err(invalid)?;
        if !roll.accumulate {
            bundle = Bundle::default();
        }
        rules.roll(&mut rand, &counts, roll.coin, &mut bundle);
        if i > 0 {
            println!();
        }
        print(&bundle, &rules);
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
    // Experience goes to a copy, whose changes are saved with the lists.
    let mut advanced = character.clone();
    let mut rules = None;
    let mut dice = cr_store::create::Dice::default();
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
        if field == "xp" || field == "limits" {
            if rules.is_none() {
                rules = Some(install.advance_rules()?);
            }
            let rules = rules.as_ref().unwrap();
            if field == "limits" {
                advanced.data.unknown_0x1dc4 = number()?.min(2);
            } else {
                let (class, amount) = value
                    .split_once(':')
                    .ok_or_else(|| invalid("xp takes CLASS:AMOUNT".into()))?;
                let amount: i64 = amount
                    .parse()
                    .map_err(|_| invalid(format!("{amount} is not a number")))?;
                let n = advanced.data.classes.names.len();
                let (classes, amount) = match class {
                    "all" => ((0..n).collect::<Vec<_>>(), amount),
                    "split" => ((0..n).collect(), amount / n.max(1) as i64),
                    _ => {
                        let g: usize = class
                            .parse()
                            .ok()
                            .filter(|&g| g < n)
                            .ok_or_else(|| invalid(format!("no class {class}")))?;
                        (vec![g], amount)
                    }
                };
                for g in classes.into_iter().rev() {
                    rules.add_experience(&mut advanced, g, amount, true, 3, &mut dice)?;
                }
            }
            lists.advancement = Some(cr_store::advance::Advancement::between(
                &character, &advanced,
            ));
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
