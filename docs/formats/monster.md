# Monsters and encounters

Rust: [`monster.rs`](../../crates/cr-formats/src/monster.rs) (`CMonsterOb`, `CMonsterArsenal`,
`CMonsterAttackOb`, `CMonsterTypeOb`) and [`encounter.rs`](../../crates/cr-formats/src/encounter.rs)
(templates, encounters, frequency tables, `EncTabU.dat`). Prototype reader: `py/monster.py`.
`CInventoryBundle` / `CInventoryItem` are in [`inventory.rs`](../../crates/cr-formats/src/inventory.rs),
`CPart` in [`parts.rs`](../../crates/cr-formats/src/parts.rs) (see [parts.md](parts.md)).

## Conventions

* `u32` = 4-byte little-endian load (helpers 0x40d250 / 0x40d1b0 / 0x591770 are all 4-byte).
  `CString` = MFC string (u8 length, 0xff -> u16, 0xffff -> u32). `CStringArray` = count + strings.
  `CObArray<X>` = MFC count (u16, 0xffff -> u32) + tagged objects (`ReadObject`).
* **embedded X** = a C++ member object (not a pointer). Its own virtual `Serialize` is called
  directly, so it is written inline with **no class tag**.
* Every Serialize below begins with an empty base call (0x401f40) and, for some classes, a
  progress-bar update (global 0x862bfc, 0x489400 + vcall 0xc); neither touches the stream.
* None of these classes is VERSIONABLE and no Serialize in this module reads a schema or
  version, except the `EncTabU.dat` document header (a version string that is only checked).

## Files

| file | framing | records in shipped data | result |
|---|---|---|---|
| Database/Monster.dat | CObList: count + tagged `CMonsterOb` | 1524 | reads to EOF |
| UserDbas/MonsterU.dat | same | 0 (2 bytes) | reads to EOF |
| Database/Attack.dat | count + `CMonsterAttackOb` | 21 | reads to EOF |
| UserDbas/AttackU.dat | same | 0 | reads to EOF |
| Database/MonType.dat | count + `CMonsterTypeOb` | 298 | reads to EOF |
| UserDbas/MonTypeU.dat | same | 0 | reads to EOF |
| Database/EncPlat.dat | count + `CEncTemplateOb` | 887 | reads to EOF |
| UserDbas/EncPlatU.dat | same | 0 | reads to EOF |
| Database/Encount.dat | count + `CEncounterOb` (CEncounterDoc::Serialize 0x64738a: only doc+0x54 Serialize) | 0 | reads to EOF |
| UserDbas/EncountU.dat | same | 3 | reads to EOF |
| UserDbas/EncTabU.dat | frequency-table document, see below | 0 tables | reads to EOF |
| UserDbas/enctable.dat | 0 bytes; the name does not occur in Tsrp3.exe (case-insensitive search), so this build never opens it | - | empty |

### EncTabU.dat (frequency-table document, Serialize 0x63ef52, vtable 0x7ad568)

| # | field | type | meaning |
|---|---|---|---|
| 1 | version | CString | writes `"TSRP3/FreqDoc/0001"`. On load, if it is neither that nor `"TSRP2/FreqDoc/0001"`, AfxMessageBox "Encounter Frequency Tables file format is version '...'. Expecting version '...'. Attempting to continue anyway." and loading continues. Shipped file: `TSRP2/FreqDoc/0001`. |
| 2 | doc+0x50 | CObArray<CEncounterFreqTableOb> | shipped file: count 0. Element type from the doc's add code (new CEncounterFreqTableOb, ctor 0x63fde1, at 0x649427 / 0x649811). |

## CMonsterOb (schema 88, size 0x4e8, Serialize 0x67dde6, load branch 0x67e325)

The 59 scalar fields come first (one chained `ar >> a >> b >> ...`), then:

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 60 | t_4 | u32[150] | +0x4 .. +0x258 | per-index values, mostly 0/1 (Aboleth: indices 2,4..36 set) |
| 61 | t_25c / t_2c4 | 26 x (u32, u32), interleaved | +0x25c[i], +0x2c4[i] | each loop step reads +0x25c[i] then +0x2c4[i]; 26 = the same count as the CEncTemplateOb / CEncounterOb 26-entry tables |
| 62 | sa_3e0 | CStringArray | +0x3e0 | AC qualifiers for m_3f4 (Ankheg: "Overall", "Underside", "", "", ""); length 0 or 5 |
| 63 | oa_4c0 | CObArray<CMonsterAttackOb> | +0x4c0 | empty in all data; element type from CMonsterOb copy code 0x67dc84 (new + ctor 0x68874e = CMonsterAttackOb) |
| 64 | oa_4d4 | CObArray<CSpellsOb> | +0x4d4 | spells (257 entries in Monster.dat) |
| 65 | arsenal_494 | embedded CMonsterArsenal | +0x494 | see below |
| 66 | inv_34c | embedded CInventoryBundle | +0x34c | carried items (386 items across Monster.dat) |

