//! `cr`: inspect an AD&D Core Rules 2.0 installation from the command line.

use std::io::{ErrorKind, Write};
use std::process::ExitCode;

use cr_formats::character::Character;
use cr_formats::database::survey;
use cr_store::Installation;
use cr_store::edit::CharacterDetails;
use cr_store::lists::{CharacterLists, Item};

mod party;
mod transfer;
mod userdb;

const USAGE: &str = "usage:
  cr extract-data SOURCE [DEST_DIR]
                            copy what the app uses out of a Core Rules 2.0
                            or Expansion CD into a data package, as Extract
                            Data from CD... does: SOURCE is the CD's ISO
                            image, the mounted CD, an installation (the
                            folder holding Database/) or a Wine prefix;
                            DEST_DIR, which must not exist, defaults to
                            ~/.local/share/corerules/data while that is
                            free, else a corerules-data-EDITION folder
                            next to it (the Expansion takes the data
                            folder over from 2.0 data, which moves there)
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
  cr encounter INSTALL_DIR --templates [TEXT]
                            list the encounter templates (whose name or
                            monster type contains TEXT)
  cr encounter INSTALL_DIR --template-text NAME
                            print a template as the template printer does
  cr encounter INSTALL_DIR [--seed N] --template NAME [--save NAME]
                            make an encounter from a template as the
                            original does and print it with its totals and
                            treasure; --save adds it to UserDbas/EncountU.dat
  cr encounter INSTALL_DIR --saved [NAME]
                            list the saved encounters, or print one
  cr encounter INSTALL_DIR --tables [NAME]
                            list the 2-20 encounter tables, or print one
                            with the candidates of its filter
  cr encounter INSTALL_DIR [--seed N] --roll TABLE
                            Roll and Generate on a 2-20 table
  cr npc INSTALL_DIR [--seed N] [--race NAME] [--class NAME [--level N]]...
         [--count N] [--po] [--save [--party NAME]]
                            make NPCs as Create NPC does (a race, by default
                            Human, and classes with their levels, default
                            1) and print a summary of each; without --class,
                            as Create Multiple NPCs does (a random race
                            unless --race, classes from AllowedU.dat, every
                            level --level, default 1); --count makes that
                            many (default 1), --po uses Player's Option,
                            --save adds them to Chars/ as NPCs
  cr books [INSTALL_DIR]    list the rule books in Books/ by shelf
  cr book INSTALL_DIR NAME [--outline|--text]
                            print the contents outline (default) or the
                            text of a book; NAME is the file (PhbBk), the
                            registry number or the title
  cr table INSTALL_DIR      list the Table Rolls entries with their keys
  cr table INSTALL_DIR ID [--seed N] [--param N]...
                            roll on a table as Table Rolls does: ID is a
                            list key (dmg:88, phb:10, poct:critical) with
                            --param giving each parameter's option number
                            (from 0), or a table id (20088) rolled as a
                            follow-up roll would
  cr userdb INSTALL_DIR KIND list [--standard|--custom]
  cr userdb INSTALL_DIR KIND copy FROM NEW_NAME
  cr userdb INSTALL_DIR KIND delete NAME
                            list the standard and custom records of a
                            user database, copy one to a new custom record
                            or delete a custom one (UserDbas/ is rewritten,
                            keeping a .bak); KIND: spells, powers, traps,
                            monster-types, monsters, races, classes, parts
  cr export INSTALL_DIR --list
  cr export INSTALL_DIR OUTPUT (--all | ITEM...)
                            write a DM Toolkit package (.xpx with the
                            Expansion, .xpt with Core Rules 2.0) of the
                            user's records and characters; ITEM is a name
                            or CATEGORY:NAME as --list prints them
  cr import INSTALL_DIR FILE [--list] [ITEM...]
  cr import INSTALL_DIR CORE_RULES_2.0_DIR [--list] [ITEM...]
                            list or import a package, or the characters
                            and records of a Core Rules 2.0 installation
                            (converted as the Expansion converts a 2.0
                            package; everything unless items are named):
                            records replace the user's of the same name,
                            characters join Chars/;
                            .mon files do not exist (both programs export
                            monsters in packages)
  cr package INSTALL_DIR FILE [--new] [--add ITEM]... [--remove ITEM]...
                            create (--new) or edit a package: --add puts
                            in a record or character of the installation
                            (as cr export --list names them), replacing
                            the one of that name; --remove takes an item
                            out (as cr import --list names them); with
                            neither, lists the package
  cr party [INSTALL_DIR]    list the parties and their members (Expansion)
  cr party INSTALL_DIR PARTY CHARACTER...
  cr party INSTALL_DIR --none CHARACTER...
  cr party INSTALL_DIR --delete PARTY
                            put characters in a party (new or existing),
                            take them out of theirs, or delete every member
                            of a party, as Party Management... does; each
                            change saves the character under a new file
                            name; CHARACTER is a name, the start of
                            one, or a file in Chars/
  cr delete INSTALL_DIR CHARACTER...
                            delete characters from Chars/ (no undo)

