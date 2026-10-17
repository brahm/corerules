# misc: Treasure, TreaBun, Psionic, Traps, Phrase, Convert

Rust: [`treasure.rs`](../../crates/cr-formats/src/treasure.rs) (`TreasureTable`,
`TreasureTableEntry`, `Treasure`), [`inventory.rs`](../../crates/cr-formats/src/inventory.rs)
(`InventoryBundle`, `InventoryItem`, `InventoryExtra`),
[`psionic.rs`](../../crates/cr-formats/src/psionic.rs) (`PsionicPower`),
[`traps.rs`](../../crates/cr-formats/src/traps.rs) (`Trap`),
[`phrase.rs`](../../crates/cr-formats/src/phrase.rs) (`PhraseSpacing`) and
[`convert.rs`](../../crates/cr-formats/src/convert.rs) (`ConversionMaps`). The list files are
read with `ObjectList` ([`database.rs`](../../crates/cr-formats/src/database.rs)). Meanings come
from the treasure generator ([`treasure.rs`](../../crates/cr-store/src/treasure.rs),
[`treasure/`](../../crates/cr-store/src/treasure)), the treasure-table editor
([`userdb/tables.rs`](../../crates/cr-store/src/userdb/tables.rs)), the power and trap editors
([`userdb/powers.rs`](../../crates/cr-store/src/userdb/powers.rs),
[`userdb/traps.rs`](../../crates/cr-store/src/userdb/traps.rs)) and the equipment lists
([`lists.rs`](../../crates/cr-store/src/lists.rs)). The base encoding is in
[archive.md](archive.md).

Types: `u32` = 4-byte load (0x40d250 / 0x40d1b0; signedness not distinguished by the helper),
`u8` = 1-byte load 0x40d1f0, `bool` = a byte stored as `!= 0`, `CString` = 0x741623, `count` =
CArchive::ReadCount, `object` = ReadObject with class tag (u16 0 = NULL); see
[archive.md](archive.md). Field names are the Rust ones; the offset column is the member in the
original object. None of these classes reads a version, and the Expansion and Core Rules 2.0
write the same layouts (the treasure engine differs only in how it rolls).

## File-level framing

| File | Document class (Serialize) | Content |
|---|---|---|
| Database/Treasure.dat, UserDbas/Treasure.dat | CTreasureTablesDoc (0x54d174) | CObList at doc+0x54: count + `CTreasureSingleTableOb` objects |
| Database/TreaBun.dat, UserDbas/TreaBunU.dat | CTreasureDoc (0x53d734) | CObList at doc+0x54 of `CTreasureOb` (both files empty: count 0) |
| Database/Psionic.dat, UserDbas/PsionicU.dat | CPsionicPowerDoc (0x4ec7c4) | CObList at doc+0x54 of `CPsionicPowerOb` |
| Database/Traps.dat, UserDbas/TrapsU.dat | CTrapDoc (0x539cf4) | CObList at doc+0x54 of `CTrapOb` |
| Database/Phrase.DAT | CPhraseSpacingDoc (0x4eb985) | CObList at doc+0x250 of `CPhraseSpacingOb` |
| Database/Convert.dat | CConversionMapDoc (0x4561bd, size 0x130) | 8 x CMapStringToOb back to back, see below |

The list docs read nothing besides the CObList (load branch only empties the old list first).
The DB manager (DatabaseOb.cpp, 0x472b40 / 0x473xxx) opens each file through the doc's
OnOpenDocument; TreaBunU.dat doc lives at manager+0x11fc and its ctor 0x53d4e4 installs the
CTreasureDoc vtable 0x7a3070. CTreasureDoc 0x53d9f3 does `new CTreasureOb` (0x74 bytes, ctor
0x53fb9e) and AddTail's it to doc+0x54, which identifies the TreaBun element class.

Unlike the other user files, `UserDbas/Treasure.dat` is a full copy of every table, standard and
user (`flag_0x4` tells them apart); without it the program uses `Database/Treasure.dat`. New
tables are appended; a table name only must not be empty. `TreaBunU.dat` is rewritten whole when
the Saved Treasure Bundles dialog closes (this app keeps the old file as `TreaBunU.dat.bak`).

Treasure.dat in Database and UserDbas differ in one byte (file offset 0x3f89a):
`CTreasureTableEntry.unknown_0x54` of one "Spell Scroll, Wizard" row in "DMG 90a: Scrolls, Spell,
Wizard" is 1 in Database and 0 in UserDbas.

### Convert.dat

