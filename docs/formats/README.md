# Core Rules 2.0 file formats

The specification of every file Core Rules for Linux reads or writes, from
the 2.0 CD (`EVERMORE.EXE`, August 1998) and the Expansion CD (`TSRP3.EXE`,
September 1999). Both programs are Win32 MFC applications with MFC linked
statically. Each document names the Rust module that reads and writes the
format; the code and these documents describe the same layouts, field by
field.

## Documents

| document | covers |
|---|---|
| [archive.md](archive.md) | the `CArchive` encoding every data file uses: integers, counts, strings, collections, object tags, schemas, version strings, how the app saves |
| [character.md](character.md) | `Chars/index.chr` and the character files `Chars/<id>.dat` |
| [race.md](race.md) | `Race.dat`, `RaceU.dat` |
| [classes.md](classes.md) | `Class.dat`, `ClassU.dat`, `AllowedU.dat` |
| [parts.md](parts.md) | `Parts.dat`, `PartsU.dat`: weapons, armor, proficiencies, kits and the other parts |
| [spells.md](spells.md) | `Spells.dat`, `SpellsU.dat` |
| [monster.md](monster.md) | `Monster.dat`, `Attack.dat`, `MonType.dat`, encounter templates, saved encounters, frequency tables, `CInventoryBundle` |
| [misc.md](misc.md) | `Treasure.dat`, `TreaBun.dat`, `Psionic.dat`, `Traps.dat`, `Phrase.DAT`, `Convert.dat`, `CInventoryItem` |
| [export.md](export.md) | the DM Toolkit packages `.xpx` (Expansion) and `.xpt` (2.0), Import and Export |
| [tables.md](tables.md) | the Table Rolls tables `Tables/*.TBL` |
| [books.md](books.md) | the rule books `Books/*.rtf` |
| [help.md](help.md) | the WinHelp files `Help/*.hlp` and their contents files `Help/*.cnt` |
| [program.md](program.md) | the program images `Program/Tsrp3.exe` and `Program/Evermore.exe`, which the app reads texts from |

## Installation layout

The app opens the program folder of an installation (the one holding
`Database`), the data package Extract Data from CD… makes from it or from the CD
(`~/.local/share/corerules/data`), or the default folder of a Wine prefix
(`drive_c/Program Files/AD&D Core Rules 2.0`). Folder and file names differ
in case between the CD, the installer and the programs' own writes
(`Database`, `DATABASE`; `index.chr`, `INDEX.CHR`), so every lookup ignores
case.

An installation is the Expansion when it has `UserDbas/AllowedU.dat`, and
Core Rules 2.0 otherwise. Core Rules 2.0 installations are opened read only:
their characters and user content are imported into an Expansion
installation and converted to the Expansion's layouts (see
[export.md](export.md) and [character.md](character.md)).