Core Rules 2.0 installations are read-only: new content (characters,
NPCs, treasure, encounters, records and packages) follows the Expansion,
and 2.0 content is brought over with cr import.

Without INSTALL_DIR, the data package in ~/.local/share/corerules/data
is used, else the installation in the Wine prefix.";

/// `print!` to standard output that ends the program quietly when the
/// reader has gone away (`cr encounter … | head`), where `print!` panics
/// with "failed printing to stdout: Broken pipe".
macro_rules! out {
    ($($arg:tt)*) => {
        write_stdout(format_args!($($arg)*))
    };
}

/// `println!` with [`out!`]'s handling of a closed pipe.
macro_rules! outln {
    () => {
        out!("\n")
    };
    ($($arg:tt)*) => {
        out!("{}\n", format_args!($($arg)*))
    };
}

/// Writes to standard output. A closed pipe exits with success, as a
/// program killed by SIGPIPE would quietly stop; any other write error
/// exits with failure.
fn write_stdout(args: std::fmt::Arguments) {
    if let Err(e) = std::io::stdout().lock().write_fmt(args) {
        stdout_failed(e);
    }
}

fn stdout_failed(e: std::io::Error) -> ! {
    if e.kind() == ErrorKind::BrokenPipe {
        std::process::exit(0);
    }
    eprintln!("cr: {e}");
    std::process::exit(1);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    if command == "extract-data" {
        return match cmd_extract_data(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("cr: {e}");
                ExitCode::FAILURE
            }
        };
    }
    // INSTALL_DIR may be left out whenever the next argument is not a
    // folder: the data package or the Wine installation is used.
    let (install, rest) = match args.get(1).filter(|d| std::path::Path::new(d).is_dir()) {
        Some(dir) => (Installation::open(dir), &args[2..]),
        None => (
            Installation::discover().ok_or_else(|| {
                cr_store::Error::NotAnInstallation(cr_store::DEFAULT_WINE_PATH.into())
            }),
            &args[1..],
        ),
    };
    let result = install.and_then(|install| {
        // Core Rules 2.0 content is only read and imported into the
        // Expansion; nothing is made or changed in a 2.0 installation.
        let writes = match command.as_str() {
            "edit" | "userdb" | "import" | "package" | "party" | "delete" => {
                !(command == "userdb" && rest.get(1).is_some_and(|a| a == "list"))
            }
            // Generators make new content, which follows the Expansion;
            // listing what is saved only reads.
            "npc" => true,
            "treasure" => !rest.iter().any(|a| a == "--saved"),
            "encounter" => rest
                .iter()
                .any(|a| matches!(a.as_str(), "--template" | "--roll")),
            "export" => !rest.iter().any(|a| a == "--list"),
            _ => false,
        };
        if writes && install.is_core20() {
            return Err(cr_store::Error::Invalid(
                "Core Rules 2.0 installations are read-only: import their content into an \
                 Expansion installation with cr import EXPANSION_DIR CORE_RULES_2.0_DIR"
                    .into(),
            ));
        }
        Ok(install)
    });
    let result = result.and_then(|install| match command.as_str() {
        "survey" => cmd_survey(&install),
        "chars" => cmd_chars(&install),
        "edit" => cmd_edit(&install, rest),
        "sheet" => cmd_sheet(&install, rest),
        "treasure" => cmd_treasure(&install, rest),
        "encounter" => cmd_encounter(&install, rest),
        "npc" => cmd_npc(&install, rest),
        "books" => cmd_books(&install),
        "book" => cmd_book(&install, rest),
        "table" => cmd_table(&install, rest),
        "userdb" => userdb::cmd(&install, rest),
        "export" => transfer::cmd_export(&install, rest),
        "import" => transfer::cmd_import(&install, rest),
        "package" => transfer::cmd_package(&install, rest),
        "party" => party::cmd_party(&install, rest),
        "delete" => party::cmd_delete(&install, rest),
        _ => {
            eprintln!("{USAGE}");
            Ok(())
        }
    });
    // What is still buffered goes out before the exit status is decided.
    if let Err(e) = std::io::stdout().flush() {
        stdout_failed(e);
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cr: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_extract_data(args: &[String]) -> cr_store::Result<()> {
    use cr_store::extract::Extraction;
    let (source, dest) = match args {
        [source] => (source, None),
        [source, dest] => (source, Some(std::path::PathBuf::from(dest))),
        _ => {
            eprintln!("{USAGE}");
            return Ok(());
        }
    };
    let extraction = Extraction::probe(std::path::Path::new(source))?;
    let dest = match dest {
        Some(dest) => {
            extraction.write(&dest)?;
            dest
        }
        None => {
            let installed = extraction.install()?;
            if let Some(moved) = &installed.moved {
                outln!("Moved the core20 data to {}", moved.display());
            }
            installed.path
        }
    };
    outln!(
        "Extracted the {} data into {}",
        extraction.edition().id(),
        dest.display()
    );
    Ok(())
}

fn cmd_survey(install: &Installation) -> cr_store::Result<()> {
    for folder in ["Database", "UserDbas"] {
        outln!("{folder}/");
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
                    outln!("  {name:<14} {:>6} records  {class}", s.records);
                }
                Err(e) => outln!("  {name:<14} not a record list: {e}"),
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
        // The lines after "Race: ...": the Expansion stores system,
        // player, party and race before them, 2.0 only system and race.
        let start = entry
            .details
            .iter()
            .position(|d| d.starts_with("Race:"))
            .map_or(4, |i| i + 1);
        outln!(
            "{:<24} {:<10} {}  [{}, {}]",
            entry.name,
            entry.race,
            entry.details.get(start..).unwrap_or_default().join(", "),
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
    let bytes = sheet.export(html, &date);
    if let Err(e) = std::io::stdout().lock().write_all(&bytes) {
        stdout_failed(e);
    }
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
        out!("{}", bundle.to_text());
        outln!();
        for line in rules.summary(bundle).lines() {
            outln!("{line}");
        }
    };

    if let Some(name) = saved {
        let list = install.saved_treasures()?;
        let Some(name) = name else {
            let mut names: Vec<&str> = list.iter().map(|t| t.name.as_str()).collect();
            names.sort_unstable();
            for name in names {
                outln!("{name}");
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
            out!("{}", Bundle { items: vec![item] }.to_text());
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
            outln!();
        }
        print(&bundle, &rules);
    }
    Ok(())
}

/// What `cr encounter` was asked for.
enum EncounterCommand {
    Templates(Option<String>),
    TemplateText(String),
    Generate(String),
    Saved(Option<String>),
    Tables(Option<String>),
    Roll(String),
}

fn cmd_encounter(install: &Installation, args: &[String]) -> cr_store::Result<()> {
    use cr_store::encounter::{Rand, roll_and_pick};
    let invalid = |message: String| cr_store::Error::Invalid(message);
    let mut seed = None;
    let mut save = None;
    let mut command = None;
    let mut args = args.iter().peekable();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .cloned()
                .ok_or_else(|| invalid(format!("{name} needs a value")))
        };
        let next = match arg.as_str() {
            "--seed" => {
                let text = value("--seed")?;
                seed = Some(
                    text.parse::<u32>()
                        .map_err(|_| invalid(format!("{text} is not a number")))?,
                );
                continue;
            }
            "--save" => {
                save = Some(value("--save")?);
                continue;
            }
            "--template" => EncounterCommand::Generate(value("--template")?),
            "--template-text" => EncounterCommand::TemplateText(value("--template-text")?),
            "--roll" => EncounterCommand::Roll(value("--roll")?),
            "--templates" | "--saved" | "--tables" => {
                let name = args.next_if(|a| !a.starts_with("--")).cloned();
                match arg.as_str() {
                    "--templates" => EncounterCommand::Templates(name),
                    "--saved" => EncounterCommand::Saved(name),
                    _ => EncounterCommand::Tables(name),
                }
            }
            _ => return Err(invalid(format!("unexpected argument {arg}"))),
        };
        command = Some(next);
    }
    let command = command.ok_or_else(|| {
        invalid("encounter needs --templates, --template, --saved, --tables or --roll".into())
    })?;

    let print = |rules: &cr_store::encounter::EncounterRules,
                 encounter: &cr_store::encounter::Encounter| {
        out!("{}", rules.encounter_text(encounter));
        let t = rules.totals(encounter);
        use cr_store::treasure::thousands;
        outln!();
        outln!("Gold Value: {}", thousands(t.gold_value));
        outln!("Monster XP: {}", thousands(t.monster_xp));
        outln!("Treasure XP: {}", thousands(t.treasure_xp));
    };
    let generate = |rules: &cr_store::encounter::EncounterRules,
                    template: &str,
                    rand: &mut Rand|
     -> cr_store::Result<cr_store::encounter::Encounter> {
        let generated = rules.generate(template, rand)?;
        for message in &generated.messages {
            eprintln!("{message}");
        }
        let mut encounter = generated.encounter;
        if let Some(name) = &save {
            encounter.name = name.clone();
        }
        print(rules, &encounter);
        if !generated.pending_npcs.is_empty() {
            outln!();
            outln!("NPCs the NPC generator could not make:");
            for npc in &generated.pending_npcs {
                let classes: Vec<String> = npc
                    .npc
                    .classes
                    .iter()
                    .map(|(c, l)| format!("{c} {l}"))
                    .collect();
                outln!(
                    "{} {} {} {}",
                    npc.count,
                    npc.npc.name,
                    npc.npc.race,
                    classes.join("/")
                );
            }
        }
        Ok(encounter)
    };
    let seeded = || {
        let seed = seed.unwrap_or_else(Rand::clock_seed);
        eprintln!("seed {seed}");
        Rand::seeded(seed)
    };

    match command {
        EncounterCommand::Templates(text) => {
            let rules = install.encounter_rules()?;
            let text = text.map(|t| t.to_lowercase());
            for t in rules.templates() {
                let matches = text.as_ref().is_none_or(|text| {
                    t.name.to_lowercase().contains(text)
                        || t.monster_type.to_lowercase().contains(text)
                });
                if matches {
                    let user = if t.user { "  (user)" } else { "" };
                    outln!("{:<40} {}{user}", t.name, t.monster_type);
                }
            }
        }
        EncounterCommand::TemplateText(name) => {
            let rules = install.encounter_rules()?;
            let t = rules
                .template(&name)
                .ok_or_else(|| invalid(format!("no encounter template named {name}")))?;
            out!("{}", rules.template_text(t));
        }
        EncounterCommand::Generate(template) => {
            let rules = install.encounter_rules()?;
            let encounter = generate(&rules, &template, &mut seeded())?;
            if save.is_some() {
                let mut list = install.saved_encounters()?;
                cr_store::encounter::store_encounter(&mut list, encounter, None)
                    .map_err(|e| invalid(e.to_string()))?;
                install.write_saved_encounters(&list)?;
            }
        }
        EncounterCommand::Saved(name) => {
            let list = install.saved_encounters()?;
            let Some(name) = name else {
                for e in &list {
                    outln!("{:<40} {}", e.name, e.template);
                }
                return Ok(());
            };
            let encounter = list
                .iter()
                .find(|e| e.name == name)
                .ok_or_else(|| invalid(format!("no saved encounter named {name}")))?;
            print(&install.encounter_rules()?, encounter);
        }
        EncounterCommand::Tables(name) => {
            let tables = install.frequency_tables()?;
            let Some(name) = name else {
                for t in &tables.tables {
                    outln!("{}", t.name);
                }
                return Ok(());
            };
            let table = tables
                .table(&name)
                .ok_or_else(|| invalid(format!("no 2-20 table named {name}")))?;
            for (i, slot) in table.slots.iter().enumerate() {
                outln!("{:>2}  {slot}", i + 2);
            }
            let rules = install.encounter_rules()?;
            let candidates = rules.candidates(&table.filter);
            outln!();
            for (label, bucket) in ["Common", "Uncommon", "Rare", "Very rare"]
                .iter()
                .zip(&candidates)
            {
                outln!("{label}: {} templates", bucket.len());
            }
        }
        EncounterCommand::Roll(name) => {
            let tables = install.frequency_tables()?;
            let table = tables
                .table(&name)
                .ok_or_else(|| invalid(format!("no 2-20 table named {name}")))?;
            let rules = install.encounter_rules()?;
            let mut rand = seeded();
            let i = roll_and_pick(&rules, table, &mut rand)
                .ok_or_else(|| invalid("Unable to find a Template in the selected table".into()))?;
            outln!("Roll {}: {}", i + 2, table.slots[i]);
            outln!();
            generate(&rules, &table.slots[i], &mut rand)?;
        }
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

fn cmd_npc(install: &Installation, args: &[String]) -> cr_store::Result<()> {
    use cr_store::npc::{GroupOptions, Npc, NpcOptions, Rand};
    let invalid = |message: String| cr_store::Error::Invalid(message);
    let mut seed = None;
    let mut race = None;
    let mut classes: Vec<(String, i32)> = Vec::new();
    let mut level = None;
    let mut count = 1i32;
    let mut po = false;
    let mut save = false;
    let mut party = String::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .cloned()
                .ok_or_else(|| invalid(format!("{name} needs a value")))
        };
        let number = |text: String| {
            text.parse::<i32>()
                .map_err(|_| invalid(format!("{text} is not a number")))
        };
        match arg.as_str() {
            "--seed" => {
                let text = value("--seed")?;
                seed = Some(
                    text.parse::<u32>()
                        .map_err(|_| invalid(format!("{text} is not a number")))?,
                );
            }
            "--race" => race = Some(value("--race")?),
            "--class" => classes.push((value("--class")?, 1)),
            "--level" => {
                let n = number(value("--level")?)?;
                match classes.last_mut() {
                    Some(c) => c.1 = n,
                    None => level = Some(n),
                }
            }
            "--count" => count = number(value("--count")?)?,
            "--po" => po = true,
            "--save" => save = true,
            "--party" => party = value("--party")?,
            _ => return Err(invalid(format!("unexpected argument {arg}"))),
        }
    }
    let rules = install.npc_rules()?;
    let seed = seed.unwrap_or_else(Rand::clock_seed);
    eprintln!("seed {seed}");
    let mut rand = Rand::seeded(seed);
    let npcs: Vec<Npc> = if classes.is_empty() {
        let l = level.unwrap_or(1);
        rules.generate_group(
            &mut rand,
            &GroupOptions {
                number: (count, count),
                race,
                levels: [(l, l); 5],
                players_option: po,
                ..GroupOptions::default()
            },
        )?
    } else {
        let race = race.unwrap_or_else(|| "Human".into());
        let options = NpcOptions {
            players_option: po,
            abilities: None,
        };
        (0..count.max(1))
            .map(|_| rules.generate(&mut rand, &race, &classes, &options))
            .collect::<cr_store::Result<_>>()?
    };
    for (i, npc) in npcs.iter().enumerate() {
        if i > 0 {
            outln!();
        }
        print_npc(npc);
        for m in &npc.messages {
            eprintln!("{}: {m}", npc.character.data.name);
        }
        if save {
            let file = install.save_npc(npc, &party)?;
            outln!("Saved as Chars/{file}");
        }
    }
    Ok(())
}

