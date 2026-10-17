# Parts (Parts.dat, PartsU.dat) and the files around them

Rust: [`parts.rs`](../../crates/cr-formats/src/parts.rs) (`PartsFile`, `Part`, `PartKit`,
`PartId`, `PartKitLevel`, `NumList`, `ListOfListsOfOptions`). The spells files are read by
[`spells.rs`](../../crates/cr-formats/src/spells.rs) (see [spells.md](spells.md)), the
allowed-classes file by [`allowed.rs`](../../crates/cr-formats/src/allowed.rs) (see
[classes.md](classes.md#allowed-classes-per-race)). Meanings come from the part editor
([`userdb/parts.rs`](../../crates/cr-store/src/userdb/parts.rs),
[`parts/pages.rs`](../../crates/cr-store/src/userdb/parts/pages.rs),
[`parts/kit.rs`](../../crates/cr-store/src/userdb/parts/kit.rs)), the character sheet, the
character creator and the NPC generator of `cr-store`. The base encoding (CString, counts,
class tags, `ReadObject`) is in [archive.md](archive.md).

Document class `CPartDoc`, Serialize 0x67b10d (TSRP3.EXE). The Expansion writes every part
with the `TSRP3 V7` version block and every kit as version 30; Core Rules 2.0 writes parts
without the block (version 0, so none of the fields the block switches on, and no kits).

Type notes: `i32` is a 4-byte load (all three 4-byte helpers 0x40d250 / 0x40d1b0 / 0x447560);
`bool` is a byte (0x40d1f0) the program stores as `!= 0` (the Rust keeps the raw byte); `u8`
rows are raw bytes. `f32 bits` fields are written as 4-byte integers but hold IEEE floats.
Field names are the Rust ones; `unknown_0x30c` is the member at `this + 0x30c`, and an array
`T[N]` covers N consecutive members (`unknown_0x30c` `i32[19]` is +0x30c to +0x354), so the
meaning column names elements by their member offset. The editor pages and labels quoted are
the original dialogs' (IDD numbers in `parts/pages.rs` and `parts/kit.rs`).

## File framing

| file | document class / Serialize | framing | parses to EOF |
|---|---|---|---|
| Database/Parts.dat | CPartDoc 0x67b10d | **21 consecutive CObArrays** of `CPart`, no outer count | yes |
| UserDbas/PartsU.dat | CPartDoc 0x67b10d | same: 21 empty arrays = 21 x u16 0 = 42 bytes | yes |
| Database/Spells.dat, UserDbas/SpellsU.dat | CSpellsDoc 0x4f9d89 | two CObArrays (wizard, priest); see [spells.md](spells.md) | yes |
| UserDbas/AllowedU.dat | CClassesAllowedDoc 0x680e21 | one CObArray of CClassesAllowedOb; see [classes.md](classes.md#allowed-classes-per-race) | yes |
| UserDbas/ClassU.dat | CClassDoc 0x68416a | one CObArray (+0x54), same as Class.dat; shipped file is count 0; see [classes.md](classes.md) | yes |
| UserDbas/RaceU.dat | CRaceDoc 0x6849fa | one CObArray (+0x54), same as Race.dat; shipped file is count 0; see [race.md](race.md) | yes |

All doc Serialize functions call DeleteContents first when loading and then each array's own
`Serialize` (`call [vtbl+8]`), i.e. `count + objects`. The class-tag table is shared across all
arrays of one file (one archive), so a later array may refer back to a class first seen in an
earlier one ([archive.md](archive.md)).

### Parts.dat array index = part type

CPartDoc +0x54 is a CObArray of 21 CObArrays created in DeleteContents (0x677978, 21 calls to
`Add`). The same function fills a name -> type table (+0x324, CPartIDOb-like) whose indices match
the data (`PART_TYPES` in `parts.rs`):

| idx | type (string in 0x677978) | records in Parts.dat |
|---|---|---|
| 0 | Weapons | 733 |
| 1 | Armor | 573 |
| 2 | "NOT" (unused) | 0 |
| 3 | Weapon Proficiencies | 0 (the program makes them from the weapons, see `unknown_0x420`) |
| 4 | Non-Weapon Proficiencies | 122 |
| 5 | Racial Abilities | 167 |
| 6 | Languages | 61 |
| 7 | Equipment | 306 |
| 8 | Magical Items | 689 |
| 9 | Schools Of Magic | 22 |
| 10 | "NOT" (unused) | 0 |
| 11 | Traits | 28 |
| 12 | Disadvantages | 38 |
| 13 | Gems and Art | 1383 |
| 14 | Class Abilities | 174 (the Expansion only; 2.0's editor does not offer the type) |
| 15 | Character Kits | 210 (each carries a CPartKitOb) |
| 16 | Currency Types | 5 |
| 17 | Count (units: "Charge", "Dose", ...) | 45 |
| 18 | Spheres | 28 |
| 19, 20 | (no name) | 0 |

All 4584 CParts are version 7 (`"TSRP3 V7"`); all 210 CPartKitObs are version 30. A part name
must be unique within its type (the editor's "Name already used in catagory ...").

## CPart (schema 93, size 0x894, Serialize 0x65c3a0, load at 0x65dd44)

`Part` = `PartHead`, then the version word, then `PartData`. After the head an i32 is read: if
it is < -8888 (the writer stores -8889, `0xffffdd47`) a version CString and then `unknown_0x728`
follow; otherwise that i32 **is** `unknown_0x728` and the record is version 0. The version a
string announces (`version_of`): 7 when it is exactly `"TSRP3 V7"`, else `atol` of the text after
the first 7 characters when it is longer than 7, else 6. A version 0 record cannot hold a value
below -8888 in `unknown_0x728` (it would read back as a marker; the writer refuses it).

A new part (`new_part`) has `unknown_0x30c[0]` 1, `unknown_0x2a8` -1, AC (+0x270) 100,
`unknown_0x278` -1.0, the container limits -1, `unknown_0x420` [0, 1], `unknown_0x42c` 1,
`unknown_0x79c` 1, `unknown_0x7cc[0]` 14; a non-kit part has `unknown_0x660` 0x80000000 and
`unknown_0x65c` 100, a kit `unknown_0x5e4` and `unknown_0x644` all -1.

### PartHead (before the version word)

| # | field | type | present | meaning |
|---|---|---|---|---|
| 1 | `name` (+0x198) | CString | | name ("Belaying pin"); at most 50 characters in the editor |
| 2 | `unknown_0x300` | CString | | unknown |
| 3 | `unknown_0x308` | i32 | | character point cost: "Cost" (Traits, Spheres), "CP Cost" (Racial), "Character Point Cost" (Class Abilities); 0 makes the creator read a cost from the name |
| 4 | `unknown_0x230` | i32 | | "XP Value" (enabled for magical parts) |
| 5 | `description` (+0x304) | CString | | rules text (Description page) |
| 6 | `unknown_0x30c` | i32[19] | | +0x30c "Effects on Ability Scores": 0 set to the amount, 1 adjust by it (new parts 1); +0x310..+0x33c the 12 sub-abilities Stamina, Muscle, Aim, Balance, Health, Fitness, Reason, Knowledge, Intuition, Willpower, Leadership, Appearance (-25..25); +0x340..+0x354 unknown |
| 7 | `unknown_0x394` | i32[3] | | unknown |
| 8 | `unknown_0x2ac` | i32 | | container bulk "Same as character's strength" |
| 9 | `unknown_0x80c` | i32[4] | | "Functions As": +0x80c Ring of Clumsiness, +0x810 Holy Avenger, +0x814 Girdle of Dwarvenkind (sheet: Constitution save bonus for a non-dwarf), +0x818 Gauntlets of Dexterity |
| 10 | `unknown_0x2a8` | i32 | | container "Bulk Points"; -1 (default) not limited by bulk |
| 11 | `unknown_0x4cc` | i32 | | Restrict Class special case "Psionicist (10 CP Ability)" |
| 12 | `unknown_0x470` | i32[4] | | nonweapon proficiency: +0x470 "Additional Languages", +0x474 "Proficiency Bonus", +0x478 "Cost in character points" (S&P), +0x47c "Cost in slots" (Core Rules) |
| 13 | `unknown_0x240` | CString | | "Equipment Group" ("Misc", "Clothing", "Animal") |
| 14 | `unknown_0x460` | CString | | NWP "Ability Score Base" (Core Rules, "Intelligence") |
| 15 | `unknown_0x45c` | CString | | NWP "Second Relevant Ability Score" (S&P, "Wisdom/Intuition") |
| 16 | `unknown_0x244` | CString | | "Demihuman Equipment" group ("Elven Equipment") |
| 17 | `unknown_0x1b4` | CString | | school "Scheme for this School" ("School of Effects") |
| 18 | `unknown_0x464` | CString | | NWP rating bonus: "Proficiency" whose holder gets `unknown_0x470[1]` |
| 19 | `unknown_0x1e4` | i32 | | unknown |
| 20 | `unknown_0x1e8` | CString | | unknown |
| 21 | `unknown_0x1ec` | f32 bits | | weight in pounds, Skills & Powers (0x3dcccccd = 0.1 most common) |
| 22 | `unknown_0x1f0` | f32 bits | | weight in pounds, Player's Handbook; shown as the S&P weight while not above 0 |
| 23 | `unknown_0x1f8` | i32[8] | | cost: +0x1f8..+0x204 platinum, gold, silver, copper (Skills & Powers), +0x208..+0x214 the same (Player's Handbook; the S&P values stand in while all four are 0) |
| 24 | `unknown_0x224` | i32 | | "Magic"; magical items are forced to 1, and the shops do not sell magical parts |
| 25 | `unknown_0x234` | CString | | a number as text ("209"); unknown |
| 26 | `unknown_0x238` | i32[2] | | unknown |
| 27 | `unknown_0x24c` | i32 | | "Number of hands needed" (0..2) |
| 28 | `unknown_0x248` | i32 | | "Armor Type", an id 0..4 into the program's armor-type list (1 body armor, 3 shield) |
| 29 | `unknown_0x250` | i32[10] | | +0x250 "This is something a character could wear" (armor forced to 1); worn as: +0x254 robe, +0x258 shoes or boots, +0x25c hat or helmet, +0x260 scabbard, +0x264 quiver, +0x268 knife sheath, +0x26c gloves; +0x270 armor class, 100 = "This item has no Armor Class"; +0x274 "This can be used to carry other items" |
| 30 | `unknown_0x278` | f32 bits | | container "Stowage Capacity in pounds"; -1.0 (default) not limited by weight |
| 31 | `unknown_0x27c` | i32[11] | | container holds (-1 not limited by number): +0x27c quarrels, +0x284 knives, +0x298 arrows, +0x29c swords, +0x2a0 javelins, +0x2a4 staves; "This item is the same as": +0x280 boots or shoes, +0x288 a hat, +0x28c a transport, +0x290 a pack animal, +0x294 tack (saddle) |
| 32 | `unknown_0x2b4` | i32[9] | | +0x2b4 "This can be carried in a container"; carried in: +0x2b8 bolt case, +0x2bc boots or shoes, +0x2c0 knife sheath, +0x2c4 a hat, +0x2c8 quiver, +0x2cc scabbard, +0x2d0 javelin case, +0x2d4 staff case |
| 33 | `unknown_0x458` | CString | | NWP "Relevant Ability Score" (S&P, "Dexterity/Aim") |
| 34 | `unknown_0x3b4` | CString | | weapon type, S&P ("P", "S", "B", "P/S", ...) |
| 35 | `unknown_0x3e4` | CString | | weapon type, PHB; the S&P value stands in while empty |
| 36 | `unknown_0x3b8` | i32[4] | | +0x3b8 "Melee Weapon", +0x3bc "Missile Weapon", +0x3c0 "Needs Ammo", +0x3c4 S&P damage vs large: dice count |
| 37 | `unknown_0x3e8` | i32 | | PHB damage vs large: dice count |
| 38 | `unknown_0x3c8` | i32 | | S&P damage vs large: sides |
| 39 | `unknown_0x3ec` | i32 | | PHB damage vs large: sides |
| 40 | `unknown_0x3cc` | i32 | | S&P damage vs large: bonus |
| 41 | `unknown_0x3f0` | i32 | | PHB damage vs large: bonus |
| 42 | `unknown_0x3d0` | CString | | weapon size, S&P ("T", "S", "M", "L", "H", "G") |
| 43 | `unknown_0x3f4` | CString | | weapon size, PHB |
| 44 | `unknown_0x3d4` | i32 | | S&P damage vs small-medium: sides |
| 45 | `unknown_0x3f8` | i32 | | PHB damage vs small-medium: sides |
| 46 | `unknown_0x3d8` | i32 | | S&P damage vs small-medium: dice count |
| 47 | `unknown_0x3fc` | i32 | | PHB damage vs small-medium: dice count |
| 48 | `unknown_0x3dc` | i32 | | S&P damage vs small-medium: bonus |
| 49 | `unknown_0x400` | i32 | | PHB damage vs small-medium: bonus |
| 50 | `unknown_0x3e0` | i32 | | speed factor, S&P |
| 51 | `unknown_0x404` | i32 | | speed factor, PHB |
| 52 | `unknown_0x40c` | i32[4] | | +0x40c "Knockdown (d)", +0x410 long range, +0x414 short range, +0x418 medium range |
| 53 | `unknown_0x41c` | CString | | rate of fire, S&P ("2/1") |
| 54 | `unknown_0x408` | CString | | rate of fire, PHB |
| 55 | `unknown_0x44c` | i32[3] | | adjustments: +0x44c "AC Adjustment" (armor's magical bonus), +0x450 "To Damage Adjustment", +0x454 "To Hit Adjustment" |
| 56 | `unknown_0x2d8` | i32[10] | | saving throw adjustments, +0x2d8.. in order paralyzation, poison, death magic, rod, staff, wand, petrification, polymorph, spell, breath weapon |

PHB fields fall back to their S&P partners while empty (damage, speed, size, type, rate of
fire, the four coins, the weight); a Core Rules character uses the PHB set when it is filled in,
and the editor's OK stores the fallbacks.

### Version word

| # | field | type | present | meaning |
|---|---|---|---|---|
| 57 | `version.marker` | i32 | word < -8888 | -8889 in every shipped part; if the word is >= -8888 it **is** `unknown_0x728` and v = 0 |
| 58 | `version.string` | CString | word < -8888 | `"TSRP3 V7"`; v as described above |
| 59 | `unknown_0x728` | i32 | | Strength adjustment (Abilities page, -25..25; +0x728 is the member before the data) |

### PartData (after the version word; v = the announced version, 0 without one)

| # | field | type | present | meaning |
|---|---|---|---|---|
| 60 | `unknown_0x72c` | i32[6] | | Abilities page: +0x72c Dexterity, +0x730 Constitution, +0x734 Intelligence, +0x738 Wisdom, +0x73c Charisma, +0x740 "Extra Strength" (exceptional strength; above 0 lets a character have it) |
| 61 | `unknown_0x480` | i32[3] | | NWP: +0x480 "Initial Rating" (S&P), +0x484 "Check Modifier" (Core Rules), +0x488 "Trait Bonus" |
| 62 | `unknown_0x420` | i32[2] | | +0x420 "Ammunition"; +0x424 "Include this weapon name as a weapon proficiency" (otherwise `unknown_0x428` names it) |
| 63 | `unknown_0x42c` | i32 | | "Include this Armor name as an Armor proficiency" (default 1; otherwise `unknown_0x430`) |
| 64 | `old_class_flags` (+0x4a4) | i32[9] | | class flags of records older than v 1: Bard, Cleric, Druid, Fighter, Mage, Paladin, Ranger, (+0x4c0), Thief. When v = 0, 0x661697 appends the names of the set flags (all but +0x4c0) to `allowed_classes` and clears them. +0x4c0 is the Restrict Class page's "Any Specialist" check |
| 65 | `unknown_0x4c8` | i32 | | Restrict Class special case "Psionicist (5 CP Ability)" |
| 66 | `unknown_0x510` | i32[7] | | Adjustments conditions, one flag each: +0x510 with selected weapons (`unknown_0x4fc`), +0x514 any weapon of the user's choice, +0x518 any melee weapon; +0x51c racial "Purchases will be at a discount of (percent)"; +0x520 racial hit points per level; +0x524 "Starting Value" and +0x528 "Increase Per Level" of an ability's own percentage (racial and class abilities) |
| 67 | `unknown_0x52c` | i32 | v > 6 | class ability "Maximum" of that percentage (new parts 100) |
| 68 | `unknown_0x530` | i32[16] | | racial: +0x530 NWP "Bonus" (for `unknown_0x57c`), +0x534 saving throw "Bonus (zero uses the chart bonuses)", +0x538 experience points per level (percent), +0x53c and +0x540 the levels of the WPs `unknown_0x574[0]` and `[1]`, +0x544 "Give this Ability it's own value", then the saves it applies to: +0x548 breath, +0x54c death magic, +0x550 paralyzation, +0x554 petrification, +0x558 poison, +0x55c polymorph, +0x560 rod, +0x564 spell, +0x568 staff, +0x56c wand |
| 69 | `unknown_0x48c` | i32[6] | | NWP groups, 1 for each it belongs to: general, priest, rogue, warrior, wizard, psionicist |
| 70 | `unknown_0x584` | i32[5] | | +0x584 magic item "Category" (0 none, 1 potions and oils, 2 scrolls, 3 rings, 4 rods, 5 staves, 6 wands, 7..16 the miscellaneous magic groups); "Functions As": +0x588 Bracers of Defense, +0x58c Cloak of Protection, +0x590 Ring of Protection, +0x594 Staff of Power |
| 71 | `unknown_0x220` | i32 | | character points gained (Disadvantages, Class Abilities; e.g. 3, severe 8) |
| 72 | `unknown_0x59c` | i32[13] | | thieving skill adjustments in percent (armor -40, ...), in order pick pockets, open locks, find/remove traps, move silently, hide in shadows, detect noise, climb walls, read languages, detect magic, detect illusion, bribe, tunneling, escape bonds |
| 73 | `unknown_0x598` | i32 | | "Thieving Skills are adjusted" (enables `unknown_0x59c`) |
| 74 | `unknown_0x4` | i32[101] | v > 5 | +0x4..+0x194; all 99 except 7 records; unknown (Core Rules 2.0 parts converted by the Expansion get 0s) |
| 75 | `unknown_0x570` | i32 | | unknown |
| 76 | `unknown_0x5e4` | i32[24] | | +0x5e4..+0x640; -1 on new kits; unknown |
| 77 | `unknown_0x664` | i32[2] | | unknown |
| 78 | `unknown_0x678` | i32 | | unknown |
| 79 | `unknown_0x648` | i32[2] | | unknown |
| 80 | `unknown_0x644` | i32 | | -1 on new kits; unknown |
| 81 | `unknown_0x228` | i32 | | "This item contains metal" |
| 82 | `unknown_0x19c` | i32 | | unknown |
| 83 | `unknown_0x2b0` | i32 | | container "Do Not Add to Encumbrance" |
| 84 | `unknown_0x22c` | i32 | | "This item contains silver" |
| 85 | `unknown_0x218` | i32 | | spheres: "Minor Access" character points |
| 86 | `unknown_0x75c` | i32[2] | | +0x75c "This Nonweapon Proficiency has Languages associated with it"; +0x760 unknown |
| 87 | `flag_0x724` | bool | v > 4 | "Applies to Cavalier horseman's bonus (Melee weapons only)" |
| 88 | `unknown_0x868` | CString[10] | | +0x868..+0x88c; unknown |
| 89 | `unknown_0x468` | CString | | NWP rating bonus "Trait" (with `unknown_0x480[2]`) |
| 90 | `unknown_0x46c` | CString | | NWP group ("GENERAL", "PRIEST") |
| 91 | `unknown_0x428` | CString | | weapon "Proficiency needed" ("Sword, short"); the editor fills in the part's name when empty |
| 92 | `unknown_0x430` | CString | | "Armor Proficiency" (when `unknown_0x42c` is 0) |
| 93 | `unknown_0x574` | CString[3] | | racial: +0x574 and +0x578 weapon proficiencies granted at the levels in `unknown_0x530`, +0x57c the NWP whose rating gets the racial bonus |
| 94 | `unknown_0x580` | CString | | racial ability's race |
| 95 | `unknown_0x21c` | CString | | trait: the race that saves one character point; disadvantage: the race that receives one more |
| 96 | `unknown_0x1e0` | CString | | General Info "Only applies for selected race" |
| 97 | `unknown_0x1f4` | f32 bits | | General Info "Bulk Points" (shown as 0 outside 0..1e7) |
| 98 | `unknown_0x744` | i32 | | traits: "Allow Character access to Wild Talents" |
| 99 | `unknown_0x778` | i32[2] | | class abilities: +0x778 "Turn Undead starting at level (0=never)"; +0x77c the ability's value kind: 0 none, 1 Dexterity plus level, 2 and 7 a ranger hide-in-shadows table, 3 and 8 a ranger move-silently table, 4 the chance to use scrolls, 5 base and increment (`unknown_0x510[5..7]`, `unknown_0x52c`), 6 a rogue skill |
| 100 | `flags_0x780` | bool[6] | | class abilities: +0x780 "Defense Bonus", +0x781 "Increase hit dice", +0x782 "Specialization", +0x783 "Multiple Specialization", +0x784 "Access to improved spell tables", +0x785 "Psychic Adept" |
| 101 | `unknown_0x7a0` | i32 | | class abilities: "Cleric Weapon Allowance" |
| 102 | `unknown_0x66c` | i32[3] | | unknown |
| 103 | `unknown_0x7b8` | i32 | | weapon "Do not apply low STR Penalty" |
| 104 | `unknown_0x7a4` | i32[4] | | class abilities: +0x7a4 "Specialist Mage Learning Penalty", +0x7a8 "Learning Bonus", +0x7ac "Bonus Spells", +0x7b0 "Gains benefits of high Con as a warrior" |
| 105 | `unknown_0x7bc` | i32 | | class abilities: "PSP Bonus" |
| 106 | `unknown_0x71c` | i32 | | class abilities: "Apply Ranger Bonus" |
| 107 | `unknown_0x78c` | i32 | | class abilities: "Mental Defense (10)" (2.0 new parts 1) |
| 108 | `unknown_0x788` | i32 | | "Contact (10)" (2.0 new parts 1) |
| 109 | `unknown_0x794` | i32 | | "Contact (5)" (2.0 new parts 1) |
| 110 | `unknown_0x79c` | i32 | | NWP "MAC Bonus" (new parts 1) |
| 111 | `unknown_0x790` | i32 | | "Mental Defense (5)" (2.0 new parts 1) |
| 112 | `unknown_0x7cc` | i32[7] | | +0x7cc weapon "Max Damage Bonus from STR" (new parts 14); +0x7d0 unknown; Shield Type, one flag each: +0x7d4 buckler, +0x7d8 small, +0x7dc medium, +0x7e0 body; +0x7e4 "Pages for spells" (an item with pages gets an inventory spell record, see [misc.md](misc.md#inventoryextra)) |
| 113 | `unknown_0x7b4` | i32 | | class abilities: "Allowed Exceptional Strength" |
| 114 | `unknown_0x4f0` | i32 | | unknown |
| 115 | `unknown_0x4e4` | i32 | | unknown |
| 116 | `unknown_0x7e8` | i32[9] | | movement: +0x7e8 land base rate, +0x7ec "Changes Land Movement Rate"; "Functions As" +0x7f0 Boots of Speed, +0x7f4 Bracers of Archery; +0x7f8 swimming rate, +0x7fc flying rate, +0x800 and +0x804 their "Changes" checks; +0x808 "This item sets AC; No bonuses apply" |
| 117 | `unknown_0x7c0` | i32[3] | | class abilities: "Melee Weapons Only", "Cleric Weapons Only", "Thief Weapons Only" |
| 118 | `unknown_0x81c` | i32[4] | | class abilities: "Weapon Use Lesser", "Weapon Use Major", "Armor Use Lesser", "Armor Use Major" |
| 119 | `unknown_0x798` | i32 | | NWP "Allow Psionic Attacks" |
| 120 | `unknown_0x710` | CString[2] | | unknown |
| 121 | `unknown_0x650` | i32 | | unknown |
| 122 | `flag_0x67e` | bool | | unknown |
| 123 | `flags_0x680` | bool[2] | | unknown |
| 124 | `unknown_0x654` | i32 | | unknown |
| 125 | `flag_0x67f` | bool | | unknown |
| 126 | `unknown_0x718` | CString | | class abilities: "Free NWP" ("Tracking") |
| 127 | `unknown_0x660` | i32 | | 0x80000000 on all non-kit parts, 0 on kits |
| 128 | `flag_0x67d` | bool | | unknown |
| 129 | `unknown_0x720` | CString | | unknown |
| 130 | `unknown_0x65c` | i32 | | 100 on non-kits, 0 on kits |
| 131 | `flag_0x67c` | bool | | unknown |
| 132 | `unknown_0x658` | i32 | | unknown |
| 133 | `flag_0x4d0` | bool | v > 1 | "Wizard must pay 3 extra CPs to be proficient with this weapon" |
| 134 | `flag_0x448` | bool | v > 2 | weapon "Thief Weapon" |
| 135 | `kit` (+0x890) | object CPartKitOb | v > 3 | ReadObject(RUNTIME_CLASS(CPartKitOb)) (0x66ae55); the null tag (u16 0) on every non-kit part |
| 136 | `unknown_0x6e8` | CStringArray | | unknown |
| 137 | `unknown_0x358` | CStringArray | | unknown |
| 138 | `unknown_0x36c` | CStringArray | | unknown |
| 139 | `unknown_0x380` | CStringArray | | unknown |
| 140 | `unknown_0x3a0` | CStringArray | | unknown |
| 141 | `unknown_0x1a0` | CStringArray | | "Schools of Opposition to the current School" (Schools of Magic) |
| 142 | `unknown_0x1b8` | CStringArray | | "Settings" (historical eras) |
| 143 | `unknown_0x1cc` | CStringArray | | "Weapon Groups to which this item belongs" ("Swords: Large") |
| 144 | `unknown_0x840` | CStringArray | | unknown |
| 145 | `unknown_0x854` | CStringArray | | unknown |
| 146 | `unknown_0x434` | CStringArray | | "Weapons which can fire the current Ammo" |
| 147 | `unknown_0x4fc` | CStringArray | | "Weapons which recieve the bonus" of `unknown_0x44c` |
| 148 | `unknown_0x5d0` | CStringArray | | unknown |
| 149 | `unknown_0x684` | CStringArray | | unknown |
| 150 | `unknown_0x698` | CStringArray | | unknown |
| 151 | `unknown_0x6c0` | CStringArray | | unknown |
| 152 | `unknown_0x6ac` | CStringArray | | unknown |
| 153 | `unknown_0x6d4` | CStringArray | | unknown |
| 154 | `unknown_0x6fc` | CStringArray | | unknown |
| 155 | `unknown_0x764` | CStringArray | | class ability names; unknown use |
| 156 | `unknown_0x748` | CObArray<CSpellsOb> | | always empty in data; element class from 0x65ff46 (new 0x74 + CSpellsOb ctor 0x4ff04e) |
| 157 | `allowed_classes` (+0x82c) | CStringArray | v > 0 | "Classes which CANNOT use the current selection" ("Cleric", "Druid", "Mage"): the creator and the NPC generator treat the names as barred. The Rust name is historical |

## CPartKitOb (schema 1, size 0x678, Serialize 0x66ce22, load at 0x66e1a8)

Hung off `CPart::kit` (only for parts of type 15, Character Kits), Rust `PartKit` /
`PartKitData`. Self-versioned: a header CString then a u32 version (current 30). If the header
is not `"CPKO"` the program reports "Attempting to load kit information, found header %s
expecting header %s" and returns without reading further (`data` is `None`); a version > 30 only
produces a warning. The version is compared as a signed number. Conditions below use v.

A new kit (`new_kit`) has every gender and alignment allowed, `flag_0x4b4` 1, the six ability
maxima 25, `unknown_0x88` [0, 0, 100, 0, 0x80000000], `flags_0x73` [1, 1], `unknown_0x120`
[-1, -1], `unknown_0x18c` [101, 10, 0, -10, 0], `unknown_0x1f4` [10, 0], `unknown_0x200`
[50, 50], `unknown_0x318` -1, `unknown_0x450` [99, -1, -1, -1, -1], `unknown_0x51c` 101,
`unknown_0x564` 205, `unknown_0x5e0` [-1, -1, 0, ...].

| # | field | type | present | meaning |
|---|---|---|---|---|
| 1 | `header` | CString | | `"CPKO"` |
| 2 | `version` | u32 | | 30 in all data |
| 3 | `flags_0x4` | bool[11] | | genders: +0x4 male, +0x5 female; alignments +0x6..+0xe: LG, LN, LE, NG, TN, NE, CG, CN, CE |
| 4 | `flags_0x71` | bool[2] | | +0x71 "Acquire one weapon at no cost"; +0x72 "Allow one free non-weapon proficiency" (the editor sets +0x78 with it) |
| 5 | `flag_0x444` | bool | | "Consider all class allowed weapons to be recommended" |
| 6 | `flag_0x4b4` | bool | v > 29 | "May select Unarmed Combat (even if normally not allowed by class)" |
| 7 | `flags_0x75` | bool[8] | | +0x75 "No metal armor or weapons at start", +0x76 "Limited to recommended and required WPs at start", +0x77 "Apply Acrobatic skill penalties for heavy armor", +0x78 (see +0x72), +0x79 "No fired WPs at start", +0x7a "+1 THAC0 to chosen melee weapon", +0x7b "+1 THAC0 and damage to chosen missile weapon", +0x7c "Light armor -2 AC Bonus" |
| 8 | `flag_0x84` | bool | | "Assign maximum starting funds allowed for class" |
| 9 | `unknown_0x4c` | u8[37] | | raw bytes: minimum and maximum of each score, +0x4c/+0x4d Strength, then Stamina, Muscle, Dexterity, Aim, Balance, Constitution, Health, Fitness, Intelligence, Reason, Knowledge, Wisdom, Intuition, Willpower, Charisma, Leadership, Appearance (to +0x6e/+0x6f); +0x70 minimum score in the class' prime requisite |
| 10 | `unknown_0x80` | i32 | | "Add to all damage" |
| 11 | `old_id_type` | i32 (local) | v < 3 | part type of `old_id_names` |
| 12 | `unknown_0x88` | i32[5] | | +0x88 "Percent added to normal starting funds", +0x8c "Percent of class starting funds assigned", +0x90 "Percent of actual price paid for equipment", +0x94 "Player picks this many NWPs for +1 score", +0x98 "Heaviest starting armor" (0x80000000 any, 10 none, else the AC) |
| 13 | `unknown_0x9c` | CString[3] | | +0x9c "Display on Selection", +0xa0 "Display on Equip" (long texts), +0xa4 "Free NWP (selected from the recommended list)" |
| 14 | `unknown_0x670` | i32 | v > 19 | "Bonus to Level Limits" |
| 15 | `unknown_0x674` | CString | v > 19 | the class of that bonus ("Fighter") |
| 16 | `old_id_names` | CStringArray (local) | v < 3 | converted to `CPartIDOb(name, old_id_type)` in `unknown_0x244` |
| 17 | `unknown_0x10` | CStringArray | | "No races except these" ("Deep dwarf", "Dwarf") |
| 18 | `unknown_0x24` | CStringArray | | unknown |
| 19 | `unknown_0x38` | CStringArray | | unknown |
| 20 | `unknown_0xa8` | CStringArray | | "All races except these" (monster races "Aarakocra", ...) |
| 21 | `unknown_0x384` | CStringArray | | "Recommended WPs" ("Quarterstaff"), first copy; see field 59 |
| 22 | `unknown_0xbc` | CStringArray | | "Barred WPs during creation" |
| 23 | `unknown_0xd0` | CStringArray | | "Not recommended WPs" |
| 24 | `unknown_0x258` | CStringArray | | "Recommended NWPs" ("Disguise", "Etiquette", ...) |
| 25 | `unknown_0xe4` | CStringArray | | "Recommended Traits" ("Alertness", "Ambidexterity") |
| 26 | `unknown_0xf8` | CStringArray | | "Weapon (THAC0 Bonus)" ("Net") |
| 27 | `unknown_0x10c` | CStringArray | | social rank by 2d6 roll: 13 strings indexed by the roll ("", "", "Lower class", ...); the editor pads a table of another size to 13 |
| 28 | `unknown_0x244` | CObArray<CPartIDOb> | v >= 3 | "NWP Modifiers": NWP name and modifier |
| 29 | `flags_0x73` | bool[2] | v >= 4 | "Available to Core Rules Characters", "Available to Skills & Powers Characters" |
| 30 | `unknown_0x120` | i32[2] | v >= 5 | rogue skill points: +0x120 at start, +0x124 per level; -1 standard |
| 31 | `unknown_0x13c` | i32 | v >= 5 | "Add to one skill at level 1" |
| 32 | `unknown_0x154` | i32 | v >= 5 | "Add to one skill at levels beyond 1st" |
| 33 | `unknown_0x16c` | i32[2] | v >= 5 | "Choose abilities" (+0x16c) "for a bonus of" (+0x170) |
| 34 | `unknown_0x128` | CStringArray | v >= 5 | "Starting skill points may only be spent on these abilities" ("Thief, Move silently* (5)") |
| 35 | `unknown_0x140` | CStringArray | v >= 5 | the skills of field 31 |
| 36 | `unknown_0x158` | CStringArray | v >= 5 | the skills of field 32 |
| 37 | `unknown_0x174` | CObArray<CPartIDOb> | v >= 5 | "Modifiers to Rogue Skill Scores": skill name and modifier ("Bard, Climb walls...") |
| 38 | `flags_0x188` | bool[2] | v >= 6 | "No armor restrictions (overriding class restrictions)", "Can wear out of class armor with +1 AC penalty" |
| 39 | `unknown_0x18c` | i32[5] | v >= 6 | base armor class: +0x18c at level, +0x190 becomes, +0x194 improving by, +0x198 each level until a minimum of; +0x19c "Add to AC" |
| 40 | `unknown_0x1c8` | CStringArray | v >= 7 | "Barred class abilities" |
| 41 | `unknown_0x1dc` | CStringArray | v >= 7 | "Barred schools of magic" |
| 42 | `unknown_0x208` | CStringArray | v >= 7 | "Choose 1 free major sphere" |
| 43 | `flag_0x1f0` | bool | v >= 7 | militant wizard "Different oppositional schools" |
| 44 | `flag_0x1fc` | bool | v >= 7 | "Is limited to 5 schools" |
| 45 | `unknown_0x1f4` | i32[2] | v >= 7 | +0x1f4 "Maximum level of spell", +0x1f8 Intelligence modifier for wizard spell casting |
| 46 | `unknown_0x200` | i32[2] | v >= 7 | "Maximum number of Minor Spheres", "... Major Spheres" |
| 47 | `unknown_0x26c` | CStringArray | v >= 8 | NWPs "Barred during creation" |
| 48 | `unknown_0x280` | CStringArray | v >= 8 | "Required NWPs" |
| 49 | `unknown_0x294` | CStringArray | v >= 8 | unknown (NWP names) |
| 50 | `unknown_0x2a8` | CStringArray | v >= 8 | "Pick NWP from list for +2 bonus" |
| 51 | `unknown_0x2bc` | CStringArray | v >= 8 | NWPs with "No out of class penalty" |
| 52 | `unknown_0x2d0` | CStringArray | v >= 8 | "Can pick secondary skill from" |
| 53 | `unknown_0x2e4` | CStringArray | v >= 8 | "Required secondary skills" |
| 54 | `unknown_0x2f8` | CObArray<CPartIDOb> | v >= 8 | "Gain bonus NWPs at specific levels": NWP name and level |
| 55 | `flags_0x30c` | bool[9] | v >= 8 | +0x30c "Replace NWP advancement with 1 NWP at levels 1, 3, and 6", +0x30d "Might receive bonus NWP each level", +0x30e "May use WPs as NWPs", no out of group penalty for +0x30f warrior, +0x310 priest, +0x311 wizard, +0x312 rogue, +0x313 psionicist; +0x314 "No secondary skills" |
| 56 | `unknown_0x318` | i32 | v >= 8 | NWP slots at start in Core Rules; -1 standard |
| 57 | `unknown_0x320` | i32 | v >= 8 | "Must learn" NWP `unknown_0x31c` "by level" |
| 58 | `unknown_0x32c` | i32 | v >= 8 | "Rating bonus": the bonus |
| 59 | `unknown_0x31c` | CString | v >= 8 | "Must learn NWP" ("Riding, Land") |
| 60 | `unknown_0x324` | CString[2] | v >= 8 | "Rating bonus": "If has NWP" (+0x324) "then NWP" (+0x328) |
| 61 | `unknown_0x330` | ListOfListsOfOptions | v >= 9 | "Bonus NWPs" (choose N of each list) |
| 62 | `flag_0x348` | bool | v >= 10 | "Gains two weapon style specialist bonus" |
| 63 | `flag_0x380` | bool | v >= 10 | "Cavalier to hit bonus" |
| 64 | `flags_0x44c` | bool[3] | v >= 10 | +0x44c "Replace WP advancement with 1 WP at levels 1, 3, and 6", +0x44d "May spend NWP slots as WPs", +0x44e "Must spend at least half of NWP slots in this way" |
| 65 | `flags_0x464` | bool[2] | v >= 10 | +0x464 unknown, +0x465 "Must take one wooden WP" |
| 66 | `flags_0x4ac` | bool[8] | v >= 10 | +0x4ac "This kit chooses a weapon to focus on"; +0x4ad..+0x4af melee / missile / hurled (radio, none = any); +0x4b0 "Use fighter THAC0 with this weapon", +0x4b1 "Additional -1 THAC0 penalty for other weapons", +0x4b2 "May only use magical weapons of this type", +0x4b3 "May specialize in this weapon" |
| 67 | `flag_0x508` | bool | v >= 10 | "No ranged weapons during character generation" |
| 68 | `flag_0x518` | bool | v >= 10 | "May specialize (even if normally not allowed by class)" |
| 69 | `flags_0x520` | bool[2] | v >= 10 | must specialize in a melee (+0x520) or missile (+0x521) weapon (radio, none = any) |
| 70 | `flag_0x538` | bool | v >= 10 | "May not specialize in a melee weapon at first level" |
| 71 | `unknown_0x34c` | i32[2] | v >= 10 | +0x34c "THAC0 Mod", +0x350 THAC0 mod for the list `unknown_0x354` |
| 72 | `unknown_0x368` | i32 | v >= 10 | damage mod for the list `unknown_0x36c` |
| 73 | `unknown_0x448` | i32 | v >= 10 | "Bonus WP Slots (CPs for Skills & Powers)" |
| 74 | `unknown_0x450` | i32[5] | v >= 10 | +0x450 penalty for non-proficiency (99 standard); -1 standard for: +0x454 WP slots at start, +0x458 gain WP slots every X levels, +0x45c future WPs starting at level, +0x460 extra slots for out of class WPs |
| 75 | `unknown_0x468` | i32[2] | v >= 10 | "Bonus WP slots" (+0x468) "at level" (+0x46c) |
| 76 | `unknown_0x50c` | i32[3] | v >= 10 | required at start: +0x50c slots of missile WPs, +0x510 different missile WPs, +0x514 different melee WPs |
| 77 | `unknown_0x51c` | i32 | v >= 10 | "Must specialize by level" (101 = never) |
| 78 | `unknown_0x564` | i32 | v > 23 | free specialization "Number to pick" from `unknown_0x550` |
| 79 | `unknown_0x470` | CObArray<CPartIDOb> | v >= 10 | "Bonus WPs at given levels": weapon name and level ("Long bow") |
| 80 | `unknown_0x354` | CStringArray | v >= 10 | "Weapons with the THAC0 modifier" |
| 81 | `unknown_0x36c` | CStringArray | v >= 10 | "Weapons with the damage modifier" |
| 82 | `unknown_0x384_again` | CStringArray | v >= 10 | `unknown_0x384` serialized a second time; the second read wins, and the editor writes both |
| 83 | `unknown_0x484` | CStringArray | v >= 10 | "These WPs are never out of class" |
| 84 | `unknown_0x498` | CStringArray | v >= 10 | unknown |
| 85 | `unknown_0x4b8` | CStringArray | v >= 10 | "During generation, limited to WPs from list" |
| 86 | `unknown_0x4cc` | CStringArray | v >= 10 | unknown |
| 87 | `unknown_0x4e0` | CStringArray | v >= 10 | "Must always take WPs from list" |
| 88 | `unknown_0x4f4` | CStringArray | v >= 10 | "Half of all WPs must come from list" |
| 89 | `unknown_0x524` | CStringArray | v >= 10 | "May only specialize in WPs from" |
| 90 | `unknown_0x53c` | CStringArray | v >= 10 | "Free to specialize in all of these" |
| 91 | `unknown_0x550` | CStringArray | v > 23 | "Allowed to pick as many as indicated below" |
| 92 | `unknown_0x568` | ListOfListsOfOptions | v >= 11 | "Required WPs" (choose N of each list) |
| 93 | `unknown_0x1a0` | CStringArray | v >= 12 | "Limited to armor from" |
| 94 | `unknown_0x398` | CStringArray | v >= 12 | "First sword must be from list" |
| 95 | `unknown_0x3ac` | CStringArray | v >= 12 | WPs "Forever barred from list" |
| 96 | `unknown_0x3c0` | CStringArray | v >= 12 | "Forever barred from broad groups in list" |
| 97 | `unknown_0x3d4` | CStringArray | v >= 12 | "Forever barred from tight groups in list" |
| 98 | `unknown_0x3e8` | CStringArray | v >= 13 | "Must specialize in 1 WP from" |
| 99 | `unknown_0x3fc` | CStringArray | v >= 13 | "Always restricted to WP from list and all thrown/hurled weapons" |
| 100 | `unknown_0x410` | CStringArray | v >= 13 | "At least every second WP must come from list" |
| 101 | `unknown_0x424` | CStringArray | v >= 13 | the list of "Must fill X of first Y WPs from list" |
| 102 | `unknown_0x1b4` | CStringArray | v >= 13 | "Limited to armor from (during generation)" |
| 103 | `flags_0x438` | bool[2] | v >= 13 | +0x438 "Odd WPs (1, 3, 5 ...) must be missile or thrown", +0x439 unknown |
| 104 | `flags_0x5d4` | bool[6] | v >= 13 | +0x5d4 "Bow specialization costs -1 slots", +0x5d5 "Move Silently as Ranger", +0x5d6 "Languages cost half", +0x5d7 "Never uses spells", +0x5d8 "Negate all penalties for fighting with 2 weapons", +0x5d9 unknown |
| 105 | `unknown_0x43c` | i32[2] | v >= 13 | "Must fill" X (+0x43c) "of first" Y (+0x440) |
| 106 | `unknown_0x5e0` | i32[7] | v >= 13 | +0x5e0 "Maximum retained funds", +0x5e4 "% XP bonus for high stats", +0x5e8 "Add to my level when calculating turning undead", +0x5ec "% to XP requirements", +0x5f0 "Add % to learn spells", +0x5f4 and +0x5f8 unknown |
| 107 | `unknown_0x61c` | CString | v >= 13 | unknown |
| 108 | `unknown_0x5fc` | i32[2] | v >= 13 | "Gain 1 language for each (levels)", "starting at level" |
| 109 | `unknown_0x598` | CStringArray | v >= 13 | "Free Equipment" ("Quiver") |
| 110 | `unknown_0x608` | i32[4] | v >= 14 | starting money: +0x608 number of dice, +0x60c sides, +0x610 multiplier, +0x614 add |
| 111 | `bonus_spells` (+0x634) | CObArray<CSpellsOb> | v >= 15 | "Automatically knows spells from this list" ("Detect Magic"); see [spells.md](spells.md) |
| 112 | `flag_0x5da` | bool | v >= 16 | "Use custom spell progression" |
| 113 | `levels` (+0x648) | CObArray<CPartKitLevelOb> | v >= 16 | spells per spell level, one row per character level (the editor makes 100 rows) |
| 114 | `flag_0x580` | bool | v >= 17 | "Must purchase chosen weapon during character generation" |
| 115 | `starting_equipment` (+0x584) | CStringArray | v >= 17 | "Must purchase equipment during character generation" ("Backpack") |
| 116 | `flag_0x5dc` | bool | v >= 18 | "Maximum retained funds during character generation" (enables +0x5e0) |
| 117 | `flag_0x618` | bool | v >= 19 | "Starting money = dice * multiplier + add" |
| 118 | `unknown_0x604` | i32 | v >= 21 | "Additional HP per level" |
| 119 | `unknown_0x620` | CStringArray | v >= 22 | "Forbidden Weapons" |
| 120 | `allowed_classes` (+0x65c) | CObArray<CClassesAllowedHelperOb> | v >= 23 | "Allow only these classes": classes and multi-class combinations; see [classes.md](classes.md#allowed-classes-per-race) |
| 121 | `unknown_0x21c` | CStringArray | v >= 25 | "Limited to these spheres" ("Animal") |
| 122 | `unknown_0x230` | CStringArray | v >= 26 | "Receive free minor spheres" ("Necromantic") |
| 123 | `flags_0x5dd` | bool[2] | v >= 27 | +0x5dd "Maximum % XP bonus for high stats", +0x5de "Add % to XP requirements" |
| 124 | `flag_0x5df` | bool | v >= 28 | "Use Sharpshooter Attacks Table (Applies only to crossbows)" |
| 125 | `unknown_0x5ac` | CStringArray | v >= 29 | "Free Weapons" |
| 126 | `unknown_0x5c0` | CStringArray | v >= 29 | "Free Magic Items" |

Post-load (no read): if +0x30e (`flags_0x30c[2]`) and +0x44d (`flags_0x44c[1]`) are both set,
+0x30e is cleared; the editor keeps the two exclusive ("Character Kits do not support both NWP
slots as WP slots and WP slots as NWP slots").

## CPartIDOb (schema 94, size 12, Serialize 0x6589ff)

Rust `PartId`. A part named with its type; the kit lists of `CPartIDOb`s use the second member
for a number (a modifier or a level), and encounter NPC packages for hit points.

| # | field | type | meaning |
|---|---|---|---|
| 1 | `name` (+0x4) | CString | part name ("Tracking") |
| 2 | `part_type` (+0x8) | i32 | part type index (see the table above), or the number of the list it is in |

## CPartKitLevelOb (schema 0x80000001 VERSIONABLE, size 44, Serialize 0x66aadd)

Rust `PartKitLevel`. One character level of a kit's custom spell progression.

| # | field | type | meaning |
|---|---|---|---|
| 1 | `version` | u32 (local) | 1 in data (the editor writes 1); read but not tested |
| 2 | `unknown_0x4` | i32[10] | spells of spell levels 1 to 10 at this character level |

## CNumList (schema 1, size 28, Serialize 0x4b2a5f)

Rust `NumList`.

| # | field | type | meaning |
|---|---|---|---|
| 1 | `count` (+0x4) | i32 | how many to pick (1, 2) |
| 2 | `options` (+0x8) | CStringArray | option names |

## CListOfListsOfOptions (not a tagged object here; embedded member, vtable 0x79ea60, Serialize 0x4b290a)

Rust `ListOfListsOfOptions`: `lists` (+0x4), a CObArray (count + objects); elements are
CNumList in all data.

## CSpellsOb, CClassesAllowedOb, CClassesAllowedHelperOb

`CSpellsOb` (Spells.dat, `CPart::unknown_0x748`, `CPartKitOb::bonus_spells`) is described in
[spells.md](spells.md). `CClassesAllowedOb` and `CClassesAllowedHelperOb` (AllowedU.dat,
`CPartKitOb::allowed_classes`) are described in
[classes.md](classes.md#allowed-classes-per-race).

## Notes

* `CPart::unknown_0x748` (CObArray) is empty in every record; its element class is CSpellsOb (copy
  code at 0x65ff46 allocates 0x74 bytes and calls the CSpellsOb constructor 0x4ff04e).
* Many CPart / CPartKitOb fields are still unknown: the editor pages do not touch them and nothing
  else in the app reads them.
* The CPartKitOb "v < 3" path reads a local i32 and a local CStringArray; the reader returns them
  as `old_id_type` / `old_id_names` (the program rebuilds `unknown_0x244` from them).
* The eighth old class flag (+0x4c0) is not converted by 0x661697; it is the "Any Specialist"
  restriction of the current layout, not a Psionicist flag.
* Parts.dat array 17 is labelled "Count" by the type table; its records are units ("Charge", "Dose").
