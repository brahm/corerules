//! The user-database editors against the originals' own files.
//!
//! With `CORERULES_DIR` pointing at an installation and `CORERULES_GOLDEN`
//! at the `userdb` golden folder (holding `t3/` and `ev/`), each golden
//! user file is rebuilt through the editor API from the values the README
//! lists, starting from the installation's own user files, and compared
//! byte for byte. The installation is never written: the tests work on a
//! temporary copy of `UserDbas` next to a link to its `Database`. The
//! Expansion's goldens (`t3/`) are used when the installation has
//! `AllowedU.dat`, 2.0's (`ev/`) otherwise. Without the variables the
//! tests pass without checking.

use std::fs;
use std::path::PathBuf;

use cr_store::Installation;
use cr_store::userdb::allowed::{Allowed, AllowedDb};
use cr_store::userdb::classes::Classes;
use cr_store::userdb::monsters::{MonsterTypes, Monsters};
use cr_store::userdb::parts::{KITS, Parts, WEAPONS};
use cr_store::userdb::powers::Powers;
use cr_store::userdb::races::Races;
use cr_store::userdb::spells::Spells;
use cr_store::userdb::tables::{TableDb, new_entry, push_entry};
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{FieldDesc, Sources, UserDb, UserKind, Value};

struct Scratch {
    root: PathBuf,
    install: Installation,
    golden: PathBuf,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A scratch installation for `test`, with the golden folder of its
/// program, or `None` when the variables are not set.
fn scratch(test: &str) -> Option<Scratch> {
    let dir = PathBuf::from(std::env::var_os("CORERULES_DIR")?);
    let golden = PathBuf::from(std::env::var_os("CORERULES_GOLDEN")?);
    if !golden.join("t3").is_dir() {
        return None;
    }
    let original = Installation::open(&dir).ok()?;
    let database = original.path("Database")?;
    let user = original.path("UserDbas")?;
    let expansion = original.path("UserDbas/AllowedU.dat").is_some();
    let root = std::env::temp_dir().join(format!("cr-userdb-golden-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("UserDbas")).unwrap();
    std::os::unix::fs::symlink(&database, root.join("Database")).unwrap();
    for entry in fs::read_dir(&user).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), root.join("UserDbas").join(entry.file_name())).unwrap();
        }
    }
    let install = Installation::open(&root).unwrap();
    let golden = golden.join(if expansion { "t3" } else { "ev" });
    Some(Scratch {
        root,
        install,
        golden,
    })
}

impl Scratch {
    fn expansion(&self) -> bool {
        self.golden.ends_with("t3")
    }

    /// Compares the scratch file `relative` with golden `name`.
    fn check(&self, relative: &str, name: &str) {
        let golden = self.golden.join(name);
        let ours = fs::read(self.root.join(relative)).unwrap();
        let theirs = fs::read(&golden).unwrap();
        assert!(
            ours == theirs,
            "{relative} differs from {} ({} vs {} bytes)",
            golden.display(),
            ours.len(),
            theirs.len()
        );
    }
}

