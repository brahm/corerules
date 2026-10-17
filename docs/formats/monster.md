# Monsters and encounters

Rust: [`monster.rs`](../../crates/cr-formats/src/monster.rs) (`Monster` = `CMonsterOb`,
`Arsenal` = `CMonsterArsenal`, `MonsterAttack` = `CMonsterAttackOb`, `MonsterType` =
`CMonsterTypeOb`) and [`encounter.rs`](../../crates/cr-formats/src/encounter.rs) (templates,
encounters, frequency tables, `EncTabU.dat`). `CInventoryBundle` / `CInventoryItem` are in
[`inventory.rs`](../../crates/cr-formats/src/inventory.rs) (see [misc.md](misc.md)), `CPart` in
[`parts.rs`](../../crates/cr-formats/src/parts.rs) (see [parts.md](parts.md)), `CTrapOb` in
[`traps.rs`](../../crates/cr-formats/src/traps.rs) (see [misc.md](misc.md)). The list files
are read with `ObjectList` ([`database.rs`](../../crates/cr-formats/src/database.rs)). Meanings
come from the monster editor ([`userdb/monsters.rs`](../../crates/cr-store/src/userdb/monsters.rs))
and the encounter generator ([`encounter.rs`](../../crates/cr-store/src/encounter.rs),
[`encounter/`](../../crates/cr-store/src/encounter)). The base encoding is in
[archive.md](archive.md).

## Conventions

* Types as in [archive.md](archive.md): `i32` = 4-byte little-endian load (helpers 0x40d250 /
  0x40d1b0 / 0x591770 are all 4-byte), `CString`, `CStringArray`, `CObArray<X>` (count + tagged
  objects, `ReadObject`). `T[N]` is N values back to back; `(i32, i32)[26]` is 26 pairs, each
  pair's two values read one after the other.
* **embedded X** = a C++ member object (not a pointer). Its own virtual `Serialize` is called
  directly, so it is written inline with **no class tag**.
* Field names are the Rust ones; the offset column is the member in the original object
  (`unknown_0x3b8` is `this + 0x3b8`; an array covers consecutive members).
* Every Serialize below begins with an empty base call (0x401f40) and, for some classes, a
  progress-bar update (global 0x862bfc, 0x489400 + vcall 0xc); neither touches the stream.
* None of these classes is VERSIONABLE and no Serialize in this module reads a schema or
  version, except the `EncTabU.dat` document header (a version string that is only checked).
  The Expansion and Core Rules 2.0 write the same layouts; they differ in the character an NPC
  package embeds and in the `EncTabU.dat` version string.

## Files

| file | framing | records in shipped data | result |
|---|---|---|---|
| Database/Monster.dat | CObList: count + tagged `CMonsterOb` | 1524 | reads to EOF |
| UserDbas/MonsterU.dat | same | 0 (2 bytes) | reads to EOF |
| Database/Attack.dat | count + `CMonsterAttackOb` | 21 | reads to EOF |
| UserDbas/AttackU.dat | same (the app only reads it) | 0 | reads to EOF |
| Database/MonType.dat | count + `CMonsterTypeOb` | 298 | reads to EOF |
| UserDbas/MonTypeU.dat | same | 0 | reads to EOF |
| Database/EncPlat.dat | count + `CEncTemplateOb` | 887 | reads to EOF |
| UserDbas/EncPlatU.dat | same | 0 | reads to EOF |
| Database/Encount.dat | count + `CEncounterOb` (CEncounterDoc::Serialize 0x64738a: only doc+0x54 Serialize) | 0 | reads to EOF |
| UserDbas/EncountU.dat | same; rewritten whole when the Encounters screen closes, the old file kept as `EncountU.dat.bak` | 3 | reads to EOF |
| UserDbas/EncTabU.dat | frequency-table document, see below | 0 tables | reads to EOF |
| UserDbas/enctable.dat | 0 bytes; the name does not occur in Tsrp3.exe (case-insensitive search), so this build never opens it | - | empty |

A monster type name starting with `~` is reserved (`~All`, `~Custom`).

### EncTabU.dat (frequency-table document, Serialize 0x63ef52, vtable 0x7ad568)

