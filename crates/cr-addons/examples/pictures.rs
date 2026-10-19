//! Lists the pictures the records of a package name, for building the
//! add-ons' pictures: one line per record and picture,
//! `CATEGORY<TAB>NAME<TAB>STORED PATH<TAB>PATH IN THE INSTALLATION`.

use cr_formats::Object;
use cr_formats::export::ExportFile;
use cr_store::pictures::{Kind, joined, relative};

fn main() {
    let path = std::env::args().nth(1).expect("usage: pictures FILE.xpx");
    let file = ExportFile::parse(&std::fs::read(path).unwrap()).unwrap();
    let print = |category: &str, name: &str, stored: &str, kind: Kind| {
        if let Some(rel) = relative(stored, kind) {
            println!("{category}\t{name}\t{stored}\t{rel}");
        }
    };
    for o in &file.monster_types {
        if let Object::MonsterType(t) = o {
            print("Monster Types", &t.name, &t.picture, Kind::Monster);
        }
    }
    for o in &file.races {
        if let Object::Race(r) = o {
            print(
                "Races",
                &r.name,
                &joined(&r.portrait_folder, &r.portrait),
                Kind::Portrait,
            );
            for p in &r.portraits {
                print(
                    "Races",
                    &r.name,
                    &joined(&r.portrait_folder, p),
                    Kind::Portrait,
                );
            }
        }
    }
    for c in &file.characters {
        let [folder, name] = &c.data.unknown_0x1db8;
        print(
            "Characters",
            &c.data.name,
            &joined(folder, name),
            Kind::Portrait,
        );
    }
}
