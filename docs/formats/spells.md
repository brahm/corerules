# spells: Spells.dat, SpellsU.dat and CSpellsOb

Reader and writer: [`crates/cr-formats/src/spells.rs`](../../crates/cr-formats/src/spells.rs)
(`SpellsFile`, `Spell`). The Custom Spells editor is
[`crates/cr-store/src/userdb/spells.rs`](../../crates/cr-store/src/userdb/spells.rs).
Encoding: [archive.md](archive.md).

## Files

| file | document class | content |
|---|---|---|
| `Database/Spells.dat` | `CSpellsDoc` (Serialize 0x4f9d89) | the wizard spells, then the priest spells |
| `UserDbas/SpellsU.dat` | the same | the user's spells, same layout; empty = `00 00 00 00` |

The file is two `CObArray<CSpellsOb>` back to back with no outer count:
document member +0x58 holds the wizard spells, +0x6c the priest spells. They
share one archive, so the priest array refers to the class by index: in the
shipped `Spells.dat` it opens with the count `c5 01` (453) and the tag
`01 80` (class index 1, `CSpellsOb`, defined by the wizard array). The
shipped file has 480 wizard and 453 priest spells, in both programs.

Core Rules 2.0 uses the same class, schema and layout: the 2.0 `Spells.dat`
and the spells inside `.xpt` packages read with the same code.

A spell's type is its array. The `wizard` and `priest` members repeat it, and
the app writes a user spell into the array its `priest` member selects.
Names are unique within a type: a priest spell may share a wizard spell's
name, but not another priest spell's (compared trimmed, without regard to
case, against the standard and the user spells).

## CSpellsOb (schema 88, size 0x74, constructor 0x4ff04e)

The class reads no version. Members in archive order; the editor column is
the Spells editor (IDD 169) control that edits the member.

| # | field | type | member | meaning |
|---|---|---|---|---|
| 1 | `name` | CString | +0x4 | spell name, up to 50 characters (Name) |
| 2 | `unknown_0x60` | i32 | +0x60 | Never Ban Cantrip, 0/1; enabled only for wizard spells of level 0; 0 in every shipped spell but one |
| 3 | `unknown_0x64` | i32 | +0x64 | Reversible, 0/1 (1 on 48 wizard and 112 priest spells) |
| 4 | `wizard` | i32 | +0x68 | 1 for a wizard spell, else 0 |
| 5 | `priest` | i32 | +0x6c | 1 for a priest spell, else 0 |
| 6 | `level` | i32 | +0x70 | spell level: wizard 0 (cantrip) to 9, priest 0 to 7; a new spell starts at 1 |
| 7 | `area_of_effect` | CString | +0x38 | Area of Effect ("10 ft radius") |
| 8 | `casting_time` | CString | +0x3c | Casting Time ("1", "1 rd") |
| 9 | `components` | CString | +0x40 | Components; the editor's combo offers `V`, `S`, `M`, `V, S, M`, `V, S`, `V, M`, `S, M` |
| 10 | `unknown_0x44` | CString | +0x44 | Critical (Player's Option combat): mostly empty, "None", "Medium (1 hit)" |
| 11 | `duration` | CString | +0x48 | Duration |
| 12 | `unknown_0x4c` | CString | +0x4c | Knockdown: mostly empty, "None", "d8" |
| 13 | `range` | CString | +0x50 | Range ("Touch") |
| 14 | `saving_throw` | CString | +0x54 | Saving Throw ("None") |
| 15 | `unknown_0x58` | CString | +0x58 | Sensory: mostly empty, "Small visual" |
| 16 | `unknown_0x5c` | CString | +0x5c | Subtlety: mostly empty, "+2" |
| 17 | `id` | i32 | +0x8 | identifier and help topic (HelpID, a hidden control); a copy made in the editor gets 0 |
| 18 | `unknown_0xc` | CString | +0xc | description text (Description dialog, IDD 174); a new spell starts with the word "Description" |
| 19 | `schools` | CStringArray | | schools of magic, names of Schools Of Magic parts |
| 20 | `spheres` | CStringArray | | priest spheres, names of Spheres parts; empty for wizard spells |

The editor's Quest check box (priest spells) is kept by the dialog only and
is not stored.

## Where spells appear

| place | how |
|---|---|
| `Spells.dat`, `SpellsU.dat` | the records themselves |
| a character ([character.md](character.md)) | by name: spells known (`spells`, `priest_spells`) and memorized, per class and level |
| a spell scroll or spellbook in an inventory ([misc.md](misc.md)) | by name, in the item's extra data |
| a kit (`CPartKitOb`, [parts.md](parts.md)) | full `CSpellsOb` copies: the kit's bonus spells |
| a part (`CPart`, [parts.md](parts.md)) | a `CObArray<CSpellsOb>` member, empty in every shipped part |
| a monster (`CMonsterOb`, [monster.md](monster.md)) | full `CSpellsOb` copies: the spells it casts |
| an export package ([export.md](export.md)) | the wizard and the priest spells as two of its arrays |

Copies are not linked to the database: editing a spell does not change the
copies kits and monsters hold. Characters find their spells by name, wizard
or priest by the list they are in.

## Notes

* The member offsets of `schools` and `spheres` are not established.
* The one shipped spell with `unknown_0x60` set is the only evidence for
  that member; the editor shows it as Never Ban Cantrip.