Scalar fields, in archive order:

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | s_32c | CString | +0x32c | name ("Aboleth", "Ankheg, 5 Hit Dice"); unique |
| 2 | s_334 | CString | +0x334 | "Description" placeholder in 1502 records |
| 3 | m_330 | u32 | +0x330 | monster-type picture number (Aboleth 6 = MonType "6.bmp"); always equals m_3b8 |
| 4 | m_3b8 | u32 | +0x3b8 | same value as m_330 |
| 5 | m_3c0 | u32 | +0x3c0 | frequency %: 65 common / 20 uncommon / 11 rare / 4 very rare (Aboleth 4) |
| 6 | s_3c4 | CString | +0x3c4 | organization ("Solitary", "Tribal", "Brood") |
| 7 | s_3c8 | CString | +0x3c8 | activity cycle ("Any", "Day", "Night") |
| 8 | s_3cc | CString | +0x3cc | diet ("Omnivores", "Carnivores") |
| 9 | s_3d0 | CString | +0x3d0 | intelligence text ("13-14  Highly intelligent") |
| 10 | m_3d8 | u32 | +0x3d8 | number appearing, min (Aboleth 1) |
| 11 | m_3dc | u32 | +0x3dc | number appearing, max (Aboleth 4, Ankheg 6) |
| 12 | m_3f4 | u32 | +0x3f4 | armor class (Aboleth 4, Ankheg 2); sa_3e0 holds AC qualifiers |
| 13 | m_408 | u32 | +0x408 | movement rate (Aboleth 3, Ankheg 12) |
| 14 | s_40c | CString | +0x40c | fly maneuverability class (A-E) for m_410 |
| 15 | m_410 | u32 | +0x410 | fly rate (30, 24, 18) |
| 16 | s_414 | CString | +0x414 | always empty |
| 17 | m_418 | u32 | +0x418 | swim rate (Aboleth 18) |
| 18 | s_41c | CString | +0x41c | always empty |
| 19 | m_420 | u32 | +0x420 | burrow rate (Ankheg 6) |
| 20 | s_424 | CString | +0x424 | always empty |
| 21 | m_428 | u32 | +0x428 | further movement rate (mostly 0) |
| 22 | s_42c | CString | +0x42c | always empty |
| 23 | m_430 | u32 | +0x430 | further movement rate (mostly 0) |
| 24 | m_434 | u32 | +0x434 | hit dice (Aboleth 8, Ankheg 5) |
| 25 | m_438 | u32 | +0x438 | hit dice bonus/extra |
| 26 | m_43c | u32 | +0x43c | hit die size (8 in 1500 records) |
| 27 | s_440 | CString | +0x440 | "Damage" placeholder |
| 28 | s_444 | CString | +0x444 | always empty |
| 29 | s_448 | CString | +0x448 | special-defense text ("Immune to cold.") |
| 30 | s_464 | CString | +0x464 | always empty |
| 31 | m_468 | u32 | +0x468 | magic resistance % (0, 20, 30, 50) |
| 32 | m_46c | u32 | +0x46c | morale (Aboleth 13, Ankheg 9) |
| 33 | m_474 | u32 | +0x474 | XP value (Aboleth 5000, Ankheg 5 HD 420) |
| 34 | m_478 | u32 | +0x478 | always 0 |
| 35 | m_3bc | u32 | +0x3bc | THAC0 (Aboleth 13, Ankheg 5 HD 15) |
| 36 | m_470 | u32 | +0x470 | morale-like, 2nd value (Aboleth 13, Ankheg 9) |
| 37 | m_3f8 | u32 | +0x3f8 | mostly 100 |
| 38 | m_3fc | u32 | +0x3fc | mostly 100 |
| 39 | m_400 | u32 | +0x400 | always 100 |
| 40 | m_404 | u32 | +0x404 | always 100 |
| 41 | s_338 | CString | +0x338 | type/race ("Ankheg", "Fish", "Orc") |
| 42 | s_3d4 | CString | +0x3d4 | alignment ("Lawful Evil") |
| 43 | s_4bc | CString | +0x4bc | climate/terrain ("Any Land", "~Custom") |
| 44 | s_33c | CString | +0x33c | "String" placeholder |
| 45 | s_340 | CString | +0x340 | "String" placeholder |
| 46 | s_344 | CString | +0x344 | "String" placeholder |
| 47 | s_348 | CString | +0x348 | notes / long description of special abilities |
| 48 | m_47c | u32 | +0x47c | 0/1 flag |
| 49 | m_480 | u32 | +0x480 | 0/1 flag |
| 50 | m_484 | u32 | +0x484 | 0/1 flag |
| 51 | m_488 | u32 | +0x488 | 0/1 flag |
| 52 | m_48c | u32 | +0x48c | 0/1 flag |
| 53 | m_490 | u32 | +0x490 | 0/1 flag |
| 54 | m_44c | u32 | +0x44c | 0/1 flag |
| 55 | m_450 | u32 | +0x450 | 0/1 flag |
| 56 | m_454 | u32 | +0x454 | 0/1 flag |
| 57 | m_458 | u32 | +0x458 | 0/1 flag |
| 58 | m_45c | u32 | +0x45c | 0/1 flag |
| 59 | m_460 | u32 | +0x460 | 0/1 flag |

