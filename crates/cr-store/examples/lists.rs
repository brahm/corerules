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
        for i in &d.inventory[2].items {
            println!(
                "  carried {} x{} 8={} contents={}",
                i.name,
                i.quantity,
                i.unknown_0x8,
                i.contents.is_some()
            );
        }
    }
}
