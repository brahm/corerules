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

## Database classes (Expansion)

| File | Records | Class (schema) |
| --- | --- | --- |
| Attack.dat | 21 | CMonsterAttackOb (88) |
| Class.dat | 26 | CClassOb (88) |
| Convert.dat | 1454 | CStringArray |
| EncPlat.dat | 887 | CEncTemplateOb (88) |
| Monster.dat | 1524 | CMonsterOb (88) |
| MonType.dat | 298 | CMonsterTypeOb (95) |
| Parts.dat | 733 | CPart (93) |
| Phrase.dat | 442 | CPhraseSpacingOb (88) |
| Psionic.dat | 231 | CPsionicPowerOb (96) |
| Race.dat | 48 | CRaceOb (88) |
| Spells.dat | 480 | CSpellsOb (88) |
| Traps.dat | 3 | CTrapOb (95) |
| Treasure.dat | 496 | CTreasureSingleTableOb (1) |

## `CCharacterMiniOb` (index.chr entry)

| Field | Type | Notes |
| --- | --- | --- |
| name | CString | |
| file | CString | e.g. `90688519.dat` |
| race | CString | |
| ? | i32 | 3 in both fixtures |
| ? | i32 | 8 in both fixtures |
| ? | i32 | 0 in both fixtures |
| ? | u16 | 1 for a Core Rules character, 0 for Skills & Powers |
| details | CStringArray | rules system, player, party, race, class, level, kit |

## Character file (start)

| Field | Type | Notes |
| --- | --- | --- |
| version | CString | `TSRP3 V54` from the Expansion |
| rules | i32 | 0 Core Rules, 1 Skills & Powers |
| party | CString | inferred; empty in both fixtures |
| player | CString | |
| name | CString | |
| class | CString | |
| kit | CString | `NONE` |

Further on: height, weight and age as i32, then the six abilities as three
i32 each. The rest is still being mapped.
