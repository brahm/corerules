# Core Rules 2.0 file formats

Findings from the 2.0 CD (`EVERMORE.EXE`, August 1998) and the Expansion CD
(`TSRP3.EXE`, September 1999). Both programs are Win32 MFC applications with
MFC linked statically.

## Serialization

Every data file is written with MFC `CArchive`. See
[`crates/cr-formats/src/archive.rs`](../../crates/cr-formats/src/archive.rs)
for the exact encoding of counts, strings and object tags. Text is
Windows-1252.

## Installation layout

Folder names differ in case between the CD and the installer; match them
without regard to case.

| Path | Content | Top-level structure |
| --- | --- | --- |
| `Database/*.dat` | Rules data (read only) | `CObList` of one main class |
| `UserDbas/*U.dat` | User additions, one file per type | Same as the matching `Database` file; empty = `00 00` |
| `Chars/index.chr` | Character list | `CObList` of `CCharacterMiniOb` (schema 95) |
| `Chars/<id>.dat` | One character | Document serialization, starts with `"TSRP3 V54"` |
| `Maps/index.crm` | Map list | `CObList` of `CMapMiniOb` |
| `MapMaker/moi*.dat` | Map object sets, listed in `mapmaker.ini` | Own binary format with embedded BMPs |
| `Tables/*.tbl` | DMG tables for the generators | Plain text |
| `*.xpt` / `*.xpx` | Exports from 2.0 / the Expansion | `CImportExportDoc` |

## How the formats were mapped

Each class's layout comes from the load branch of its `Serialize` function
in `TSRP3.EXE`, disassembled with radare2, and is checked by reading every
shipped file to its last byte. Field names come from the data and the
program's own strings; fields without enough evidence are named after their
member offset in the original object (`unknown_0x2c` = `this + 0x2c`), so
they can be traced back to the disassembly. Several classes carry their own
version and skip fields older versions lacked; those conditions are copied
from the code even where the shipped data only uses the newest version.

## Status

| File | Framing | Documented in | Rust |
| --- | --- | --- | --- |
| `Race.dat`, `RaceU.dat` | one `CObList` of `CRaceOb` (88) | [`race.rs`](../../crates/cr-formats/src/race.rs) | yes |
| `Class.dat`, `ClassU.dat` | one `CObList` of `CClassOb` (88), each with a `TSRP3 V39` version string | [`classes.rs`](../../crates/cr-formats/src/classes.rs) | yes |
| `Chars/<id>.dat` | `CCharacterDoc`, version `TSRP V27` to `TSRP3 V54` (2.0 writes `TSRP2 V29`) | [`character.rs`](../../crates/cr-formats/src/character.rs) | yes |
| `Chars/index.chr` | one `CObList` of `CCharacterMiniOb` (95) | [`character.rs`](../../crates/cr-formats/src/character.rs) | yes |
| `Parts.dat`, `PartsU.dat` | 21 `CObArray`s of `CPart` (93), one per part type | [parts.md](parts.md) | yes |
| `Spells.dat`, `SpellsU.dat` | two `CObArray`s of `CSpellsOb` (88): wizard, then priest | [parts.md](parts.md) | yes |
| `AllowedU.dat` | one `CObArray` of `CClassesAllowedOb` (95) | [parts.md](parts.md) | yes |
| `Treasure.dat`, `TreaBun.dat` | one `CObList` | [misc.md](misc.md) | yes |
| `Psionic.dat`, `Traps.dat`, `Phrase.DAT` | one `CObList` | [misc.md](misc.md) | yes |
| `Convert.dat` | eight `CMapStringToOb`s | [misc.md](misc.md) | yes |
| `Monster.dat`, `Attack.dat`, `MonType.dat` | one `CObList` of `CMonsterOb` (88), `CMonsterAttackOb` (88), `CMonsterTypeOb` (95) | [monster.md](monster.md) | yes |
| `EncPlat.dat`, `Encount.dat`, `EncountU.dat` | one `CObList` of `CEncTemplateOb` (88) / `CEncounterOb` (88) | [monster.md](monster.md) | yes |
| `EncTabU.dat` | version string, then one `CObArray` of `CEncounterFreqTableOb` (1) | [monster.md](monster.md) | yes |

Equipment (`CInventoryBundle`) is shared by characters, treasure and
monsters: [`inventory.rs`](../../crates/cr-formats/src/inventory.rs).