`CConversionMapDoc::Serialize` calls the Serialize of 8 embedded `CMapStringToOb` (each 0x1c
bytes, at doc+0x50, +0x6c, +0x88, +0xa4, +0xc0, +0xdc, +0xf8, +0x114) on the same CArchive,
so the class-tag table carries over: `CStringArray` is defined once (new-class tag, schema 0) at
the start of map 0 and then referenced as `0x8001` everywhere. Rust `ConversionMaps::maps`, each
a list of `(key, object)` pairs in file order. Nothing in the app reads the maps.

CMapStringToOb::Serialize (0x73754e) load: `count`, then `count` times `{CString key, object
value}` (ReadObject with no class check). Every value in the shipped file is a `CStringArray`
object (`CStringArray::Serialize`: count + CStrings); no NULLs or back-references occur.

| # | doc offset | entries | content (inferred from data) |
|---|---|---|---|
| 0 | +0x50 | 1454 | old item/equipment name -> new Parts names (`Sling stone` -> [`Sling, sling stone`]) |
| 1 | +0x6c | 1348 | old monster name -> new monster names (some empty arrays) |
| 2 | +0x88 | 6 | old race -> new race (`Dwarf` -> [`Hill dwarf`]) |
| 3 | +0xa4 | 407 | spell -> spell (wizard spells, e.g. `Trap the Soul`) |
| 4 | +0xc0 | 312 | spell -> spell (priest spells, e.g. `Tongues`) |
| 5 | +0xdc | 1329 | monster name -> monster names (e.g. `Whale, Narwhal 4 HD` -> [`Whale, Narwhal`]) |
| 6 | +0xf8 | 65 | nonweapon proficiency -> nonweapon proficiency |
| 7 | +0x114 | 13 | language -> list of languages/dialects (`Dwarf` -> 5 dialects) |

**Writing order.** On store MFC iterates hash buckets 0..16 (hash table size 17, ctor
0x737269; block size 10 from 0x455b80) and each bucket's chain from its head. Hash is
MFC 4 `HashKey(LPCTSTR)`: `h = h*33 + (signed char)c` over the key bytes, bucket = `h % 17`.
Verified: in all 8 maps the keys in the file are in non-decreasing bucket order. SetAt inserts
at the chain head, so a load-then-save in the original program reverses the order of keys
inside each bucket. For byte-exact round trips keep the file order as read (the Rust does).

## CInventoryItem (not tagged; serialized inline)

