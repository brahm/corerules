# misc: Treasure, TreaBun, Psionic, Traps, Phrase, Convert

Python: `py/misc.py` (readers registered in `arc.READERS`; `python3 -I py/misc.py Convert.dat`
for Convert). Shared inventory classes are in `py/common.py` (`inventory_item`,
`inventory_bundle`, `inv_extra`; `READERS['CInventoryBundle']`).

Types: `u32` = 4-byte load (0x40d250 / 0x40d1b0; signedness not distinguished by the helper),
`u8` = 1-byte load 0x40d1f0, `CString` = 0x741623, `count` = CArchive::ReadCount (u16, 0xFFFF
-> u32), `object` = ReadObject with class tag (u16 0 = NULL).

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

Treasure.dat in Database and UserDbas differ in one byte (file offset 0x3f89a): `CTreasureTableEntry.m64` of
one "Spell Scroll, Wizard" row in "DMG 90a: Scrolls, Spell, Wizard" is 1 in Database and 0 in
UserDbas.

### Convert.dat

`CConversionMapDoc::Serialize` calls the Serialize of 8 embedded `CMapStringToOb` (each 0x1c
bytes, at doc+0x50, +0x6c, +0x88, +0xa4, +0xc0, +0xdc, +0xf8, +0x114) on the same CArchive,
so the class-tag table carries over: `CStringArray` is defined once (new-class tag, schema 0) at
the start of map 0 and then referenced as `0x8001` everywhere.

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
inside each bucket. For byte-exact round trips keep the file order as read.

## CInventoryItem (not tagged; serialized inline)

rtc 0x81de50, schema 1, size 0x1c, vtable 0x79e81c, Serialize 0x4acb0e. Ctor 0x4ac7ce sets
m10 = 1, m8 = 999, m4 = 0, m14 = m18 = NULL. Appears as element of CInventoryBundle's
`CList<CInventoryItem, CInventoryItem&>` and embedded in CTreasureTableEntry+0xc. It never has
a class tag of its own.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | name | CString | +0x0c | item name, or the name of another treasure table (`DMG 89: Potions and Oils`) |
| 2 | m10 | u32 | +0x10 | quantity: always 1 in table rows; 2 per patch in `Robe of Useful Items`, 10/20/30/40 gems in `Helm of Brilliance` |
| 3 | bundle | object `CInventoryBundle` or NULL | +0x14 | ReadObject(RUNTIME_CLASS(CInventoryBundle)) via 0x4aa752; contents of the item (226 non-NULL in Treasure.dat) |
| 4 | m8 | u32 | +0x08 | always 999 (ctor default) |
| 5 | m4 | u8 -> BOOL | +0x04 | stored as `byte != 0`; always 0 in data |
| 6 | has_extra | u8 | (local) | if nonzero: `new` 0x50-byte object (ctor 0x4ad650) at +0x18 and call its Serialize inline (no tag); else +0x18 = NULL. Always 0 in the shipped data |
| 7 | extra | InventoryItemExtra | +0x18 | only if has_extra |

### InventoryItemExtra (anonymous, 0x50 bytes, vtable 0x79e838, Serialize 0x4adc02)

