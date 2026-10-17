# classes: Class.dat, ClassU.dat, AllowedU.dat

Reader and writer: [`crates/cr-formats/src/classes.rs`](../../crates/cr-formats/src/classes.rs)
(`Class`, `ClassData`, `Core20ClassData`, `KitBonus`, `version_of`) and
[`crates/cr-formats/src/allowed.rs`](../../crates/cr-formats/src/allowed.rs)
(`ClassesAllowed`, `ClassesAllowedHelper`). The Custom Classes editor is
[`crates/cr-store/src/userdb/classes.rs`](../../crates/cr-store/src/userdb/classes.rs)
with its pages in [`classes/pages.rs`](../../crates/cr-store/src/userdb/classes/pages.rs)
and the member map in [`slots.rs`](../../crates/cr-store/src/userdb/slots.rs);
the Allowed Classes dialog is
[`crates/cr-store/src/userdb/allowed.rs`](../../crates/cr-store/src/userdb/allowed.rs).
Encoding: [archive.md](archive.md).

## Files

| file | document class | content | program |
|---|---|---|---|
| `Database/Class.dat` | `CClassDoc` (Serialize 0x68416a) | the shipped classes | both |
| `UserDbas/ClassU.dat` | the same | the user's classes; empty = `00 00` | Expansion only |
| `UserDbas/AllowedU.dat` | `CClassesAllowedDoc` (Serialize 0x680e21) | the classes and multi-class combinations each race allows, shipped and user entries together | Expansion only |

`CClassDoc` writes one collection (document member +0x54): a count, then each
`CClassOb` with its tag. `CClassesAllowedDoc` writes one `CObArray` (member
+0x54) of `CClassesAllowedOb` the same way. There is no `Database/Allowed.dat`:
the shipped combinations are in `UserDbas/AllowedU.dat` itself, marked as
shipped entries.

| file | Expansion | Core Rules 2.0 |
|---|---|---|
| `Class.dat` | 26 classes, all `TSRP3 V39` (515114 bytes) | 21 classes in the 2.0 layout (221721 bytes) |
| `ClassU.dat` | `00 00` | does not exist (2.0 has no custom classes) |
| `AllowedU.dat` | 30 races, 208 entries (6075 bytes) | does not exist (2.0 keeps the classes in the race, see [race.md](race.md)) |

The Expansion writes every class with the version string `TSRP3 V39`. Core
Rules 2.0 (`EVERMORE.EXE`, `CClassOb::Serialize` 0x48c090) writes no version
string and a shorter fixed layout. Both use schema 88 for `CClassOb`.

## Telling the two layouts apart

Both layouts start with a `CString`. The reader looks at it:

