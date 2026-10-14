//! The books and the Table Rolls of a real installation (`CORERULES_DIR`);
//! without it the tests pass without checking.

use cr_store::Installation;
use cr_store::books::outline;
use cr_store::tables::{Session, id_map};
use cr_store::treasure::Rand;

fn install() -> Option<Installation> {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        eprintln!("CORERULES_DIR not set; skipping");
        return None;
    };
    Some(Installation::open(dir).unwrap())
}

#[test]
fn every_book_parses_with_an_outline() {
    let Some(install) = install() else { return };
    let books = install.books();
    assert!(!books.is_empty());
    for book in books {
        let doc = install.read_book(&book).unwrap();
        assert!(!doc.paragraphs.is_empty(), "{}", book.info.title);
        let headings = outline(&doc, book.info.heading_colors);
        assert!(!headings.is_empty(), "{} has no outline", book.info.title);
        assert!(
            headings.iter().all(|h| h.paragraph < doc.paragraphs.len()),
            "{}",
            book.info.title
        );
    }
}

#[test]
fn every_mapped_table_loads() {
    let Some(install) = install() else { return };
    let rolls = install.table_rolls().unwrap();
    // Mapped but absent from both installs: the original rolls show
    // nothing for them (most are worked out in code instead).
    const ABSENT: [i32; 17] = [
        10043, 20084, 21999, 22070, 22999, 23999, 30025, 30031, 40014, 40015, 40044, 40048, 40049,
        50018, 50029, 90033, 90040,
    ];
    assert_eq!(rolls.missing(), ABSENT);
    for (id, file, count) in id_map(rolls.flavor()) {
        let Some(table) = rolls.table(id) else {
            continue;
        };
        assert_eq!(table.entries.len() + 1, count, "{id} {file}");
    }
}

#[test]
fn every_list_entry_rolls() {
    let Some(install) = install() else { return };
    let rolls = install.table_rolls().unwrap();
    for seed in [1, 2, 3, 857, 12345] {
        let mut rand = Rand::seeded(seed);
        let mut session = Session::default();
        for book in rolls.books() {
            for entry in rolls.entries(book) {
                // Every option of the first parameter, the first of the
                // others.
                let first = rolls.params(entry, &[]);
                let n = first.first().map_or(1, |p| p.options.len().max(1));
                for option in 0..n {
                    let chosen: Vec<usize> = (0..first.len())
                        .map(|i| if i == 0 { option } else { 0 })
                        .collect();
                    let result = rolls.roll(entry, &chosen, &mut session, &mut rand);
                    assert!(!result.blocks.is_empty(), "{}", rolls.entry_title(entry));
                    assert!(!result.to_text().is_empty());
                }
            }
        }
        for (id, _, _) in id_map(rolls.flavor()) {
            let result = rolls.roll_id(id, &mut session, &mut rand);
            assert!(!result.to_text().is_empty(), "{id}");
        }
    }
}