No CRuntimeClass of its own (GetRuntimeClass is CObject's), so it can only appear inline.
Never present in shipped data; layout from code.

| # | field | type | offset |
|---|---|---|---|
| 1 | m4 | u32 | +0x04 |
| 2 | m8 | u32 | +0x08 |
| 3 | mc | u32 | +0x0c |
| 4 | sa10 | CStringArray | +0x10 |
| 5 | dw24 | CDWordArray (count + count x u32, read as one raw block) | +0x24 |
| 6 | by38 | CByteArray (count + count raw bytes) | +0x38 |

## CInventoryBundle

rtc 0x81db88, schema 1, size 0x6c, vtable 0x79e760, Serialize 0x4aa81b. Load first empties
the list (0x4ac070), then calls the Serialize of the `CList<CInventoryItem>` at +0x50
(vtable 0x79e810, Serialize 0x4ac370):

| # | field | type | offset |
|---|---|---|---|
| 1 | items | count + count x CInventoryItem (inline) | +0x50 |

Nothing else of the object is serialized.

## CTreasureSingleTableOb (Treasure.dat records)

rtc 0x831050, schema 1, size 0x2c, Serialize 0x546175.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | name | CString | +0x28 | table name, `DMG 88: Magical Items` |
| 2 | m4 | u8 -> BOOL | +0x04 | 1 in all 496 tables |
| 3 | entries | CObList<CTreasureTableEntry> | +0x08 | rows |

## CTreasureTableEntry

rtc 0x831cd8, schema 1, size 0x68, Serialize 0x54963d. Ctor 0x54869e (CStrings at +0x40,
+0x50; CInventoryItem at +0x0c).

| # | field | type | offset | meaning (from data) |
|---|---|---|---|---|
| 1 | weight | u32 | +0x04 | relative chance; per table the weights sum to 20, 100, 1296, 1440, ... |
| 2 | is_table | u8 | +0x08 | 1 iff item.name is the name of another table (500/500) |
| 3 | item | CInventoryItem (inline) | +0x0c | result item / sub-table |
| 4 | qty_dice | u32 | +0x28 | quantity dice count (`Dust of Appearance`: 5) |
| 5 | qty_sides | u32 | +0x2c | quantity die sides (10) |
| 6 | qty_bonus | u32 | +0x30 | quantity add; 1 when no dice, 0 with dice |
| 7 | sub1_dice | u32 | +0x34 | number of rolls on sub-table `s40`: dice |
| 8 | sub1_sides | u32 | +0x38 | sides |
| 9 | sub1_bonus | u32 | +0x3c | add (`Rod of Absorption`: 0d0+1 on `DMG Treasure: Rod Charges`) |
| 10 | sub1_table | CString | +0x40 | sub-table name (`DMG Treasure: Wand Charges`) |
| 11 | sub2_dice | u32 | +0x44 | second sub-table: `Necklace of Prayer Beads` 1d4+2 |
| 12 | sub2_sides | u32 | +0x48 | |
| 13 | sub2_bonus | u32 | +0x4c | |
| 14 | sub2_table | CString | +0x50 | `DMG Treasure: Necklace of Prayer Beads Magical Beads` |
| 15 | m54 | u8 | +0x54 | 25 on swords, 5 on other weapons, else 0 (percent; likely intelligent-weapon chance) |
| 16 | spell_min_level | u32 | +0x58 | spell scrolls: lowest spell level (1..4) |
| 17 | spell_max_level | u32 | +0x5c | highest spell level (4..9) |
| 18 | spell_count | u32 | +0x60 | number of spells (1..7) |
| 19 | spell_wizard | u8 | +0x64 | 1 on `DMG 90a: Scrolls, Spell, Wizard` rows, 0 on Priest rows |

(The names for 4..19 are inferred from DMG tables; use member offsets if unsure.)

## CTreasureOb (TreaBun.dat / TreaBunU.dat)

rtc 0x8307d8, schema 95 (0x5f), size 0x74, vtable 0x7a349c, Serialize 0x53fc98.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | name | CString | +0x04 | bundle name ("Saved Treasure" UI) |
| 2 | bundle | CInventoryBundle (inline, no tag) | +0x08 | count + CInventoryItems |

## CPsionicPowerOb (Psionic.dat)

rtc 0x823a78, schema 96, size 0x6c, Serialize 0x4ed36c. Read order (single chained `ar >>`):

| # | field | type | offset | meaning (from data) |
|---|---|---|---|---|
| 1 | name | CString | +0x20 | `Aura Sight` |
| 2 | m24 | u32 | +0x24 | unique per record (2, 3, 4, ...); likely an id / help id |
| 3 | m28 | u32 | +0x28 | 0..4, small category (likely discipline) |
| 4 | m2c | u32 (0x40d1b0) | +0x2c | 0/1 (173 x 1, 58 x 0; likely devotion vs science) |
| 5 | m30 | u32 | +0x30 | 3..10 |
| 6 | m34 | u32 (0x40d1b0) | +0x34 | 0/1 |
| 7 | m38 | u32 (0x40d1b0) | +0x38 | 0/1 |
| 8 | cost | CString | +0x3c | PSP cost text `9/3`, `varies` |
| 9 | m48 | u32 | +0x48 | often equals the initial cost (`9/3` -> 9) |
| 10 | m44 | u32 | +0x44 | |
| 11 | range | CString | +0x4c | `50 yards`, `touch`, `0` |
| 12 | area | CString | +0x50 | `Personal`, `individual` |
| 13 | description | CString | +0x68 | long text |
| 14 | m40 | u32 | +0x40 | |
| 15 | m1c | u32 | +0x1c | |
| 16 | prerequisites | CStringArray | +0x54 | power names (`Mindlink`; up to 3) |

## CTrapOb (Traps.dat)

rtc 0x830218, schema 95, size 0x20, Serialize 0x53cc18: 7 CStrings.

| # | field | type | offset | sample (`Deadfall`) |
|---|---|---|---|---|
| 1 | name | CString | +0x1c | `Deadfall` |
| 2 | damage | CString | +0x18 | `5-30 (5d6)` |
| 3 | s14 | CString | +0x14 | `none` |
| 4 | s10 | CString | +0x10 | `` |
| 5 | sc | CString | +0x0c | `` |
| 6 | s8 | CString | +0x08 | `` |
| 7 | s4 | CString | +0x04 | `various` |

## CPhraseSpacingOb (Phrase.DAT)

rtc 0x823a38, schema 88, size 0x19c, Serialize 0x4ec3e5.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | name | CString | +0x194 | phrase / layout key (`12`, `10`, `Roll Method ...`) |
| 2 | spacing | 100 x u32 | +0x04 .. +0x190 | increasing positions (150, 169, 201, ...) |
| 3 | m198 | u32 | +0x198 | 0xFFFFFFFF (88), 0 (351), or 1000/1010/1011 |
