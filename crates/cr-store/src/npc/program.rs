//! Tables compiled into the Expansion's executable (`Tsrp3.exe`) that the
//! NPC generator uses: name syllables, backgrounds, the carrying capacity
//! table, the tight-group ids and the hard-coded weapon-name tests. They
//! are program code, transcribed here (the rule tables of the weapon tests
//! read as straight-line code in the original).

/// A level test inside a weapon-name rule list: `cmp +0x738, K`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// `jg`: the level is above K.
    Above(i32),
    /// `jl`: the level is below K.
    Below(i32),
}

/// One test of 0x4ce3d6 / 0x4d6dc6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameRule {
    /// The name equals the string: the test hits.
    Eq(&'static str),
    /// The name contains the string.
    Find(&'static str),
    /// A race test (the race record's parent name, or the character's
    /// race), then the name contains `find`.
    Race {
        parent: bool,
        equal: bool,
        race: &'static str,
        find: &'static str,
    },
    /// When the level test holds, go on at rule `to` (`None`: leave the
    /// list with the "no hit" result).
    Gate { when: Level, to: Option<usize> },
    /// 0x4d6dc6 goes on with the kit tests.
    Stop,
}

/// 0x4b8b0b, local_194.
pub const HUMAN_MALE_194: &[&str] = &[
    "Aaron",
    "Alan",
    "Albert",
    "Alexander",
    "Alexi",
    "Alfred",
    "Andrew",
    "Arthur",
    "Bart",
    "Bert",
    "Brian",
    "Carl",
    "Cedric",
    "Cerdic",
    "Christopher",
    "Chuck",
    "David",
    "Dextel",
    "Dexter",
    "Dov",
    "Durant",
    "Edwin",
    "Eli",
    "Eric",
    "Erik",
    "Ferris",
    "Franklin",
    "Gary",
    "Gebhart",
    "Geoff",
    "George",
    "Gerry",
    "Gil",
    "Godfrey",
    "Godwin",
    "Greg",
    "Guardian",
    "Gunther",
    "Hank",
    "Harris",
    "Henri",
    "Henry",
    "Isaac",
    "James",
    "Jarrod",
    "Jake",
    "Jeff",
    "Jim",
    "John",
    "Joseph",
    "Karl",
    "Kasim",
    "Keith",
    "Kevin",
    "Kip",
    "Klaus",
    "Kurt",
    "Kyo",
    "Lance",
    "Larry",
    "Lars",
    "Lee",
    "Linus",
    "Luke",
    "Malechi",
    "Malek",
    "Marco",
    "Mark",
    "Matthew",
    "Michael",
    "Mitchell",
    "Motubo",
    "Nicolo",
    "Nigel",
    "Osmund",
    "Peter",
    "Phil",
    "Phillip",
    "Pip",
    "Po",
    "Ricardo",
    "Richard",
    "Ringo",
    "Robert",
    "Saldrin",
    "Scott",
    "Slaine",
    "Sneb",
    "Steve",
    "Sven",
    "Theo",
    "Thomas",
    "Timothy",
    "Triolus",
    "Tyler",
    "Victor",
    "Walter",
    "William",
    "Zodac",
];
/// 0x4b8f34, local_164.
pub const HUMAN_FEMALE_164: &[&str] = &[
    "Amy",
    "Anandi",
    "Angela",
    "Anne",
    "Annette",
    "Ashley",
    "Babette",
    "Beth",
    "Bridget",
    "Cassandra",
    "Cora",
    "Corri",
    "Dana",
    "Dawn",
    "Diana",
    "Dido",
    "Elenor",
    "Elise",
    "Ellen",
    "Erica",
    "Erin",
    "Eve",
    "Gabrielle",
    "Gina",
    "Hanna",
    "Helen",
    "Hildegard",
    "Holly",
    "Jade",
    "Jane",
    "Jean",
    "Jenna",
    "Jennifer",
    "Jessie",
    "Jo",
    "Johanna",
    "Joy",
    "Kaija",
    "Kara",
    "Karen",
    "Kari",
    "Kat",
    "Katherine",
    "Kay",
    "Kelli",
    "Kelly",
    "Laura",
    "Lauri",
    "Laurin",
    "Leigh",
    "Lido",
    "Linda",
    "Lisa",
    "Lita",
    "Lucretia",
    "Lucy",
    "Lynn",
    "Marguaret",
    "Mary",
    "Maureen",
    "Michelle",
    "Mimosa",
    "Mina",
    "Minmay",
    "Monique",
    "Natasha",
    "Nicole",
    "Olivia",
    "Petra",
    "Rae",
    "Raye",
    "Ralla",
    "Reba",
    "Rebecca",
    "Renee",
    "Rhiannon",
    "Sara",
    "Serena",
    "Sharon",
    "Stella",
    "Stephane",
    "Susan",
    "Svetlana",
    "Tamara",
    "Torun",
    "Twylla",
    "Vivian",
];
/// 0x4b928b, local_94.
pub const ELF_94: &[&str] = &["Th", "L", "Ll", "R", "S", "B", "N", "G"];
/// 0x4b928b, local_c4.
pub const ELF_C4: &[&str] = &["i", "a", "e", "o"];
/// 0x4b928b, local_70.
pub const ELF_70: &[&str] = &["y", "io", "ae", "ia", "ei"];
/// 0x4b928b, local_58.
pub const ELF_58: &[&str] = &["r", "l", "ll", "n", "nn", "s", "th"];
/// 0x4b928b, local_a8.
pub const ELF_A8: &[&str] = &["i", "a", "e", "o"];
/// 0x4b928b, local_38.
pub const ELF_38: &[&str] = &["th", "las", "les", "s", "n", "nn", "r", "l"];
/// 0x4b97dd, local_74.
pub const DWARF_74: &[&str] = &["D", "R", "G", "T", "M"];
/// 0x4b97dd, local_5c.
pub const DWARF_5C: &[&str] = &["u", "o", "i", "e", "a"];
/// 0x4b97dd, local_44.
pub const DWARF_44: &[&str] = &["mm", "m", "n", "k", "g", "dt", "d", "t", "tt", "c", "r"];
/// 0x4b97dd, local_c8.
pub const DWARF_C8: &[&str] = &["ia", "a", "ee"];
/// 0x4b97dd, local_130.
pub const DWARF_130: &[&str] = &[
    "Anvil", "Gem", "Axe", "Hammer", "Shield", "Blade", "Silver", "Stone", "Rock", "Gold", "Iron",
    "Flint", "Granite",
];
/// 0x4b97dd, local_b8.
pub const DWARF_B8: &[&str] = &[
    "splitter", "bender", "worker", "breaker", "bearer", "smith", "forge", "master", "binder",
    "delver",
];
/// 0x4b97dd, local_8c.
pub const DWARF_8C: &[&str] = &["Elf", "Goblin", "Orc", "Troll", "Giant"];
/// 0x4b97dd, local_f8.
pub const DWARF_F8: &[&str] = &[
    "breaker",
    "cleaver",
    "splitter",
    "slayer",
    "destroyer",
    "crusher",
    "bane",
    "foe",
    "hunter",
    "fist",
    "reaver",
];
/// 0x4ba82f, local_8c.
pub const HALFLING_8C: &[&str] = &[
    "Fuzzy Feet",
    "Pip",
    "Sam",
    "Bill",
    "Fred",
    "Homer",
    "Barton",
];
/// 0x4ba82f, local_a8.
pub const HALFLING_A8: &[&str] = &["Mary", "Freda", "Samantha", "Molly", "Mirth", "Kris"];
/// 0x4ba82f, local_68.
pub const HALFLING_68: &[&str] = &["Jolly", "Merry", "Bright", "Warm", "Good", "Fair"];
/// 0x4ba82f, local_4c.
pub const HALFLING_4C: &[&str] = &["mead", "meadow", "pipe", "field", "home", "weather"];
/// 0x4ba82f, local_30.
pub const HALFLING_30: &[&str] = &["Friend", "Home", "Pipe", "Trouble"];
/// 0x4ba82f, local_1c.
pub const HALFLING_1C: &[&str] = &["maker", "finder"];
/// 0x4ba740, local_70.
pub const OGRE_70: &[&str] = &[
    "Archibald",
    "Bam",
    "Club",
    "Crunch",
    "Crash",
    "Dep",
    "Doog",
    "Doop",
    "Glob",
    "Gor",
    "Gronk",
    "Mag",
    "Ock",
    "Og",
    "Oot",
    "Ped",
    "Smash",
    "Thag",
    "Thak",
    "Thod",
    "Thud",
    "Tiny",
    "Toog",
    "Whack",
    "Wham",
    "Wuk",
];
/// 0x4bc074, local_98.
pub const ORC_98: &[&str] = &["Mug", "Gurk", "Bul", "Dek", "Lig", "Bok", "Juk", "Thak"];
/// 0x4bc074, local_114.
pub const ORC_114: &[&str] = &[
    "grk", "grnk", "wort", "mak", "brk", "dat", "thuk", "glb", "kil", "smsh",
];
/// 0x4bc074, local_44.
pub const ORC_44: &[&str] = &["gleb", "dun", "mok", "brck", "ded", "thok"];
/// 0x4bc074, local_c0.
pub const ORC_C0: &[&str] = &["a", "oo"];
/// 0x4bc074, local_b4.
pub const ORC_B4: &[&str] = &["Elf", "Human", "Dwarf", "Halfling", "Gnome", "Pixie"];
/// 0x4bc074, local_74.
pub const ORC_74: &[&str] = &[
    "breaker",
    "eater",
    "cleaver",
    "splitter",
    "slayer",
    "destroyer",
    "crusher",
    "bane",
    "foe",
    "hunter",
    "reaver",
];
/// 0x4bbd92, local_34.
pub const LIZARD_34: &[&str] = &["T'sk", "Shu", "Sha", "Hou", "Hya", "Shi", "S'ske"];
/// 0x4bbd92, local_44.
pub const LIZARD_44: &[&str] = &["cho", "chee", "'ooka"];
/// 0x4bbd92, local_6c.
pub const LIZARD_6C: &[&str] = &["sshi", "'shu", "mali", "sho", "s'cri", "slumi", "tz'se"];
/// 0x4bbbda, local_34.
pub const THRIKREEN_34: &[&str] = &["Gli", "Ki", "Cli", "Ti", "Li", "Kli", "G'th"];
/// 0x4bbbda, local_50.
pub const THRIKREEN_50: &[&str] = &["ki", "gak", "gri", "gi", "luk", "lok"];
/// 0x4bba12, local_40.
pub const WEMIC_40: &[&str] = &["Ma", "Gu", "Bu", "De", "Li", "Balo", "Jum"];
/// 0x4bba12, local_5c.
pub const WEMIC_5C: &[&str] = &["gumo", "balo", "mali", "bi", "ji", "lum"];
/// 0x4bba12, local_20.
pub const WEMIC_20: &[&str] = &["o", "a"];
/// 0x4bb80a, local_44.
pub const BULLYWUG_44: &[&str] = &["Pa", "Bli", "Blu", "Mo", "Po", "Bulo", "Bla"];
/// 0x4bb80a, local_64.
pub const BULLYWUG_64: &[&str] = &["bla", "bo", "pup", "mo", "ba", "bu", "pa"];
/// 0x4bb80a, local_24.
pub const BULLYWUG_24: &[&str] = &["o", "a", "oo"];
/// 0x4bae7e, local_2c.
pub const AARAKOCRA_2C: &[&str] = &["K", "B", "A", "K", "R", "A"];
/// 0x4bae7e, local_90.
pub const AARAKOCRA_90: &[&str] = &["o", "a", "e", "o", "a"];
/// 0x4bae7e, local_78.
pub const AARAKOCRA_78: &[&str] = &["k", "t", "a", "h", "r", "r", "c", "k"];
/// 0x4bae7e, local_54.
pub const AARAKOCRA_54: &[&str] = &["ok", "rak", "rk", "wrk", "rck", "ook", "k", "lok"];
/// 0x4bae7e, local_b0.
pub const AARAKOCRA_B0: &[&str] = &["o", "a", "i", "o", "a", "oo", "oa"];
/// 0x4bb2d7, local_44.
pub const GIFF_44: &[&str] = &["Om", "Bo", "Mu", "Mo", "Po", "O"];
/// 0x4bb2d7, local_60.
pub const GIFF_60: &[&str] = &["om", "bo", "mu", "mo", "po", "pom"];
/// 0x4bb2d7, local_28.
pub const GIFF_28: &[&str] = &["o", "o", "u", "a"];
/// 0x4bb4b5, local_9c.
pub const SATYR_9C: &[&str] = &[
    "Hy", "Cly", "Aj", "Pha", "Pon", "Omn", "Eur", "Xi", "Per", "Cro", "Anax", "Kons", "Pen",
    "Alex", "Tor",
];
/// 0x4bb4b5, local_4c.
pub const SATYR_4C: &[&str] = &[
    "kyr", "mens", "ax", "phad", "phon", "kan", "ipi", "roki", "oman", "mon", "and", "tor", "or",
];
/// 0x4bb4b5, local_b0.
pub const SATYR_B0: &[&str] = &["os", "os", "as", "os"];
/// 0x4bb4b5, local_5c.
pub const SATYR_5C: &[&str] = &["ia", "ia", "io"];
/// 0x4babc0, local_7c.
pub const GITHZERAI_7C: &[&str] = &["K", "S", "T", "N", "H", "M", "Y", "R", "W"];
/// 0x4babc0, local_54.
pub const GITHZERAI_54: &[&str] = &["i", "a", "e", "o", "u"];
/// 0x4babc0, local_3c.
pub const GITHZERAI_3C: &[&str] = &["k", "s", "t", "n", "h", "m", "y", "r", "w"];

/// 0x81f280 (Gnome, male).
pub const GNOME_MALE: &str = "Norm";
/// 0x81f67c (Bullywug suffix).
pub const BULLYWUG_SUFFIX: &str = "bup";

/// The backgrounds the program loads at 0x54f660 (app +0x1f84), "NONE" first.
pub const BACKGROUNDS: [&str; 21] = [
    "NONE",
    "Accused of a crime",
    "Apprenticeship",
    "Enslaved",
    "Exiled",
    "Failed business venture",
    "Fell in love",
    "Fled a disaster",
    "Found or stole a valuable item",
    "Homeless",
    "Ideological differences",
    "Joined the circus",
    "Kidnapped",
    "Killed someone",
    "Made a powerful enemy",
    "Orphaned",
    "Ran away from home",
    "Reformed",
    "Touched by magic",
    "Went to sea",
    "Witnessed a crime",
];

/// 0x4290f5, column 2: weight allowance by Strength (19..23 are 18/01..18/00).
pub const CAPACITY: [i32; 31] = [
    0, 1, 4, 8, 17, 17, 39, 39, 66, 66, 77, 77, 94, 94, 116, 116, 131, 158, 189, 214, 239, 264,
    314, 414, 564, 614, 714, 864, 1014, 1314, 1614,
];
/// Tight groups whose id (0x677978) is 6.
pub const TIGHT_GROUPS_6: &[&str] = &["Bows"];

/// 0x4ce3d6: the "common weapon" tests on the upper-cased name.
pub const COMMON_WEAPON_RULES: &[NameRule] = &[
    NameRule::Eq("CESTUS"),
    NameRule::Eq("OIL FLASK"),
    NameRule::Eq("ANKUS"),
    NameRule::Find("ARROW"),
    NameRule::Find("MATCHLOCK"),
    NameRule::Find("SNAPLOCK"),
    NameRule::Find("WHEELLOCK"),
    NameRule::Find("PISTOL"),
    NameRule::Find("FLINTLOCK"),
    NameRule::Find("ARQUEBUS"),
    NameRule::Find("CURSED"),
    NameRule::Race {
        parent: true,
        equal: false,
        race: "Elf",
        find: "ELVEN",
    },
    NameRule::Race {
        parent: true,
        equal: true,
        race: "Halfling",
        find: "LONG BOW",
    },
    NameRule::Race {
        parent: true,
        equal: true,
        race: "Dwarf",
        find: "LONG BOW",
    },
    NameRule::Race {
        parent: true,
        equal: true,
        race: "Gnome",
        find: "LONG BOW",
    },
    NameRule::Race {
        parent: true,
        equal: true,
        race: "Kobold",
        find: "LONG BOW",
    },
    NameRule::Race {
        parent: false,
        equal: true,
        race: "Aarakocra",
        find: "LANCE",
    },
    NameRule::Gate {
        when: Level::Above(1),
        to: Some(19),
    },
    NameRule::Find("STAFF-MACE"),
    NameRule::Gate {
        when: Level::Below(4),
        to: None,
    },
    NameRule::Find("ADZE"),
    NameRule::Find("ANKUS"),
    NameRule::Find("BAGH NAKH"),
    NameRule::Find("BELAYING PIN"),
    NameRule::Find("BLOWGUN"),
    NameRule::Find("BO STICK"),
    NameRule::Find("BOLAS"),
    NameRule::Find("BOOMERANG"),
    NameRule::Find("SPRITE"),
    NameRule::Find("BRANDISTOCK"),
    NameRule::Find("CALTROP"),
    NameRule::Find("CATAPULT"),
    NameRule::Find("CESTUS"),
    NameRule::Find("CHAIN"),
    NameRule::Find("CHAKRAM"),
    NameRule::Find("CHIJIKIRI"),
    NameRule::Find("CLUB"),
    NameRule::Find("COMBINED"),
    NameRule::Find("TULWAR"),
    NameRule::Find("THREE-PIECE ROD"),
    NameRule::Find("SWORD-AXE"),
    NameRule::Find("SPATHA"),
    NameRule::Find("RAPIER"),
    NameRule::Find("SABRE"),
    NameRule::Find("SAPARA"),
    NameRule::Find("KOPESH"),
    NameRule::Find("CUTLASS"),
    NameRule::Find("DRUSUS"),
    NameRule::Find("ESTOC"),
    NameRule::Find("FALCHION"),
    NameRule::Find("GLADIUS"),
    NameRule::Find("KATANA"),
    NameRule::Find("BONE"),
    NameRule::Find("GRAIN"),
    NameRule::Find("GRAPPLE"),
    NameRule::Find("GUNSEN"),
    NameRule::Find("GYTHKA"),
    NameRule::Find("GAFF"),
    NameRule::Find("GLOVE"),
    NameRule::Find("HARPOON"),
    NameRule::Find("HATCHET"),
    NameRule::Find("HEAD"),
    NameRule::Find("JITTE"),
    NameRule::Find("KAMA"),
    NameRule::Find("CHO-KU-NO"),
    NameRule::Find("PELLET"),
    NameRule::Find("JAMBIYA"),
    NameRule::Find("KATAR"),
    NameRule::Find("MAIN-GAUCHE"),
    NameRule::Find("PARRYING"),
    NameRule::Find("STILETTO"),
    NameRule::Find("STONE"),
    NameRule::Find("PARRYING"),
    NameRule::Find("FORK"),
    NameRule::Find("HOLY SYMBOL"),
    NameRule::Find("JITTE"),
    NameRule::Find("KAMA"),
    NameRule::Find("KAU SIN KE"),
    NameRule::Find("KNEE"),
    NameRule::Find("KUSARI"),
    NameRule::Find("LASSO"),
    NameRule::Find("MACE-AXE"),
    NameRule::Find("MACHETE"),
    NameRule::Find("MAN CATCHER"),
    NameRule::Find("MACHETE"),
    NameRule::Find("MAUL"),
    NameRule::Find("MORNING STAR"),
    NameRule::Find("NUNCHAKU"),
    NameRule::Find("OIL"),
    NameRule::Find("PARANG"),
    NameRule::Find("PIKE"),
    NameRule::Find("PILUM"),
    NameRule::Find("PRY BAR"),
    NameRule::Find("ROCK"),
    NameRule::Find("SAI"),
    NameRule::Find("SANG KAUW"),
    NameRule::Find("SAP"),
    NameRule::Find("SCOURGE"),
    NameRule::Find("SCYTHE"),
    NameRule::Find("ROCK"),
    NameRule::Find("SHURIKEN"),
    NameRule::Find("SICKLE"),
    NameRule::Find("SLEDGE"),
    NameRule::Find("SPADE"),
    NameRule::Find("CLAYMORE"),
    NameRule::Find("CUTLASS"),
    NameRule::Find("DRUSUS"),
    NameRule::Find("FALCHION"),
    NameRule::Find("GLADIUS"),
    NameRule::Find("ESTOC"),
    NameRule::Find("SCIMITAR"),
    NameRule::Find("KATANA"),
    NameRule::Find("KHOPESH"),
    NameRule::Find("NINJA-TO"),
    NameRule::Find("NO-DACHI"),
    NameRule::Find("RAPIER"),
    NameRule::Find("WAKIZASHI"),
    NameRule::Find("WHIP"),
    NameRule::Find("TETSUBO"),
    NameRule::Find("POLEARM"),
    NameRule::Find("JOUSTING"),
    NameRule::Find("AXE, TWO-HANDED"),
];
/// 0x4d6dc6 up to its call of 0x43fac6: the excluded-weapon tests.
pub const EXCLUDED_WEAPON_RULES: &[NameRule] = &[
    NameRule::Race {
        parent: false,
        equal: true,
        race: "Aarakocra",
        find: "LANCE",
    },
    NameRule::Race {
        parent: true,
        equal: true,
        race: "Halfling",
        find: "LONG BOW",
    },
    NameRule::Race {
        parent: true,
        equal: true,
        race: "Dwarf",
        find: "LONG BOW",
    },
    NameRule::Race {
        parent: true,
        equal: true,
        race: "Gnome",
        find: "LONG BOW",
    },
    NameRule::Race {
        parent: true,
        equal: true,
        race: "Kobold",
        find: "LONG BOW",
    },
    NameRule::Find("ARROW"),
    NameRule::Find("BALLISTA"),
    NameRule::Find("CATAPULT"),
    NameRule::Stop,
];
