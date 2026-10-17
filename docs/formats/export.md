# export: DM Toolkit packages (.xpx, .xpt)

Reader and writer: `crates/cr-formats/src/export.rs`. Import and Export
semantics: `crates/cr-store/src/export.rs`. Document class
`CImportExportDoc`, Serialize at 0x49fc52 (TSRP3.EXE) and 0x4e0130
(EVERMORE.EXE).

The file is a single `CArchive`, so class tags carry over between sections.

| # | section | Expansion (`.xpx`) | 2.0 (`.xpt`) |
|---|---|---|---|
| 1 | `-1`, then the version string `TSRP3 V1` | yes | no |
| 2 | character count, then each character as in a character file | yes | yes |
| 3 | drawing count, then `{CString file name, u32 size, bytes}` per `.FCW` | yes | yes |
| 4 | `CObArray` of `CClassesAllowedOb`, then `CObArray` of `CClassOb` | yes | no |
| 5 | 12 `CObArray`s: psionic powers, traps, races, wizard spells, priest spells, monsters, monster attacks, saved encounters, encounter templates, treasure bundles, an unused array, monster types | yes | yes |
| 6 | the parts, laid out like `PartsU.dat` (21 arrays) | yes | yes |
| 7 | `CObArray` of Map Maker II `CMapData`, kept undecoded | yes | yes |

A version string other than `TSRP3 V1` makes the Expansion say so and read
the rest like a 2.0 file.

## Import

| | Expansion (0x49d529) | 2.0 (0x4def60) |
|---|---|---|
| user record of the same name (trimmed, case ignored) | removed, the import appended | kept, the import dropped |
| part named like a standard part | `*` appended until unique | added as is |
| race | its `AllowedU.dat` record is replaced by the file's, or built from the race's class lists (0x47c91e) | - |
| class | the file's combinations naming it are added per race (0x49f33c) | - |
| character | new file in `Chars/`; the list entry has no Class/Level lines | new file; entry like a new character's |
| drawing | Save dialog per drawing, default its file name (this app does not import drawings) | same |
| Map Maker II maps | Map Maker's own import | same |

The Expansion converts characters (to `TSRP3 V54`) and parts (version
block `TSRP3 V7`, class flags to class names) of a 2.0 package. The list
entry's byte at +0x28 of an imported character is never initialized by the
Expansion (0x5a1372 copies the entry through 0x5a05db, which skips it).

## Editing (this app only)

Adding an item replaces the one of the same category and name, and in the
Expansion brings the race's record or the class's combinations along, as
Export does. Removing a race or class keeps the combinations of the races
still in the package and, for other races, only those naming a class still
in it. Any change leaves out the Map Maker II maps, whose bytes may refer
to class tags of the sections before them. Campaign Mapper drawings are
kept as they are.

There is no `.mon` format: the monster dialogs' Export and Import buttons
only point at the DM Toolkit.
