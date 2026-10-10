//! Prints a character's lists: `cargo run -p cr-store --example lists -- FILE.dat`.
fn main() {
    for f in std::env::args().skip(1) {
        let c = cr_formats::character::Character::parse(&std::fs::read(&f).unwrap()).unwrap();
        let d = &c.data;
        println!(
            "{f}\n  weapons {:?}\n  nwp {:?}\n  costs {:?} bonuses {:?} details {}\n  languages {:?}\n  spells {:?} / {:?}\n  weight {}",
            d.weapon_proficiencies,
            d.nonweapon_proficiencies,
            d.nonweapon_costs,
            d.nonweapon_bonuses,
            d.nonweapon_details.len(),
            d.languages,
            d.spells,
            d.priest_spells,
            f32::from_bits(d.carried_weight)
        );
        let lists = cr_store::lists::CharacterLists::of(&c);
        for (bundle, items) in lists.equipment.iter().enumerate() {
            for i in items {
                let name = cr_store::lists::BUNDLE_NAMES[bundle];
                println!("  {name}: {} x{} {:?}", i.name, i.quantity, i.contents);
            }
        }
        println!(
            "  coins {:?} cache {:?}",
            lists.coins,
            &d.unknown_0x1778[1..6]
        );
        for book in &lists.spellbooks {
            println!(
                "  {} ({}/{}) {:?}",
                book.name,
                book.pages_used(),
                book.capacity,
                book.spells
            );
        }
        for (slot, levels) in lists.memorized.iter().enumerate() {
            for (level, (wizard, priest)) in levels.iter().enumerate() {
                if !wizard.is_empty() || !priest.is_empty() {
                    println!("  memorized class {slot} level {level}: {wizard:?} {priest:?}");
                }
            }
        }
    }
}
