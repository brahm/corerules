//! The installation's WinHelp files, with `CORERULES_DIR` (an Expansion
//! installation): every one reads, and every topic its contents file
//! lists is found.

use std::path::Path;

use cr_formats::winhelp::{Item, Link, Macro};
use cr_store::Installation;

fn install() -> Option<Installation> {
    let install = Installation::open(Path::new(&std::env::var_os("CORERULES_DIR")?)).unwrap();
    install.path("Help").map(|_| install)
}

fn help_names(install: &Installation) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(install.path("Help").unwrap())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.to_ascii_lowercase().ends_with(".hlp"))
        .collect();
    names.sort();
    names
}

#[test]
fn every_help_file_reads_with_its_topics_and_pictures() {
    let Some(install) = install() else { return };
    for name in help_names(&install) {
        let help = install
            .help_file(&name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!help.topics.is_empty(), "{name}");
        assert!(help.contents < help.topics.len(), "{name}");
        for topic in &help.topics {
            for p in &topic.paragraphs {
                for item in &p.items {
                    if let Item::Picture(n) = item {
                        assert!(help.picture(*n).is_some(), "{name}: bm{n}");
                    }
                }
            }
        }
    }
}

#[test]
fn every_contents_entry_opens_a_topic() {
    let Some(install) = install() else { return };
    let mut files = std::collections::HashMap::new();
    for name in help_names(&install) {
        let Some(contents) = install.help_contents(&name) else {
            continue;
        };
        let help = install.help_file(&name).unwrap();
        for entry in &contents.entries {
            let Some(context) = &entry.context else {
                continue;
            };
            let other = entry
                .file
                .as_ref()
                .filter(|f| !f.eq_ignore_ascii_case(&name));
            let found = match other {
                None => help.topic_of_context(context),
                Some(file) => files
                    .entry(file.to_ascii_lowercase())
                    .or_insert_with(|| install.help_file(file).ok())
                    .as_ref()
                    .and_then(|h| h.topic_of_context(context)),
            };
            assert!(found.is_some(), "{name}: {context}");
        }
    }
}

#[test]
fn the_online_manual_reads_as_the_original_shows_it() {
    let Some(install) = install() else { return };
    let manual = install.help_file("Manual.hlp").unwrap();
    assert_eq!(manual.title, "Online Manual");
    let topic = &manual.topics[manual.topic_of_context("Online_Manual").unwrap()];
    assert_eq!(topic.title, "Online Manual");
    assert_eq!(
        manual.contents,
        manual.topic_of_context("Online_Manual").unwrap()
    );
    // The heading is bold and larger than the body.
    let heading = &topic.paragraphs[0];
    assert_eq!(heading.text(), "Online Manual");
    let Item::Text(run) = &heading.items[0] else {
        panic!()
    };
    assert!(run.format.bold);
    // The screens are bitmaps of the original's size.
    let picture = manual.picture(2).unwrap();
    assert_eq!((picture.width, picture.height), (560, 420));
    assert_eq!(&picture.bmp[..2], b"BM");
}

#[test]
fn links_and_jump_macros_to_other_files_open_a_topic_there() {
    let Some(install) = install() else { return };
    let mut files = std::collections::HashMap::new();
    let mut broken = Vec::new();
    for name in help_names(&install) {
        let help = install.help_file(&name).unwrap();
        for topic in &help.topics {
            for p in &topic.paragraphs {
                for item in &p.items {
                    let Item::Text(run) = item else { continue };
                    let found = match &run.link {
                        Some(Link::Topic {
                            hash,
                            file: Some(file),
                            ..
                        }) => files
                            .entry(file.to_ascii_lowercase())
                            .or_insert_with(|| install.help_file(file).ok())
                            .as_ref()
                            .is_some_and(|h| h.topic_of_hash(*hash).is_some()),
                        Some(Link::Macro(text)) => match Macro::parse(text) {
                            Some(Macro::Jump { file, context, .. }) => {
                                let file = file.unwrap_or_else(|| name.clone());
                                files
                                    .entry(file.to_ascii_lowercase())
                                    .or_insert_with(|| install.help_file(&file).ok())
                                    .as_ref()
                                    .is_some_and(|h| h.topic_of_context(&context).is_some())
                            }
                            _ => true,
                        },
                        _ => true,
                    };
                    if !found {
                        broken.push(format!("{name}: {:?} {:?}", run.text, run.link));
                    }
                }
            }
        }
    }
    // Five links name topics their files do not have, so WinHelp could not
    // open them either; the viewer says the topic is missing.
    assert!(broken.len() <= 5, "{}", broken.join("\n"));
}