Rust `FrequencyTables`.

| # | field | type | meaning |
|---|---|---|---|
| 1 | `version` | CString | the Expansion writes `"TSRP3/FreqDoc/0001"`, Core Rules 2.0 `"TSRP2/FreqDoc/0001"` (also the shipped file). On load, if it is neither, AfxMessageBox "Encounter Frequency Tables file format is version '...'. Expecting version '...'. Attempting to continue anyway." and loading continues. |
| 2 | `tables` (doc+0x50) | CObArray<CEncounterFreqTableOb> | shipped file: count 0. Element type from the doc's add code (new CEncounterFreqTableOb, ctor 0x63fde1, at 0x649427 / 0x649811). |

## CMonsterOb (schema 88, size 0x4e8, Serialize 0x67dde6, load branch 0x67e325)

Rust `Monster`. The scalar fields come first (one chained `ar >> a >> b >> ...`), then the
arrays and the embedded members. Editor labels are those of the monster editor (IDD 263). A new
monster has AC 10, the other armor classes 100, hit die size 8, both morale values 2.

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | `name` | CString | +0x32c | "Name" ("Aboleth", "Ankheg, 5 Hit Dice"); unique |
| 2 | `description` | CString | +0x334 | Text page "Appearance"; "Description" placeholder in 1502 records |
| 3 | `picture` | i32 | +0x330 | monster-type picture number (Aboleth 6 = MonType "6.bmp"); always equals `unknown_0x3b8` |
| 4 | `unknown_0x3b8` | i32 | +0x3b8 | same value as `picture` |
| 5 | `frequency` | i32 | +0x3c0 | "Frequency" in percent: 65 common / 20 uncommon / 11 rare / 4 very rare, any other value "Other" (Aboleth 4) |
| 6 | `organization` | CString | +0x3c4 | "Solitary", "Tribal", "Brood" |
| 7 | `activity_cycle` | CString | +0x3c8 | "Any", "Day", "Night" |
| 8 | `diet` | CString | +0x3cc | "Omnivores", "Carnivores" |
| 9 | `intelligence` | CString | +0x3d0 | "13-14  Highly intelligent" |
| 10 | `number_appearing_min` | i32 | +0x3d8 | Number Appearing "Fewest" (Aboleth 1) |
| 11 | `number_appearing_max` | i32 | +0x3dc | "Greatest" (Aboleth 4, Ankheg 6) |
| 12 | `armor_class` | i32 | +0x3f4 | armor class (Aboleth 4, Ankheg 2); `armor_class_notes` qualify it |
| 13 | `movement` | i32 | +0x408 | movement "On Land" (Aboleth 3, Ankheg 12) |
| 14 | `fly_maneuverability` | CString | +0x40c | "Maneuverability Class" A-E for `fly_rate` |
| 15 | `fly_rate` | i32 | +0x410 | "Flying" (30, 24, 18) |
| 16 | `unknown_0x414` | CString | +0x414 | always empty |
| 17 | `swim_rate` | i32 | +0x418 | "Swimming" (Aboleth 18) |
| 18 | `unknown_0x41c` | CString | +0x41c | always empty |
| 19 | `burrow_rate` | i32 | +0x420 | "Burrowing" (Ankheg 6) |
| 20 | `unknown_0x424` | CString | +0x424 | always empty |
| 21 | `unknown_0x428` | i32 | +0x428 | "Climbing" rate (mostly 0) |
| 22 | `unknown_0x42c` | CString | +0x42c | always empty |
| 23 | `unknown_0x430` | i32 | +0x430 | "Web" rate (mostly 0) |
| 24 | `hit_dice` | i32 | +0x434 | "Hit Dice" count (Aboleth 8, Ankheg 5) |
| 25 | `unknown_0x438` | i32 | +0x438 | hit dice bonus, the `+ M` of the editor's dice (0..100) |
| 26 | `hit_die_size` | i32 | +0x43c | hit die sides (8 in 1500 records) |
| 27 | `unknown_0x440` | CString | +0x440 | "Damage" placeholder |
| 28 | `unknown_0x444` | CString | +0x444 | always empty |
| 29 | `defense_notes` | CString | +0x448 | Special Defenses "Defense Description" ("Immune to cold.") |
| 30 | `unknown_0x464` | CString | +0x464 | always empty |
| 31 | `magic_resistance` | i32 | +0x468 | "Magic Resistance" in percent (0, 20, 30, 50) |
| 32 | `morale` | i32 | +0x46c | morale "Highest" (Aboleth 13, Ankheg 9) |
| 33 | `experience` | i32 | +0x474 | "Experience Points" (Aboleth 5000, Ankheg 5 HD 420) |
| 34 | `unknown_0x478` | i32 | +0x478 | always 0 |
| 35 | `thac0` | i32 | +0x3bc | THAC0 (Aboleth 13, Ankheg 5 HD 15) |
| 36 | `unknown_0x470` | i32 | +0x470 | morale "Lowest" (usually equal to `morale`) |
| 37 | `unknown_0x3f8` | i32 | +0x3f8 | second armor class, 100 for none (mostly 100) |
| 38 | `unknown_0x3fc` | i32 | +0x3fc | third armor class (mostly 100) |
| 39 | `unknown_0x400` | i32 | +0x400 | fourth armor class (always 100) |
| 40 | `unknown_0x404` | i32 | +0x404 | fifth armor class (always 100) |
| 41 | `monster_type` | CString | +0x338 | "Type" ("Ankheg", "Fish", "Orc"), a monster type name |
| 42 | `alignment` | CString | +0x3d4 | "Lawful Evil" |
| 43 | `climate_terrain` | CString | +0x4bc | Terrain "Category" ("Any Land"); `"~Custom"` enables the flags of `unknown_0x4` |
| 44 | `unknown_0x33c` | CString | +0x33c | Text page "Habitat/Society"; "String" placeholder |
| 45 | `unknown_0x340` | CString | +0x340 | "Ecology"; "String" placeholder |
| 46 | `unknown_0x344` | CString | +0x344 | "Variants"; "String" placeholder |
| 47 | `notes` | CString | +0x348 | "Combat": long description of special abilities |
| 48 | `unknown_0x47c` | i32[6] | +0x47c .. +0x490 | Size checks: Tiny, Small, Man-Sized, Large, Huge, Gargantuan |
| 49 | `unknown_0x44c` | i32[6] | +0x44c .. +0x460 | Special Defenses checks: hit only by magical/silver weapons, immunity to any spell, immunity to any weapon, invisible at will, regeneration, special defense form |
| 50 | `unknown_0x4` | i32[150] | +0x4 .. +0x258 | climate flags [0..5] (arctic to tropical) and terrain flags [5..37] of the Terrain and Climate dialog (IDD 273); the rest unused; mostly 0/1 (Aboleth: indices 2, 4..36 set) |
| 51 | `unknown_0x25c` | (i32, i32)[26] | +0x25c[i], +0x2c4[i] | treasure types A to Z: each step reads +0x25c[i] (lair count) then +0x2c4[i] (individual count); the book writes them as "A, Bx2, Q" |
| 52 | `armor_class_notes` | CStringArray | +0x3e0 | descriptions of the five armor classes (Ankheg: "Overall", "Underside", "", "", ""); length 0 or 5 (the editor writes all five once any is set, at most 25 characters each) |
| 53 | `unknown_0x4c0` | CObArray<CMonsterAttackOb> | +0x4c0 | empty in all data; element type from CMonsterOb copy code 0x67dc84 (new + ctor 0x68874e = CMonsterAttackOb) |
| 54 | `spells` | CObArray<CSpellsOb> | +0x4d4 | spells (257 entries in Monster.dat); see [spells.md](spells.md) |
| 55 | `arsenal` | embedded CMonsterArsenal | +0x494 | see below |
| 56 | `inventory` | embedded CInventoryBundle | +0x34c | carried items (386 items across Monster.dat) |

