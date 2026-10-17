# race: Race.dat, RaceU.dat and CRaceOb

Reader and writer: [`crates/cr-formats/src/race.rs`](../../crates/cr-formats/src/race.rs)
(`Race`, `MultiClass`). The Custom Races editor is
[`crates/cr-store/src/userdb/races.rs`](../../crates/cr-store/src/userdb/races.rs).
Encoding: [archive.md](archive.md).

## Files

| file | document class | content |
|---|---|---|
| `Database/Race.dat` | `CRaceDoc` (Serialize 0x6849fa) | the shipped races and subraces |
| `UserDbas/RaceU.dat` | the same | the user's races and subraces; empty = `00 00` |

The document's `Serialize` writes one collection (document member +0x54): a
count, then each `CRaceOb` with its tag. `CObArray` and `CObList` give the
same bytes ([archive.md](archive.md)). Both shipped files hold 48 records
(43261 bytes in the Expansion, 44888 in 2.0); both shipped `RaceU.dat` are
`00 00`.

`CRaceOb` has no version: Core Rules 2.0 (`EVERMORE.EXE`) and the Expansion
(`TSRP3.EXE`) write the same layout under the same schema, and the 2.0 files
read with the same code. The programs differ in which members they fill:

* Core Rules 2.0 keeps a race's classes in the race itself: the allowed
  classes (`unknown_0x1b8`), the multi-class combinations (`multi_classes`),
  the level limits (`level_limits`) and their "Unlimited" checks
  (`unknown_0x84`). 29 races of the 2.0 `Race.dat` list allowed classes and
  6 have combinations.