* it starts with `TSRP`: an Expansion class. The string is the version
  string; the [Expansion layout](#cclassob-expansion-layout-schema-88-serialize-0x6644af)
  follows, read with `v = version_of(string)`;
* otherwise: a 2.0 class. The string is the class name, and the rest of the
  [2.0 layout](#cclassob-core-rules-20-layout-schema-88-serialize-0x48c090-in-evermoreexe)
  follows.

A 2.0 class whose name starts with `TSRP` cannot be told apart, so the writer
refuses to write one (`Error::Unencodable`).

### version_of

The class version a version string announces, copied from the Expansion's
load code:

1. exactly `TSRP3 V39`: 39, the current version;
2. longer than 7 characters: the decimal number (C `atol`: leading blanks and a
   sign allowed, digits up to the first non-digit, 0 when there are none)
   after the first 7 characters, whatever those are. `TSRP3 V21` is 21;
3. 7 characters or shorter: 38, the version before the current one.

A string whose tail is not a number gives version 0, so every field with a
condition below is absent. The reader writes a class back with the version
string it was read with, in that version's layout; the Expansion itself only
ever writes `TSRP3 V39`.

## CClassOb, Expansion layout (schema 88, Serialize 0x6644af)

The version string, then `ClassData`. "member" is the offset in the object
(several arrays are stored apart from their neighbours and give each
element's offset); "page" is the class editor (IDD 192) page that edits it,
as ported in `classes/pages.rs`.

| # | field | type | present | member | meaning |
|---|---|---|---|---|---|
| 0 | `version` | CString | | | `TSRP3 V39` |
| 1 | `name` | CString | | +0x4 | class name, up to 18 characters, no `/` (the multi-class separator); unique among standard and user classes |
| 2 | `unknown_0x14` | i32[2] | | +0x14, +0x18 | [0] class group: 0 warrior, 1 rogue, 2 priest, 3 wizard, 4 psionicist (main page, Group). [1] unknown and unused by the app: 0 to 7 for Fighter, Paladin, Ranger, Thief, Bard, Cleric, Druid, Mage, 8 for every specialist wizard, -1 for the psionicist |
| 3 | `ability_requirements` | i32[6] | | +0x1c .. +0x30 | minimum Str, Dex, Con, Int, Wis, Cha, 0 to 25 (Ability Scores; Paladin Str 12) |
| 4 | `unknown_0x34` | i32[6] | | +0x34 .. +0x48 | prime requisite flags, 0/1, same order (Ability Scores, Prime Requisite) |
| 5 | `unknown_0x4c` | i32 | | +0x4c | 0 in every shipped class; unknown |
| 6 | `unknown_0x54` | i32[2] | | +0x54, +0x58 | [0] Skills & Powers character points a 1st-level character spends on class abilities (main page, Character Points; Fighter 15); [1] maximum level, 0 to 100 (Levels, Max Level; 30 for most classes); the Levels tables show the rows up to it |
| 7 | `unknown_0x84` | i32[3] | | +0x84, +0x88, +0x8c | hit points (More): hit die (Starting HP: die sides), last level that rolls a hit die (Maximum hit dice rolled), flat hit points per level after it (Hit point adds) |
| 8 | `unknown_0xb4` | i32[2] | | +0xb4, +0xb8 | rogue skill points: at first level, per level (More, Rogue Skills) |
| 9 | `id` | i32 | | +0x10 | database identifier: 1000s warriors, 2000s rogues, 3000s priests, 4000s wizards, 5000 psionicist (Fighter 1000, Paladin 1001). Not unique: six wizard classes have 4001. `id / 1000 - 1` is the group when `unknown_0x14[0]` is out of range. 0 for a copied class |
| 10 | `unknown_0x50` | i32 | | +0x50 | -1 in every shipped class; unknown |
| 11 | `unknown_0xbc` | i32[3] | | +0xbc, +0xc0, +0xc4 | nonweapon proficiencies (More): starting character points, starting slots, levels per extra slot |
| 12 | `unknown_0xcc` | i32[6] | | +0xcc .. +0xe0 | weapon proficiencies (More): starting character points, character point cost of a slot, starting slots, levels per extra slot, non-proficiency penalty (-20 to 20; the sheet uses the least of the character's classes), familiarity penalty |
| 13 | `experience` | i32[101] | | | experience needed for each level, indexed by level (index 0 unused). Levels' OK sets level 1 to 0 and makes each level need at least one point more than the one before |
| 14 | `unknown_0x27c` | i32[101] | v > 20 | +0x27c | THAC0 by level (Levels table, THAC0) |
| 15 | `unknown_0x414` | (i32[11])[101] | | +0x414 | spell slots by class level (row) and spell level (columns 1 to 10; column 0 unused) |
| 16 | `unknown_0x159c` | (i32[11])[101] | | +0x159c | the alternative spell table a class ability selects (Levels, "Alt. 1" to "Alt. 10") |
| 17 | `unknown_0x3050` | i32[102] | v > 21 | +0x3050 .. +0x31e4 | attacks by level (Levels table, Attacks); element 101 (+0x31e4) is the Maximum attacks of Attacks Per Round. A new class has 1 everywhere |
| 18 | `unknown_0x31e8` | i32[102] | v > 21 | +0x31e8 .. +0x337c | the "per rounds" of each level's attacks; element 101 (+0x337c) goes with Maximum attacks. New class: 1 everywhere |
| 19 | `unknown_0x3380` | i32[101] | v > 22 | +0x3380 | armor class by level (Levels table, AC); the sheet takes the lowest non-zero value of the character's classes and the race's natural AC. New class: 10 |
| 20 | `unknown_0x48c4` | i32[101] | v > 29 | +0x48c4 | a per-level table, not zero in every shipped class; unknown |
| 21 | `unknown_0x356c` | (i32[12])[101] | v > 34 | +0x356c, +0x3700, ... +0x46c8 | ability score bonus by level (IDD 221): twelve arrays of 101 values in the object, written interleaved by level. A level's twelve values are the adjustments to Cha, Con, Dex, Int, Str, Wis, then the maxima in the same order. New class: [0 x 6, 25 x 6] per level |
| 22 | `flag_0x278c` | u8 | | +0x278c | rogue: only half of the points allocated per level may go to one skill |
| 23 | `flag_0x278d` | u8 | v > 1 | +0x278d | no editor control; 0 in every shipped class. The sheet does not add the 3-point penalty for a weapon that needs a class (part +0x80c[1]) when one of the character's classes has it set to 1 (0x447720, 0x447780) |
| 24 | `flags_0x486e` | u8[5] | v > 2 | +0x486e, +0x48ac, +0x4a58, +0x27a5, +0x486d | permanently add up to 2 points to Str at first level (Ability Scores); allow any magical items (main page); no control, unknown; rogue: use the thief average ability table for base chances; combine nonweapon slots when multi-classed (More) |
| 25 | `flags_0x278e` | u8[2] | v > 24 | +0x278e, +0x278f | priest: access to 8th and 9th level wizard spells at higher levels (memorization checks Wis and Int); no more than three quarters of the spells taken as wizard spells |
| 26 | `flag_0x2790` | u8 | v > 27 | +0x2790 | priest: double the bonus spells for high Wisdom |
| 27 | `flag_0x2791` | u8 | v > 3 | +0x2791 | warrior: weapon specialization costs 2 CP when single-classed |
| 28 | `flag_0x2792` | u8 | v > 4 | +0x2792 | warrior: weapon specialization allowed at first level |
| 29 | `flags_0x2793` | u8[2] | v > 5 | +0x2793, +0x2794 | warrior: weapon mastery costs 2 CP when single-classed; mastery allowed at fifth level |
| 30 | `flag_0x279d` | u8 | v > 6 | +0x279d | wizard: is a specialist (applied to item restrictions); clearing it empties `unknown_0x2788` |
| 31 | `flags_0x279e` | u8[2] | v > 7 | +0x279e, +0x279f | psionicist: apply item restriction as 5 CP, as 10 CP (the dialog labels both "Not used") |
| 32 | `flags_0x27a0` | u8[3] | v > 8 | +0x27a0, +0x27a2, +0x27a3 | priest: cannot use an edged weapon; wizard: cannot wear armor; wizard: allow the effects of the class ability "Armored Wizard" |
| 33 | `flag_0x27a4` | u8 | v > 9 | +0x27a4 | rogue: candidate for rogue skills |
| 34 | `flags_0x2795` | u8[2] | v > 10 | +0x2795, +0x2796 | warrior: ignore the extra CP cost of two-weapon fighting; allow unarmed combat |
| 35 | `flag_0x2797` | u8 | v > 11 | +0x2797 | warrior: allow weapon specialization for a Core Rules character |
| 36 | `flag_0x27a1` | u8 | v > 13 | +0x27a1 | priest: cannot purchase weapon groups |
| 37 | `flag_0x2ffc` | u8 | v > 16 | +0x2ffc | Armor: do not allow a shield |
| 38 | `flag_0x3554` | u8 | v > 17 | +0x3554 | Ability Scores: allow exceptional strength |
| 39 | `flags_0x3044` | u8[5] | v > 18 | +0x3044 .. +0x3048 | Crossover Penalty (IDD 152), class groups warrior to psionicist. New class: [1, 0, 0, 0, 0] |
| 40 | `flag_0x279c` | u8 | v > 28 | +0x279c | warrior: the special warrior Constitution hit point adjustment |
| 41 | `flags_0x3049` | u8[6] | v > 30 | +0x3049 .. +0x304e | Crossover Penalty, relevant ability scores Str to Cha |
| 42 | `flags_0x4864` | u8[2] | v > 31 | +0x4864, +0x4865 | More: do not average hit points when multi-classed (the level-1 rolls are not divided); double starting gold |
| 43 | `flag_0x486c` | u8 | v > 33 | +0x486c | priest: always use the priest's THAC0 for attack rolls |
| 44 | `unknown_0x27a8` | (i32[5])[101] | v > 14 | +0x27a8 | saving throws by level: paralyzation/poison/death magic, rod/staff/wand, petrification/polymorph, breath weapon, spell (Levels table) |
| 45 | `unknown_0xc` | i32 | | +0xc | 301 to 323 in the shipped classes, numbered in alphabetical order of the names; probably a help topic id (the race editor's hidden HelpID edit is the race's +0xc) |
| 46 | `unknown_0x2798` | i32 | | +0x2798 | warrior: most gold the class may keep, 0 for no limit, 0 to 1000000000 |
| 47 | `unknown_0x2ff8` | i32 | | +0x2ff8 | Armor: best armor class allowed, -10 to 10; set back to -10 unless the choice is 1. New class: -10 |
| 48 | `unknown_0x3040` | i32 | | +0x3040 | Armor choice: 0 any, 1 best armor class allowed, 2 only armor from the list `unknown_0x302c` (choices 0 and 1 empty the list) |
| 49 | `unknown_0x2fc8` | i32[2] | v > 23 | +0x2fc8, +0x2ff4 | how many to pick, 1 to 100: from the free list `unknown_0x2fb4`, from the required list `unknown_0x2fe0`. New class: [1, 1] |
| 50 | `unknown_0x3550` | i32 | | +0x3550 | More, Gender: 0 male and female, 1 male only, 2 female only |
| 51 | `unknown_0x90` | i32 | | +0x90 | More: hit points added at first level ("Plus", 0 to 1000); the NPC generator adds the classes' values when the race gives no bonus |
| 52 | `unknown_0x4a70` | i32[12] | v > 25 | +0x4a70 | unknown; [0 x 6, 25 x 6] in every shipped class and in a new one, the shape of one level of `unknown_0x356c` |
| 53 | `unknown_0x485c` | i32[2] | | +0x485c, +0x4860 | priest: turn undead as if this many levels higher (0 to 100); bonus hit points at each level (More) |
| 54 | `unknown_0x94` | i32[3] | v > 32 | +0x94, +0x98, +0x9c | More: hit dice at first level (1 to 100), then the sides and count of extra dice rolled at first level. New class: [1, 0, 0] |
| 55 | `unknown_0xc8` | i32 | | +0xc8 | More: extra nonweapon slots gained every `unknown_0xbc[2]` levels (0 to 100). New class: 1 |
| 56 | `unknown_0x2788` | CString | | +0x2788 | wizard: specialist school, a Schools Of Magic part ("Shadow", "Song"); empty otherwise |
| 57 | `abbreviation` | CString | | +0x354c | short name in multi-class titles and sheets ("F", "Pa"), up to 2 characters |
| 58 | `unknown_0x353c` | CString[3] | | +0x353c, +0x3540, +0x3544 | reminders shown on entering the Inventory, the Nonweapon Proficiency and the Weapon Proficiency screens (More, IDD 175) |
| 59 | `unknown_0x4868` | CString | | +0x4868 | More: the sphere from which the class receives one extra spell per spell level |
| 60 | `unknown_0x3548` | CString | | +0x3548 | reminder shown on entering the Language screen |
| 61 | `unknown_0x8` | CString | | +0x8 | the class description (Description page) |
| 62 | `unknown_0x4aa0` | CString[2] | | +0x4aa0, +0x4aa4 | empty in every shipped class; unknown |
| 63 | `unknown_0x4aa8` | i32[9] | | +0x4aa8 .. +0x4ac8 | 0 in every shipped class; unknown |
| 64 | `unknown_0x2774` | i32[5] | | +0x2774 .. +0x2784 | More, Magic, 0/1: casts priest spells, casts wizard spells, priestly wizard, wizardly priest, may not change the default spheres |
| 65 | `unknown_0x4a5c` | i32[5] | | +0x4a5c .. +0x4a6c | 0 in every shipped class; unknown |
| 66 | `abilities` | CStringArray | | +0x5c | class abilities Skills & Powers offers (Class Abilities page) |
| 67 | `unknown_0x4870` | CStringArray | v > 37 | +0x4870 | empty in every shipped class; unknown |
| 68 | `unknown_0x70` | CStringArray | | +0x70 | empty in every shipped class; unknown |
| 69 | `alignments` | CStringArray | | +0xa0 | alignments the class allows, by name; the editor writes them in the order Lawful Good, Lawful Neutral, Lawful Evil, Neutral Good, True Neutral, Neutral Evil, Chaotic Good, Chaotic Neutral, Chaotic Evil |
| 70 | `default_abilities` | CStringArray | | +0x2724 | class abilities a new character starts with (More, Default abilities); dual-classing adds those that exist as Class Abilities parts |
| 71 | `unknown_0x2738` | CStringArray | | +0x2738 | major spheres (Spheres parts) |
| 72 | `unknown_0x2760` | CStringArray | | +0x2760 | default schools of magic (More, Schools) |
| 73 | `unknown_0x274c` | CStringArray | | +0x274c | minor spheres |
| 74 | `unknown_0x2fa0` | CStringArray | | +0x2fa0 | nonweapon proficiencies that are all free (Bonus Profs) |
| 75 | `unknown_0x2fcc` | CStringArray | | +0x2fcc | nonweapon proficiencies that must all be purchased (Req. Profs) |
| 76 | `unknown_0x2fb4` | CStringArray | v > 23 | +0x2fb4 | free nonweapon proficiencies to pick `unknown_0x2fc8[0]` of |
| 77 | `unknown_0x2fe0` | CStringArray | v > 23 | +0x2fe0 | required nonweapon proficiencies to purchase `unknown_0x2fc8[1]` of |
| 78 | `unknown_0x3558` | CStringArray | v > 26 | +0x3558 | priest spells the class may not cast |
| 79 | `unknown_0x3004` | CStringArray | v > 19 | +0x3004 | weapons always allowed (Weapons page; 11 for the wild mage and the elementalists) |
| 80 | `unknown_0x3000` | CString | v > 19 | +0x3000 | weapon category allowed: "Any", "Only from list below", "None" or a damage type ("P", "S/B", ...). Shipped: "Any", "Only from list below" for the psionicist, the wild mage and the elementalists. New class: "Any" |
| 81 | `unknown_0x3018` | CStringArray | | +0x3018 | magical items: the same as these standard classes (used when "allow any magical items" is off) |
| 82 | `unknown_0x302c` | CStringArray | | +0x302c | armor always allowed, with armor choice 2 |
| 83 | `unknown_0x4898` | CStringArray | | +0x4898 | wizard: schools the class may not cast spells from (one school for each specialist and each elementalist; none for the wild mage) |
| 84 | `unknown_0x48b0` | CStringArray | v > 38 | +0x48b0 | standard kits a character of the class may not select |
| 85 | `unknown_0x2f8c` | CObArray | | +0x2f8c | empty in every shipped class; element class unknown |
| 86 | `unknown_0x3514` | CObArray<[CPartKitBonusOb](#cpartkitbonusob-versionable-schema-1)> | | +0x3514 | nonweapon proficiencies by level (IDD 172): `name`, `unknown_0x8` the level, `unknown_0xc` 0 for one granted free. Below v 36 the program converts the stored objects after loading; same bytes |
| 87 | `kit_bonuses` | CObArray<[CPartKitBonusOb](#cpartkitbonusob-versionable-schema-1)> | | +0x3528 | proficiency score bonuses by level: `name`, `unknown_0x8` the level from which `unknown_0xc` is added to the proficiency's score (Ranger: Tracking). Below v 37 converted after loading; same bytes |
| 88 | `unknown_0x4884` | CObArray<[CPartKitBonusOb](#cpartkitbonusob-versionable-schema-1)> | | +0x4884 | wizard: non-standard bonuses and penalties to learn spells (IDD 244): `name` a school, `unknown_0xc` the % adjustment (-100 to 100), `unknown_0x8` 0 (22 entries for the wild mage and each elementalist) |
| 89 | `unknown_0x4acc` | CObArray[6] | | +0x4acc | six object arrays, empty in every shipped class; element class unknown |

A new class (the Expansion's constructor 0x662281, `userdb::classes::new_class`)
is all zeros and empty strings, version `TSRP3 V39`, except the defaults
given in the table (`unknown_0x3050`, `unknown_0x31e8`, `unknown_0x3380`,
`unknown_0x356c`, `unknown_0x4a70`, `flags_0x3044`, `unknown_0x3000`,
`unknown_0x2ff8`, `unknown_0x2fc8`, `unknown_0x94`, `unknown_0xc8`).

## CClassOb, Core Rules 2.0 layout (schema 88, Serialize 0x48c090 in EVERMORE.EXE)

`Core20ClassData`, a fixed layout with no version. Member offsets are those of
the 2.0 object. The first 33 values and the three level tables have the order
and sample values of the Expansion's members (Fighter: id 1000 at value 22;
Paladin: minimum Str 12 at value 2).

| # | field | type | meaning (Expansion equivalent) |
|---|---|---|---|
| 1 | `name` | CString | class name |
| 2 | `unknown_0x10` | i32[33] | the Expansion's `unknown_0x14` (2), `ability_requirements` (6), `unknown_0x34` (6), `unknown_0x4c`, `unknown_0x54` (2), `unknown_0x84` (3), `unknown_0xb4` (2), `id`, `unknown_0x50`, `unknown_0xbc` (3), `unknown_0xcc` (6), in that order: [0] group, [1] the per-class index, [2..8] minimum scores, [8..14] prime requisites, [17] hit die, [18] last hit-die level, [19] flat hit points, [22] id, [25] nonweapon starting slots, [29] weapon starting slots, [31] non-proficiency penalty |
| 3 | `experience` | i32[101] | `experience` |
| 4 | `unknown_0x264` | (i32[11])[101] | `unknown_0x414`, spell slots |
| 5 | `unknown_0x13ec` | (i32[11])[101] | `unknown_0x159c`, the alternative spell table |
| 6 | `unknown_0x8` | i32 | the values of the Expansion's `unknown_0xc` (Fighter 309) |
| 7 | `unknown_0x25f0` | i32[8] | 0 in every shipped class; unknown |
| 8 | `unknown_0x25d8` | CString | specialist school, `unknown_0x2788` |
| 9 | `unknown_0x2610` | CString[9] | [0] the abbreviation ("F", "Pa"), `abbreviation`; the others empty in every shipped class |
| 10 | `unknown_0x2634` | i32[9] | 0 in every shipped class; unknown |
| 11 | `unknown_0x25c4` | i32[10] | 0/1 values (Paladin [1, 0, 0, 0, 1, 0, ...]); unknown |
| 12 | `unknown_0x58` | CStringArray[12] | [0] class abilities (`abilities`), then the app reads [1] to [8] as the Expansion's `unknown_0x70`, `alignments`, `default_abilities`, `unknown_0x2738` (major spheres), `unknown_0x2760` (schools), `unknown_0x274c` (minor spheres), `unknown_0x2fa0`, `unknown_0x2fcc`. Shipped classes fill only [0] and [2] to [6]; [9] to [11] unknown |
| 13 | `unknown_0x26bc` | CObArray[10] | ten object arrays, empty in every shipped class; element class unknown |

The 2.0 layout has none of the Expansion's flags, THAC0, armor class, attack
and saving throw tables, or the other lists.

### How the app uses a 2.0 class

The app never converts a 2.0 class to the Expansion layout and never writes
one. Core Rules 2.0 has no `ClassU.dat`, and its `.xpt` packages have no class
section ([export.md](export.md)), so a 2.0 class cannot be imported into an
Expansion installation: an import brings 2.0 races, whose combinations go to
`AllowedU.dat` ([below](#keeping-allowedudat-in-step)), but no classes. A 2.0
installation is opened read only, and each part of the app reads its
`Class.dat` in place, taking from the 2.0 layout what the Expansion layout
would give:

| use | Expansion source | 2.0 source |
|---|---|---|
| group (`userdb::classes::class_group`, sheet, NPC) | `unknown_0x14[0]` | `unknown_0x10[0]` |
| id (`class_id`, character choices) | `id` | `unknown_0x10[22]` |
| ability minimums, prime requisites (`dual.rs`, catalog) | `ability_requirements`, `unknown_0x34` | `unknown_0x10[2..8]`, `[8..14]` |
| hit die, last die level, flat hit points (`advance.rs`) | `unknown_0x84` | `unknown_0x10[17..20]` |
| weapon and nonweapon starting slots (`dual.rs`) | `unknown_0xcc[2]`, `unknown_0xbc[1]` | `unknown_0x10[29]`, `[25]` |
| non-proficiency penalty (sheet) | `unknown_0xcc[4]` | `unknown_0x10[31]` |
| experience, spell tables | `experience`, `unknown_0x414`, `unknown_0x159c` | `experience`, `unknown_0x264`, `unknown_0x13ec` |
| alignments, default abilities, spheres, schools (`dual.rs`, NPC) | the named lists | `unknown_0x58[2..7]` |
| class abilities offered (catalog) | `abilities` | `unknown_0x58[0]` |
| abbreviation (sheet titles) | `abbreviation` | `unknown_0x2610[0]` |
| specialist school (NPC) | `unknown_0x2788` | `unknown_0x25d8` |
| casts priest / wizard spells | `unknown_0x2774[0]`, `[1]` | group 2 / group 3, or a warrior / rogue with any spell slot |
| THAC0 and saving throws | `unknown_0x27c`, `unknown_0x27a8` | the app's built-in 2.0 tables by group and level |
| weapon category | `unknown_0x3000` | taken as "Any" |

Skills & Powers values (class ability points, proficiency points) count as 0
for a 2.0 class, and the flags as unset. The class editor edits Expansion
classes only: on a 2.0 class every field but the name reads as 0 and changes
to it are dropped.

## CPartKitBonusOb (versionable schema 1)

A name with two numbers, used by classes (`unknown_0x3514`, `kit_bonuses`,
`unknown_0x4884`) and by characters (the kit's rogue skill bonuses and the
proficiency chosen at each level, see [character.md](character.md)). The
class tag stores schema 1 ([archive.md](archive.md#schemas)); the object
then stores a version of its own.

| # | field | type | meaning |
|---|---|---|---|
| 1 | `version` | u32 | 1 in every record; the app writes 1. No field depends on it |
| 2 | `name` | CString | a proficiency, school or skill name ("Tracking", "Wild Magic") |
| 3 | `unknown_0x8` | i32 | a level (class lists), 0 in `unknown_0x4884` |
| 4 | `unknown_0xc` | i32 | the number that goes with the name: 0 for a free proficiency, a score bonus, a % adjustment; for a character's per-level choice, the class index |

## Allowed classes per race

`UserDbas/AllowedU.dat` (Expansion only) says which classes and multi-class
combinations each race may take, and the race's level limit in each class.
It is one `CObArray` of `CClassesAllowedOb`, one record per race, matched by
the race's name. A subrace has no record: the creation pages, the NPC
generator and level limits look up the parent race (the character's base
race). Core Rules 2.0 has no such file; its races carry the same information
themselves ([race.md](race.md)).

### CClassesAllowedOb (schema 95, size 36, Serialize 0x68954b)

| # | field | type | meaning |
|---|---|---|---|
| 1 | `version` | i32 | 0 in every shipped record and in records the app writes; the program does not test it, and no field depends on it |
| 2 | `race` | CString | race name ("Aarakocra"), member +0x4 |
| 3 | `unknown_0x1c` | CString | empty in every shipped record; the app writes "" |
| 4 | `flag_0x20` | u8 | 0 in every shipped record; the app writes 0 |
| 5 | `classes` | CObArray<CClassesAllowedHelperOb> | the race's entries, member +0x8 |

### CClassesAllowedHelperOb (schema 95, size 40, Serialize 0x671f50)

One allowed class or multi-class combination. The same class lists the
classes a kit allows (`CPartKitOb` member +0x65c, see [parts.md](parts.md)).

| # | field | type | present | meaning |
|---|---|---|---|---|
| 1 | `version` | i32 | | 1 in every shipped entry; the app writes 1 |
| 2 | `warrior` | CString | | warrior class of the entry ("Fighter"), member +0x8; empty for none |
| 3 | `rogue` | CString | | rogue class ("Thief"), +0xc |
| 4 | `priest` | CString | | priest class ("Cleric"), +0x10 |
| 5 | `wizard` | CString | | wizard class ("Mage", "Illusionist"), +0x14 |
| 6 | `psionicist` | CString | | "Psionicist", +0x18 |
| 7 | `level_limit` | i32 | v > 0 | the race's maximum level, member +0x1c: for a single-class entry the limit in that class (Aarakocra Cleric 7), 0 for no racial limit (every Human entry); 0 in every multi-class entry and in kits. The app clamps what it writes to 0 to 100 |
| 8 | `unknown_0x20` | CString | | empty in every shipped entry; the app writes "" |
| 9 | `flag_0x4` | u8 | | 1 for a shipped entry, 0 for one the user added (and in kits). The Allowed Classes dialog edits only entries with 0 |
| 10 | `flag_0x24` | u8 | | 0 in every shipped entry; the app writes 0 |

An entry with one class name is a single class; one with two or more is a
multi-class combination, its name the class names joined with `/` in slot
order ("Fighter/Thief"). Each slot takes a class of its group, so an entry has
at most five classes. When the program makes an entry from a list of names
(`47cf69`, `689761`; the app's `Allowed::from_names`) it refuses a list of six
names or more, puts each name in the slot of its class's group, keeps the
first name of a group, and skips names that are not classes; the app also
refuses a list in which no name is a class.

In the shipped file each record lists its multi-class entries first, then its
single classes (30 records, 208 entries, all version 1 with `flag_0x4` 1).

What reads it:

* character creation (the class page lists every entry the character's
  ability scores meet) and the NPC generator (its class choices);
* level limits: for a character that applies racial limits (character member
  `unknown_0x1dc4` 1, or 2 with prime requisite bonuses), the first
  single-class entry of the race for the class gives the limit; 0, or a value
  above the class's maximum level, gives the class's maximum level;
* the Allowed Classes dialog and the Export and Import of the DM Toolkit.

### Keeping AllowedU.dat in step

The Expansion keeps no link from a race or class to its entries other than
the names, so the file changes only at these points, in the program and in the
app alike:

* **Allowed Classes… on a race** (IDD 189, `userdb::allowed`): opening it for
  a race without a record appends an empty record (version 0, the race's
  name, "", 0, no entries) and writes the file at once. OK removes the
  race's user entries (`flag_0x4` 0) and inserts the dialog's entries, each
  with version 1, its level limit clamped to 0 to 100 and `flag_0x4` 0, at
  the position right after the last shipped multi-class entry (or at the
  start), each new one at that same position, so they end up in the reverse
  of the dialog's sorted order. Shipped entries are kept as they are. A race
  set to dual-class may not get a combination ("This Race is set to allow
  dual-class.\nA Race may not dual-class and multi-class.").
* **A new class** gets no entry: after New or Copy the editor says to assign
  the class to races from the Allowed Classes screen.
* **Changing a class's group** makes the original warn ("Changing the Class
  group will remove all Race/Class combination information.") and remove
  the class from `AllowedU.dat` on Yes. The app has the message but does not
  remove the entries.
* **Deleting** a race or class changes nothing in `AllowedU.dat` (nothing
  cascades in the program); entries naming a deleted class stay and fail at
  use. The app's editors do not touch the file when a race or class is
  renamed either.
* **Export** (Expansion, [export.md](export.md)): a race takes its record
  along, replacing the package's record of that race (a warning when the race
  has none); a class takes along, for each race, the entries that name it,
  added to the package's record of the race (created empty when missing)
  unless the record has an entry with the same five slots.
* **Import** into the Expansion:
  * an Expansion race removes its record from `AllowedU.dat` and appends a
    new one built from the package's record of that race, entry by entry
    with the name rule above, keeping each entry's `flag_0x4` and level
    limit and skipping an entry whose five slots are already there; a
    package without a record for the race gives a warning and leaves the race
    with none;
  * a 2.0 race (the package has no combinations section) that has no record
    gets one built from its own lists (0x47c91e): an entry for each name of
    `unknown_0x1b8`, then one for each `multi_classes` combination, its ids
    turned into names by the last class in the class list with that id; all
    with `flag_0x4` 0 and level limit 0. A race that has a record keeps it;
  * a class adds, to each race's record (created when missing), every entry
    of the package that names it, the same way (0x49f33c), with a warning for
    a race where none could be added.

## Notes

* `classes.rs` says `unknown_0x3000` is "Any" in every shipped class; the
  psionicist, the wild mage and the four elementalists have "Only from list
  below".
* `classes.rs` calls `unknown_0x54[1]` "a second value (30 for most
  classes)"; the class editor's Levels page edits it as the maximum level
  (`slots.rs` +0x58).
* `parts.md` gives `flag_0x4` as "1 in AllowedU.dat, 0 in kits"; user entries
  in `AllowedU.dat` have 0 too, and the dialog and Import rely on that.
* The reader writes an older class back in its own version's layout, while
  the Expansion would save it as `TSRP3 V39`; no shipped or user file has an
  older version, so this has never been exercised.
* The element classes of `unknown_0x2f8c`, `unknown_0x4acc` and the 2.0
  `unknown_0x26bc`, and the members marked unknown, are open. How the
  Expansion itself reacts to a 2.0 `Class.dat` is not known; the reader's
  `TSRP` test is this app's own.