### CMonsterArsenal (schema 0x58, size 0x28, rtc 0x84e948, vtable 0x7af198, Serialize 0x68a30a)

Rust `Arsenal`. Only ever embedded (CMonsterOb+0x494), so no tag.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `damage` | CString | +0x8 | "Damage/Attack" ("1-6 (x4)", "3-18 (crush) + 1-4 (acid)") |
| 2 | `attacks` | CString | +0x4 | "No. of Attacks" ("4", "1 or 2") |
| 3 | `special_attacks` | CString | +0xc | "Special Attacks" ("Psionics, enslavement, tentacles") |
| 4 | `special_defenses` | CString | +0x10 | "Special Defenses" ("Slime", "+1 or better weapon to hit") |
| 5 | `attack_list` | CObArray<CMonsterAttackOb> | +0x14 | the Attacks list: copies of attack types from Attack.dat / AttackU.dat (107 entries in Monster.dat) |

## CMonsterAttackOb (schema 88, size 0x48, Serialize 0x6889ac)

Rust `MonsterAttack`.

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | `name` | CString | +0x4 | attack name ("Blood drain", "Employs psionic devotions") |
| 2 | `unknown_0xc` | CString | +0xc | unknown |
| 3 | `unknown_0x8` | i32 | +0x8 | unknown |
| 4 | `unknown_0x10` | i32[10] | +0x10 .. +0x34 | unknown |
| 5 | `unknown_0x3c` | i32 | +0x3c | unknown |
| 6 | `unknown_0x38` | i32 | +0x38 | unknown |
| 7 | `unknown_0x40` | i32 | +0x40 | 1 or 2 in data (devotions 1, sciences 2) |
| 8 | `unknown_0x44` | CString | +0x44 | unknown |

