# export: DM Toolkit packages (.xpx, .xpt)

Reader and writer: [`export.rs`](../../crates/cr-formats/src/export.rs) (`ExportFile`,
`CampaignMap`, `MapMakerMaps`). Import, Export and package editing:
[`cr-store/src/export.rs`](../../crates/cr-store/src/export.rs), driven by `cr export`,
`cr import` and `cr package` ([`cr-cli/src/transfer.rs`](../../crates/cr-cli/src/transfer.rs))
and the Import..., Export..., New Package..., Edit Package... and Import Core Rules 2.0... menu
items ([`cr-gtk/src/transfer.rs`](../../crates/cr-gtk/src/transfer.rs)). Document class
`CImportExportDoc`, Serialize at 0x49fc52 (TSRP3.EXE) and 0x4e0130 (EVERMORE.EXE).

The Expansion writes `.xpx` packages (with the version string), Core Rules 2.0 writes `.xpt`
packages (without it). The file is a single `CArchive`, so class tags carry over between
sections ([archive.md](archive.md)).

| # | section | type | Expansion (`.xpx`) | 2.0 (`.xpt`) |
|---|---|---|---|---|
| 1 | `-1`, then the version string `TSRP3 V1` | i32, CString | yes | no |
| 2 | character count, then each character exactly as in a character file ([character.md](character.md); the export flag at +0x1ea1 only changes the messages) | u32 count | yes | yes |
| 3 | drawing count, then per Campaign Mapper `.FCW` drawing `{file name, size, bytes}` | u32 count, then CString, u32, u8[size] | yes | yes |
| 4 | the race/class combinations (`AllowedU.dat` records, see [classes.md](classes.md#allowed-classes-per-race)), then the classes | CObArray<CClassesAllowedOb>, CObArray<CClassOb> | when the version string is `TSRP3 V1` | no |
| 5 | 12 arrays: psionic powers, traps, races, wizard spells, priest spells, monsters, monster attacks, saved encounters, encounter templates, treasure bundles, an unused array (+0x140, neither program fills it), monster types | 12 x CObArray | yes | yes |
| 6 | the parts, laid out like `PartsU.dat` (21 arrays, see [parts.md](parts.md)) | 21 x CObArray | yes | yes |
| 7 | Map Maker II `CMapData` maps | count, then the objects, kept undecoded to the end of the file | yes | yes |

The reader takes a negative first word as the start of section 1; otherwise that word is the
character count. A version string other than `TSRP3 V1` makes the Expansion say so ("Dazed and
confused") and read the rest like a 2.0 file (no section 4). The Map Maker maps come last and may
refer to class tags of the sections before them, so their bytes are only valid behind the same
sections.

## Export

Export offers what the user made: the records of the user files (`UserDbas/*U.dat`) and the
characters of `Chars/index.chr`; Class Abilities parts and classes only in the Expansion. The
package has the layout of the installation's program, items in the order chosen; an item takes
the place of the one of its category and name the package holds already.

| | Expansion | 2.0 |
|---|---|---|
| race | takes its `AllowedU.dat` record along, replacing the package's record for it (0x4a9c8c); warns "No race/class combinations found for the race ..." when there is none | - |
| class | takes along, for each race, the combinations that name it, adding a record for the race when the package has none (0x681868) | - |

Core Rules 2.0 itself exports `.xpt` packages. This app writes only `.xpx`: a Core Rules 2.0
installation is opened read only, and a `.xpt` is only imported into the Expansion. `cr import` also takes a Core Rules 2.0 installation folder in
place of a file: every user record and character of it, as a 2.0 package would carry them
(Import Core Rules 2.0... in the GUI lets the user pick among them).

## Import

Import does what the original's Import All does (0x49d529, 0x49d704 in TSRP3.EXE, 0x4def60 in
EVERMORE.EXE), taking the items sorted by name (case ignored). Core Rules 2.0 cannot import an
Expansion package.

| | Expansion (0x49d529) | 2.0 (0x4def60) |
|---|---|---|
| user record of the same name (trimmed, case ignored) | removed, the import appended | kept, the import dropped (its dialog lists both as done) |
| scope of a name | parts: every part type; spells: wizard and priest lists together; other records: their own file. Standard records are not looked at | same |
| part named like a standard part | `*` appended until no part has the name | added as is |
| race | its `AllowedU.dat` record is replaced by the file's, or, for a file without combinations (from 2.0) and a race without a record, built from the race's class lists (0x47c91e, 0x49eff1) | - |
| class | the file's combinations naming it are added to each race's record (0x49f33c) | skipped |
| character | new eight-digit file in `Chars/`; the list entry has no Class/Level lines (the kit line only for a kit the parts know) | new file; entry like a new character's |
| drawing | Save dialog per drawing, default its file name (this app does not import drawings) | same |
| Map Maker II maps | Map Maker's own import (this app does not import them and says so) | same |

The Expansion converts the characters and parts of a 2.0 package to its own layout: characters
to `TSRP3 V54`; parts get the `TSRP3 V7` version block, the members the newer layouts add get
their defaults (`unknown_0x52c` 100, `unknown_0x4` zeros, `flag_0x724`, `flag_0x4d0`,
`flag_0x448` 0, no kit), and the old class flags become class names (the program's strings
0x84c580.. for Bard to Thief, the eighth flag not converted) in `allowed_classes`, as the
Expansion's loader does (0x65c3a0, 0x661697). The list entry's byte at +0x28 of an imported
character is never initialized by the Expansion (0x5a1372 copies the entry through 0x5a05db,
which skips it).

## Editing (this app only)

Only Expansion packages (`.xpx`) are edited. A `.xpt` package is read only: it is imported, and
converted to the Expansion's layout on the way in, but never edited in place; New Package... and
`cr package --new` make an Expansion package, and an installation adds items only to a package
of its own program's layout.

Adding an item replaces the one of the same category and name, and brings the race's record or
the class's combinations along, as Export does. Removing a race or class keeps the combinations
of the races still in the package and, for other races, only those naming a class still in it
(a record left with none is dropped). Any change leaves out the Map Maker II maps, whose bytes
may refer to class tags of the sections before them, with a warning. Campaign Mapper drawings
are kept as they are.

There is no `.mon` format: the monster dialogs' Export and Import buttons only point at the DM
Toolkit, so monsters travel in these packages.

## Notes

* The `CMapData` layout is not decoded; a package's maps are kept as bytes and survive only an
  unchanged package.
* `ExportFile::is_expansion` tests for a version string, while the reader decides on section 4
  by its value; the two differ only for a version string other than `TSRP3 V1`, which no
  program writes.