fn print_npc(npc: &cr_store::npc::Npc) {
    let d = &npc.character.data;
    let levels: Vec<String> = npc.classes.iter().map(|(_, l)| l.to_string()).collect();
    outln!("{}", d.name);
    outln!(
        "{} ({}), {} {}, {}",
        d.race,
        d.subrace,
        d.class_title,
        levels.join("/"),
        if d.unknown_0x163c == 0 {
            "female"
        } else {
            "male"
        }
    );
    outln!("Kit: {}  Alignment: {}", d.kit, d.alignment);
    outln!(
        "Social class: {}  Background: {}",
        d.social_class,
        d.unknown_0x90
    );
    let strength = if d.exceptional_strength > 0 {
        format!("{}/{:02}", d.strength[0], d.exceptional_strength % 100)
    } else {
        d.strength[0].to_string()
    };
    let a = &d.abilities;
    outln!(
        "Str {strength} Dex {} Con {} Int {} Wis {} Cha {}",
        a[0][0],
        a[1][0],
        a[2][0],
        a[3][0],
        a[4][0]
    );
    outln!(
        "Hit points: {}  Height {} in, weight {} lb, age {}",
        npc.hit_points,
        d.height,
        d.weight,
        d.age
    );
    let list = |title: &str, items: &[String]| {
        if !items.is_empty() {
            outln!("{title}: {}", items.join(", "));
        }
    };
    let specialized: Vec<String> = d
        .weapon_proficiencies
        .iter()
        .map(|w| {
            if d.unknown_0xac[3].contains(w) {
                format!("{w} (specialized)")
            } else {
                w.clone()
            }
        })
        .collect();
    list("Weapons", &specialized);
    list("Nonweapon proficiencies", &d.nonweapon_proficiencies);
    list("Languages", &d.languages);
    list("Wizard spells", &d.spells);
    list("Priest spells", &d.priest_spells);
    let memorized: Vec<String> = d
        .unknown_0x3c8
        .iter()
        .flat_map(|c| c.iter())
        .flat_map(|(w, p)| w.iter().chain(p))
        .cloned()
        .collect();
    list("Memorized", &memorized);
    let powers: Vec<String> = d.unknown_0x196c[1]
        .iter()
        .chain(&d.unknown_0x196c[2])
        .cloned()
        .collect();
    list("Psionic disciplines", &d.unknown_0x196c[0]);
    list("Psionic powers", &powers);
    let items: Vec<String> = d
        .inventory
        .iter()
        .flat_map(|b| &b.items)
        .map(|i| {
            if i.quantity > 1 {
                format!("{} x{}", i.name, i.quantity)
            } else {
                i.name.clone()
            }
        })
        .collect();
    list("Equipment", &items);
}