### CMonsterArsenal (schema 0x58, size 0x28, rtc 0x84e948, vtable 0x7af198, Serialize 0x68a30a)

Only ever embedded (CMonsterOb+0x494), so no tag.

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | s_8 | CString | +0x8 | damage/attack ("1-6 (x4)", "3-18 (crush) + 1-4 (acid)") |
| 2 | s_4 | CString | +0x4 | number of attacks ("4", "1 or 2") |
| 3 | s_c | CString | +0xc | special attacks ("Psionics, enslavement, tentacles") |
| 4 | s_10 | CString | +0x10 | special defenses ("Slime", "+1 or better weapon to hit") |
| 5 | oa_14 | CObArray<CMonsterAttackOb> | +0x14 | 107 entries in Monster.dat |

## CMonsterAttackOb (schema 88, size 0x48, Serialize 0x6889ac)

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | s_4 | CString | +0x4 | attack name ("Blood drain", "Employs psionic devotions") |
| 2 | s_c | CString | +0xc |  |
| 3 | m_8 | u32 | +0x8 |  |
| 4 | m_10 | u32 | +0x10 |  |
| 5 | m_14 | u32 | +0x14 |  |
| 6 | m_18 | u32 | +0x18 |  |
| 7 | m_1c | u32 | +0x1c |  |
| 8 | m_20 | u32 | +0x20 |  |
| 9 | m_24 | u32 | +0x24 |  |
| 10 | m_28 | u32 | +0x28 |  |
| 11 | m_2c | u32 | +0x2c |  |
| 12 | m_30 | u32 | +0x30 |  |
| 13 | m_34 | u32 | +0x34 |  |
| 14 | m_3c | u32 | +0x3c |  |
| 15 | m_38 | u32 | +0x38 |  |
| 16 | m_40 | u32 | +0x40 | 1 or 2 in data (devotions 1, sciences 2) |
| 17 | s_44 | CString | +0x44 |  |

All u32 fields are 0 in the shipped data except m_40.

## CMonsterTypeOb (schema 95, size 0x20, Serialize 0x6890dd)

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | s_4 | CString | +0x4 | type name ("Aarakocra", "Aboleth") |
| 2 | s_8 | CString | +0x8 | picture-page bitmap ("5to34.bmp") |
| 3 | s_14 | CString | +0x14 | empty in data |
| 4 | s_10 | CString | +0x10 | empty in data |
| 5 | m_18 | u32 | +0x18 | 0..12 in data |
| 6 | m_1c | u32 | +0x1c | 0 or small |
| 7 | s_c | CString | +0xc | picture bitmap ("6.bmp") |

## CInventoryBundle (schema 1, size 0x6c, Serialize 0x4aa81b)

* +0x50: `CList<CInventoryItem>` (vtable 0x79e810, Serialize 0x4ac370; head +4, tail +8,
  count +0xc). Load: RemoveAll (0x4ac070), then ReadCount (0x741bfb) and that many
  `CInventoryItem` elements, each read inline by SerializeElements 0x4aa889 (no tag).
