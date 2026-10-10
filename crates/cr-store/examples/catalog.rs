//! Prints a summary of an installation's catalog: `cargo run -p cr-store --example catalog -- DIR`.
fn main() {
    let dir = std::env::args().nth(1).expect("usage: catalog DIR");
    let inst = cr_store::Installation::open(dir).expect("open");
    for s in inst.catalog() {
        println!("{:<22} {:>5} {:?}", s.title, s.entries.len(), s.error);
        if let Some(e) = s.entries.first() {
            println!(
                "    {} | {} | groups {} | text {}",
                e.title,
                e.subtitle,
                e.groups.len(),
                e.text.chars().take(60).collect::<String>()
            );
        }
    }
}