fn cmd_books(install: &Installation) -> cr_store::Result<()> {
    for book in install.books() {
        outln!(
            "{:<17} {:<10} {:>8}  {}",
            book.info.category.title(),
            book.info.short_name(),
            book.size,
            book.info.title
        );
    }
    Ok(())
}

fn cmd_book(install: &Installation, args: &[String]) -> cr_store::Result<()> {
    let invalid = |message: String| cr_store::Error::Invalid(message);
    let mut name = None;
    let mut text = false;
    for arg in args {
        match arg.as_str() {
            "--outline" => text = false,
            "--text" => text = true,
            _ if name.is_none() => name = Some(arg.as_str()),
            _ => return Err(invalid(format!("unexpected argument {arg}"))),
        }
    }
    let name = name.ok_or_else(|| invalid("book needs a book name".into()))?;
    let book = install
        .book(name)
        .ok_or_else(|| invalid(format!("no book {name} in Books/")))?;
    let doc = install.read_book(&book)?;
    if text {
        for p in &doc.paragraphs {
            outln!("{}", p.text());
            for picture in p.pictures() {
                outln!(
                    "[picture: {:?}, {} bytes, {}x{} twips]",
                    picture.format,
                    picture.data.len(),
                    picture.width,
                    picture.height
                );
            }
        }
        return Ok(());
    }
    for h in cr_store::books::outline(&doc, book.info.heading_colors) {
        outln!("{}{}", "  ".repeat(h.level as usize), h.title);
    }
    Ok(())
}