* CInventoryItem (rtc 0x81de50 "CInventoryItem", schema 1, size 0x1c, vtable 0x79e81c,
  Serialize 0x4acb0e):

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | name | CString | +0xc | item name |
| 2 | m10 | u32 | +0x10 | |
| 3 | bundle | object (ReadObject, RUNTIME_CLASS CInventoryBundle 0x81db88; may be null tag) | +0x14 | contents of a container |
| 4 | m8 | u32 | +0x8 | |
| 5 | m4 | u8 (bool) | +0x4 | |
| 6 | has_extra | u8 | - | if nonzero, an anonymous 0x50-byte object is created at +0x18 and serialized; else +0x18 = NULL |
| 6a | extra.m4, m8, mc | u32 x3 | +0x4,+0x8,+0xc | (object vtable 0x79e838, Serialize 0x4adc02) |
| 6b | extra.sa10 | CStringArray | +0x10 | |
| 6c | extra.dw24 | CDWordArray (count + count x u32) | +0x24 | |
| 6d | extra.by38 | CByteArray (count + count bytes) | +0x38 | |

## Encounter templates (EncPlat.dat)

### CEncTemplateOb (schema 88, size 0x33c, Serialize 0x648648)

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | s_2c4 | CString | +0x2c4 | template name ("Orc, Wandering"); unique |
| 2 | m_2d0 | u32 | +0x2d0 | always 0 |
| 3 | m_334 | u32 | +0x334 | frequency % (65/20/11/4), as CMonsterOb m_3c0 |
| 4 | m_338 | u32 | +0x338 | "Number Of Hit Dice" of the 2-20 tables (0..28) |
| 5 | s_2c8 | CString | +0x2c8 | type ("Human", "Mammal") |
| 6 | s_2cc | CString | +0x2cc | description, copied into each encounter |
| 7 | s_2d4 | CString | +0x2d4 | climate/terrain ("Any Land") |
| 8 | m_328 | u32 | +0x328 | one-hot with m_32c, m_330: both lair and wandering |
| 9 | m_32c | u32 | +0x32c | lair only |
| 10 | m_330 | u32 | +0x330 | wandering only |

Then:

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 11 | oa_2d8 | CObArray<CEncTemplateMonsterPropertiesOb> | +0x2d8 | monsters of the template |
| 12 | oa_2ec | CObArray<CEncTemplateNPCPropertiesOb> | +0x2ec | NPCs |
| 13 | oa_300 | CObArray<CTrapOb> | +0x300 | traps (4 in data); see [misc.md](misc.md) |
| 14 | oa_314 | CObArray<CEncTemplateItemPropertiesOb> | +0x314 | items (8 in data) |
| 15 | t_4 | u32[26] | +0x4 | lair treasure: count per treasure type A..Z |
| 16 | t_6c | u32[150] | +0x6c | same shape as CMonsterOb t_4 (150 entries) |

### CEncTemplateMonsterPropertiesOb (schema 88, size 0x7c, Serialize 0x651f4a)

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | m_50 | u32 | +0x50 |  |
| 2 | m_54 | u32 | +0x54 |  |
| 3 | m_58 | u32 | +0x58 |  |
| 4 | m_5c | u32 | +0x5c |  |
| 5 | m_60 | u32 | +0x60 |  |
| 6 | m_64 | u32 | +0x64 |  |
| 7 | m_68 | u32 | +0x68 |  |
| 8 | m_6c | u32 | +0x6c |  |
| 9 | m_38 | u32 | +0x38 | 0/1 flag |
| 10 | m_3c | u32 | +0x3c | 0/1 flag |
| 11 | m_40 | u32 | +0x40 | 0/1 flag |
| 12 | m_70 | u32 | +0x70 | mostly 100 (percent?) |
| 13 | m_74 | u32 | +0x74 |  |
| 14 | m_78 | u32 | +0x78 |  |
| 15 | s_44 | CString | +0x44 | monster name ("Ogre", "Orc, Leader") |
| 16 | s_48 | CString | +0x48 | 2nd monster name or empty ("Orc assistant") |
| 17 | s_4c | CString | +0x4c | 3rd name or empty ("Kuo-Toa") |

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 18 | montype_18 | embedded CMonsterTypeOb | +0x18 | usually all empty; else e.g. "Orc" |
| 19 | oa_4 | CObArray<CMonsterOb> | +0x4 | full monster copies (2207 in EncPlat.dat) |