#[test]
fn the_about_box_is_read_from_the_program() {
    let Some(install) = install() else { return };
    let about = install.original_about();
    assert_eq!(about.lines.len(), 5);
    assert_eq!(about.lines[2], "Version 3.10");
    assert!(about.license.is_some());
    // Fourteen roles, the lines that go on joined to theirs.
    assert_eq!(about.credits.len(), 14);
    assert!(
        about
            .credits
            .iter()
            .all(|c| !c.starts_with(' ') && c != "OK")
    );
}

#[test]
fn every_screen_s_topic_is_in_the_program_help() {
    let Some(install) = install() else { return };
    let program = install.help_file("Program.hlp").unwrap();
    for n in cr_store::help::topic::ALL
        .into_iter()
        .chain(cr_store::help::topic::ROLL_METHODS)
    {
        assert!(program.topic_of_number(n).is_some(), "{n:#x}");
    }
    for section in install.catalog().iter().map(|s| s.id) {
        let n = cr_store::help::topic::of_section(section);
        assert!(n != cr_store::help::topic::MAIN_MENU, "{section}");
    }
}

#[test]
fn the_screens_controls_have_their_popup_help() {
    let Some(install) = install() else { return };
    let program = install.help_file("Program.hlp").unwrap();
    let screens: Vec<u32> = cr_store::help::topic::ALL
        .into_iter()
        .chain(cr_store::help::topic::ROLL_METHODS)
        .filter(|&n| !install.screen_help(n).is_empty())
        .collect();
    // The screens the original draws as pictures with hotspots.
    assert!(screens.len() >= 30, "{}", screens.len());
    let help = install.screen_help(cr_store::help::topic::ROLL_METHOD);
    let topic = help.topic_of("Help...").unwrap();
    assert_eq!(help.topic_of("_help"), Some(topic));
    assert!(program.topic_of_number(topic).is_some());
    assert_eq!(help.topic_of("no such control"), None);
}

#[test]
fn the_shipped_records_open_their_books_help() {
    use cr_store::help::{ItemKind, topic};
    let Some(install) = install() else { return };
    let items = install.item_help();
    let sections = install.catalog();
    let mut files = std::collections::HashMap::new();
    for (id, screen, least) in [
        ("races", topic::RACES, 100),
        ("classes", topic::CLASSES, 100),
        ("weapons", topic::PARTS, 100),
        ("armor", topic::PARTS, 100),
        ("equipment", topic::PARTS, 100),
        ("magic-items", topic::PARTS, 100),
        ("nonweapon", topic::PARTS, 100),
        ("wizard-spells", topic::SPELLS, 100),
        ("priest-spells", topic::SPELLS, 100),
        ("psionics", topic::PSIONIC_POWERS, 100),
        ("monsters", topic::MONSTERS, 98),
        ("monster-types", topic::MONSTERS, 90),
    ] {
        let section = sections.iter().find(|s| s.id == id).unwrap();
        let order = ItemKind::order(screen);
        let found = section
            .entries
            .iter()
            .filter(|e| {
                items.topic_of(&e.title, &order).is_some_and(|t| {
                    files
                        .entry(t.file)
                        .or_insert_with(|| install.help_file(t.file).ok())
                        .as_ref()
                        .is_some_and(|h| h.topic_of_number(t.number).is_some())
                })
            })
            .count();
        assert!(
            found * 100 >= section.entries.len() * least,
            "{id}: {found} of {}",
            section.entries.len()
        );
    }
    // A multi-class: the topic on multi-class characters.
    let classes = &sections.iter().find(|s| s.id == "classes").unwrap().entries;
    let multi = format!("{}/{}", classes[0].title, classes[1].title);
    let topic = items
        .topic_of(&multi, &ItemKind::order(topic::CLASSES))
        .unwrap();
    assert_eq!((topic.file, topic.number), ("Charactr.hlp", 100));
}