/// Sets the field `label` (in group `group`, "" for any) of `record`.
fn set<R>(fields: &[FieldDesc<R>], record: &mut R, group: &str, label: &str, value: Value) {
    let field = fields
        .iter()
        .find(|f| f.label == label && (group.is_empty() || f.group == group))
        .unwrap_or_else(|| panic!("no field {group}/{label}"));
    field.set(record, &value, &Sources::default());
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn add_and_save<K: UserKind>(s: &Scratch, db: &mut UserDb<K>, record: K::Rec) {
    db.add(record).unwrap();
    db.save(&s.install).unwrap();
}

#[test]
fn spells_golden() {
    let Some(s) = scratch("spells") else { return };
    if !s.expansion() {
        return;
    }
    let mut db = UserDb::<Spells>::load(&s.install).unwrap();
    let f = Spells::fields(db.flavor);
    let mut spell = db.new_record(0, "");
    set(&f, &mut spell, "", "Name", text("Udb bolt"));
    set(&f, &mut spell, "", "Range", text("60 yds"));
    set(&f, &mut spell, "", "Duration", text("Instantaneous"));
    set(&f, &mut spell, "", "Area of Effect", text("1 creature"));
    set(&f, &mut spell, "", "Components", text("V, S"));
    set(&f, &mut spell, "", "Casting Time", text("1"));
    set(&f, &mut spell, "", "Saving Throw", text("None"));
    set(&f, &mut spell, "", "Level", Value::Int(2));
    set(
        &f,
        &mut spell,
        "",
        "Schools",
        Value::List(vec!["Alteration".into()]),
    );
    set(
        &f,
        &mut spell,
        "",
        "Description",
        text("Udb spell description."),
    );
    // Created, deleted, created again.
    db.add(spell.clone()).unwrap();
    db.delete(0);
    add_and_save(&s, &mut db, spell);
    s.check("UserDbas/SpellsU.dat", "SpellsU_01_udbbolt.dat");
}

#[test]
fn traps_golden() {
    let Some(s) = scratch("traps") else { return };
    if !s.expansion() {
        return;
    }
    let mut db = UserDb::<Traps>::load(&s.install).unwrap();
    let f = Traps::fields(db.flavor);
    let mut trap = db.new_record(0, "");
    for (label, value) in [
        ("Trap Name", "Udb pit"),
        ("Location", "Corridor"),
        ("Damage", "2d6"),
        ("Detection Modifier", "-10%"),
        ("Number of Characters Trapped", "1d2"),
        ("Saving Throw", "Dex check"),
        ("Special Effects", "Spikes"),
    ] {
        set(&f, &mut trap, "", label, text(value));
    }
    add_and_save(&s, &mut db, trap);
    s.check("UserDbas/TrapsU.dat", "TrapsU_01_udbpit.dat");
}

#[test]
fn powers_golden() {
    let Some(s) = scratch("powers") else { return };
    if !s.expansion() {
        return;
    }
    let mut db = UserDb::<Powers>::load(&s.install).unwrap();
    let f = Powers::fields(db.flavor);
    let mut power = db.new_record(0, "");
    set(&f, &mut power, "", "Name", text("Udb power"));
    set(&f, &mut power, "", "Type", Value::Int(1));
    set(&f, &mut power, "", "PSP Cost", text("5/round"));
    set(&f, &mut power, "", "Range", text("touch"));
    set(&f, &mut power, "", "Area of Effect", text("personal"));
    set(&f, &mut power, "", "Minimum Level", Value::Int(3));
    set(
        &f,
        &mut power,
        "",
        "Minimum PSP Cost to Use Once",
        Value::Int(5),
    );
    set(
        &f,
        &mut power,
        "",
        "Description",
        text("Udb power description."),
    );
    add_and_save(&s, &mut db, power);
    s.check("UserDbas/PsionicU.dat", "PsionicU_01_udbpower.dat");
}

#[test]
fn monsters_golden() {
    let Some(s) = scratch("monsters") else { return };
    if !s.expansion() {
        return;
    }
    let mut types = UserDb::<MonsterTypes>::load(&s.install).unwrap();
    let mut kind = types.new_record(0, "");
    set(
        &MonsterTypes::fields(types.flavor),
        &mut kind,
        "",
        "Name of Type",
        text("Udbtype"),
    );
    add_and_save(&s, &mut types, kind);
    s.check("UserDbas/MonTypeU.dat", "MonTypeU_01_udbtype.dat");

    let mut db = UserDb::<Monsters>::load(&s.install).unwrap();
    let f = Monsters::fields(db.flavor);
    let sources = Sources::load(&s.install, &f).unwrap();
    let mut beast = db.new_record(0, "Udbtype");
    set(&f, &mut beast, "", "Name", text("Udb beast"));
    set(&f, &mut beast, "", "Frequency", Value::Int(3));
    set(&f, &mut beast, "Number Appearing", "Fewest", Value::Int(1));
    set(
        &f,
        &mut beast,
        "Number Appearing",
        "Greatest",
        Value::Int(4),
    );
    set(&f, &mut beast, "", "Hit Dice", Value::Dice([3, 8, 0]));
    set(&f, &mut beast, "", "THAC0", Value::Int(17));
    set(&f, &mut beast, "", "Man-Sized", Value::Bool(true));
    set(&f, &mut beast, "Morale", "Highest", Value::Int(12));
    set(&f, &mut beast, "Morale", "Lowest", Value::Int(11));
    set(&f, &mut beast, "", "Experience Points", Value::Int(65));
    let attacks = f.iter().find(|d| d.label == "Attacks").unwrap();
    attacks.set(&mut beast, &Value::List(vec!["Poison".into()]), &sources);
    set(&f, &mut beast, "", "No. of Attacks", text("1"));
    set(&f, &mut beast, "", "Damage/Attack", text("1d6"));
    add_and_save(&s, &mut db, beast);
    s.check("UserDbas/MonsterU.dat", "MonsterU_01_udbbeast.dat");
}

#[test]
fn treasure_table_golden() {
    let Some(s) = scratch("treasure") else { return };
    if !s.expansion() {
        return;
    }
    let mut db = TableDb::load(&s.install).unwrap();
    let mut table = TableDb::new_table();
    table.name = "Udb table".into();
    push_entry(&mut table, new_entry("Udb sword", false, 3));
    db.add(table).unwrap();
    db.save(&s.install).unwrap();
    s.check("UserDbas/Treasure.dat", "Treasure_01_udbtable.dat");
}

fn physical(f: &[FieldDesc<cr_formats::race::Race>], r: &mut cr_formats::race::Race, v: [i64; 11]) {
    let groups = [
        (
            "Height (inches)",
            ["Male", "Female", "Dice", "Sides"].as_slice(),
        ),
        (
            "Weight (lbs.)",
            ["Male", "Female", "Dice", "Sides"].as_slice(),
        ),
        ("Starting Age (years)", ["Base", "Dice", "Sides"].as_slice()),
    ];
    let mut values = v.iter();
    for (group, labels) in groups {
        for label in labels {
            set(f, r, group, label, Value::Int(*values.next().unwrap()));
        }
    }
}

#[test]
fn races_golden() {
    let Some(s) = scratch("races") else { return };
    let mut db = UserDb::<Races>::load(&s.install).unwrap();
    let f = Races::fields(db.flavor);
    let abilities = [
        "Strength",
        "Dexterity",
        "Constitution",
        "Intelligence",
        "Wisdom",
        "Charisma",
    ];

    let mut allowed = AllowedDb::load(&s.install).unwrap();
    if s.expansion() {
        // (1) A subrace of Dwarf.
        let mut sub = db.new_record(0, "Dwarf");
        set(&f, &mut sub, "", "Name", text("Udb dwarf"));
        physical(&f, &mut sub, [40, 38, 1, 10, 120, 100, 2, 20, 40, 5, 6]);
        set(&f, &mut sub, "", "Number of Hands", Value::Int(2));
        set(
            &f,
            &mut sub,
            "",
            "Description",
            text("Udb dwarf description."),
        );
        set(&f, &mut sub, "", "Penalties", text("Udb penalties."));
        set(&f, &mut sub, "", "Lawful Good", Value::Bool(true));
        set(
            &f,
            &mut sub,
            "",
            "Selected Abilities",
            Value::List(vec!["Dwarf, Axe bonus (5)".into()]),
        );
        set(&f, &mut sub, "", "Character Point Cost", Value::Int(5));
        let movement = f
            .iter()
            .find(|d| d.page == "Movement" && d.label == "Unencumbered Movement")
            .unwrap();
        movement.set(&mut sub, &Value::Int(6), &Sources::default());
        db.add(sub).unwrap();
    }
    // (2) A race.
    let mut race = db.new_record(0, "");
    set(&f, &mut race, "", "Name", text("Udbling"));
    set(&f, &mut race, "", "Starting", Value::Int(10));
    set(&f, &mut race, "", "Retained", Value::Int(5));
    physical(&f, &mut race, [36, 34, 2, 4, 60, 50, 1, 10, 20, 2, 4]);
    set(&f, &mut race, "", "Number of Hands", Value::Int(2));
    for ability in abilities {
        set(&f, &mut race, ability, "Minimum", Value::Int(3));
        set(&f, &mut race, ability, "Maximum", Value::Int(18));
    }
    set(&f, &mut race, "Strength", "Adjustment", Value::Int(1));
    if s.expansion() {
        // The '-' key was refused, so Charisma got +1.
        set(&f, &mut race, "Charisma", "Adjustment", Value::Int(1));
        let movement = f
            .iter()
            .find(|d| d.page == "Special" && d.label == "Unencumbered Movement")
            .unwrap();
        movement.set(&mut race, &Value::Int(12), &Sources::default());
    } else {
        set(
            &f,
            &mut race,
            "",
            "Allowed Classes",
            Value::List(vec!["Fighter".into()]),
        );
        set(&f, &mut race, "", "Fighter", Value::Int(10));
    }
    db.add(race).unwrap();
    db.save(&s.install).unwrap();
    if !s.expansion() {
        s.check("UserDbas/RaceU.dat", "RaceU_01_udbling.dat");
        return;
    }
    s.check("UserDbas/RaceU.dat", "RaceU_01_udbdwarf_udbling.dat");

    // Allowed Classes: Dwarf gets UdbClass (12) and UdbClass/Thief; then
    // opening the dialog for Udbling creates its record and writes the
    // file.
    let class = |w: &str, r: &str| {
        [
            w.into(),
            r.into(),
            String::new(),
            String::new(),
            String::new(),
        ]
    };
    allowed.set_user(
        "Dwarf",
        &[
            Allowed::new(class("UdbClass", ""), 12),
            Allowed::new(class("UdbClass", "Thief"), 0),
        ],
    );
    let (_, created) = allowed.ensure("Udbling");
    assert!(created);
    allowed.save(&s.install).unwrap();
    s.check("UserDbas/AllowedU.dat", "AllowedU_01.dat");
    allowed.set_user("Udbling", &[Allowed::new(class("Fighter", ""), 10)]);
    allowed.save(&s.install).unwrap();
    s.check("UserDbas/AllowedU.dat", "AllowedU_02_udbling_fighter.dat");
}

#[test]
fn parts_golden() {
    let Some(s) = scratch("parts") else { return };
    let mut db = UserDb::<Parts>::load(&s.install).unwrap();
    let f = Parts::fields(db.flavor);
    let phb = "Player's Handbook";
    let mut sword = db.new_record(WEAPONS, "");
    set(&f, &mut sword, "", "Name", text("Udb sword"));
    set(
        &f,
        &mut sword,
        phb,
        "Damage, Small-Medium",
        Value::Dice([1, 8, 0]),
    );
    set(
        &f,
        &mut sword,
        phb,
        "Damage, Large",
        Value::Dice([1, 12, 0]),
    );
    set(&f, &mut sword, phb, "Speed Factor", Value::Int(5));
    set(&f, &mut sword, phb, "Weapon Size", text("M"));
    set(&f, &mut sword, phb, "Weapon Type", text("S"));
    set(&f, &mut sword, "", "Melee Weapon", Value::Bool(true));
    set(&f, &mut sword, "", "# of hands needed", Value::Int(1));
    set(&f, &mut sword, phb, "Gold Pieces", Value::Int(15));
    db.add(sword).unwrap();
    if s.expansion() {
        let mut kit = db.new_record(KITS, "");
        set(&f, &mut kit, "", "Name", text("Udb kit"));
        set(
            &f,
            &mut kit,
            "",
            "Starting money = dice * multiplier + add",
            Value::Bool(true),
        );
        set(&f, &mut kit, "", "# of dice", Value::Int(3));
        set(&f, &mut kit, "", "# of sides", Value::Int(6));
        set(&f, &mut kit, "", "Multiplier", Value::Int(10));
        db.add(kit).unwrap();
    }
    db.save(&s.install).unwrap();
    s.check(
        "UserDbas/PartsU.dat",
        if s.expansion() {
            "PartsU_01_udbsword_udbkit.dat"
        } else {
            "PartsU_01_udbsword.dat"
        },
    );
}

#[test]
fn classes_golden() {
    let Some(s) = scratch("classes") else { return };
    if !s.expansion() {
        return;
    }
    let mut db = UserDb::<Classes>::load(&s.install).unwrap();
    let f = Classes::fields(db.flavor);
    let mut class = db.new_record(0, "");
    set(&f, &mut class, "", "Name", text("UdbClass"));
    set(&f, &mut class, "", "Group", Value::Int(0));
    // Typed "7"; the field then showed "07".
    set(&f, &mut class, "", "Character Points", text("07"));
    set(&f, &mut class, "", "Gold", Value::Int(123));
    set(
        &f,
        &mut class,
        "",
        "Allow unarmed combat (Fighter)",
        Value::Bool(true),
    );
    set(
        &f,
        &mut class,
        "Strength",
        "Prime Requisite",
        Value::Bool(true),
    );
    set(&f, &mut class, "Strength", "Minimum", Value::Int(9));
    set(
        &f,
        &mut class,
        "",
        "Description",
        text("Udb class description."),
    );
    add_and_save(&s, &mut db, class);
    s.check("UserDbas/ClassU.dat", "ClassU_01_udbclass.dat");
}