fn cmd_table(install: &Installation, args: &[String]) -> cr_store::Result<()> {
    use cr_store::tables::{LIST, Leftovers, Session, TableBook};
    use cr_store::treasure::Rand;

    let invalid = |message: String| cr_store::Error::Invalid(message);
    let key = |book: TableBook| match book {
        TableBook::HighLevel => "dmo",
        TableBook::Dmg => "dmg",
        TableBook::Phb => "phb",
        TableBook::CombatTactics => "poct",
        TableBook::SkillsPowers => "posp",
        TableBook::SpellsMagic => "posm",
    };
    // The unnumbered entries are the critical hit dialogs, one a book.
    let entry_key = |e: &cr_store::tables::ListEntry| match e.number {
        Some(n) => format!("{}:{n}", key(e.book)),
        None => format!("{}:critical", key(e.book)),
    };
    let rolls = install.table_rolls()?;
    let mut id = None;
    let mut seed = None;
    let mut params = Vec::new();
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
        match arg.as_str() {
            "--seed" => seed = Some(number(value("--seed")?)?),
            "--param" => params.push(number(value("--param")?)? as usize),
            _ if id.is_none() => id = Some(arg.clone()),
            _ => return Err(invalid(format!("unexpected argument {arg}"))),
        }
    }
    let Some(id) = id else {
        for book in rolls.books() {
            outln!("{}", book.title());
            for e in rolls.entries(book) {
                outln!("  {:<14} {}", entry_key(e), rolls.entry_title(e));
                for (i, p) in rolls.params(e, &[]).iter().enumerate() {
                    outln!("      --param {i}: {} ({})", p.label, p.options.join(", "));
                }
            }
        }
        return Ok(());
    };
    let seed = seed.unwrap_or_else(Rand::clock_seed);
    eprintln!("seed {seed}");
    let mut rand = Rand::seeded(seed);
    let mut session = Session::default();
    let result = if let Ok(table) = id.parse::<i32>() {
        if rolls.table(table).is_none() {
            return Err(invalid(format!("no table {table} in Tables/")));
        }
        rolls.roll_id(table, &mut session, &mut rand)
    } else {
        let wanted = id.to_lowercase();
        let entry = LIST
            .iter()
            .find(|e| entry_key(e) == wanted)
            .ok_or_else(|| invalid(format!("no table {id}; cr table INSTALL_DIR lists them")))?;
        // The first Roll Results dialog after the program starts.
        session = Leftovers::default().open(entry);
        rolls.roll(entry, &params, &mut session, &mut rand)
    };
    out!("{}", result.to_text());
    Ok(())
}
