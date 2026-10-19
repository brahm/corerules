# character: Chars/index.chr and Chars/<id>.dat

Reader and writer: [`crates/cr-formats/src/character.rs`](../../crates/cr-formats/src/character.rs)
(`CharacterIndex`, `CharacterEntry`, `Character`, `CharacterData`, `ClassList`,
`RogueSkills`, `SheetOptions`, `ClassSlot`) and
[`crates/cr-formats/src/inventory.rs`](../../crates/cr-formats/src/inventory.rs)
(`InventoryBundle`, `InventoryItem`, `InventoryExtra`). What the app does with
the files: [`create.rs`](../../crates/cr-store/src/create.rs) (new characters),
[`edit.rs`](../../crates/cr-store/src/edit.rs) (changes in place),
[`lists.rs`](../../crates/cr-store/src/lists.rs) (proficiencies, spells,
equipment), [`advance.rs`](../../crates/cr-store/src/advance.rs) and
[`dual.rs`](../../crates/cr-store/src/dual.rs) (experience, levels, a new
class), [`party.rs`](../../crates/cr-store/src/party.rs) (parties, delete),
[`npc.rs`](../../crates/cr-store/src/npc.rs) and `npc/**` (generated NPCs),
[`export.rs`](../../crates/cr-store/src/export.rs) (import) and `sheet/**`
(the character sheet). Encoding: [archive.md](archive.md).

## Files