Rust `InventoryItem` (custom `impl Field`). rtc 0x81de50, schema 1, size 0x1c, vtable 0x79e81c,
Serialize 0x4acb0e. Ctor 0x4ac7ce sets +0x10 = 1, +0x8 = 999, +0x4 = 0, +0x14 = +0x18 = NULL.
Appears as element of CInventoryBundle's `CList<CInventoryItem, CInventoryItem&>` and embedded
in CTreasureTableEntry+0xc. It never has a class tag of its own.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x0c | item name (a part name), or the name of another treasure table (`DMG 89: Potions and Oils`) |
| 2 | `quantity` | u32 | +0x10 | how many: always 1 in table rows; 2 per patch in `Robe of Useful Items`, 10/20/30/40 gems in `Helm of Brilliance`; 3 torches |
| 3 | `contents` | object `CInventoryBundle` or NULL | +0x14 | ReadObject(RUNTIME_CLASS(CInventoryBundle)) via 0x4aa752; what the item holds (226 non-NULL in Treasure.dat) |
| 4 | `unknown_0x8` | u32 | +0x08 | always 999 (ctor default); kept as read |
| 5 | `flag_0x4` | bool | +0x04 | always 0 in data; a set item is left out of the bundle name walk 0x4abc84 |
| 6 | presence byte | u8 | (local) | if nonzero: `new` 0x50-byte object (ctor 0x4ad650) at +0x18 and call its Serialize inline (no tag); else +0x18 = NULL. Always 0 in the shipped data; the writer stores 1 |
| 7 | `extra` | [InventoryExtra](#inventoryextra) | +0x18 | only if the presence byte is nonzero |

### InventoryExtra

Rust `InventoryExtra`. Anonymous, 0x50 bytes, vtable 0x79e838, Serialize 0x4adc02. No
CRuntimeClass of its own (GetRuntimeClass is CObject's), so it can only appear inline. Never
present in shipped data. It is the spell record of an item that holds spells (a spell book or a
scroll): buying a part with "Pages for spells" (`CPart::unknown_0x7cc[6]`, +0x7e4) gives the
item one with that capacity, and the treasure roller adds one to a spell scroll.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `unknown_0x4` | u32[3] | +0x04, +0x08, +0x0c | pages: capacity, used; then the number of spells |
| 2 | `unknown_0x10` | CStringArray | +0x10 | spell names |
| 3 | `unknown_0x24` | CDWordArray (count + count x u32, read as one raw block) | +0x24 | pages of each spell |
| 4 | `unknown_0x38` | CByteArray (count + count raw bytes) | +0x38 | 1 wizard, 0 priest, per spell |

Writing a spell (0x4ad755) is refused when it needs more pages than are left; erasing one
(0x4ad7d0) lowers the count and the pages used and removes the name and pages but leaves the
wizard flags, so the flags after it no longer line up.

## CInventoryBundle

Rust `InventoryBundle`. rtc 0x81db88, schema 1, size 0x6c, vtable 0x79e760, Serialize 0x4aa81b.
Load first empties the list (0x4ac070), then calls the Serialize of the `CList<CInventoryItem>` at
+0x50 (vtable 0x79e810, Serialize 0x4ac370):

| # | field | type | offset |
|---|---|---|---|
| 1 | `items` | count + count x CInventoryItem (inline) | +0x50 |

Nothing else of the object is serialized. Characters embed nine bundles; monsters, encounters
and saved treasures embed one or two; an item's `contents` holds one as a tagged object.

## CTreasureSingleTableOb (Treasure.dat records)

Rust `TreasureTable`. rtc 0x831050, schema 1, size 0x2c, Serialize 0x546175.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x28 | table name, `DMG 88: Magical Items` |
| 2 | `flag_0x4` | bool | +0x04 | 1 = a shipped table (the editor offers Restore from `Database/Treasure.dat`), 0 = a user table (Delete); 1 in all 496 shipped tables |
| 3 | `entries` | CObList<CTreasureTableEntry> | +0x08 | rows |

## CTreasureTableEntry

Rust `TreasureTableEntry`. rtc 0x831cd8, schema 1, size 0x68, Serialize 0x54963d. Ctor 0x54869e
(CStrings at +0x40, +0x50; CInventoryItem at +0x0c). A roll (0x5462c7, 2.0 0x5cdbb0) draws
`rand() % total + 1` against the running sum of the weights; dice below are `bonus + count x
(1 + rand() % sides)` (the Expansion rolls nothing for dice with no sides).

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `weight` | u32 | +0x04 | relative chance; per table the weights sum to 20, 100, 1296, 1440, ... |
| 2 | `is_table` | u8 | +0x08 | 1 iff `item.name` is the name of another table (500/500): the roll goes on there (nesting depth limit 127) |
| 3 | `item` | CInventoryItem (inline) | +0x0c | result item / sub-table |
| 4 | `quantity_dice` | u32[3] | +0x28, +0x2c, +0x30 | quantity: dice count, sides, bonus (`Dust of Appearance`: 5d10; 0d0+1 for a single item) |
| 5 | `sub_table_1_dice` | u32[3] | +0x34, +0x38, +0x3c | number of rolls on `sub_table_1`, whose results go into the item's contents (`Rod of Absorption`: 0d0+1 on `DMG Treasure: Rod Charges`) |
| 6 | `sub_table_1` | CString | +0x40 | sub-table name (`DMG Treasure: Wand Charges`) |
| 7 | `sub_table_2_dice` | u32[3] | +0x44, +0x48, +0x4c | second sub-table: `Necklace of Prayer Beads` 1d4+2 |
| 8 | `sub_table_2` | CString | +0x50 | `DMG Treasure: Necklace of Prayer Beads Magical Beads` |
| 9 | `unknown_0x54` | u8 | +0x54 | 25 on swords, 5 on other weapons, else 0: percent chance of an intelligent weapon (`rand() % 100 < n`) |
| 10 | `spell_min_level` | u32 | +0x58 | spell scrolls: lowest spell level (1..4) |
| 11 | `spell_max_level` | u32 | +0x5c | highest spell level (4..9) |
| 12 | `spell_count` | u32 | +0x60 | number of spells (1..7); nonzero makes the roller write spells into the item ([InventoryExtra](#inventoryextra)) |
| 13 | `spell_wizard` | u8 | +0x64 | 1 on `DMG 90a: Scrolls, Spell, Wizard` rows, 0 on Priest rows |

The Expansion picks a scroll's spell level as `rand() % (max - min + 1) + min` (at least 1, at
most 9 for wizard and 7 for priest spells); Core Rules 2.0 as `min + 1 + rand() % (max - min)` (no clamps; `min` when the two are equal).

## CTreasureOb (TreaBun.dat / TreaBunU.dat)

Rust `Treasure`. rtc 0x8307d8, schema 95 (0x5f), size 0x74, vtable 0x7a349c, Serialize 0x53fc98.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x04 | bundle name ("Saved Treasure" UI); must not be empty and is unique (trimmed, case ignored); new bundles go at the end |
| 2 | `bundle` | CInventoryBundle (inline, no tag) | +0x08 | count + CInventoryItems |

## CPsionicPowerOb (Psionic.dat)

Rust `PsionicPower`. rtc 0x823a78, schema 96, size 0x6c, Serialize 0x4ed36c. Read order (single
chained `ar >>`). Editor labels are those of IDD 346 "Psionic Power Information". A name must be
unique (the message names the discipline).

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x20 | `Aura Sight` |
| 2 | `unknown_0x24` | u32 | +0x24 | help id: unique per record (2, 3, 4, ...); 0xFFFFFFFF on a new power |
| 3 | `unknown_0x28` | u32 | +0x28 | "Discipline" 0..4: clairsentient, psychokinetic, psychometabolic, psychoportive, telepathic |
| 4 | `unknown_0x2c` | u32 (0x40d1b0) | +0x2c | "Type": 0 science, else devotion (173 x 1, 58 x 0) |
| 5 | `unknown_0x30` | u32 | +0x30 | "MAC" (3..10; new powers 10) |
| 6 | `unknown_0x34` | u32 (0x40d1b0) | +0x34 | MAC "Base" check |
| 7 | `unknown_0x38` | u32 (0x40d1b0) | +0x38 | MAC "Varies" check |
| 8 | `cost` | CString | +0x3c | "PSP Cost" text `9/3`, `varies` |
| 9 | `unknown_0x48` | u32 | +0x48 | "Minimum PSP Cost to Use Once" (often the initial cost: `9/3` -> 9) |
| 10 | `unknown_0x44` | u32 | +0x44 | "Minimum Level" (the NPC generator's level of the power) |
| 11 | `range` | CString | +0x4c | "Range": `50 yards`, `touch`, `0` |
| 12 | `area` | CString | +0x50 | "Area of Effect": `Personal`, `individual` |
| 13 | `description` | CString | +0x68 | long text |
| 14 | `unknown_0x40` | u32 | +0x40 | unknown |
| 15 | `unknown_0x1c` | u32 | +0x1c | unknown |
| 16 | `prerequisites` | CStringArray | +0x54 | "Prerequisites": power names (`Mindlink`; up to 3) |

## CTrapOb (Traps.dat)

Rust `Trap`. rtc 0x830218, schema 95, size 0x20, Serialize 0x53cc18: 7 CStrings, read from +0x1c
down to +0x04. The trap editor (IDD 296) limits each to 50 characters. Templates and encounters
hold traps as copies (see [monster.md](monster.md)).

| # | field | type | offset | meaning (sample `Deadfall`) |
|---|---|---|---|---|
| 1 | `name` | CString | +0x1c | "Trap Name": `Deadfall` |
| 2 | `damage` | CString | +0x18 | "Damage": `5-30 (5d6)` |
| 3 | `unknown_0x14` | CString | +0x14 | "Detection Modifier": `none` |
| 4 | `unknown_0x10` | CString | +0x10 | "Number of Characters Trapped": `` |
| 5 | `unknown_0xc` | CString | +0x0c | "Saving Throw": `` |
| 6 | `unknown_0x8` | CString | +0x08 | "Special Effects": `` |
| 7 | `unknown_0x4` | CString | +0x04 | "Location": `various` |

## CPhraseSpacingOb (Phrase.DAT)

Rust `PhraseSpacing`. rtc 0x823a38, schema 88, size 0x19c, Serialize 0x4ec3e5. Nothing in the
app reads it.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x194 | phrase / layout key (`12`, `10`, `Roll Method ...`) |
| 2 | `spacing` | u32[100] | +0x04 .. +0x190 | increasing positions (150, 169, 201, ...) |
| 3 | `unknown_0x198` | u32 | +0x198 | 0xFFFFFFFF (88), 0 (351), or 1000/1010/1011 |

## Notes

* The names of the CTreasureTableEntry dice fields come from the DMG tables the rows reproduce
  and from the roller; `unknown_0x54` is confirmed only as the intelligent-weapon chance.
* CPsionicPowerOb `unknown_0x40` and `unknown_0x1c` are unknown (0 on a new power).
* Phrase.DAT and Convert.dat are decoded but unused; their meanings come from the data only.