| path | content | format | read | written |
|---|---|---|---|---|
| `Database/*.dat` | rules databases | `CArchive`, one document per file (below) | yes | no |
| `UserDbas/*U.dat`, `UserDbas/Treasure.dat` | user additions, one file per type | same as the matching `Database` file | yes | yes |
| `Chars/index.chr` | character list | [character.md](character.md) | yes | yes |
| `Chars/<id>.dat` | one character each | [character.md](character.md) | yes | yes |
| `*.xpx`, `*.xpt` | DM Toolkit packages, anywhere | [export.md](export.md) | yes | `.xpx` only |
| `Tables/*.TBL` | Table Rolls tables | [tables.md](tables.md) | yes | no |
| `Books/*.rtf` | rule books | [books.md](books.md) | yes | no |
| `Program/Tsrp3.exe`, `Program/Evermore.exe` | the programs | [program.md](program.md) | yes | no |
| `Bitmaps/Portrait/*.bmp` | portraits | Windows BMP | named by races and characters | no |
| `Maps/index.crm`, `Maps/*.dat` | Map Maker II maps | `CArchive` (`CMapMiniOb`, `CMapData`); not decoded | no | no |
| `MapMaker/Moi*.dat`, `Mapmaker.ini`, `*.pal`, `*.mtf` | Map Maker II object sets and settings | not decoded | no | no |
| `*.FCW` | Campaign Mapper drawings | carried inside packages as bytes; not decoded | no | inside packages |
| `UserDbas/enctable.dat` | 0 bytes in both programs; the name does not occur in `Tsrp3.exe` | - | no | no |
| `Help/*.hlp`, `Help/*.cnt` | WinHelp help: the Online Manual, the program help, the books' help versions | [help.md](help.md) | yes | no |
| `WebHelp/**` | HTML help (the books' HTML versions) | HTML | no | no |
| `Sounds`, `Animate`, `Wizard`, `Or`, `Userbmp`, other `Bitmaps` | program artwork and sounds (BMP, GIF, WAV) | standard formats | no | no |

Map Maker II, the Campaign Mapper and the dice roller are outside the app's
scope, so their files are listed but not specified.

## Rules databases

Every `Database` file has a `UserDbas` counterpart with the same layout
(`Treasure.dat` keeps its name in both folders). A user file may be empty
(0 bytes) until the program first writes it; Core Rules 2.0 has no
`AllowedU.dat` and no `ClassU.dat`.

| file | document class, Serialize | framing | records (Expansion) | specified in | Rust |
|---|---|---|---|---|---|
| `Race.dat` | `CRaceDoc` 0x6849fa | one `CObList` of `CRaceOb` (88) | 48 | [race.md](race.md) | [`race.rs`](../../crates/cr-formats/src/race.rs) |
| `Class.dat` | `CClassDoc` 0x68416a | one `CObList` of `CClassOb` (88), each with its own version string | 26 (21 in 2.0) | [classes.md](classes.md) | [`classes.rs`](../../crates/cr-formats/src/classes.rs) |
| `AllowedU.dat` | `CClassesAllowedDoc` 0x680e21 | one `CObArray` of `CClassesAllowedOb` (95) | 30 | [classes.md](classes.md) | [`allowed.rs`](../../crates/cr-formats/src/allowed.rs) |
| `Parts.dat` | `CPartDoc` 0x67b10d | 21 `CObArray`s of `CPart` (93), one per part type | 4584 | [parts.md](parts.md) | [`parts.rs`](../../crates/cr-formats/src/parts.rs) |
| `Spells.dat` | `CSpellsDoc` 0x4f9d89 | two `CObArray`s of `CSpellsOb` (88): wizard, then priest | 480 + 453 | [spells.md](spells.md) | [`spells.rs`](../../crates/cr-formats/src/spells.rs) |
| `Monster.dat` | | one `CObList` of `CMonsterOb` (88) | 1524 | [monster.md](monster.md) | [`monster.rs`](../../crates/cr-formats/src/monster.rs) |
| `Attack.dat` | | one `CObList` of `CMonsterAttackOb` (88) | 21 | [monster.md](monster.md) | [`monster.rs`](../../crates/cr-formats/src/monster.rs) |
| `MonType.dat` | | one `CObList` of `CMonsterTypeOb` (95) | 298 | [monster.md](monster.md) | [`monster.rs`](../../crates/cr-formats/src/monster.rs) |
| `EncPlat.dat` | | one `CObList` of `CEncTemplateOb` (88) | 887 | [monster.md](monster.md) | [`encounter.rs`](../../crates/cr-formats/src/encounter.rs) |
| `Encount.dat`, `EncountU.dat` | `CEncounterDoc` 0x64738a | one `CObList` of `CEncounterOb` (88) | 0, 3 | [monster.md](monster.md) | [`encounter.rs`](../../crates/cr-formats/src/encounter.rs) |
| `EncTabU.dat` | frequency-table document 0x63ef52 | version string, then one `CObArray` of `CEncounterFreqTableOb` (1) | 0 | [monster.md](monster.md) | [`encounter.rs`](../../crates/cr-formats/src/encounter.rs) |
| `Treasure.dat` | `CTreasureTablesDoc` 0x54d174 | one `CObList` of `CTreasureSingleTableOb` (1) | 496 | [misc.md](misc.md) | [`treasure.rs`](../../crates/cr-formats/src/treasure.rs) |
| `TreaBun.dat` | `CTreasureDoc` 0x53d734 | one `CObList` of `CTreasureOb` (95) | 0 | [misc.md](misc.md) | [`treasure.rs`](../../crates/cr-formats/src/treasure.rs) |
| `Psionic.dat` | `CPsionicPowerDoc` 0x4ec7c4 | one `CObList` of `CPsionicPowerOb` (96) | 231 | [misc.md](misc.md) | [`psionic.rs`](../../crates/cr-formats/src/psionic.rs) |
| `Traps.dat` | `CTrapDoc` 0x539cf4 | one `CObList` of `CTrapOb` (95) | 3 | [misc.md](misc.md) | [`traps.rs`](../../crates/cr-formats/src/traps.rs) |
| `Phrase.DAT` | `CPhraseSpacingDoc` 0x4eb985 | one `CObList` of `CPhraseSpacingOb` (88) | 442 | [misc.md](misc.md) | [`phrase.rs`](../../crates/cr-formats/src/phrase.rs) |
| `Convert.dat` | `CConversionMapDoc` 0x4561bd | eight `CMapStringToOb`s of `CStringArray`s | 1454 keys | [misc.md](misc.md) | [`convert.rs`](../../crates/cr-formats/src/convert.rs) |

Equipment (`CInventoryBundle`, `CInventoryItem`) is shared by characters,
treasures, monsters and encounters:
[`inventory.rs`](../../crates/cr-formats/src/inventory.rs), specified in
[monster.md](monster.md) (the bundle) and [misc.md](misc.md) (the item).

Every record class the app reads is listed with its MFC name and schema in
[`object.rs`](../../crates/cr-formats/src/object.rs); a class not listed
there makes the reader stop with "class X is not supported yet".

## How the formats were mapped

Each class's layout comes from the load branch of its `Serialize` function
in `TSRP3.EXE` (and `EVERMORE.EXE` where 2.0 differs), disassembled with
radare2, and is checked by reading every shipped file to its last byte and
writing it back byte for byte, for both programs. Field names come from the
data, the editors' dialogs and the program's own strings; fields without
enough evidence are named after their member offset in the original object
(`unknown_0x2c` = `this + 0x2c`), so they can be traced back to the
disassembly. Several classes carry their own version and skip fields older
versions lacked; those conditions are copied from the code even where the
shipped data only uses the newest version.

Addresses are virtual addresses in `TSRP3.EXE` unless a document says
`EVERMORE.EXE` or 2.0. Sample values come from the shipped data; they are
kept to a name or a number, enough to recognise a field, and the documents
reproduce none of the books' or databases' content.