All integer fields are 0 in the shipped data except `unknown_0x40`.

## CMonsterTypeOb (schema 95, size 0x20, Serialize 0x6890dd)

Rust `MonsterType`. The type editor (IDD 294) sets only the name and the bitmap.

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | `name` | CString | +0x4 | "Name of Type" ("Aarakocra", "Aboleth") |
| 2 | `page_picture` | CString | +0x8 | picture-page bitmap ("5to34.bmp") |
| 3 | `unknown_0x14` | CString | +0x14 | empty in data |
| 4 | `unknown_0x10` | CString | +0x10 | empty in data |
| 5 | `unknown_0x18` | i32 | +0x18 | 0..12 in data |
| 6 | `unknown_0x1c` | i32 | +0x1c | 0 or small |
| 7 | `picture` | CString | +0xc | "Bitmap" ("6.bmp") |

## CInventoryBundle (schema 1, size 0x6c, Serialize 0x4aa81b)

Rust `InventoryBundle` / `InventoryItem` / `InventoryExtra`; described in full in
[misc.md](misc.md#cinventorybundle).

* +0x50: `CList<CInventoryItem>` (vtable 0x79e810, Serialize 0x4ac370; head +4, tail +8,
  count +0xc). Load: RemoveAll (0x4ac070), then ReadCount (0x741bfb) and that many
  `CInventoryItem` elements, each read inline by SerializeElements 0x4aa889 (no tag).
* CInventoryItem (rtc 0x81de50 "CInventoryItem", schema 1, size 0x1c, vtable 0x79e81c,
  Serialize 0x4acb0e):

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0xc | item name |
| 2 | `quantity` | u32 | +0x10 | how many |
| 3 | `contents` | object CInventoryBundle (ReadObject, RUNTIME_CLASS CInventoryBundle 0x81db88; may be the null tag) | +0x14 | contents of a container |
| 4 | `unknown_0x8` | u32 | +0x8 | 999 (constructor) |
| 5 | `flag_0x4` | bool | +0x4 | item left out of the name walk 0x4abc84 |
| 6 | presence byte | u8 | - | if nonzero, an anonymous 0x50-byte object is created at +0x18 and serialized; else +0x18 = NULL |
| 7 | `extra` | InventoryExtra | +0x18 | only if the presence byte is nonzero: `unknown_0x4` u32[3], `unknown_0x10` CStringArray, `unknown_0x24` CDWordArray, `unknown_0x38` CByteArray (the spells written in the item) |

## Encounter templates (EncPlat.dat)

### CEncTemplateOb (schema 88, size 0x33c, Serialize 0x648648)

Rust `EncounterTemplate`.

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | `name` | CString | +0x2c4 | template name ("Orc, Wandering"); unique |
| 2 | `unknown_0x2d0` | i32 | +0x2d0 | always 0 |
| 3 | `frequency` | i32 | +0x334 | frequency % (65/20/11/4) as CMonsterOb `frequency`; 0 is exempt from the 2-20 tables |
| 4 | `hit_dice` | i32 | +0x338 | "Number Of Hit Dice" of the 2-20 tables (0..28) |
| 5 | `monster_type` | CString | +0x2c8 | type ("Human", "Mammal") |
| 6 | `notes` | CString | +0x2cc | description, copied into each encounter |
| 7 | `climate_terrain` | CString | +0x2d4 | climate/terrain category ("Any Land") |
| 8 | `placement` | i32[3] | +0x328, +0x32c, +0x330 | one-hot: both lair and wandering, lair only, wandering only |
| 9 | `monsters` | CObArray<CEncTemplateMonsterPropertiesOb> | +0x2d8 | monster lines of the template |
| 10 | `npcs` | CObArray<CEncTemplateNPCPropertiesOb> | +0x2ec | NPC lines |
| 11 | `traps` | CObArray<CTrapOb> | +0x300 | traps (4 in data); see [misc.md](misc.md) |
| 12 | `items` | CObArray<CEncTemplateItemPropertiesOb> | +0x314 | item lines (8 in data) |
| 13 | `lair_treasure` | i32[26] | +0x4 | lair treasure: count per treasure type A..Z |
| 14 | `climates` | i32[150] | +0x6c | climate (0-4) and terrain (5-36) flags, the same shape as CMonsterOb `unknown_0x4` |

### CEncTemplateMonsterPropertiesOb (schema 88, size 0x7c, Serialize 0x651f4a)

Rust `TemplateMonsterProperties`.

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | `dice` | i32[8] | +0x50 .. +0x6c | number appearing N d S * M + B (+0x50..+0x5c), then the second box (+0x60..+0x6c) |
| 2 | `flags` | i32[3] | +0x38, +0x3c, +0x40 | 0/1: dependent (runs in the second pass and counts the first pass's monsters), "and" (the second box appears too), "or" (a d100 over 50 takes the second box instead) |
| 3 | `percent` | i32 | +0x70 | percent chance; mostly 100, which rolls nothing |
| 4 | `per` | i32 | +0x74 | "for every [n]"; 0 adds once |
| 5 | `beyond` | i32 | +0x78 | "beyond [n]" |
| 6 | `monster_name` | CString | +0x44 | monster name ("Ogre", "Orc, Leader") |
| 7 | `second_name` | CString | +0x48 | monster of the second box, or empty ("Orc assistant") |
| 8 | `reference_name` | CString | +0x4c | the monster a dependent line counts when `monster_type` has no name ("Kuo-Toa") |
| 9 | `monster_type` | embedded CMonsterTypeOb | +0x18 | the monster type a dependent line counts; usually all empty, else e.g. "Orc" |
| 10 | `monsters` | CObArray<CMonsterOb> | +0x4 | "randomly selected from the following list": full monster copies (2207 in EncPlat.dat), used from two entries on; repeated entries weigh the draw. The generator re-copies them from the monster databases by name before each generation |

The generator is described in [`encounter.rs`](../../crates/cr-store/src/encounter.rs).

### CEncTemplateNPCPropertiesOb (schema 88, size 0x104, Serialize 0x65295d)

Rust `TemplateNpcProperties`; the fields mean what they mean on a monster line.

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | `dice` | i32[8] | +0xd4 .. +0xf0 | number appearing N d S * M + B (+0xd4..+0xe0), then the second box (+0xe4..+0xf0) |
| 2 | `flags` | i32[3] | +0xc8, +0xcc, +0xd0 | 0/1: dependent, "and", "or" |
| 3 | `percent` | i32 | +0xf4 | percent chance; mostly 100 |
| 4 | `per` | i32 | +0xf8 | "for every [n]" |
| 5 | `beyond` | i32 | +0xfc | "beyond [n]" |
| 6 | `reference_name` | CString | +0x100 | the monster a dependent line counts ("Elf", "Dwarf, Hill") or empty |
| 7 | `monster_type` | embedded CMonsterTypeOb | +0xa8 | the monster type a dependent line counts ("Halfling", "Gnome") |
| 8 | `npc` | embedded CEncounterNPCOb | +0x18 | the NPC of the first box |
| 9 | `second_npc` | embedded CEncounterNPCOb | +0x60 | the NPC of the second box (often empty strings, levels -1) |
| 10 | `npcs` | CObArray<CEncounterNPCOb> | +0x4 | the random list (571 in EncPlat.dat) |

### CEncTemplateItemPropertiesOb (schema 88, size 0xe18, Serialize 0x650cde)

Rust `TemplateItemProperties`.

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | `percent` | i32 | +0xda0 | percent chance |
| 2 | `dice` | i32[9] | +0xdb0 .. +0xdd0 | number of items N d S * M + B (+0xdb0..+0xdbc), the coin (+0xdc0: 0 platinum, 1 gold, 2 silver, 3 copper), "containing" coins per item N d S * M + B (+0xdc4..+0xdd0) |
| 3 | `dependency` | i32[3] | +0xda4, +0xda8, +0xdac | "Dependent" (shown only; generation looks at the reference), "for every", "beyond" |
| 4 | `monster` | embedded CMonsterOb | +0x898 | the monster a dependent item counts, by name (the full CMonsterOb layout above) |
| 5 | `monster_type` | embedded CMonsterTypeOb | +0xd80 | the monster type a dependent item counts; no names = not dependent |
| 6 | `part` | embedded CPart | +0x4 | the item ("Catapult, Heavy", "Cart"); CPart is versioned, see [parts.md](parts.md) |
| 7 | `monster_names` | CStringArray | +0xdd4 | "Double the number of these Monsters" ("Orc, Leader", "Orc assistant") |
| 8 | `escorts` | CObArray<CEncTemplateNumAppearOb> | +0xde8 | "These Monsters will appear if the previous Item(s) appear" (3 in data) |
| 9 | `per_item` | CObArray<CEncTemplateNumAppearOb> | +0xdfc | "For each Item" (1 in data) |

### CEncTemplateNumAppearOb (schema 88, size 0x4fc, Serialize 0x6517d3)

Rust `TemplateNumAppear`.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `dice` | i32[4] | +0x4ec .. +0x4f8 | number appearing N d S * M + B (1d6, 10d6, 1d1+9) |
| 2 | `monster` | embedded CMonsterOb | +0x4 | |

### CEncounterNPCOb (schema 95, size 0x48, Serialize 0x6531e0)

Rust `EncounterNpc`.

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | `name` | CString | +0x4 | NPC name ("Bob", "Norm") |
| 2 | `race` | CString | +0x44 | race ("Human", "Elf") |
| 3 | `classes` | CString[5] | +0x8 .. +0x18 | class 1 ("Fighter"), then further classes or empty |
| 4 | `levels` | i32[5] | +0x1c .. +0x2c | level in each class; -1 (0xffffffff) = none (the last two always -1) |
| 5 | `unknown_0x30` | CObArray<CPartIDOb> | +0x30 | empty in all data; element type from the copy code at 0x653472 (new + ctor 0x65889e, vtable 0x7ae87c = CPartIDOb; see [parts.md](parts.md)) |

## Encounters (Encount.dat, EncountU.dat)

### CEncounterOb (schema 88, size 0x194, Serialize 0x63c659)

Rust `Encounter`.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x6c | encounter name ("Wandering Bugbear", "Huge Orc City"); unique (trimmed, case ignored) |
| 2 | `unknown_0x7c` | i32 | +0x7c | always 0 |
| 3 | `monster_type` | CString | +0x70 | monster type ("Bugbear", "Orc"); "~All" when made from the "~All" list |
| 4 | `template` | CString | +0x74 | source template name ("Orc, Lair (Huge city)") |
| 5 | `description` | CString | +0x78 | |
| 6 | `monsters` | CObArray<CEncounterMonsterPackageOb> | +0x80 | monsters (copy code 0x63c8b0: ctor 0x6403b1) |
| 7 | `npcs` | CObArray<CEncounterNPCPackageOb> | +0x94 | NPCs; empty in data; element type from copy code 0x63ca01 (ctor 0x63b961) |
| 8 | `traps` | CObArray<CTrapOb> | +0x180 | traps; empty in data; element type from copy code 0x63cb52 (ctor 0x53ca2e) |
| 9 | `treasure` | embedded CInventoryBundle | +0xa8 | treasure (copy code error "Treasure was lost.") |
| 10 | `items` | embedded CInventoryBundle | +0x114 | items ("Items were lost.") |
| 11 | `lair_treasure` | i32[26] | +0x4 | the template's lair treasure types, A to Z |

### CEncounterMonsterPackageOb (schema 88, size 0x504, Serialize 0x6405b8)

Rust `MonsterPackage`.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `count` | i32 | +0x4ec | how many (1, 5, 544, 1081...) |
| 2 | `monster` | embedded CMonsterOb | +0x4 | |
| 3 | `hit_points` | CObArray<CEncounterMonHpOb> | +0x4f0 | one entry per distinct hit point value, in first-occurrence order; "Double these monsters" leaves it as it was, so it may add up to less than `count` |

### CEncounterMonHpOb (schema 88, size 0xc, Serialize 0x650612)

Rust `MonsterHp`.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `hit_points` | i32 | +0x4 | hit points |
| 2 | `count` | i32 | +0x8 | how many monsters of the package have them |

### CEncounterNPCPackageOb (schema 88, size 0x21a4 (2.0 0x2028), Serialize 0x63bc0f) - not in shipped data

Rust `NpcPackage`.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `count` | i32 | +0x218c | how many |
| 2 | `character` | embedded CCharacterDoc | +0x4 | vtable 0x79b428, Serialize 0x41cd5b: the same stream as a Chars/<id>.dat file (`TSRP3 V54` or `TSRP2 V29`, see [character.md](character.md)); every individual shares it. Core Rules 2.0 can only hold `TSRP2 V29` characters |
| 3 | `hit_points` | CObArray<CPartIDOb> | +0x2190 | one per individual, hit points in `part_type`; empty for a saved character added with "Add NPC..." |

The embedded character is read with the same archive reader as the surrounding file, so
class tags inside the character continue the archive's tag numbering.

## Frequency tables (EncTabU.dat)

### CEncounterFreqTableOb (schema 1, size 0x28c, Serialize 0x63ff92) - not in shipped data

Rust `FrequencyTable` and `FrequencyTableData` (the embedded filter).

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x274 | table name |
| 2 | `slots` | CStringArray | +0x278 | the template names for the rolls 2 to 20 |
| 3 | `table` | embedded anonymous object (vtable 0x7adf98, size 0x270, Serialize 0x64fe45) | +0x4 | the filter Fill picks candidates with (0x64ff72); constructor 0x64fd10 |
| 3a | `hit_dice_min` | i32 | +0x260 | lowest template hit dice; ctor 0 |
| 3b | `hit_dice_max` | i32 | +0x264 | highest; ctor 100 |
| 3c | `source` | i32 | +0x268 | templates: 0 all, 1 standard, 2 custom; ctor 0 |
| 3d | `category` | CString | +0x25c | climate/terrain category; "Any" takes every template |
| 3e | `lair` | i32 | +0x26c | 0 templates marked lair and wandering, 1 any, 2 wandering, 3 lair; ctor 0 |
| 3f | `climates` | i32[150] | +0x4 | climate (0-4) and terrain (5-36) flags; ctor sets all to 1 |

## Notes

* The integers of CMonsterAttackOb (all 0 in the shipped data but `unknown_0x40`) and the
  CMonsterTypeOb fields `unknown_0x10` to `unknown_0x1c` are unknown; the app's editors do not
  set them.
* CMonsterOb `unknown_0x4` has 150 flags, of which the dialog uses 37; the rest are 0 in data.
* `UserDbas/AttackU.dat` is read for the monster editor's attack list but this app has no editor
  for attack types.