m_50..m_5c are the number appearing N d S * M + B, m_60..m_6c the second box; m_38, m_3c,
m_40 are dependent / "and" / "or"; m_70 percent, m_74 "for every", m_78 "beyond"; s_4c and
montype_18 name what a dependent line counts. The generator is described in
[`encounter.rs`](../../crates/cr-store/src/encounter.rs).

### CEncTemplateNPCPropertiesOb (schema 88, size 0x104, Serialize 0x65295d)

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | m_d4 | u32 | +0xd4 |  |
| 2 | m_d8 | u32 | +0xd8 |  |
| 3 | m_dc | u32 | +0xdc |  |
| 4 | m_e0 | u32 | +0xe0 |  |
| 5 | m_e4 | u32 | +0xe4 |  |
| 6 | m_e8 | u32 | +0xe8 |  |
| 7 | m_ec | u32 | +0xec |  |
| 8 | m_f0 | u32 | +0xf0 |  |
| 9 | m_c8 | u32 | +0xc8 | 0/1 flag |
| 10 | m_cc | u32 | +0xcc | 0/1 flag |
| 11 | m_d0 | u32 | +0xd0 | 0/1 flag |
| 12 | m_f4 | u32 | +0xf4 | mostly 100 |
| 13 | m_f8 | u32 | +0xf8 |  |
| 14 | m_fc | u32 | +0xfc |  |
| 15 | s_100 | CString | +0x100 | race filter ("Elf", "Dwarf, Hill") or empty |

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 16 | montype_a8 | embedded CMonsterTypeOb | +0xa8 | ("Halfling", "Gnome") |
| 17 | npc_18 | embedded CEncounterNPCOb | +0x18 | first NPC description |
| 18 | npc_60 | embedded CEncounterNPCOb | +0x60 | second NPC description (often empty strings, levels -1) |
| 19 | oa_4 | CObArray<CEncounterNPCOb> | +0x4 | 571 in EncPlat.dat |

### CEncTemplateItemPropertiesOb (schema 88, size 0xe18, Serialize 0x650cde)

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | m_da0 | u32 | +0xda0 |  |
| 2 | m_db0 | u32 | +0xdb0 |  |
| 3 | m_db4 | u32 | +0xdb4 |  |
| 4 | m_db8 | u32 | +0xdb8 |  |
| 5 | m_dbc | u32 | +0xdbc |  |
| 6 | m_dc0 | u32 | +0xdc0 |  |
| 7 | m_dc4 | u32 | +0xdc4 |  |
| 8 | m_dc8 | u32 | +0xdc8 |  |
| 9 | m_dcc | u32 | +0xdcc |  |
| 10 | m_dd0 | u32 | +0xdd0 |  |
| 11 | m_da4 | u32 | +0xda4 |  |
| 12 | m_da8 | u32 | +0xda8 |  |
| 13 | m_dac | u32 | +0xdac |  |

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 14 | monster_898 | embedded CMonsterOb | +0x898 | (the full CMonsterOb layout above) |
| 15 | montype_d80 | embedded CMonsterTypeOb | +0xd80 | |
| 16 | part_4 | embedded CPart | +0x4 | the item ("Catapult, Heavy", "Cart"); CPart is versioned, see [parts.md](parts.md) |
| 17 | sa_dd4 | CStringArray | +0xdd4 | monster names ("Orc, Leader", "Orc assistant") |
| 18 | oa_de8 | CObArray<CEncTemplateNumAppearOb> | +0xde8 | 3 in data |
| 19 | oa_dfc | CObArray<CEncTemplateNumAppearOb> | +0xdfc | 1 in data |

### CEncTemplateNumAppearOb (schema 88, size 0x4fc, Serialize 0x6517d3)

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | m_4ec | u32 | +0x4ec | (1, 5, 10) |
| 2 | m_4f0 | u32 | +0x4f0 | (1, 6) |
| 3 | m_4f4 | u32 | +0x4f4 | (1) |
| 4 | m_4f8 | u32 | +0x4f8 | (0, 9) |
| 5 | monster_4 | embedded CMonsterOb | +0x4 | |

### CEncounterNPCOb (schema 95, size 0x48, Serialize 0x6531e0)