* The Expansion keeps them in `UserDbas/AllowedU.dat` (see
  [classes.md](classes.md#allowed-classes-per-race)). Its `Race.dat` has the
  two lists empty in every race; `level_limits` and `unknown_0x84` still hold
  the 2.0 values.

Races and subraces share one list and one name space. A subrace has
`unknown_0x184` = 1 and its parent race's name in `unknown_0x188` (19 of the
48 shipped records). The character creation code of this app tells them
apart another way, which gives the same result on the shipped data: a main
race has a maximum above 0 in `ability_limits`, a subrace has none and is
attached to the main race whose name is one of its words ("Hill dwarf").

User records get `id` 0 when copied. A new race (the editor's New) is all
zeros except `unknown_0x16c` = [10, 0] and `portrait_folder` =
`C:\Program Files\AD&D Core Rules 2.0\Bitmaps\Portrait\`; a new subrace also
has `unknown_0x184` 1, `unknown_0x188` the parent, `unknown_0xa8` 45 and
`unknown_0xac` 5.

## CRaceOb (schema 88, Serialize 0x686857)

Members in archive order. The member column gives the offset in the object
where a dialog map or the field name fixes it; "editor" names the Race editor
(IDD 142) control or page.

| # | field | type | member | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x4 | race name, up to 50 characters, unique among races and subraces |
| 2 | `unknown_0x8` | CString | +0x8 | General Description text (IDD 184, the editor's Description page) |
| 3 | `ability_limits` | (i32, i32)[6] | +0x10 .. +0x3c | minimum and maximum score per ability, Str, Dex, Con, Int, Wis, Cha (Dwarf Str 8/18); Ability Scores page |
| 4 | `ability_adjustments` | i32[6] | +0x44 .. +0x58 | ability adjustments in the same order (Dwarf: Con +1, Cha -1) |
| 5 | `level_limits` | i32[9] | | Core Rules 2.0 maximum level per class, in the slots Fighter, Ranger, Paladin, Thief, Bard, Cleric, Druid, Mage, Specialist (Level Limits dialog, IDD 186, "Enter 0 for classes which this race cannot choose", 0 to 100). The app's 2.0 class list for a race: every class with a limit above 0, every class when all nine are 0 (Human), the psionicist always, plus the half-elf bard and the specialists the handbook gives a race. The Expansion reads its limits from `AllowedU.dat` instead |
| 6 | `unknown_0xa8` | i32 | +0xa8 | Skills & Powers character points to start with (Character Points: Starting, ctl 1058; Dwarf 45, Human 10, 0 for the monster races). The NPC generator spends it less the subrace's `unknown_0xe0` |
| 7 | `unknown_0xac` | i32 | +0xac | character points a character may keep for later (Retained, ctl 1083; 5 for demihumans, 10 for humans) |
| 8 | `id` | i32 | | database identifier (Dwarf 106, Elf 107, Human 119; subraces 201 to 216; "Standard half-elf" shares 114 with "Half-elf"); 0 for user records |
| 9 | `unknown_0x244` | i32[5] | +0x244 | unknown, not used by the app. In the shipped races [0] to [3] equal the largest height, smallest weight, largest weight and smallest starting age `physical` can give, and [4] is larger than any starting age (Dwarf 53, 105, 170, 45, 450); monster races have only [0] set or all 0 |
| 10 | `unknown_0xdc` | CString | +0xdc | empty in every shipped race; unknown |
| 11 | `unknown_0xe0` | i32 | +0xe0 | character point cost of the Skill Package (subraces) |
| 12 | `description` | CString | +0x10c | the Penalties text (IDD 188); the app's catalog shows it as the race's text. The Rust name is historical: the general description is `unknown_0x8` |
| 13 | `unknown_0x188` | CString | +0x188 | a subrace's parent race; shown disabled as Parent Race; empty for a race |
| 14 | `unknown_0x2ac` | CString | +0x2ac | empty in every shipped race; unknown |
| 15 | `unknown_0x16c` | i32[2] | +0x16c, +0x170 | Special page, Combat: [0] natural armor class (10 standard; the sheet reads 0 as 10), [1] bonus hit points at first level (the NPC generator adds it at level 1, and adds the classes' `unknown_0x90` instead when it is below 1) |
| 16 | `movement` | i32 | | base movement rate (Dwarf 6, Human 12), Unencumbered Movement; a subrace's own rate (Movement page, IDD 404) replaces the race's when it is not 0 |
| 17 | `unknown_0x178` | i32[2] | +0x178, +0x17c | Swimming and Flying movement rates |
| 18 | `unknown_0x180` | CString | +0x180 | Natural Attacks (without a weapon), a dice expression; "1d2" in every shipped race |
| 19 | `unknown_0x40` | i32 | +0x40 | 0/1, "Allow Exceptional Strength" (2.0: "Allow Extra Strength"); the sheet and the NPC generator give an exceptional strength roll to a score of 18 only when it is set |
| 20 | `thief_adjustments` | i32[8] | | thieving skill adjustments: pick pockets, open locks, find/remove traps, move silently, hide in shadows, detect noise, climb walls, read languages (Dwarf: open locks +10) |
| 21 | `unknown_0x158` | i32[5] | +0x158 | further thieving skill adjustments: detect magic, detect illusion, bribe, tunneling, escape bonds (skill ids 9 to 13 of the sheet) |
| 22 | `unknown_0x18c` | i32 | +0x18c | the Races list's hidden "No Main Race" check (IDD 141, ctl 1623); 0 in every shipped race |
| 23 | `unknown_0x1e0` | i32[3] | +0x1e0, +0x1e4, +0x1e8 | [0] Core Rules Race (ctl 1787): for a subrace, the one a character of the parent race gets when no subrace is chosen (creation lists it first, the NPC generator takes it); [1] Standard Skills & Powers Race (ctl 1788, hidden): listed under "Standard Races Only"; [2] Dual-Class (ctl 2422; on the Class Info page in 2.0): the race dual-classes instead of multi-classing (only Human; read at 0x6879d3). The NPC generator dresses a race with [0] or [1] set. Main races ship [0, 1, 0], Human [1, 1, 1] |
| 24 | `unknown_0x5c` | i32 | +0x5c | 0/1, "Plus one to any score (Example: Mongrelman)" |
| 25 | `unknown_0x21c` | i32[5] | +0x21c | [0] Number of Hands (ctl 2303; 2 in the shipped races), read by the sheet's combat section; [1] to [4] 0 in every shipped race |
| 26 | `unknown_0x230` | CString[5] | +0x230 | empty in every shipped race; unknown |
| 27 | `physical` | i32[11] | +0xb0 .. +0xd8 | height, weight and starting age: male base height, female base height, height dice count, height dice sides, male base weight, female base weight, weight dice count and sides, base age, age dice count and sides (Dwarf height 43/41 + 1d10 in) |
| 28 | `unknown_0x258` | i32[10] | +0x258 | 0 in every shipped race; unknown |
| 29 | `portrait` | CString | +0x1ec | default portrait file ("DWARF1.bmp"), Race Image (ctl 1659, set with Select An Image) |
| 30 | `portrait_folder` | CString | | folder the portrait was authored in: `D:\tsr\Bitmaps\Portrait\` in every shipped race |
| 31 | `unknown_0x280` | CString[11] | +0x280 | empty in every shipped race; unknown |
| 32 | `unknown_0x2b0` | i32[9] | +0x2b0 | 0 in every shipped race; unknown |
| 33 | `unknown_0x84` | i32[9] | +0x84 | Core Rules 2.0 "Unlimited" checks of the Level Limits dialog, one 0/1 flag per dialog row: Bard, Cleric, Druid, Fighter, Specialist, Mage, Paladin, Ranger, Thief (not the slot order of `level_limits`). A checked row greys its limit, which is still saved as shown. Shipped: all nine set for Human, the Bard row for Half-elf, 0 elsewhere |
| 34 | `unknown_0x184` | i32 | +0x184 | 1 for a subrace, 0 for a race |
| 35 | `unknown_0x190` | i32[10] | +0x190 .. +0x1b4 | Alignments Allowed (IDD 190), 0/1 per alignment in the order Any, Chaotic Good, Chaotic Evil, Chaotic Neutral, Lawful Evil, Lawful Good, Lawful Neutral, Neutral Evil, Neutral Good, True Neutral; 47 shipped records have a check set (Dwarf and Human: Any) |
| 36 | `languages` | CStringArray | | languages a character of the race knows: the race's own language for a main race, the whole package of a subrace; Skills & Powers characters get them as native languages |
| 37 | `unknown_0xf8` | CStringArray | +0xf8 | empty in every shipped race; unknown |
| 38 | `abilities` | CStringArray | | racial abilities a new character starts with (Skill Package: Selected Abilities, names of Racial Abilities parts): a subrace's package, a monster race's fixed abilities |
| 39 | `unknown_0x1b8` | CStringArray | +0x1b8 | Core Rules 2.0 allowed classes, by name (Class Info: Allowed Classes); empty in the Expansion's `Race.dat`. Read by the NPC generator's class lists and by the Expansion's import of a 2.0 race (see [classes.md](classes.md#keeping-allowedudat-in-step)) |
| 40 | `multi_classes` | CObArray<[CClassMultiClassOb](#cclassmulticlassob-schema-94)> | | Core Rules 2.0 multi-class combinations (Class Info: Multi-Classes); empty in the Expansion's `Race.dat` |
| 41 | `portraits` | CStringArray | | portrait files offered for the race; for Human the male portraits (`MALE*.bmp`) |
| 42 | `unknown_0x208` | CStringArray | +0x208 | more portrait files; for Human the female portraits (`FEMALE*.bmp`); empty for subraces |
| 43 | `unknown_0x2d4` | CObArray[9] | +0x2d4 | nine object arrays, empty in every shipped race; their element class is not known, so the reader stops on a record that has an object in one |

## CClassMultiClassOb (schema 94)

One allowed multi-class combination of a Core Rules 2.0 race.

| # | field | type | meaning |
|---|---|---|---|
| 1 | `classes` | i32[5] | one class id per class group, index = group: warrior (1000s), rogue (2000s), priest (3000s), wizard (4000s), psionicist (5000); 0 for none. Fighter/Thief is [1000, 2000, 0, 0, 0] |

A combination has two or three classes from different groups; the 2.0
race editor refuses two classes of one group. Class ids are not unique
(six wizard classes share 4001), so an id names a class only up to that:
the original's lookup (0x68ad26), the NPC generator and the Expansion's
import take the last class in the class list with the id; this app's race
editor shows the first. Names are joined with `/` in group order
("Fighter/Thief").

## Notes

* The doc comment of `MultiClass` in `race.rs` gives the groups as "warrior
  1000s, wizard 2000s, priest 3000s, rogue 4000s". The code (`group_of`,
  `multiclass_of`) and the data use rogue 2000s and wizard 4000s, as above.
* The offsets of the named members `id`, `level_limits`, `movement`,
  `thief_adjustments`, `languages`, `abilities`, `multi_classes`,
  `portraits` and `portrait_folder` are not recorded. The editor's hidden
  HelpID edit (ctl 1059) writes +0xc, which may be `id`.
* `unknown_0x2d4`'s element class, and the meaning of the members marked
  unknown, are open.