| file | document class (Serialize) | content |
|---|---|---|
| `Chars/index.chr` | `CCharacterFileList` (rtc 0x7a7cb8, size 0x88, Serialize 0x5a111a) | `CObList<CCharacterMiniOb>`: count, then one tagged entry per character; no characters = `00 00` |
| `Chars/<id>.dat` | `CCharacterDoc` (rtc 0x79b3f0, size 0x2188, vtable 0x79b428, Serialize 0x41cd5b) | a version string, then [`CharacterData`](#ccharacterdoc-serialize-0x41cd5b) |

The folder is `Directories\Characters` of the program's settings, `Chars`
in a standard installation; the app finds `Chars`, `index.chr` and the
`.dat` files without regard to case. Both files are one archive each and
nothing follows the last field.

Versions written:

| program | `index.chr` entry layout | character version string |
|---|---|---|
| Expansion (`TSRP3.EXE`) | 3 | `TSRP3 V54` |
| Core Rules 2.0 (`EVERMORE.EXE`) | 0 (a character) or 1 (an NPC) | `TSRP2 V29` |

The Expansion reads both layouts and every version from `TSRP V27` on. The
app writes only the Expansion's layout and version: a Core Rules 2.0
installation is opened read only, and a 2.0 character imported into the
Expansion is saved as `TSRP3 V54` (see
[Saving a 2.0 character in the Expansion](#saving-a-20-character-in-the-expansion)). Test fixtures written by both programs are in
`tests/fixtures/characters` (Expansion) and its `v20` folder (2.0); every one
round-trips byte for byte.

## CCharacterFileList (index.chr)

Serialize is a `CObList` of `CCharacterMiniOb` read inside a try block. An
exception marks the list damaged (+0x80), and opening the Characters screen
(0x478fe4) then asks "The index for your character and NPC files must be
rebuilt" (0x47a87f). The same prompt appears when a listed `.dat` fails to
open (view 0x44ba37, print 0x44bdf8, update 0x44c23f, delete 0x44b7cf,
convert 0x44c865). Rebuild (0x5a1671) opens every `*.dat` of the folder and
builds each entry again (below); the program never rebuilds on its own, so
a stale entry stays stale.

The first entry carries the full class tag (`CCharacterMiniOb`, schema 95),
later ones a class reference ([archive.md](archive.md#objects)). The app
writes schema 95 into a list that was empty.

## CCharacterMiniOb (schema 95, size 0x30, Serialize 0x5a06d2, load 0x5a0780)

Rust `CharacterEntry`. Custom code, not a `record!`: the fourth value decides
what follows.

| # | field | type | present | member | meaning |
|---|---|---|---|---|---|
| 1 | `name` | CString | always | +0x4 | character name, shown in the list and the first line of the details box; the default export file name comes from it |
| 2 | `file` | CString | always | +0x8 | file name inside `Chars/` (`90688519.dat`) |
| 3 | `race` | CString | always | +0x20 | race name (`Dwarf`) |
| 4 | `version` | u32 | always | - | entry layout. 2 and 3 read the same; the Expansion writes 3. For 0 and 1 this value is also `unknown_0x24` (see below) |
| 5 | `extra.unknown` | u32 | `version > 1` | - | read into a local and dropped; the Expansion writes 8 |
| 6 | `unknown_0x24` | u32 | `version > 1` | +0x24 | NPC mark: 1 for an NPC, 0 for a character; copies doc +0x1da4 (`unknown_0x1da4`) |
| 7 | `flag_0x28` | u8 | always | +0x28 | stored as `!= 0`; built as "uses the Core Rules" (`!skills_and_powers`), but see Notes |
| 8 | `extra.unknown_0x2c` | CString | `version > 1` | +0x2c | the party name, empty for none; copies doc +0x1dc0 (`unknown_0x1e68[3]`) |
| 9 | `details` | CStringArray | always | +0xc | the lines of the details box, below |

In layouts 0 and 1 there is no separate NPC mark: the program stores the
`version` value in +0x24, so 2.0 writes 0 for a character and 1 for an NPC
(Evermore 0x42c690), and the Rust writer writes `unknown_0x24` in that place.
`extra` is `Some` exactly when `version > 1`; a mismatch is a write error.

### Details lines

Built by 0x5a08fc (Expansion) from an open character and by Evermore 0x42c880
(2.0). The app builds them in `create.rs` (`entry`), `export.rs`
(`expansion_entry`, `core20_entry`) and `npc.rs` (`npc_entry`).

| line | Expansion (layout 3) | 2.0 (layout 0/1) |
|---|---|---|
| 1 | `Core Rules` or `Skills & Powers` | `Core Rules` |
| 2 | `Player's Name: ` + `player` | `Race: ` + `race` |
| 3 | `Party: ` + party (doc +0x1dc0) | `Class: ` + `class_title` |
| 4 | `Race: ` + `race` | `Level: ` + levels |
| 5 | `Class: ` + `class_title`, unless the import flag (doc +0x1ea1) is set | |
| 6 | `Level: ` + levels, same condition | |
| 7 | `Kit: ` + kit name, when the kit names a kit part | |

Levels are each class's level, computed from its experience, joined with
`/` in class order (`1`, `1/1`). A new character with no kit has the kit
`None` and gets `Kit: None`. An imported character in the Expansion has no
Class and Level lines, and its Kit line appears only for a kit the parts
know, spelled as the parts spell it. An NPC's Expansion entry has
`Core Rules`, an empty `Player's Name: ` and the kit line as stored.

### How index.chr follows the characters

The program writes `index.chr` through the list document (vtable +0x78).
What each operation does, in the original and in the app:

| operation | original | `.dat` | entry |
|---|---|---|---|
| new character | wizard finish, 0x5a1a51 ("Your character was saved as", 0x496b0b); `create.rs` | new file | built as above, appended; `unknown_0x24` 0, `flag_0x28` = `!skills_and_powers` (2.0: 1) |
| save a generated NPC | NPC generator; `npc.rs` `save_npc` | new file, party written to `unknown_0x1e68[3]` | appended; NPC mark 1, `flag_0x28` 1, party in +0x2c; 2.0 layout 1 |
| change in place | View, Print and Export re-save the sheet options; `edit.rs` | same file | the original leaves the entry alone; the app sets `name`, the `Player's Name:` line, the `Class:` line when a class was added and the `Level:` line when levels changed, and keeps everything else |
| Convert to Character | 0x44c865; `npc.rs` `convert_to_character` | `unknown_0x1da4` = 0, same file | `unknown_0x24` = 0 |
| Convert to NPC | Character Generator button, after "This will move the selected Player Character to the NPC List in the DM Toolkit"; `npc.rs` `convert_to_npc` | `unknown_0x1da4` = 1, saved under a new name, old file deleted (`tests/fixtures/characters/to-npc`) | stays in place with the new `file`, `unknown_0x24` = 1 and `flag_0x28` = 1; the details lines are left alone |
| put in or take out of a party | 0x5ab684, 0x5abb05; `party.rs` `set_party` | saved under a new name in the Expansion layout (`Character::upgraded`), party in `unknown_0x1e68[3]`; old file deleted | old entry removed, new entry appended with the Class and Level lines and the party in +0x2c |
| import from a package | 0x49d529 (2.0 0x4def60); `export.rs` `import` | one new file per character | appended per character: the Expansion without Class/Level lines; 2.0 like a new character's with the levels computed, in layout 0 or 1 by the character's NPC mark |
| Update Character | 0x44c23f | saved under a new name, old file deleted | old entry removed, a fresh one appended. The Name page writes the sex, alignment, kit, Background… event and history and, with a new kit, a social class rolled on the kit's table (`tests/fixtures/characters/update`); the app saves these in place and writes the new kit into the entry's kit line. The Race Abilities and Class Abilities pages (with Schools and Spheres) charge the character points in `unknown_0x1778[0]` and write the subrace, `racial_abilities` and `class_slots` sorted (`abilities.rs`). The Weapon Proficiencies page (with Specialization, Shield and Armor and Fighting Styles) and the traits and disadvantages of the Nonweapon Proficiencies page charge the same points and append new choices to the lists in the order taken (`combat.rs`) |
| delete | 0x44b7cf, party delete 0x5abffd; `party.rs` | removed (the app keeps `<file>.bak` until the deletion is confirmed) | removed |
| rebuild | 0x5a1671, only on the user's yes | read | every entry built again |

The Expansion has no list of parties: the parties are the party names the
entries hold (`party_of` reads +0x2c), so a party without members does not
last. `~All` is reserved, and a party name has at most 50 characters
(0x5ab2a4). 2.0 has no parties.

The app writes a new file to a temporary name and renames it, and replaces
an existing `index.chr` or `.dat` the same way after copying the old bytes
to `<file>.bak`. Every file is read back before anything is replaced.

### File names

0x5a1372 picks a random name `%08.08s.dat` that does not exist in the
folder. The app makes eight decimal digits from the clock (seconds since
1970 modulo 90,000,000, plus 10,000,000) and counts up, wrapping from
99,999,999 to 10,000,000, until the name is neither in the list nor in the
folder (both compared without regard to case); characters imported together
take the following free numbers. The name carries no meaning, and a
character changed through a party or Update Character gets a new one.

## CCharacterDoc (Serialize 0x41cd5b)

The file starts with a CString version string; `version_of` turns it into
the version `v` the field conditions use:

| string | `v` |
|---|---|
| `TSRP V27` | 27 |
| `TSRP3 V28` | 28 |
| `TSRP3 V54` | 54 (the current version) |
| anything else | `atol` of the text after the first `V`, or of the whole string when it has no `V`: `TSRP2 V29` is 29, `TSRP3 V41` is 41 |

For 27 and 28 the load code logs "Version %ld expected, got 27. I should
handle it just fine. Cross your fingers." (a debug trace, not a message
box). The version string is written back as read; the Expansion writes
`TSRP3 V54`, 2.0 `TSRP2 V29`. Fields a version predates are absent, with
the conditions below copied from the load code.

The load branch only reads fields and resolves names: the class names to
`CClassOb`s ("Unable to find the class '%s'."), the class abilities to
parts (an unknown one is a debug trace and stays in the list). Nothing
derived is written back on load. After reading `skills_and_powers` it sets
the global 0x85f478 (0x446a40) that selects the Core Rules prices and
weights of parts.

Member offsets are those of the Expansion's object; 2.0 lays out the same
stream from other offsets (experience at doc +0x15e0 instead of +0x1618,
for example). Field names follow the Expansion.

### CharacterData

Archive order. "present" uses `v`; fields without a condition are in every
version.

| # | field | type | present | member | meaning |
|---|---|---|---|---|---|
| 1 | `skills_and_powers` | u8 | always | +0x7c | 1 for a Skills & Powers (Player's Option) character, 0 for Core Rules; the program keeps `!= 0` |
| 2 | `unknown_0x1dac` | u32 | always | +0x1dac | non-zero: the sheet's proficiency box shows the secondary skills (`unknown_0x1830[2]`) instead of the nonweapon proficiencies |
| 3 | `player` | CString | always | +0x80 | player's name |
| 4 | `name` | CString | always | +0x84 | character name |
| 5 | `class_title` | CString | always | +0x88 | class as displayed (`Fighter/Mage`), written by the wizard and shown verbatim; a new class appends `/` + its name |
| 6 | `unknown_0x90` | CString | always | +0x90 | background event of Update Character's Background… dialog (`Exiled`), one of `NONE` and the program's 20 events (pushed at 0x54f66a); `NONE` for a new character; the NPC generator draws one |
| 7 | `unknown_0x94` | CString | always | +0x94 | character history (Background…), the sheet's "Character History" paragraph; the default sheet sections include it when not empty |
| 8 | `social_class` | CString | always | +0x15c4 | social class |
| 9 | `unknown_0x1618` | u32[5] | always | +0x1618 | experience per class slot; levels are computed from it (and capped by `unknown_0x1dc4`), never stored |
| 10 | `height` | u32 | always | | inches |
| 11 | `weight` | u32 | always | | pounds |
| 12 | `age` | u32 | always | | years |
| 13 | `unknown_0x163c` | u32 | always | +0x163c | sex: 0 female, non-zero male (the original writes 1 to 3 for males; the app writes 1) |
| 14 | `strength` | u32[3] | always | +0x172c, +0x1734, +0x1738 | Strength, Stamina, Muscle. The score includes the racial adjustment; with the Core Rules all three are equal |
| 15 | `exceptional_strength` | u32 | always | +0x1730 | 1 to 100 for 18/01 to 18/00, 0 for none |
| 16 | `abilities` | (u32[3])[5] | always | +0x173c to +0x1774 | Dexterity (Aim, Balance), Constitution (Health, Fitness), Intelligence (Reason, Knowledge), Wisdom (Intuition, Willpower), Charisma (Leadership, Appearance): score and two subabilities each, like `strength`. Scores go from 1 to 25; a generated NPC's subability can be below 0 and is stored as its u32 bit pattern |
| 17 | `unknown_0x1980` | CString[2] | always | +0x1980, +0x1984 | psionic primary discipline(s), marked on the sheet; `[1]` only for a psychic adept (a class ability with part +0x785). Update Character's Psionics dialog writes both on Close |
| 18 | `unknown_0x162c` | u32 | always | +0x162c | unknown; 0 in every fixture |
| 19 | `unknown_0x1778` | u32[6] | always | +0x1778 to +0x178c | `[0]` unspent Skills & Powers character points (the Core Rules nonweapon page also writes its Points Left here); `[1..5]` copper, silver, electrum, gold, platinum totals the equipment page caches. The wizard stores the starting gold in `[4]`; the page recomputes the totals from the coin items |
| 20 | `unknown_0x17bc` | u32[3] | always | +0x17bc, +0x17c0, +0x17c4 | movement rates the equipment page caches (`[0]` the base rate after armor); the sheet recomputes them. New Expansion characters get the race's or subrace's rate in `[0]` |
| 21 | `carried_weight` | f32 bits | always | +0x17b8 | weight carried in pounds. The Expansion trusts it (encumbrance, AC and THAC0 penalties), so it must change with the equipment; 2.0 writes 0 |
| 22 | `unknown_0x196c` | CStringArray[3] | always | +0x196c, +0x1988, +0x199c | psionic disciplines, sciences, devotions. The Psionics dialog writes the disciplines sorted (its list box sorts) and the powers in the order taken; a non-psionicist's wild talents are stored in `[1]` and `[2]` too |
| 23 | `race` | CString | always | +0x160 | race name |
| 24 | `subrace` | CString | always | +0x164 | subrace (`Hill dwarf`), may be empty |
| 25 | `kit` | CString | always | +0x15c0 | kit name, `None` for none |
| 26 | `unknown_0x124` | CString[3] | always | +0x124, +0x128, +0x12c | Skills & Powers weapon of choice, weapon expertise and weapon mastery, written by Update Character's Specialization dialog (`update/specialization.dat`) |
| 27 | `unknown_0x130` | u32 | always | +0x130 | an older single unarmed style (1990 punching, 1991 wrestling, 1992 martial arts), compared with those codes after loading; Update Character's Specialization dialog sets it to 0 whenever it closes |
| 28 | `unknown_0x134` | u32[4] | always | +0x134 to +0x140 | Skills & Powers shield proficiencies, 1 when held: buckler, small, medium, body shield (`update/shield-style-traits.dat`) |
| 29 | `unknown_0x144` | CString | always | +0x144 | unknown; empty in every fixture |
| 30 | `unknown_0x148` | u32[6] | always | +0x148 to +0x15c | unknown; 0 in every fixture |
| 31 | `unknown_0x1da4` | u32 | always | +0x1da4 | NPC mark: 1 for an NPC; copied into the entry's +0x24, set by Convert to NPC and cleared by Convert to Character |
| 32 | `unknown_0x186c` | u32 | always | +0x186c | read by the wild-talent test (0x42e832: non-zero, or a trait with part +0x744, makes the character a wild talent) |
| 33 | `unknown_0x1874` | u32 | always | +0x1874 | unknown |
| 34 | `unknown_0x1870` | u32 | always | +0x1870 | unknown |
| 35 | `unknown_0x1dc4` | u32 | always | +0x1dc4 | Expansion level-limit mode (Level dialog radios): 0 none, 1 racial limits, 2 limits with prime requisite bonuses. Experience is never capped, only the level |
| 36 | `flags_0x1ddc` | u8[2] | v > 30 | +0x1ddc | unknown; `[1, 0]` in new Expansion characters and NPCs |
| 37 | `flag_0x1dde` | u8 | v > 31 | +0x1dde | unknown |
| 38 | `flags_0x1ddf` | u8[6] | v > 32 | +0x1ddf | unknown |
| 39 | `flag_0x1de1` | u8 | v > 33 | +0x1de1 | unknown |
| 40 | `flags_0x1de6` | u8[2] | v > 36 | +0x1de6 | unknown |
| 41 | `flags_0x1de8` | u8[2] | v > 40 | +0x1de8 | unknown |
| 42 | `flags_0x1dea` | u8[3] | v > 34 | +0x1dea | unknown (note the condition: this field follows a v > 40 one) |
| 43 | `unknown_0x1df0` | (u32, u8) | v > 37 | +0x1df0, +0x1df4 | unknown; `(0xffffffff, 0)` in new Expansion characters and NPCs |
| 44 | `unknown_0x1df8` | u32 | v > 41 | +0x1df8 | unknown |
| 45 | `unknown_0x1dfc` | u32 | v > 44 | +0x1dfc | unknown |
| 46 | `unknown_0x39c` | CString | v > 46 | +0x39c | unknown |
| 47 | `unknown_0x1e50` | u32 | always | +0x1e50 | unknown |
| 48 | `flags_0x1e54` | u8[5] | v > 53 | +0x1e54 | Skills & Powers unarmed combat specializations, 1 when held: punching, wrestling, martial arts, then two flags nothing sets |
| 49 | `unknown_0x1e5c` | u32[3] | always | +0x1e5c | unknown |
| 50 | `alignment` | CString | always | +0x8c | alignment (`Lawful Good`) |
| 51 | `unknown_0x1db8` | CString[2] | always | +0x1db8, +0x1dbc | portrait: file name, then folder; the sheet uses `Bitmaps\Portrait\NA.bmp` when either is empty |
| 52 | `unknown_0x1e20` | CString | always | +0x1e20 | a nonweapon proficiency that costs nothing for a character with a kit (sheet slot count) |
| 53 | `unknown_0x1e88` | CString | always | +0x1e88 | unknown |
| 54 | `unknown_0x1e68` | CString[13] | v > 27 | see below | thirteen strings, members in archive order +0x1e68, +0x1e6c, +0x1e24, +0x1dc0, +0x1e14, +0x1e18, +0x1e1c, +0x1e70, +0x1e74, +0x1e78, +0x1e7c, +0x1e80, +0x1e84. `[3]` (+0x1dc0) is the party name; `[4]` and `[5]` (+0x1e14, +0x1e18) the two weapons of the Cavalier kit bonuses (the first matched as part of the weapon's name, the second exactly); `[6]` (+0x1e1c) the kit's focus weapon. The rest unknown |
| 55 | `sheet_options` | [SheetOptions](#ccharactersheetoptions-serialize-0x599eb8) | v > 28 | | the sheet sections the player chose |
| 56 | `classes` | [ClassList](#classlist) | always | +0x50, +0x190, +0x194 | the classes |
| 57 | `weapon_proficiencies` | CStringArray | always | +0xc0 | weapon proficiencies: names of Weapons parts, in the order taken |
| 58 | `unknown_0xac` | CStringArray[7] | always | +0xac, +0xd4, +0xe8, +0xfc, +0x110, +0x1e8c, +0x19b0 | `[0]` unknown; `[1]` and `[2]` Skills & Powers broad and tight weapon groups (old notes, from code only); `[3]` weapons the character specializes in (sheet `(Specialist)`); `[4]` Skills & Powers armor proficiencies (halve that armor's weight; Update Character's Shield and Armor dialog appends to it); `[5]` nonweapon proficiencies that get +1 to their score; `[6]` psionic attack modes |
| 59 | `unknown_0x19c4` | CStringArray | always | +0x19c4 | psionic defense modes, names the Psionic Attacks and Defenses dialog takes from the program (Tsrp3.exe 0x81a4c8 to 0x81a51c, attacks 0x81a468 to 0x81a4b4) |
| 60 | `racial_abilities` | CStringArray | always | +0x168 | racial ability part names (with the cost suffix the part name has); new characters store them sorted in byte order; Update Character writes the subrace's package plus the abilities bought, sorted as its list box sorts |
| 61 | `spells` | CStringArray | always | +0x3a0 | wizard spells known, in the order learned |
| 62 | `priest_spells` | CStringArray | always | +0x3b4 | priest spells known |
| 63 | `unknown_0x3c8` | ((CStringArray, CStringArray)[11])[5] | always | +0x3c8, +0x814 | memorized spells: per class slot (index into `classes.names`), per spell level 0 to 10 (0 unused), the wizard list (+0x3c8 + 0xdc * class + 0x14 * level) then the priest list (+0x814 + ...); one entry per memorized copy. A spellbook's contents belong to the spellbook item |
| 64 | `unknown_0x1878` | CStringArray | always | +0x1878 | unknown; only the copy constructor and Serialize touch it |
| 65 | `rogue_skills` | [RogueSkills](#ccharacterdocrogueskillsob-serialize-0x59a8ab) | always | +0x15c8 | thief and bard skills and the points added to them |
| 66 | `unknown_0xc60` | (u32[100])[5] | always | +0xc60 | hit point roll per class slot (400 bytes each) and level: `[g][L-1]` is the roll of level L. Losing levels keeps the rolls. Update Character's Hit Points arrows change them (0x5bf569, `advance.rs` `adjust_hit_points`) |
| 67 | `unknown_0x1430` | u32[100] | always | +0x1430 | PSPs gained at each level, `[L-1]` for level L (one array for the character); the sheet shows the sum. Creating a psionicist sets `[0]` once its class is chosen (0x5ba3e0): 15 + 1d6 + the Wisdom, Intelligence and Constitution points over 15 (0x42e916). Closing the Wild Talents dialog with talents and no PSPs sets `[0]` to 10 + 1d4 + the Wisdom, Constitution and Intelligence bonuses (0 below 16, then 1 per point); it then takes the old talents' initial costs (power +0x48) off `[0]` and adds the new ones' |
| 68 | `nonweapon_costs` | CDWordArray | always | +0x17e0 | what each nonweapon proficiency cost: slots (Core Rules) or character points (Skills & Powers); one per `nonweapon_proficiencies` entry |
| 69 | `nonweapon_bonuses` | CDWordArray | v > 50 | +0x17f4 | bonus ranks per nonweapon proficiency, added to its score; 0 wherever the program adds one; parallel |
| 70 | `class_slots` | [ClassSlot](#classslot)[5] | always | +0x1a8, +0x270, +0x2d4, +0x338 | per class slot: class abilities, schools, spheres |
| 71 | `armor` | CStringArray | always | +0x1790 | the Armor parts among the top-level worn items (`inventory[5]`), in bundle order. Rebuilt by the Expansion whenever it needs it; 2.0 leaves it empty |
| 72 | `unknown_0x17a4` | CStringArray | always | +0x17a4 | the other top-level worn items that are magical (part +0x224), rebuilt the same way |
| 73 | `nonweapon_proficiencies` | CStringArray | always | +0x17cc | nonweapon proficiency part names, in the order taken. A Core Rules proficiency taken again is another entry, a Skills & Powers one the same entry with a higher cost |
| 74 | `unknown_0x1830` | CStringArray[3] | always | +0x1830, +0x1844, +0x1858 | `[0]` Skills & Powers traits (Traits parts), `[1]` disadvantages (Disadvantages parts), `[2]` secondary skills; new characters store `[0]` and `[1]` sorted in byte order; Update Character appends to them in the order taken |
| 75 | `unknown_0x98` | CStringArray | always | +0x98 | part names the armor class computation reads (hands taken, part +0x24c, and a magic adjustment); empty in every fixture |
| 76 | `nonweapon_details` | CObArray<CStringArray> | always | +0x1808 | one `CStringArray` object per nonweapon proficiency, parallel: the languages learned through Modern Languages and the like, empty for the others. See [below](#cstringarray-objects-in-nonweapon_details) |
| 77 | `languages` | CStringArray | always | +0x181c | native languages, names of Languages parts, kept in alphabetical order; 2.0 writes none |
| 78 | `unknown_0x1930` | CStringArray[2] | v > 29 | +0x1930, +0x1944 | `[0]` unknown (old notes: bonus proficiencies a class requires, low confidence); `[1]` proficiencies chosen as class or kit bonuses, which the sheet counts as granted |
| 79 | `unknown_0x1dc8` | CStringArray | v > 35 | +0x1dc8 | unknown (old notes: free kit weapon specializations, low confidence) |
| 80 | `unknown_0x1958` | CStringArray | v > 38 | +0x1958 | nonweapon proficiencies that get +2 to their score |
| 81 | `inventory` | [CInventoryBundle](#inventory-bundles)[9] | always | +0x19d8 + 0x6c * i | the nine equipment bundles, inline |
| 82 | `unknown_0x1ea4` | (u8, CStringArray[2]) | v > 39 | +0x1ea4, +0x214c, +0x2160 | unknown; the rogue-skill total's kit adjustment (0x442f93) reads +0x2160 |
| 83 | `flag_0x1610` | u8 | v > 42 | +0x1610 | 1 once the kit's fixed rogue skill adjustments (kit +0x174) have been added to level 1, so they are added only once |
| 84 | `unknown_0x160c` | u32 | v >= 44 | +0x160c | unknown |
| 85 | `unknown_0x15e4` | (CObArray<CPartKitBonusOb>)[2] | v >= 46 | +0x15e4, +0x15f8 | the kit's rogue skill bonuses: `[0]` the random bonus of each level, `[0][L-1]` for level L, NULL for a level without one; `[1]` the fixed ones (skill, level 1, amount), NULL when the kit grants more than there are skills. See [below](#cpartkitbonusob-in-unknown_0x15e4-and-unknown_0x1e28) |
| 86 | `unknown_0x1e28` | CObArray<CPartKitBonusOb> | v > 47 | +0x1e28 | the nonweapon proficiency chosen per class and level: `name` the proficiency, `unknown_0x8` the level, `unknown_0xc` the class slot; the sheet counts the first match per level as granted |
| 87 | `unknown_0x1e3c` | CObArray | v > 49 | +0x1e3c | the NPC generator records here, per class, how many copies of a language-teaching proficiency it granted (0x4d9527); element class not identified, empty in every fixture |
| 88 | `flag_0x1ea8` | u8 | v > 48 | +0x1ea8 | unknown |
| 89 | `unknown_0x1e00` | CStringArray | v > 51 | +0x1e00 | unknown (old notes: kit school restrictions, low confidence) |
| 90 | `unknown_0x2174` | CObArray | v > 52 | +0x2174 | unknown; element class not identified, empty in every fixture |

Two conditions are out of version order: field 42 (v > 34) follows field 41
(v > 40), and field 87 (v > 49) comes before field 88 (v > 48). The reader
follows the archive order and tests each field's own condition.

Fields first present in each version:

| v | fields added |
|---|---|
| 27 | none of the conditional fields |
| 28 | `unknown_0x1e68` |
| 29 | `sheet_options` (the 2.0 layout) |
| 30 | `unknown_0x1930` |
| 31 | `flags_0x1ddc` |
| 32 | `flag_0x1dde` |
| 33 | `flags_0x1ddf` |
| 34 | `flag_0x1de1` |
| 35 | `flags_0x1dea` |
| 36 | `unknown_0x1dc8` |
| 37 | `flags_0x1de6` |
| 38 | `unknown_0x1df0` |
| 39 | `unknown_0x1958` |
| 40 | `unknown_0x1ea4` |
| 41 | `flags_0x1de8` |
| 42 | `unknown_0x1df8` |
| 43 | `flag_0x1610` |
| 44 | `unknown_0x160c` |
| 45 | `unknown_0x1dfc` |
| 46 | `unknown_0x15e4` |
| 47 | `unknown_0x39c` |
| 48 | `unknown_0x1e28` |
| 49 | `flag_0x1ea8` |
| 50 | `unknown_0x1e3c` |
| 51 | `nonweapon_bonuses` |
| 52 | `unknown_0x1e00` |
| 53 | `unknown_0x2174` |
| 54 | `flags_0x1e54` (the Expansion layout) |

A version above 54 reads like 54.

### ClassList

Inline, custom code (Rust `ClassList`). The count is a plain u32, not a
WriteCount.

| # | field | type | member | meaning |
|---|---|---|---|---|
| 1 | count | u32 | +0x190 | number of classes |
| 2 | `unknown_0x50` | u32[10] | +0x50 | Skills & Powers fighting styles in the order learned, 0 for none: ids 1993 to 1999 (one-handed weapon, weapon and shield, two-handed weapon, two weapon, missile, horse archer, thrown weapon/sling), 2001 two weapon +2 and 2002 one-handed weapon +2 (2000 is unused; the program's style list, built at 0x464d71). Not class data: a new class leaves it alone |
| 3 | `names` | CString[count] | +0x194 | class names in slot order, resolved to `CClassOb`s by name at load (pointers at +0x194 + 4 * slot) |

The class slot g (0 to 4) indexes `unknown_0x1618`, `unknown_0xc60`,
`unknown_0x3c8` and `class_slots`. There is no dual-class flag: a character
with two or more classes is dual-classed when its race can dual-class, and
multi-classed otherwise. Dual Class (0x619f14) appends the name, extends
`class_title` and fills the new slot's `class_slots` entry, and leaves the
experience at 0 (level 1).

### CCharacterDocRogueSkillsOb (Serialize 0x59a8ab)

Rust `RogueSkills`. rtc 0x7a7b10, size 0x1c; called directly, so inline with
no tag. In memory a `CObArray` of entries plus the points left over (+0x18,
not saved).

| # | field | type | meaning |
|---|---|---|---|
| 1 | count | u32 | number of skills |
| 2 | `entries` | (CString, u32[100])[count] | per skill: the class ability that grants it, as a part name (`Thief, Pick pockets* (10)`), and the points added at each level, `[L-1]` for level L |

The sheet's discretionary column is the sum of all 100 values; the base,
racial, ability, armor, trait and kit parts are computed. A new character's
1st-level points go in `[0]`. The list holds the skills (class abilities with
part +0x77c = 6) of every class with class flag +0x27a4, each name once.

Update Character's Rogue Skills dialog changes only the column of the rogue
class's current level, `[L-1]`. The points to put in are class +0xb4 at 1st
level and +0xb8 at each level after (the best over the character's rogue
classes; thief 60 and 30, bard 20 and 15), less what the earlier columns
hold. With class flag +0x278c (thief) one skill may get at most half of the
points available at the level, which the dialog asks before exceeding, as it
asks before a skill passes 95%.

### CCharacterSheetOptions (Serialize 0x599eb8)

Rust `SheetOptions`, inline, with its own version. The Expansion writes
version 2; 2.0 writes version 2 as well.

| # | field | type | present | member | meaning |
|---|---|---|---|---|---|
| 1 | `version` | u32 | always | | 2; 1 is an older layout ("Attempting to import CCharacterSheetOptions version 1. I'm %d") |
| 2 | `unknown_0x260` | u32 | always | +0x260 | 0: never configured, the sections are the defaults (0x598347); 1: the stored list is used as is, in its order |
| 3 | `unknown_0x25c` | u32 | always | +0x25c | border style for printing: 0 none, 1 thin, 2 normal. Fresh 2.0 files hold 1822, which 2.0 does not use; new characters get 0 in the Expansion and 1822 in 2.0 from the app |
| 4 | `entries` (Objects) | CObArray | version = 1 | | the old list; element class not identified |
| 4 | `entries` (Pairs) | u32 count, then (CString, u32)[count] | version != 1 | | the selected sections in output order: the section's name as listed and its id (0 Personal Information to 25). The program uses only the id |

The section ids and names are in
[`crates/cr-store/src/sheet.rs`](../../crates/cr-store/src/sheet.rs)
(`SECTIONS`). View, Print and Export save the choice back into the `.dat`
without touching `index.chr`. 2.0 always prints its fixed list.

### ClassSlot

Inline record (Rust `ClassSlot`), five back to back. Slot g's lists are at
+0x1a8, +0x270, +0x2d4 and +0x338, plus 0x14 * g.

| # | field | type | meaning |
|---|---|---|---|
| 1 | `abilities` | CStringArray | class abilities as part names (`Fighter, Followers (5)`): the class's default abilities that exist as Class Abilities parts, or those chosen with Skills & Powers. Resolved at load |
| 2 | `unknown_0x270` | CStringArray | schools of magic of the class (from the class record); Update Character's Schools dialog adds and removes them (5 points each, 15 for a class limited to one school) |
| 3 | `unknown_0x2d4` | CStringArray | minor spheres (from the class record); the Spheres dialog buys them at the sphere part's `unknown_0x218` |
| 4 | `unknown_0x338` | CStringArray | major spheres (from the class record); the Spheres dialog buys them at the part's `unknown_0x308`, or a mage's single sphere for 15 |

Unused slots hold four empty arrays.

### Inventory bundles

`inventory` is nine embedded `CInventoryBundle`s (Serialize 0x4aa81b) back
to back, each written inline with no class tag: a count and that many
`CInventoryItem`s, also inline. An item's contents are a tagged
`CInventoryBundle` object or NULL. The layouts are in
[misc.md](misc.md#cinventoryitem-not-tagged-serialized-inline) (item, page
record) and [monster.md](monster.md#cinventorybundle-schema-1-size-0x6c-serialize-0x4aa81b)
(bundle).

| i | member | bundle |
|---|---|---|
| 0 | +0x19d8 | Animals |
| 1 | +0x1a44 | Daily Food and Lodging |
| 2 | +0x1ab0 | Items Carried |
| 3 | +0x1b1c | Items Readied |
| 4 | +0x1b88 | Items Stored |
| 5 | +0x1bf4 | Items Worn |
| 6 | +0x1c60 | Spending Money |
| 7 | +0x1ccc | Services |
| 8 | +0x1d38 | Transport |

* Items are part names (equipment, weapons, armor, magical items, gems,
  currency). `quantity` is the count; `unknown_0x8` is 999 and `flag_0x4` 0
  in everything the program writes.
* Each bundle, and each contents bundle, is sorted by name in byte order:
  InsertItem (0x4aa8e3) inserts before the first item whose name compares
  greater with `strcmp`, so the coins (`* Gold Pieces` and so on) come first.
  When buying (0x4aab4e) an item without contents merges into an item of
  the same name without contents (the quantities add).
* Money is the coin items of bundle 6 (`* Copper Pieces` to
  `* Platinum Pieces`); purchases pay from it. `unknown_0x1778[1..5]` only
  caches the totals.
* An item whose part has pages (part +0x7e4: spellbooks, spell scrolls)
  carries the page record behind its presence byte: capacity, pages used,
  spell count, the spell names, the pages of each and 1 (wizard) or 0
  (priest) for each. Buying one creates it empty with the part's capacity.
* The weight in `carried_weight` is the sum over bundles 2, 3 and 5,
  contents included, of the part weight times the quantity (the Core Rules
  weight when the character uses the Core Rules and the part has one, else
  the base weight); unknown items weigh nothing.
* A new character's standard pack goes into Items Carried with its contents
  as the pack's bundle; weapons go into Items Readied, armor into Items
  Worn, everything else into Items Carried.

### CStringArray objects in nonweapon_details

`nonweapon_details` is a `CObArray` whose elements are MFC `CStringArray`
objects: a class tag (`CStringArray`, schema 0; the first one in the archive
carries the full tag, later ones a class reference), then the array's own
Serialize, a count and the strings. The program creates one with every
nonweapon proficiency it adds (0x4300b5) and reads `[i]` without a bounds
check (0x430187), so `nonweapon_proficiencies`, `nonweapon_costs`,
`nonweapon_bonuses` (from v 51) and `nonweapon_details` must have the same
length. The app keeps them parallel: a proficiency it keeps keeps its cost,
bonus and details; a new one costs what its part says, with bonus 0 and an
empty array.

### CPartKitBonusOb in unknown_0x15e4 and unknown_0x1e28

`CPartKitBonusOb` (rtc 0x84cb00, schema 0x80000001 in the class, written as
1 in the tag, size 0x10, Serialize 0x6726cd; Rust
`cr_formats::classes::KitBonus`, also used in `Class.dat`, see
[classes.md](classes.md)):

| # | field | type | meaning here |
|---|---|---|---|
| 1 | `version` | u32 | 1 |
| 2 | `name` | CString | the rogue skill (`unknown_0x15e4`) or the nonweapon proficiency (`unknown_0x1e28`); empty in a random-bonus entry for a level where no skill could take it |
| 3 | `unknown_0x8` | i32 | level |
| 4 | `unknown_0xc` | i32 | `unknown_0x15e4`: the amount added; `unknown_0x1e28`: the class slot |

In `unknown_0x15e4` each array element is an object or NULL (u16 0): the
random bonuses grow with SetAtGrow, which leaves NULL for the levels
between.

## Saving a 2.0 character in the Expansion

`Character::upgraded` is what the Expansion writes when it saves a
character it loaded from an older version (an imported `.xpt` character,
a 2.0 character put in a party): the version string becomes `TSRP3 V54` and
every field the old version lacked takes the value the constructor gives
it. The app uses it on import into the Expansion and in `set_party`.

| field | value |
|---|---|
| `flags_0x1ddc` | `[1, 0]` |
| `unknown_0x1df0` | `(0, 0)` (a new Expansion character has `(0xffffffff, 0)`) |
| `sheet_options` (only for v < 29) | version 0, 0, 0, no entries |
| `unknown_0x1e68` (only for v < 28) | thirteen empty strings |
| every other absent field (`flag_0x1dde`, `flags_0x1ddf`, `flag_0x1de1`, `flags_0x1de6`, `flags_0x1de8`, `flags_0x1dea`, `unknown_0x1df8`, `unknown_0x1dfc`, `unknown_0x39c`, `flags_0x1e54`, `nonweapon_bonuses`, `unknown_0x1930`, `unknown_0x1dc8`, `unknown_0x1958`, `unknown_0x1ea4`, `flag_0x1610`, `unknown_0x160c`, `unknown_0x15e4`, `unknown_0x1e28`, `unknown_0x1e3c`, `flag_0x1ea8`, `unknown_0x1e00`, `unknown_0x2174`) | 0, empty strings, empty arrays |

For a `TSRP2 V29` file that is everything from `unknown_0x1930` on in the
version table. Fields the old file had keep their values: the weight 0, the
empty worn lists and 2.0's sheet options stay as 2.0 wrote them.
`nonweapon_bonuses` starts empty even when the character has proficiencies;
the app's list editor fills it with one 0 per proficiency when it next saves
the lists.

## Characters inside other files

A character inside another file is the same stream, version string first,
read by the same `Character` code:

* an encounter's NPC package (`CEncounterNPCPackageOb`, field `character_4`;
  see [monster.md](monster.md)), `TSRP3 V54` or `TSRP2 V29`;
* the characters of a DM Toolkit package, after the character count
  ([export.md](export.md)). The export flag at doc +0x1ea1 only changes
  the messages.

The embedded character shares the surrounding archive, so its class tags
(`CStringArray`, `CInventoryBundle`, `CPartKitBonusOb`) may be references to
classes an earlier section or character introduced, and later sections may
refer to classes it introduced.

## Notes

* `flag_0x28` of an entry: 0x5a08fc sets it on a scratch entry, but the
  copy the list keeps (0x5a1372 through 0x5a05db) skips +0x28, so the
  Expansion saves whatever its heap held for added, imported and re-saved
  characters. The app writes the scratch value (`!skills_and_powers`); 2.0
  entries and NPCs carry 1. No code that reads it was identified.
* `extra.unknown` (8 in every Expansion entry) is dropped on load; its
  meaning is unknown.
* The reader takes only new-object tags and class references (and NULL
  where a field allows it) inside the character; a back-reference to an
  object already read is an error. The originals never write one here.
* 2.0 writes the u32 at `unknown_0x1dc4` too, but has no level limits; what
  it means there is unknown. Some 2.0 fixtures hold large values in
  `unknown_0x17bc` (2.0 caches its movement elsewhere).
* Unknown names in any list (weapons, proficiencies, languages, spells,
  items) do not stop either program from loading or printing a character;
  the Update wizard's pages report "Error finding ..." for names they cannot
  resolve.
* Not identified: the element classes of `unknown_0x1e3c`, `unknown_0x2174`
  and the version 1 sheet options; the meaning of the flags at +0x1ddc to
  +0x1dec and most other `unknown_*` fields marked unknown above.