| # | field | type | offset | meaning / evidence |
|---|---|---|---|---|
| 1 | s_4 | CString | +0x4 | NPC name ("Bob", "Norm") |
| 2 | s_44 | CString | +0x44 | race ("Human", "Elf") |
| 3 | s_8 | CString | +0x8 | class 1 ("Fighter") |
| 4 | s_c | CString | +0xc | class 2 or empty |
| 5 | s_10 | CString | +0x10 | class 3 or empty |
| 6 | s_14 | CString | +0x14 | class 4? (empty) |
| 7 | s_18 | CString | +0x18 | class 5? (empty) |
| 8 | m_1c | u32 | +0x1c | level in class 1 |
| 9 | m_20 | u32 | +0x20 | level in class 2 (0xffffffff = none) |
| 10 | m_24 | u32 | +0x24 | level in class 3 (-1 = none) |
| 11 | m_28 | u32 | +0x28 | always -1 |
| 12 | m_2c | u32 | +0x2c | always -1 |
| 13 | oa_30 | CObArray<CPartIDOb> | +0x30 | empty in all data; element type from the copy code at 0x653472 (new + ctor 0x65889e, vtable 0x7ae87c = CPartIDOb; see [parts.md](parts.md)) |

## Encounters (Encount.dat, EncountU.dat)

### CEncounterOb (schema 88, size 0x194, Serialize 0x63c659)

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | s_6c | CString | +0x6c | encounter name ("Wandering Bugbear", "Huge Orc City") |
| 2 | m_7c | u32 | +0x7c | 0 in data |
| 3 | s_70 | CString | +0x70 | monster type ("Bugbear", "Orc") |
| 4 | s_74 | CString | +0x74 | source template name ("Orc, Lair (Huge city)") |
| 5 | s_78 | CString | +0x78 | |
| 6 | oa_80 | CObArray<CEncounterMonsterPackageOb> | +0x80 | monsters (copy code 0x63c8b0: ctor 0x6403b1) |
| 7 | oa_94 | CObArray<CEncounterNPCPackageOb> | +0x94 | NPCs; empty in data; element type from copy code 0x63ca01 (ctor 0x63b961) |
| 8 | oa_180 | CObArray<CTrapOb> | +0x180 | traps; empty in data; element type from copy code 0x63cb52 (ctor 0x53ca2e) |
| 9 | inv_a8 | embedded CInventoryBundle | +0xa8 | treasure (copy code error "Treasure was lost.") |
| 10 | inv_114 | embedded CInventoryBundle | +0x114 | items ("Items were lost.") |
| 11 | t_4 | u32[26] | +0x4 | |

### CEncounterMonsterPackageOb (schema 88, size 0x504, Serialize 0x6405b8)

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | m_4ec | u32 | +0x4ec | (1, 5, 544, 1081...) |
| 2 | monster_4 | embedded CMonsterOb | +0x4 | |
| 3 | oa_4f0 | CObArray<CEncounterMonHpOb> | +0x4f0 | one entry per distinct hit point value, in first-occurrence order |

### CEncounterMonHpOb (schema 88, size 0xc, Serialize 0x650612)

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | m_4 | u32 | +0x4 | hit points |
| 2 | m_8 | u32 | +0x8 | how many monsters of the package have them |

### CEncounterNPCPackageOb (schema 88, size 0x21a4, Serialize 0x63bc0f) - not in shipped data

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | m_218c | u32 | +0x218c | |
| 2 | character_4 | embedded CCharacterDoc | +0x4 | vtable 0x79b428, Serialize 0x41cd5b: the same stream as a Chars/<id>.dat file (see [`character.rs`](../../crates/cr-formats/src/character.rs)) |
| 3 | oa_2190 | CObArray<CPartIDOb> | +0x2190 | one per individual, hit points in the part type field |

The embedded character is read with the same archive reader as the surrounding file, so
class tags inside the character continue the archive's tag numbering.

## Frequency tables (EncTabU.dat)

### CEncounterFreqTableOb (schema 1, size 0x28c, Serialize 0x63ff92) - not in shipped data

| # | field | type | offset | meaning |
|---|---|---|---|---|
| 1 | s_274 | CString | +0x274 | |
| 2 | sa_278 | CStringArray | +0x278 | |
| 3 | table_4 | embedded anonymous object (vtable 0x7adf98, size 0x270, Serialize 0x64fe45) | +0x4 | |
| 3a | m_260 | u32 | +0x260 | ctor: 0 |
| 3b | m_264 | u32 | +0x264 | ctor: 100 |
| 3c | m_268 | u32 | +0x268 | ctor: 0 |
| 3d | s_25c | CString | +0x25c | |
| 3e | m_26c | u32 | +0x26c | ctor: 0 |
| 3f | t_4 | u32[150] | +0x4 | ctor sets all to 1 |
