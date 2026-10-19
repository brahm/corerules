# Program images (Tsrp3.exe, Evermore.exe)

Files: `Program/Tsrp3.exe` (the Expansion, AD&D Core Rules 2.0 Expansion) and
`Program/Evermore.exe` (AD&D Core Rules 2.0). They are not data files and have no
CArchive framing (see [archive.md](archive.md) for that); the app opens them read only,
as a source of text, and never writes them. There is no document class, Serialize or
version: one fixed build of each program is supported (see
[Identification](#identification)). The app keeps none of the programs' own text: it keeps addresses
(and, for a few functions, the shape of the code around them) and reads the strings
from the installation's program each time it loads the feature that needs them.

Rust modules:

| module | what it reads |
|---|---|
| [`exe.rs`](../../crates/cr-store/src/exe.rs) | `Image`: identification, PE sections, and the four kinds of read (string at an address, pushed strings, stack tables, id tables) |
| [`npc/program.rs`](../../crates/cr-store/src/npc/program.rs), [`npc/texts.rs`](../../crates/cr-store/src/npc/texts.rs), [`npc/names.rs`](../../crates/cr-store/src/npc/names.rs), [`npc/data.rs`](../../crates/cr-store/src/npc/data.rs) | the NPC generator's texts, name tables, rule lists and the tight weapon groups (through `Image`) |
| [`treasure/intelligent.rs`](../../crates/cr-store/src/treasure/intelligent.rs) | the intelligent weapon ability lines (through `Image`) |
| [`sheet/build/combat.rs`](../../crates/cr-store/src/sheet/build/combat.rs) | the Turning Undead labels and legend of the character sheet (through `Image`) |
| [`export.rs`](../../crates/cr-store/src/export.rs) | the class names a Core Rules 2.0 part's old class flags become on import (through `Image`) |
| [`userdb/program.rs`](../../crates/cr-store/src/userdb/program.rs) | the fixed lists the custom-data editors offer (its own PE reader, `strings_at`) |
| [`tables/program.rs`](../../crates/cr-store/src/tables/program.rs), [`tables/strings.rs`](../../crates/cr-store/src/tables/strings.rs) | the text the Table Rolls print from the programs' own code (its own PE reader, `Program`) |

`encounter/text.rs`, `treasure.rs` (apart from the intelligent weapons), `treasure/letters.rs`
and `books.rs` do not read the program; the encounter screens reach it only through the NPC
generator (encounter NPC packages) and the treasure roller.

Which program is read: the installation's flavor. An installation is the Expansion when it
has `UserDbas/AllowedU.dat` and Core Rules 2.0 otherwise (`encounter::flavor_of`); the
Expansion reads `Tsrp3.exe` and Core Rules 2.0 reads `Evermore.exe`. Every lookup of the
path ignores case (`Installation::path`). Every address in this document is a virtual
address (image base included); "Tsrp3.exe" and "Evermore.exe" columns give the address in
each program, `0` or `-` where that program has no such string.

## Identification

`Image::open` (exe.rs) and `tables::program::Program::load` accept a program only when its
size and its FNV-1a 64 hash (offset basis `0xcbf29ce484222325`, prime `0x100000001b3`,
over the whole file) are the ones below:

| flavor | path | size (bytes) | FNV-1a 64 |
|---|---|---|---|
| Expansion | `Program/Tsrp3.exe` | 5,033,984 | `0xcf096cd3eb7d22b1` |
| Core Rules 2.0 | `Program/Evermore.exe` | 3,423,232 | `0x822f6926c2e4f36e` |

The constants are `PROGRAMS` in exe.rs and the same values again as `BUILDS` in
tables/program.rs. `userdb/program.rs` does no identification (see
[The three readers](#the-three-readers)).

## PE layout and address translation

Both programs are 32-bit PE images (optional header magic `0x10b`, optional header size
`0xe0`, image base `0x400000`) with four sections:

| section | Tsrp3.exe VA | vsize | raw | rsize | Evermore.exe VA | vsize | raw | rsize | characteristics | kind |
|---|---|---|---|---|---|---|---|---|---|---|
| `.text` | 0x401000 | 0x397b76 | 0x1000 | 0x398000 | 0x401000 | 0x258642 | 0x400 | 0x258800 | `0x60000020` | code |
| `.rdata` | 0x799000 | 0x73a7c | 0x399000 | 0x74000 | 0x65a000 | 0x66290 | 0x258c00 | 0x66400 | `0x40000040` | data |
| `.data` | 0x80d000 | 0x5a5c8 | 0x40d000 | 0x53000 | 0x6c1000 | 0x2d868 | 0x2bf000 | 0x27000 | `0xc0000040` | data |
| `.rsrc` | 0x868000 | 0x6c010 | 0x460000 | 0x6d000 | 0x6ef000 | 0x5db40 | 0x2e6000 | 0x5dc00 | `0x40000040` | data |

Every string the app reads is in `.data` of the program (Tsrp3.exe 0x80e8fc..0x84e190,
Evermore.exe 0x6c119c..0x6e1854), except one Evermore address noted under
[Table Rolls](#table-rolls); the code reads are in `.text`.

Parsing, the same in all three readers:

| offset | type | field |
|---|---|---|
| 0x3c | u32 | `e_lfanew`: file offset `pe` of the PE signature |
| pe | 4 bytes | `PE\0\0` (checked by tables/program.rs and userdb/program.rs only) |
| pe + 6 | u16 | number of sections |
| pe + 20 | u16 | size of the optional header |
| pe + 24 + 28 | u32 | `ImageBase` (the PE32 position; a PE32+ image would be misread) |
| pe + 24 + optional size + 40 * i | 40 bytes | section header i |

Of each section header the readers take: +8 u32 virtual size, +12 u32 virtual address
(RVA), +16 u32 raw size, +20 u32 raw (file) offset, and (exe.rs only) +36 u32
characteristics. The section's VA is `ImageBase + RVA`. A virtual address `va` maps to the
file offset `va - section VA + raw offset` of the first section whose range holds it.

The extent of a section differs between readers: exe.rs uses `max(vsize, rsize)`, the two
others `min(vsize, rsize)`. exe.rs calls a section a data section when the
`IMAGE_SCN_CNT_CODE` bit (`0x20`) of its characteristics is clear, so `.rdata`, `.data`
and `.rsrc` are data and `.text` is code.

## The three readers

| | `exe::Image` | `tables::program::Program` | `userdb::program::strings_at` |
|---|---|---|---|
| identification | size + FNV-1a 64 | size + FNV-1a 64 | none |
| `PE\0\0` check | no | yes | yes |
| section extent | max(vsize, rsize) | min(vsize, rsize) | min(vsize, rsize) |
| string must be in a data section | yes | no | no |
| bytes accepted | any; read as Latin-1 | 0x20..0x7e only, else no string | 0x20..0x7e only and shorter than 200 bytes, else the whole list fails |
| NUL must be found | before the end of the section | before the end of the section | anywhere after the start in the file |
| address 0 | not in any section: no string | "the program has none": no string | an empty string |
| loaded | on each `Image::open` | once per Table Rolls load (`TableRolls::program`) | on each list request (whole file read) |

## Kinds of read

### A string at an address

`Image::text(va)`: `None` unless `va` lies in a data section; otherwise the bytes from `va`
up to the first NUL inside the section, each byte one char (Latin-1). `Program::get` and
`strings_at` do the same with the differences in the table above. An address may point
into the middle of a string; the read then gives its tail (two Table Rolls entries do).

### Strings pushed by code

`Image::pushed_texts(va, len)`: the strings that `push offset string` instructions
(`68 imm32`) put on the stack in the `len` bytes of code from the function at `va`, in code
order. The scan is byte by byte, not instruction by instruction: at every `0x68` byte whose
next four bytes are an address with a string in a data section (`text` above), the string
is taken and the scan goes on 5 bytes later; anything else advances one byte. A stray
`0x68` byte followed by a data address can therefore add a string, so each caller gives a
length that ends inside the function and asks for an exact number of strings: fewer is an
error, the rest are dropped (`take` in npc/program.rs).

### Name tables built on the stack

`Image::stack_tables(va, len, picker)`: the Tsrp3.exe name generators fill arrays of string
pointers in their stack frame, one `mov dword [ebp+d], offset string` per entry and a NULL
after the last, then pass the array to a picker (`lea reg, [ebp+d]; push reg; call
picker`). The scan, over `len` bytes from `va`:

| bytes | instruction | effect |
|---|---|---|
| `c7 45 d8 imm32` | `mov dword [ebp+d8], imm32` | slot d8 (sign-extended) := imm32; 7 bytes |
| `c7 85 d32 imm32` | `mov dword [ebp+d32], imm32` | slot d32 := imm32; 10 bytes |
| `8d m d8`, `m & 0xc7 == 0x45` | `lea r32, [ebp+d8]` | remember d8; 3 bytes |
| `8d m d32`, `m & 0xc7 == 0x85` | `lea r32, [ebp+d32]` | remember d32; 6 bytes |
| `e8 rel32` | `call va + i + 5 + rel32` | when the target is `picker` and a `lea` was seen since the last call: the table at the first remembered displacement d is the strings of slots d, d+4, d+8, ... up to a missing or 0 slot (a slot that is not a data string reads as an empty string). Only the first table seen at each displacement is kept. Every call forgets the remembered `lea`s; slot values persist, as the frame does. 5 bytes |
| anything else | | 1 byte |

The scan stops 10 bytes before the end. The result is keyed by displacement; npc/program.rs
stores each table under (function, local) with `local = 4 - d` (the table at `[ebp-0x190]`
is local `0x194`).

Evermore.exe is an optimized build whose generators load table entries from registers, so
this scan does not work there; npc/program.rs lists every entry's address instead
(`EV_NAME_TABLES`, keyed by the Tsrp3.exe (function, local) of the same table).

### The weapon group tables of the part document constructor

`Image::id_tables(va, len, add)`: the part document constructor (Tsrp3.exe `0x677978`)
fills name-to-id maps with a run of calls of the form

```
6a id            push id                 (imm8, read unsigned)
51               push ecx
8b cc            mov ecx, esp
89 65 d8         mov [ebp+d8], esp       (or 89 a5 d32: mov [ebp+d32], esp)
68 imm32         push offset name        (must be a data string)
...              (the CString temporary is built)
81 c1 imm32      add ecx, table          (the document member offset)
e8 rel32         call add                (target va + i + 11 + rel32)
```

Each matched `push id ... push offset name` adds (name, id) to a pending list; an
`add ecx, imm32; call rel32` pair moves the pending entries into the table `imm32` when the
call goes to `add` and drops them otherwise. The scan stops 13 bytes before the end. The
result is keyed by member offset; the NPC generator takes member `0x248`, the tight weapon
groups (`Data::tight_groups`, name to id).

### Resources and message boxes

The only resources read are dialog templates: the About window's two
([help.md](help.md#the-about-box)) and the screens' hotspots
([help.md](help.md#right-click-help)). `Image::dialog_texts` takes the resource directory from
data directory 2 of the optional header (offset `pe + 24 + 96 + 16`) and walks it type →
name → first language: a directory has u16 named and id entry counts at 12 and 14, then
8-byte entries (u32 name or id, u32 target; a set high bit points to a subdirectory), and the
leaf gives the data's RVA and size. A `DLGTEMPLATE` (or `DLGTEMPLATEEX`, which starts
`1, 0xFFFF`) gives its caption and, from each item (DWORD-aligned), the control's help id
(`DLGTEMPLATEEX` only), style, place, id, class and text, `&` mnemonics removed unless a
static control (class `0xFFFF 0x0082`) has `SS_NOPREFIX` (0x80); the About window takes the
static controls top to bottom and left to right. No string tables, menus or version
resources are read. Otherwise the dialogs, messages and message boxes the app shows are its
own text, with one exception: the NPC generator's error when no class is given is the
program's own string (`NO_CLASS` below), returned as the error text.

### Not read: kept as values

A few things the program also holds are kept in the app as values, not read: the
weight-allowance-by-Strength column of 0x4290f5 (`npc::program::CAPACITY`), the
intelligence, communication, alignment and language chains of the intelligent weapon roller
(treasure/intelligent.rs), and the Turning Undead results of the character sheet
(sheet/build/combat.rs). They are numbers or short labels in the code, not reads.

## What is read from where

### Code reads

| Tsrp3.exe | Evermore.exe | read | length | picker / add | what it gives | count | used by |
|---|---|---|---|---|---|---|---|
| 0x4b8b0b, 0x4b8f34, 0x4b928b, 0x4b97dd, 0x4ba82f, 0x4ba740, 0x4bc074, 0x4bbd92, 0x4bbbda, 0x4bba12, 0x4bb80a, 0x4bae7e, 0x4bb2d7, 0x4bb4b5, 0x4babc0 | listed entries (`EV_NAME_TABLES`) | stack tables | 0x3000 each | 0x4b8eda | the name generators' syllable and name tables, see [NPC name tables](#npc-name-tables) | 56 tables | npc/names.rs |
| 0x4b9da3 | 0x536600 | pushed strings | 0x1000 | | the race keywords of the name dispatcher, in test order (an upper-cased subrace name containing keyword i picks generator i) | 31 | npc/names.rs |
| 0x54f66a | 0x5d4dc2 | pushed strings | 0x800 | | the background list (the empty-background entry first; the app object keeps it at +0x1f84) | 21 | npc/state.rs |
| 0x4ce3d6 | - | pushed strings | 0x4000 | | the common-weapon tests on an upper-cased weapon name, laid out by `COMMON_SHAPES`: 3 equality tests, 8 contains tests, 1 parent-race test, 4 parent-race-equal tests, 1 race-equal test, a level gate (above 1, go to rule 19), 1 contains test, a level gate (below 4, no hit), 102 contains tests; a race test takes two strings (race, then the text looked for) | 126 | npc/prof.rs |
| 0x4d6dc6 | - | pushed strings | 0x800 | | the excluded-weapon tests up to the call of 0x43fac6, laid out by `EXCLUDED_SHAPES`: 1 race-equal test, 4 parent-race-equal tests, 3 contains tests, then the kit tests | 13 | npc/prof.rs |
| 0x4d8040 | - | pushed strings | 0x400 | | the upper-cased words of a club-like weapon (the club kit) | 6 | npc/prof.rs |
| 0x4d81ad | - | stack table at `[ebp-0x24]` | 0x400 | 0x4c2042 | the club kit's replacement weapons | all entries | npc/prof.rs |
| 0x4d7d99 | - | pushed strings | 0x2a0 | | the upper-cased words of the missile weapons (guns and engines) a kit's +0x3fc missile allowance does not cover | 11 | npc/prof.rs |
| 0x677978 | - | id tables | 0x3800 | 0x572030 | member 0x248: the tight weapon groups, name to id (used for the sword test, a tight group of the broad sword group, and for the specialization cost, 2 for a weapon with a tight group of id 6) | all entries | npc/prof.rs |

The count column is what the app keeps; for pushed strings, fewer found is an error and
more are dropped. Only the Expansion reads the code-shape entries; the Core Rules 2.0
branch of `npc::program::Program::read` reads the race keywords, backgrounds and listed
name tables and leaves the weapon tests, club and missile lists empty.

### NPC generator texts

`npc/texts.rs` (`ALL`) plus `NORM` and `BUP` of npc/program.rs: 239 strings, read when the
NPC generator loads (`Program::read`). Each is compared with a part, race, kit or class
name, searched for inside one, or copied into the NPC. Every Tsrp3.exe address must hold a
data string or the load fails; an Evermore.exe address of 0 (60 of them) is skipped and
reads as `"\u{1}"`, which equals and is contained in no name. Grouped by role, Tsrp3.exe
addresses ascending, Evermore.exe addresses in the same order:

| role | module (npc/) | n | Tsrp3.exe | Evermore.exe |
|---|---|---|---|---|
| ability names (compared with the names a part or kit gives) | prof.rs | 6 | 0x811120, 0x81112c, 0x811138, 0x811148, 0x811150, 0x811160 | 0x6c27a8, 0x6c279c, 0x6c278c, 0x6c2784, 0x6c2774, 0x6c2768 |
| sub-ability names (Skills & Powers) | prof.rs | 12 | 0x81118c, 0x8111a0, 0x8111b0, 0x8111c0, 0x8111d4, 0x8111e8, 0x811200, 0x811214, 0x81122c, 0x811240, 0x811254, 0x811268 | 0x6c2754, 0x6c2744, 0x6c2734, 0x6c2720, 0x6c270c, 0x6c26f4, 0x6c26e0, 0x6c26c8, 0x6c26b4, 0x6c26a0, 0x6c268c, 0x6c2678 |
| alignment names (the alignment pick and kit equipment rules) | equip.rs, state.rs | 9 | 0x80e8fc, 0x80ea2c, 0x80ea3c, 0x80ea4c, 0x80ea58, 0x80ea68, 0x80ea84, 0x80eab0, 0x817388 | 0x6c15d8, 0x6c1700, 0x6c16f0, 0x6c1730, 0x6c1720, 0x6c1710, 0x6c173c, 0x6c174c, 0x6c52fc |
| social class names | equip.rs, state.rs | 3 | 0x8108a8, 0x81fb24, 0x820084 | 0x6c2134, 0x6c52b0, 0x6d68a0 |
| race and subrace names the generator compares a race with | equip.rs, prof.rs, state.rs | 20 | 0x810904, 0x811578, 0x816d38, 0x816d40, 0x816d4c, 0x81711c, 0x81f268, 0x820108, 0x820118, 0x820134, 0x820140, 0x82014c, 0x820158, 0x820528, 0x820548, 0x820924, 0x820964, 0x82096c, 0x82198c, 0x821994 | 0x6c215c, 0x6c2350, 0x6ccff0, 0x6ccfe4, 0x6ccfe0, 0x6cd2cc, 0x6d61cc, -, -, 0x6c2344, 0x6c2338, 0x6c232c, -, -, -, 0x6c451c, -, -, -, - |
| race-name prefixes ending in a comma (subrace names), tested in the language pick | prof.rs | 4 | 0x8218a8, 0x8218b4, 0x8218bc, 0x8218d0 | -, -, -, - |
| race adjectives a weapon or item name contains (racial items) | equip.rs | 2 | 0x820974, 0x820984 | -, - |
| racial equipment group names (the racial equipment list) | equip.rs | 4 | 0x820594, 0x8205b0, 0x8205d0, 0x8205e4 | 0x6d68ec, 0x6d68d8, 0x6d68c8, 0x6d68b4 |
| equipment category names (the part category of an item) | equip.rs | 12 | 0x817510, 0x821b18, 0x821b20, 0x821b2c, 0x821b38, 0x821b40, 0x821b48, 0x821b50, 0x821b74, 0x821b7c, 0x821b8c, 0x821ba4 | 0x6c3be0, 0x6c36c8, 0x6c43b8, 0x6c4384, 0x6c437c, 0x6c4390, 0x6c43a8, 0x6c4364, 0x6c43b0, 0x6c4398, 0x6c4680, 0x6c4664 |
| language names | prof.rs | 2 | 0x8170d8, 0x82016c | 0x6cd2f0, - |
| nonweapon proficiency names (proficiency tests and the items a proficiency brings) | prof.rs, equip.rs | 25 | 0x80edb4, 0x811368, 0x811a98, 0x815d74, 0x816b54, 0x816d74, 0x816dbc, 0x816e04, 0x816edc, 0x816f60, 0x816f90, 0x816fdc, 0x817000, 0x81700c, 0x81702c, 0x817038, 0x81704c, 0x817064, 0x820b0c, 0x820b38, 0x820b54, 0x820bb0, 0x820c70, 0x820c90, 0x821820 | 0x6c2b2c, 0x6c2b40, 0x6cd434, 0x6cca40, 0x6cbf28, 0x6cd29c, 0x6cd258, 0x6cd214, 0x6c2abc, 0x6c2b0c, 0x6c2ae8, 0x6cd07c, 0x6cd05c, 0x6c2ad8, 0x6c2b00, 0x6c2b38, 0x6c2af4, 0x6cd020, 0x6d6c18, 0x6d6bf4, 0x6d6bd8, 0x6d6b88, -, -, 0x6c2acc |
| thief skill names (rogue skill points) | rogue.rs | 13 | 0x839514, 0x839524, 0x839530, 0x839544, 0x839554, 0x839564, 0x839574, 0x839580, 0x839590, 0x8395a0, 0x8395b0, 0x8395b8, 0x8395c4 | 0x6c3324, 0x6c3318, 0x6c3304, 0x6c32f4, 0x6c32e4, 0x6c32d4, 0x6c32c8, 0x6c32b8, 0x6c32a8, 0x6c3298, 0x6c3290, 0x6c3284, 0x6c3274 |
| psionic discipline names | magic.rs | 5 | 0x81e274, 0x81e284, 0x81e294, 0x81e2a4, 0x81e2b4 | 0x6c6034, 0x6c6024, 0x6c6014, 0x6c6004, 0x6c5ff8 |
| spell name the wizard spell book always holds; spell book item name | magic.rs | 2 | 0x813a50, 0x822160 | 0x6cbb0c, 0x6c7094 |
| class trait names of a class part (armor and magic item restrictions) | prof.rs | 8 | 0x811660, 0x8116a8, 0x8116c4, 0x8116e4, 0x811704, 0x811718, 0x811734, 0x811754 | 0x6c2a44, 0x6c29f0, 0x6c29d0, 0x6c29b0, 0x6c299c, 0x6c2980, 0x6c2960, 0x6c2940 |
| kit names with their own equipment or social rules | equip.rs, state.rs | 4 | 0x81089c, 0x820cb0, 0x820cc8, 0x820cdc | 0x6c2140, 0x6d6b30, 0x6d6b1c, 0x6d6b08 |
| words a kit, weapon or item name is searched for (kit equipment rules) | equip.rs | 14 | 0x820938, 0x82094c, 0x820958, 0x820a34, 0x82134c, 0x821368, 0x821370, 0x8213c4, 0x8213cc, 0x8213e8, 0x821404, 0x821420, 0x821440, 0x821448 | 0x6d6aa8, 0x6d6a9c, -, -, -, -, -, 0x6d5ba4, -, -, -, -, -, - |
| coin names (the purse) | equip.rs | 3 | 0x8177b8, 0x8177c8, 0x8177d8 | 0x6c48b4, 0x6c48a4, 0x6c4880 |
| sentinels of a weapon-allowed list: the entry that limits it to the listed items, and the damage type every list allows | prof.rs | 2 | 0x84c814, 0x84c82c | -, - |
| the error message when no class is given | state.rs, npc.rs | 1 | 0x81fa3c | 0x6d6808 |
| fixed first names of the gnome and half-orc generators | names.rs | 6 | 0x81f280, 0x81f288, 0x81f290, 0x81f298, 0x81f2a0, 0x81f2a8 | 0x6d62ec, 0x6d5ce4, 0x6d62e4, 0x6d62dc, 0x6d62fc, 0x6d62f4 |
| name pieces: the orc name separator and the bullywug male suffix | names.rs | 2 | 0x81f67c, 0x81f8e8 | 0x6d6590, 0x6d66a8 |
| equipment item names: items the generator buys, gives or compares with, mounts and animals, magic items it gives | equip.rs | 80 | 0x81fb3c, 0x820600, 0x820618, 0x820648, 0x820660, 0x820678, 0x820694, 0x8206a4, 0x8206bc, 0x8206c4, 0x8206cc, 0x8206dc, 0x8206ec, 0x820710, 0x82071c, 0x820724, 0x820730, 0x820738, 0x820748, 0x82075c, 0x820764, 0x82076c, 0x820770, 0x820778, 0x820784, 0x8207a0, 0x8207b4, 0x8207c8, 0x820804, 0x820818, 0x820860, 0x820868, 0x8208a8, 0x8208e0, 0x820900, 0x820908, 0x820a40, 0x820a54, 0x820a60, 0x820a68, 0x820a78, 0x820aa0, 0x820aac, 0x820ad4, 0x820afc, 0x820b14, 0x820b30, 0x820b44, 0x820b60, 0x820b84, 0x820b9c, 0x820bd0, 0x820bdc, 0x820c00, 0x820c1c, 0x820c38, 0x820c44, 0x820c50, 0x820c60, 0x820c80, 0x820ca0, 0x820cbc, 0x820cd0, 0x820d10, 0x820d20, 0x820d64, 0x820db0, 0x820dfc, 0x820e18, 0x821380, 0x821394, 0x8213a8, 0x821900, 0x821910, 0x821918, 0x821934, 0x8219a8, 0x8219e0, 0x8219f0, 0x821a00 | 0x6c5034, 0x6c5028, 0x6d6968, 0x6d6950, 0x6c5010, 0x6c4ff0, 0x6d6940, 0x6d6928, 0x6c4fdc, 0x6d6920, 0x6d6910, 0x6d6900, -, 0x6d6a90, 0x6d6a58, 0x6d6a4c, -, -, 0x6d69ec, 0x6d69e4, 0x6d69dc, 0x6d69d8, 0x6d69d0, 0x6d69c4, 0x6d6994, -, 0x6d6984, 0x6d6a60, 0x6d6a24, 0x6d6a10, -, 0x6d6a38, 0x6d6a80, 0x6d6a6c, -, -, 0x6d6ae0, 0x6d6ad4, 0x6d6acc, 0x6d6abc, 0x6c5000, 0x6c4fe4, 0x6d6c20, 0x6d6af4, 0x6d6c3c, 0x6d6c08, 0x6d6c00, 0x6d6be4, 0x6d6bc4, 0x6d6bac, 0x6d6b98, 0x6d6b7c, -, 0x6d6b58, 0x6d6b4c, -, -, -, -, 0x6d6b3c, -, 0x6d6b24, 0x6d6b10, 0x6d6c4c, -, -, -, -, -, -, -, -, -, -, -, -, -, -, -, - |

### NPC name tables

The tables the name generators pick from (`pick` = 0x4b8eda: `table[mod(n)]`). For the
Expansion they come from the stack-table scan of each function; for Core Rules 2.0 from the
listed Evermore.exe addresses (an entry that is not a data string reads as empty). The
generator code (which tables are joined, how often, with which odds) is in npc/names.rs,
cited by function address. The gnome and half-orc generators use fixed names (above),
the half-elf, mongrel and swanmay generators reuse others.

| Tsrp3.exe function | local | Tsrp3.exe table slot | role | Evermore.exe entries |
|---|---|---|---|---|
| 0x4b8b0b | 0x194 | [ebp-0x190] | human: male first names (whole names) | 98: 0x6d5d64, 0x6d5d5c, 0x6d5d54, 0x6d5d48, 0x6d5d40, 0x6d5d38, 0x6d5d30, 0x6d5d28, 0x6d5d20, 0x6d5d18, 0x6d5d10, 0x6d5d08, 0x6d5d00, 0x6d5cf8, 0x6d5cec, 0x6d5ce4, 0x6d5cdc, 0x6d5cd4, 0x6d5cd0, 0x6d5cc8, 0x6d5cc0, 0x6d5cbc, 0x6d5cb4, 0x6d5cac, 0x6d5ca4, 0x6d5c98, 0x6d5c90, 0x6d5c88, 0x6d5c80, 0x6d5c78, 0x6d5c70, 0x6d5c6c, 0x6d5c64, 0x6d5c5c, 0x6d5c54, 0x6d5c48, 0x6d5c40, 0x6d5c38, 0x6d5c30, 0x6d5c28, 0x6d5c20, 0x6d5c18, 0x6d5c10, 0x6d5c08, 0x6d5c00, 0x6d5bf8, 0x6d5bf4, 0x6d5bec, 0x6d5be4, 0x6d5bdc, 0x6d5bd4, 0x6d5bcc, 0x6d5bc4, 0x6d5bc0, 0x6d5bb8, 0x6d5bb0, 0x6d5bac, 0x6d5ba4, 0x6d5b9c, 0x6d5b94, 0x6d5b90, 0x6d5b88, 0x6d5b80, 0x6d5b78, 0x6d5b70, 0x6d5b68, 0x6d5b60, 0x6d5b58, 0x6d5b50, 0x6d5b44, 0x6d5b3c, 0x6d5b34, 0x6d5b2c, 0x6d5b24, 0x6d5b1c, 0x6d5b14, 0x6d5b0c, 0x6d5b08, 0x6d5b04, 0x6d5afc, 0x6d5af4, 0x6d5aec, 0x6d5ae4, 0x6d5adc, 0x6d5ad4, 0x6d5acc, 0x6d5ac4, 0x6d5abc, 0x6d5ab4, 0x6d5aac, 0x6d5aa4, 0x6d5a9c, 0x6d5a94, 0x6d5a8c, 0x6d5a84, 0x6d5a7c, 0x6d5a74, 0x6d5a6c |
| 0x4b8f34 | 0x164 | [ebp-0x160] | human: female first names (whole names) | 87: 0x6d6030, 0x6d6028, 0x6d6020, 0x6d6018, 0x6d6010, 0x6d6008, 0x6d6000, 0x6d5ff8, 0x6d5ff0, 0x6d5fe4, 0x6d5fdc, 0x6d5fd4, 0x6d5fcc, 0x6d5fc4, 0x6d5fbc, 0x6d5fb4, 0x6d5fac, 0x6d5fa4, 0x6d5f9c, 0x6d5f94, 0x6d5f8c, 0x6d5f88, 0x6d5f7c, 0x6d5f74, 0x6d5f6c, 0x6d5f64, 0x6d5f58, 0x6d5f50, 0x6d5f48, 0x6d5f40, 0x6d5f38, 0x6d5f30, 0x6d5f24, 0x6d5f1c, 0x6d5f18, 0x6d5f10, 0x6d5f0c, 0x6d5f04, 0x6d5efc, 0x6d5ef4, 0x6d5eec, 0x6d5ee8, 0x6d5edc, 0x6d5ed8, 0x6d5ed0, 0x6d5ec8, 0x6d5ec0, 0x6d5eb8, 0x6d5eb0, 0x6d5ea8, 0x6d5ea0, 0x6d5e98, 0x6d5e90, 0x6d5e88, 0x6d5e7c, 0x6d5e74, 0x6d5e6c, 0x6d5e60, 0x6d5e58, 0x6d5e50, 0x6d5e44, 0x6d5e3c, 0x6d5e34, 0x6d5e2c, 0x6d5e24, 0x6d5e1c, 0x6d5e14, 0x6d5e0c, 0x6d5e04, 0x6d5e00, 0x6d5df8, 0x6d5df0, 0x6d5de8, 0x6d5de0, 0x6d5dd8, 0x6d5dcc, 0x6d5dc4, 0x6d5dbc, 0x6d5db4, 0x6d5dac, 0x6d5da0, 0x6d5d98, 0x6d5d8c, 0x6d5d84, 0x6d5d7c, 0x6d5d74, 0x6d5d6c |
| 0x4b928b | 0x94 | [ebp-0x90] | elf: first syllable | 8: 0x6d607c, 0x6c46c4, 0x6d6078, 0x6d6074, 0x6c46c0, 0x6d6070, 0x6cf9cc, 0x6cea90 |
| 0x4b928b | 0xc4 | [ebp-0xc0] | elf: middle syllables, set A (female 40 %, male 60 % of the draws) | 4: 0x6d606c, 0x6d6068, 0x6d6064, 0x6d6060 |
| 0x4b928b | 0x70 | [ebp-0x6c] | elf: middle syllables, set B (the other draws) | 5: 0x6d605c, 0x6d6058, 0x6d6054, 0x6d6050, 0x6d604c |
| 0x4b928b | 0x58 | [ebp-0x54] | elf: joiner after each middle syllable (and after a drow apostrophe) | 7: 0x6d6048, 0x6d6044, 0x6d6040, 0x6d6088, 0x6d6084, 0x6d608c, 0x6d6080 |
| 0x4b928b | 0xa8 | [ebp-0xa4] | elf: optional ending, first part | 4: 0x6d606c, 0x6d6068, 0x6d6064, 0x6d6060 |
| 0x4b928b | 0x38 | [ebp-0x34] | elf: optional ending, second part | 8: 0x6d6080, 0x6d603c, 0x6d6038, 0x6d608c, 0x6d6088, 0x6d6084, 0x6d6048, 0x6d6044 |
| 0x4b97dd | 0x74 | [ebp-0x70] | dwarf: first syllable | 5: 0x6c2174, 0x6d6074, 0x6cea90, 0x6c2178, 0x6c46bc |
| 0x4b97dd | 0x5c | [ebp-0x58] | dwarf: repeated syllable, first part | 5: 0x6d61b8, 0x6d6060, 0x6d606c, 0x6d6064, 0x6d6068 |
| 0x4b97dd | 0x44 | [ebp-0x40] | dwarf: repeated syllable, second part | 11: 0x6d61b4, 0x6d61b0, 0x6d6088, 0x6d61ac, 0x6d61a8, 0x6d61a4, 0x6c5130, 0x6d61a0, 0x6d619c, 0x6d6198, 0x6d6048 |
| 0x4b97dd | 0xc8 | [ebp-0xc4] | dwarf: female ending | 3: 0x6d6050, 0x6d6068, 0x6d6194 |
| 0x4b97dd | 0x130 | [ebp-0x12c] | dwarf: surname form 1, first part | 13: 0x6d6178, 0x6d6174, 0x6d6170, 0x6d6168, 0x6cbb04, 0x6d6160, 0x6d6158, 0x6d6150, 0x6d6148, 0x6d6140, 0x6d6138, 0x6d6130, 0x6d6128 |
| 0x4b97dd | 0xb8 | [ebp-0xb4] | dwarf: surname form 1, second part | 10: 0x6d6188, 0x6d6110, 0x6d6108, 0x6d6180, 0x6d6100, 0x6d60f8, 0x6d60f0, 0x6d60e8, 0x6d60e0, 0x6d60d8 |
| 0x4b97dd | 0x8c | [ebp-0x88] | dwarf: surname form 2, first part | 5: 0x6ccfe0, 0x6cd2e0, 0x6cd2c8, 0x6d6120, 0x6d6118 |
| 0x4b97dd | 0xf8 | [ebp-0xf4] | dwarf: surname form 2, second part | 11: 0x6d6180, 0x6d60d0, 0x6d6188, 0x6d60c8, 0x6d60bc, 0x6d60b4, 0x6d60ac, 0x6d60a8, 0x6d60a0, 0x6d6098, 0x6d6090 |
| 0x4ba82f | 0x8c | [ebp-0x88] | halfling: male first names | 7: 0x6d648c, 0x6d5b08, 0x6d6488, 0x6d6480, 0x6d6478, 0x6d6470, 0x6d6468 |
| 0x4ba82f | 0xa8 | [ebp-0xa4] | halfling: female first names | 6: 0x6d5e58, 0x6d6460, 0x6d6454, 0x6d644c, 0x6d6444, 0x6d643c |
| 0x4ba82f | 0x68 | [ebp-0x64] | halfling: surname form 1, first part | 6: 0x6d6434, 0x6d642c, 0x6d6424, 0x6d641c, 0x6d6414, 0x6d640c |
| 0x4ba82f | 0x4c | [ebp-0x48] | halfling: surname form 1, second part | 6: 0x6d63e4, 0x6d63dc, 0x6d63d4, 0x6d63cc, 0x6d63c4, 0x6d63bc |
| 0x4ba82f | 0x30 | [ebp-0x2c] | halfling: surname form 2, first part | 4: 0x6d6404, 0x6d63fc, 0x6d63f4, 0x6d63ec |
| 0x4ba82f | 0x1c | [ebp-0x18] | halfling: surname form 2, second part | 2: 0x6d63b4, 0x6d63ac |
| 0x4ba740 | 0x70 | [ebp-0x6c] | ogre: whole names | 26: 0x6d63a0, 0x6d639c, 0x6d6394, 0x6d638c, 0x6d6384, 0x6d6380, 0x6d6378, 0x6d6370, 0x6d6368, 0x6d6364, 0x6d635c, 0x6d6358, 0x6d6354, 0x6d6350, 0x6d634c, 0x6d6348, 0x6d6340, 0x6d6338, 0x6d6330, 0x6d6328, 0x6d6320, 0x6d3d38, 0x6d6318, 0x6d6310, 0x6d6308, 0x6d6304 |
| 0x4bc074 | 0x98 | [ebp-0x94] | orc: first syllable | 8: 0x6d6728, 0x6d6720, 0x6d671c, 0x6d6718, 0x6d6714, 0x6d6710, 0x6d670c, 0x6d6330 |
| 0x4bc074 | 0x114 | [ebp-0x110] | orc: extra syllable, set A | 10: 0x6d66e4, 0x6d66dc, 0x6d66d4, 0x6d66d0, 0x6d66cc, 0x6d66c8, 0x6d66c0, 0x6d66bc, 0x6d66b8, 0x6d66b0 |
| 0x4bc074 | 0x44 | [ebp-0x40] | orc: extra syllable, set B | 6: 0x6d6704, 0x6d6700, 0x6d66fc, 0x6d66f4, 0x6d66f0, 0x6d66e8 |
| 0x4bc074 | 0xc0 | [ebp-0xbc] | orc: female ending | 2: 0x6d6068, 0x6d64b0 |
| 0x4bc074 | 0xb4 | [ebp-0xb0] | orc: epithet after the separator, first part | 6: 0x6ccfe0, 0x6c2350, 0x6c215c, 0x6ccfe4, 0x6ccff0, 0x6d66a0 |
| 0x4bc074 | 0x74 | [ebp-0x70] | orc: epithet, second part | 11: 0x6d6180, 0x6d6698, 0x6d60d0, 0x6d6188, 0x6d60c8, 0x6d60bc, 0x6d60b4, 0x6d60ac, 0x6d60a8, 0x6d60a0, 0x6d6090 |
| 0x4bbd92 | 0x34 | [ebp-0x30] | lizard man and alaghi: first part | 7: 0x6d6690, 0x6d668c, 0x6d6688, 0x6d6684, 0x6d6680, 0x6d667c, 0x6d6674 |
| 0x4bbd92 | 0x44 | [ebp-0x40] | lizard man and alaghi: second part, set A (mostly alaghi) | 3: 0x6d6644, 0x6d663c, 0x6d6634 |
| 0x4bbd92 | 0x6c | [ebp-0x68] | lizard man and alaghi: second part, set B (mostly lizard man) | 7: 0x6d666c, 0x6d6664, 0x6d65cc, 0x6d6660, 0x6d6658, 0x6d6650, 0x6d6648 |
| 0x4bbbda | 0x34 | [ebp-0x30] | thri-kreen: first part | 7: 0x6d6630, 0x6d662c, 0x6d6628, 0x6d6624, 0x6d65f0, 0x6d6620, 0x6d6618 |
| 0x4bbbda | 0x50 | [ebp-0x4c] | thri-kreen: repeated syllables | 6: 0x6d6614, 0x6d6610, 0x6d660c, 0x6d6608, 0x6d6604, 0x6d64b4 |
| 0x4bba12 | 0x40 | [ebp-0x3c] | wemic: first part | 7: 0x6d6600, 0x6d65fc, 0x6d65f8, 0x6d65f4, 0x6d65f0, 0x6d65e8, 0x6d65e4 |
| 0x4bba12 | 0x5c | [ebp-0x58] | wemic: second part | 6: 0x6d65dc, 0x6d65d4, 0x6d65cc, 0x6d65c8, 0x6d65c4, 0x6d65c0 |
| 0x4bba12 | 0x20 | [ebp-0x1c] | wemic: female ending | 2: 0x6d6060, 0x6d6068 |
| 0x4bb80a | 0x44 | [ebp-0x40] | bullywug: first part | 7: 0x6d65bc, 0x6d65b8, 0x6d65b4, 0x6d64f0, 0x6d5b04, 0x6d65ac, 0x6d65a8 |
| 0x4bb80a | 0x64 | [ebp-0x60] | bullywug: repeated syllables | 7: 0x6d65a4, 0x6d64e4, 0x6d65a0, 0x6d64dc, 0x6d659c, 0x6d6598, 0x6d6594 |
| 0x4bb80a | 0x24 | [ebp-0x20] | bullywug: female ending | 3: 0x6d6060, 0x6d6068, 0x6d64b0 |
| 0x4bae7e | 0x2c | [ebp-0x28] | aarakocra: first part | 6: 0x6d64a8, 0x6d6070, 0x6d64d0, 0x6d64a8, 0x6d6074, 0x6d64d0 |
| 0x4bae7e | 0x90 | [ebp-0x8c] | aarakocra: joiner after the first part and after each syllable | 5: 0x6d6060, 0x6d6068, 0x6d6064, 0x6d6060, 0x6d6068 |
| 0x4bae7e | 0x78 | [ebp-0x74] | aarakocra: repeated syllables | 8: 0x6d61ac, 0x6d61a0, 0x6d6068, 0x6d649c, 0x6d6048, 0x6d6048, 0x6d6198, 0x6d61ac |
| 0x4bae7e | 0x54 | [ebp-0x50] | aarakocra: ending | 8: 0x6d64cc, 0x6d64c8, 0x6d64c4, 0x6d64c0, 0x6d64bc, 0x6d64b8, 0x6d61ac, 0x6d64b4 |
| 0x4bae7e | 0xb0 | [ebp-0xac] | aarakocra: female ending | 7: 0x6d6060, 0x6d6068, 0x6d606c, 0x6d6060, 0x6d6068, 0x6d64b0, 0x6d64ac |
| 0x4bb2d7 | 0x44 | [ebp-0x40] | giff: first part | 6: 0x6d64fc, 0x6d64f8, 0x6d64f4, 0x6d64f0, 0x6d5b04, 0x6d64ec |
| 0x4bb2d7 | 0x60 | [ebp-0x5c] | giff: repeated syllables | 6: 0x6d64e8, 0x6d64e4, 0x6d64e0, 0x6d64dc, 0x6d64d8, 0x6d64d4 |
| 0x4bb2d7 | 0x28 | [ebp-0x24] | giff: female ending | 4: 0x6d6060, 0x6d6060, 0x6d61b8, 0x6d6068 |
| 0x4bb4b5 | 0x9c | [ebp-0x98] | satyr: first part | 15: 0x6d658c, 0x6d6588, 0x6d6580, 0x6d657c, 0x6d6578, 0x6d6574, 0x6d6570, 0x6d656c, 0x6d6568, 0x6d6564, 0x6d655c, 0x6d6554, 0x6d6550, 0x6d6548, 0x6d6544 |
| 0x4bb4b5 | 0x4c | [ebp-0x48] | satyr: repeated syllables | 13: 0x6d6540, 0x6d6538, 0x6d6534, 0x6d652c, 0x6d6524, 0x6d6520, 0x6d651c, 0x6d6514, 0x6d650c, 0x6d6508, 0x6d12b0, 0x6d6504, 0x6d12ac |
| 0x4bb4b5 | 0xb0 | [ebp-0xac] | satyr: male ending | 4: 0x6d6584, 0x6d6584, 0x6d6500, 0x6d6584 |
| 0x4bb4b5 | 0x5c | [ebp-0x58] | satyr: female ending | 3: 0x6d6050, 0x6d6050, 0x6d6058 |
| 0x4babc0 | 0x7c | [ebp-0x78] | githzerai: first part | 9: 0x6d64a8, 0x6c46c0, 0x6c2178, 0x6cf9cc, 0x6cea94, 0x6c46bc, 0x6d64a4, 0x6d6074, 0x6d64a0 |
| 0x4babc0 | 0x54 | [ebp-0x50] | githzerai: joiner after the first part and after each syllable | 5: 0x6d606c, 0x6d6068, 0x6d6064, 0x6d6060, 0x6d61b8 |
| 0x4babc0 | 0x3c | [ebp-0x38] | githzerai: repeated syllables | 9: 0x6d61ac, 0x6d608c, 0x6d61a0, 0x6d6088, 0x6d649c, 0x6d61b0, 0x6d605c, 0x6d6048, 0x6d6498 |

### Part document lists

The fixed lists the originals' part document constructor adds (Tsrp3.exe `0x677978`,
Evermore.exe around `0x55b000`) and the custom-data editors offer in combos and lists
(`program_list(install, flavor, name)`, asked for by `userdb.rs` when a page names one of
these lists). Each address is the string the constructor pushes before adding it; `0`
means an empty entry (Tsrp3.exe adds an empty `CString` there, Evermore.exe an empty
literal).

| list | document member | n | Tsrp3.exe | Evermore.exe | what |
|---|---|---|---|---|---|
| `settings` | +0x284 | 8 | 0x84dd40, 0x84dd54, 0x84dd68, 0x84dd84, 0x84dd90, 0x84dda0, 0x84ddc0, 0x84ddd8 | 0x6d77d8, 0x6d77c4, 0x6d77a8, 0x6d779c, 0x6d778c, 0x6d776c, 0x6d7754, 0x6d7740 | historical setting names |
| `equipment-categories` | +0x298 | 12 | 0, 0x84ddec, 0x84ddfc, 0x84de14, 0x84de2c, 0x84de48, 0x84de50, 0x84de5c, 0x84de64, 0x84de70, 0x84de78, 0x84de84 | 0, 0x6c4398, 0x6c4364, 0x6c4680, 0x6c4664, 0x6c43a8, 0x6c43b8, 0x6c43b0, 0x6c4384, 0x6c4390, 0x6c3be0, 0x6c437c | equipment category names (first entry empty) |
| `racial-equipment` | +0x2ac | 5 | 0, 0x84de8c, 0x84dea0, 0x84deb0, 0x84dec4 | 0, 0x6d68ec, 0x6d68c8, 0x6d68d8, 0x6d68b4 | racial equipment group names (first entry empty) |
| `school-schemes` | +0x2c0 | 4 | 0x84ded8, 0x84def0, 0x84df04, 0x84df1c | 0x6d7728, 0x6d7714, 0x6d76fc, 0x6d76e0 | school scheme names |
| `spheres` | +0x2d4 | 27 | 0x84df38, 0x84df3c, 0x84df44, 0x84df4c, 0x84df54, 0x84df5c, 0x84df64, 0x84df70, 0x84df7c, 0x84df8c, 0x84dfa0, 0x84dfb0, 0x84dfc4, 0x84dfd0, 0x84dfd8, 0x84dfdc, 0x84dfe8, 0x84dff0, 0x84dff8, 0x84e004, 0x84e010, 0x84e014, 0x84e01c, 0x84e024, 0x84e030, 0x84e034, 0x84e03c | 0x6c73e0, 0x6c43a8, 0x6d76d8, 0x6cb2e4, 0x6d76d0, 0x6c35f4, 0x6d76c4, 0x6ccc98, 0x6d76b4, 0x6d76a0, 0x6d7690, 0x6d767c, 0x6d5c48, 0x6cd0cc, 0x6d7678, 0x6d766c, 0x6d7664, 0x6d765c, 0x6d7650, 0x6d7644, 0x6d7640, 0x6d7638, 0x6c3a10, 0x6d762c, 0x6d7628, 0x6d7620, 0x6d7618 | priest sphere names |
| `nwp-groups` | +0x2e8 | 5 | 0x84e044, 0x84e04c, 0x84e054, 0x84e05c, 0x84e064 | 0x6d7610, 0x6c73a4, 0x6d7608, 0x6d7600, 0x6d75f8 | nonweapon proficiency group names |
| `sub-abilities` | +0x2fc | 12 | 0x84e06c, 0x84e080, 0x84e090, 0x84e0a0, 0x84e0b4, 0x84e0c8, 0x84e0e0, 0x84e0f4, 0x84e10c, 0x84e120, 0x84e134, 0x84e148 | 0x6c2754, 0x6c2744, 0x6c2734, 0x6c2720, 0x6c270c, 0x6c26f4, 0x6c26e0, 0x6c26c8, 0x6c26b4, 0x6c26a0, 0x6c268c, 0x6c2678 | sub-ability names (Skills & Powers) |
| `social-classes` | +0x310 | 4 | 0x84e15c, 0x84e168, 0x84e17c, 0x84e190 | 0x6c2134, 0x6c52d0, 0x6c52bc, 0x6c52b0 | social class names |
| `armor-types` | +0x220 | 5 | 0x84d990, 0x84d998, 0x84d9a0, 0x84d9a8, 0x84d9b0 | 0x6c1c94, 0x6c3334, 0x6cea68, 0x6cbb04, 0x6cea50 | armor type names, ids 0..4 in order |
| `damage-types` | +0x20c | 8 | 0x84d9b8, 0x84d9c0, 0x84d9c4, 0x84d9c8, 0x84d9cc, 0x84d9d0, 0x84d9d4, 0x84d9d8 | 0x6c1c94, 0x6d7b48, 0x6c46c0, 0x6d6070, 0x6d7b44, 0x6d7b40, 0x6d7b3c, 0x6d7b34 | weapon damage type names |
| `broad-weapon-groups` | +0x234 | 8 | 0x84d9e0, 0x84d9e8, 0x84d9f0, 0x84da0c, 0x84da28, 0x84da34, 0x84da48, 0x84da50 | 0x6c1c94, 0x6d7b2c, 0x6d7b10, 0x6d7af4, 0x6d7ae8, 0x6d7ad4, 0x6d7acc, 0x6d7ac0 | broad weapon group names, ids 0..7 in order |
| `tight-weapon-groups` | +0x248 | 35 | 0x84da5c, 0x84da64, 0x84da70, 0x84da7c, 0x84da8c, 0x84da9c, 0x84daa4, 0x84dac4, 0x84dae4, 0x84db08, 0x84db14, 0x84db28, 0x84db30, 0x84db50, 0x84db64, 0x84db74, 0x84db88, 0x84db9c, 0x84dbb0, 0x84dbcc, 0x84dbe8, 0x84dc08, 0x84dc18, 0x84dc28, 0x84dc40, 0x84dc54, 0x84dc64, 0x84dc74, 0x84dc84, 0x84dc9c, 0x84dcb4, 0x84dccc, 0x84dcec, 0x84dd04, 0x84dd1c | 0x6d7b2c, 0x6d7ab4, 0x6d7aa8, 0x6d7a98, 0x6d7a88, 0x6c2118, 0x6d7a68, 0x6d7a48, 0x6d7a24, 0x6d7a18, 0x6d7a04, 0x6d79fc, 0x6d79dc, 0x6d79c8, 0x6d79b8, 0x6d79a4, 0x6d7990, 0x6d797c, 0x6d7960, 0x6d7944, 0x6d7924, 0x6d7914, 0x6d7904, 0x6d78ec, 0x6d78d8, 0x6d78c8, 0x6d78b8, 0x6d78a8, 0x6d7890, 0x6d7878, 0x6d7860, 0x6d7840, 0x6d7828, 0x6d7810, 0x6d77ec | tight weapon group names, ids 1..35 in order |
| `secondary-skills` | +0x1adc | 24 | 0x818000, 0x818008, 0x818018, 0x818020, 0x818028, 0x818034, 0x81803c, 0x818044, 0x81804c, 0x818054, 0x818064, 0x818074, 0x81807c, 0x818084, 0x818090, 0x818098, 0x8180a0, 0x8180ac, 0x8180bc, 0x8180d0, 0x8180e0, 0x8180f0, 0x8180fc, 0x818114 | (none) | secondary skill names a kit may require (Expansion only; filled at 0x463737, document +0x1adc) |

Several Evermore.exe entries share an address with an NPC text (for example the equipment
categories): the optimized build stores each distinct literal once.

### Intelligent weapons

treasure/intelligent.rs (the roller at Tsrp3.exe 0x546b4b, Evermore.exe 0x5ce120), read
when the treasure rules load (`Texts::read`). The roll thresholds are kept in the app; the
lines added to the weapon's contents are the program's.

| chain | n | Tsrp3.exe | Evermore.exe | what |
|---|---|---|---|---|
| `DETECT` | 10 | 0x831320, 0x831364, 0x83139c, 0x8313d4, 0x831404, 0x83144c, 0x831488, 0x8314b4, 0x8314e4, 0x83151c | 0x6e1854, 0x6e181c, 0x6e17e4, 0x6e17b4, 0x6e176c, 0x6e1730, 0x6e1704, 0x6e16d4, 0x6e169c, 0x6e1670 | detection ability lines, by roll band |
| `EXTRAORDINARY` | 14 | 0x831548, 0x831578, 0x8315c0, 0x831608, 0x831640, 0x83167c, 0x8316a8, 0x8316c4, 0x831704, 0x83175c, 0x831790, 0x8317dc, 0x83180c, 0x831850 | 0x6e1640, 0x6e15f8, 0x6e15b0, 0x6e1578, 0x6e153c, 0x6e1510, 0x6e14f4, 0x6e14b4, 0x6e145c, 0x6e1428, 0x6e13dc, 0x6e13ac, 0x6e1368, 0x6e1324 | extraordinary power lines, by roll band |
| `CHOICE` | 1 | 0x831894 | 0x6e12f0 | the line that leaves the power to the player (extraordinary roll of 98 or more; a second one merges with the first) |
| `PURPOSE` | 10 | 0x8318fc, 0x83193c, 0x831974, 0x831998, 0x8319b8, 0x8319d8, 0x8319f8, 0x831a24, 0x831a50, 0x831a7c | 0x6e12b0, 0x6e1278, 0x6e1254, 0x6e1234, 0x6e1214, 0x6e11f4, 0x6e11c8, 0x6e119c, 0x6e1170, 0x6e1158 | special purpose lines |
| `POWER` | 7 | 0x831a94, 0x831ac4, 0x831af4, 0x831b18, 0x831b44, 0x831b74, 0x831ba4 | 0x6e1128, 0x6e10f8, 0x6e10d4, 0x6e10a8, 0x6e1078, 0x6e1048, 0x6e0ff4 | special purpose power lines |

### Character sheet: Turning Undead

sheet/build/combat.rs `turning_texts`, read when the sheet rules load (`sheet.rs`,
`rules.turning`).

| constant | n | Tsrp3.exe | Evermore.exe | what |
|---|---|---|---|---|
| `UNDEAD` | 13 | 0x83a1e8, 0x83a22c, 0x83a260, 0x83a2a0, 0x83a1fc, 0x83a234, 0x83a270, 0x83a20c, 0x83a23c, 0x83a280, 0x83a21c, 0x83a250, 0x83a294 | 0x6c3878, 0x6c3840, 0x6c3804, 0x6c37c0, 0x6c3868, 0x6c3838, 0x6c37f4, 0x6c3858, 0x6c3824, 0x6c37e0, 0x6c3848, 0x6c3814, 0x6c37d4 | the 13 undead row labels of the Turning Undead section |
| `LEGEND` | 4 | 0x83a2b4, 0x83a310, 0x83a2e8, 0x83a334 | 0x6c3768, 0x6c379c, 0x6c3714, 0x6c373c | the legend lines: the roll line, then the T, D and D* lines |

### Import: old class flags

export.rs `upgrade_part`: when the Expansion imports a Core Rules 2.0 part (no version
block), each set old class flag becomes a name in `allowed_classes` (the Expansion's
0x661697). The names are read from Tsrp3.exe, whatever the package's origin; the image is
opened only when the first set flag is met.

| flag index (`old_class_flags`) | Tsrp3.exe | what |
|---|---|---|
| 0 | 0x84c580 | class name for flag 0 (Bard) |
| 1 | 0x84c588 | class name for flag 1 |
| 2 | 0x84c590 | class name for flag 2 |
| 3 | 0x84c598 | class name for flag 3 |
| 4 | 0x84c5a0 | class name for flag 4 |
| 5 | 0x84c5a8 | class name for flag 5 |
| 6 | 0x84c5b0 | class name for flag 6 |
| 8 | 0x84c5b8 | class name for flag 8 (Thief) |

Flag 7 is not converted (see [parts.md](parts.md), `m_0x4a4` and the pre-v1 conversion).

### Table Rolls

tables/strings.rs, read through `tables::program::Program` by tables.rs and
tables/code.rs / tables/data.rs: the column headings, row labels, footnotes, titles and
parameter options of the tables the originals work out in their own code instead of
reading from `Tables/` (see [tables.md](tables.md)). The addresses were taken from the
strings the Expansion's Table Rolls code pushes, and the same strings found in Core Rules
2.0. 640 addresses; `n` counts strings, and for lists of lists the sizes of the inner lists
in order. Headings are fixed-width text: a ruler line of dashes gives the column starts
(`layout`, `heading`, `cut` in tables/program.rs).

| constant | n | Tsrp3.exe | Evermore.exe | role |
|---|---|---|---|---|
| `TURN_HEAD` | 2 | 0x8277c0, 0x8277f0 | 0x6dc82c, 0x6dc7fc | Turning Undead table: column heading lines |
| `UNDEAD` | 13 | 0x82782c, 0x82784c, 0x82786c, 0x82788c, 0x8278ac, 0x8278cc, 0x8278ec, 0x82790c, 0x82792c, 0x82794c, 0x82796c, 0x82798c, 0x8279ac | 0x6dc7e4, 0x6dc7c4, 0x6dc7ac, 0x6dc794, 0x6dc77c, 0x6dc764, 0x6dc74c, 0x6dc734, 0x6dc71c, 0x6dc704, 0x6dc6ec, 0x6dc6d4, 0x6dc6bc | Turning Undead table: the 13 undead row labels |
| `TURN_NOTE` | 4 | 0x8279c4, 0x8279f0, 0x827a24, 0x827a54 | 0x6dc690, 0x6dc65c, 0x6dc62c, 0x6dc604 | Turning Undead table: legend lines under the table |
| `PRIEST_LEVELS` | 12 | 0x828e28, 0x828e3c, 0x828e50, 0x828e64, 0x828e78, 0x828e8c, 0x828ea0, 0x828eb4, 0x828ec8, 0x828edc, 0x828ef4, 0x828f0c | 0x6ddb3c, 0x6ddb28, 0x6ddb14, 0x6ddb00, 0x6ddaec, 0x6ddad8, 0x6ddac4, 0x6ddab0, 0x6dda9c, 0x6dda84, 0x6dda6c, 0x6dda58 | Turning parameter options: priest level ranges |
| `PALADIN_LEVELS` | 12 | 0x828f20, 0x828f34, 0x828f48, 0x828f5c, 0x828f70, 0x828f84, 0x828f98, 0x828fac, 0x828fc0, 0x828fd4, 0x828fec, 0x829004 | 0x6dda44, 0x6dda30, 0x6dda1c, 0x6dda08, 0x6dd9f4, 0x6dd9e0, 0x6dd9cc, 0x6dd9b8, 0x6dd9a4, 0x6dd98c, 0x6dd974, 0x6dd960 | Turning parameter options: paladin level ranges |
| `TURNING_CLASSES` | 2 | 0x826e08, 0x826e10 | 0x6dc428, 0x6dc420 | Turning parameter options: the two turning classes |
| `RANKS` | 5 | 0x826ee0, 0x826ef0, 0x826ef8, 0x826f00, 0x826f0c | 0x6dbe7c, 0x6dbe74, 0x6dbe6c, 0x6dbe60, 0x6dbe58 | parameter options: rank choices of a worked-out table |
| `DISEASE_HEAD` | 2 | 0x8263dc, 0x826408 | 0x6db31c, 0x6db2f0 | disease table: column heading lines |
| `DISEASE_ROWS` | 5 | 0x826434, 0x826460, 0x82648c, 0x8264b8, 0x8264e4 | 0x6db2c4, 0x6db298, 0x6db26c, 0x6db240, 0x6db214 | disease table: row labels |
| `DISEASE_NOTE` | 11 | 0x826514, 0x826540, 0x826570, 0x8265a0, 0x8265d0, 0x8265fc, 0x82662c, 0x826658, 0x82668c, 0x8266c0, 0x8266f4 | 0x6db1e8, 0x6db1b8, 0x6db188, 0x6db158, 0x6db12c, 0x6db0fc, 0x6db0d4, 0x6db0a0, 0x6db06c, 0x6db038, 0x6db030 | disease table: footnote lines |
| `CRUSH_HEAD` | 2 | 0x826714, 0x826738 | 0x6daff4, 0x6dafd0 | crushing-damage table: column heading lines |
| `CRUSH_ROWS` | 5 | 0x82675c, 0x82677c, 0x82679c, 0x8267bc, 0x8267dc | 0x6dafb0, 0x6daf90, 0x6daf70, 0x6daf50, 0x6daf30 | crushing-damage table: row labels |
| `WOUNDING_HEAD` | 2 | 0x826820, 0x826838 | 0x6daefc, 0x6daee4 | wounding table: column heading lines |
| `WOUNDING_ROWS` | 5 | 0x826850, 0x826864, 0x82687c, 0x826898, 0x8268b4 | 0x6daed0, 0x6daeb8, 0x6dae9c, 0x6dae80, 0x6dae64 | wounding table: row labels |
| `WOUNDING_NOTE` | 5 | 0x8268d8, 0x826908, 0x826938, 0x826964, 0x826994 | 0x6dae34, 0x6dae04, 0x6dadd8, 0x6dada8, 0x6dada0 | wounding table: footnote lines |
| `FUNDS_FOR` | 1 | 0x827618 | 0x6dc5c4 | starting-funds result: the lead text before the class group |
| `CLASS_GROUPS` | 4 | 0x826d1c, 0x826d24, 0x826d2c, 0x826d34 | 0x6c55d0, 0x6c119c, 0x6c55c8, 0x6c55d8 | starting-funds: class group names (parameter options and result text) |
| `FUNDS_NOTE` | 7 | 0x827668, 0x827698, 0x8276cc, 0x827700, 0x827734, 0x827768, 0x82779c | 0x6dc570, 0x6dc53c, 0x6dc508, 0x6dc4d4, 0x6dc4a0, 0x6dc46c, 0x6dc448 | starting-funds: footnote lines |
| `CASTER_LEVELS` | 5 | 0x826e70, 0x826e84, 0x826e98, 0x826eac, 0x826ec0 | 0x6dc40c, 0x6dc3f8, 0x6dc3e4, 0x6dc3d0, 0x6dc3bc | parameter options: caster level ranges |
| `SPELL_FAILURE` | 2 | 0x82805c, 0x82806c | 0x6dcdf4, 0x6dcdd8 | spell-failure result: the two outcome texts |
| `RESULT_HEAD` | 2 | 0x828088, 0x828098 | 0x6dcdc8, 0x6dcdb8 | Roll Results column headings (roll, result) of the threshold tables |
| `SCROLL_LEVELS` | 8 | 0x826f20, 0x826f2c, 0x826f38, 0x826f44, 0x826f4c, 0x826f54, 0x826f5c, 0x826f68 | 0x6dbed4, 0x6dbec8, 0x6dbebc, 0x6dbeb4, 0x6dbeac, 0x6dbea4, 0x6dbe98, 0x6dbe8c | parameter options: scroll spell levels |
| `SCROLL_USE` | 2 | 0x8280b0, 0x8280c0 | 0x6dce18, 0x6dce04 | scroll-use result: the two outcome texts |
| `LANGUAGES_HEAD` | 2 | 0x82c594, 0x82c5b0 | 0x6def7c, 0x6dec28 | languages table: column heading lines |
| `LANGUAGES_NOTE` | 3 | 0x82c5e8, 0x82c614, 0x82c644 | 0x6dcd60, 0x6dcd30, 0x6dcd20 | languages table: footnote lines |
| `RARITY` | 3 | 0x826f80, 0x826f88, 0x826f94 | 0x6cd2f0, 0x6cf810, 0x6cf808 | parameter options: rarity choices |
| `FOUND_IN` | 4 | 0x8280f4, 0x828110, 0x828118, 0x82811c | 0x6dce30, 0x6dce28, 0x6ae638, 0x44f605 | purchase result: four sentence fragments (location lead, unit, plural, end) |
| `CANNOT` | 4 | 0x828128, 0x828150, 0x828174, 0x828198 | 0x6dcf74, 0x6dcf50, 0x6dcf2c, 0x6dcf00 | purchase result: the not-for-sale texts by rarity |
| `CAN` | 4 | 0x8281c4, 0x8281ec, 0x828214, 0x82823c | 0x6dced8, 0x6dceb0, 0x6dce88, 0x6dce5c | purchase result: the for-sale texts by rarity |
| `PURCHASE_HEAD` | 2 | 0x8283b0, 0x8283b8 | 0x6dce54, 0x6dce4c | purchase result: column headings |
| `TARGETS_AFFECTED` | 4 | 0x8270f8, 0x827114, 0x827130, 0x82714c | 0x6dc3a0, 0x6dc384, 0x6dc368, 0x6dc34c | parameter options: number of targets affected |
| `WHOLE_HEAD` | 3 | 0x8283e4, 0x82840c, 0x828434 | 0x6dd05c, 0x6dd034, 0x6dd00c | area result: column heading lines |
| `AREAS` | 4 | 0x82845c, 0x828478, 0x828494, 0x8284b0 | 0x6dcff0, 0x6dcfd8, 0x6dcfbc, 0x6dcfa0 | area result: area labels by choice |
| `AREA_SIZES` | 3 | 0x828490, 0x8284ac, 0x8284c8 | 0x6dcfd4, 0x6dcfb8, 0x6dcf9c | area result: area size texts |
| `MAX_DAMAGE` | 4 | 0x827174, 0x82719c, 0x8271c4, 0x8271ec | 0x6dc324, 0x6dc2fc, 0x6dc2d4, 0x6dc2ac | parameter options and result label: maximum damage choices |
| `SEVERITY_HEAD` | 3 | 0x8284ec, 0x828520, 0x828550 | 0x6dd108, 0x6dd0d8, 0x6dd0a4 | severity result: column heading lines |
| `TRAIT_HEAD` | 2 | 0x82c464, 0x82c47c | 0x6dc874, 0x6dc85c | traits table: column heading lines |
| `TRAITS` | 101 | 0x82c01c, 0x82c020, 0x82c02c, 0x82c03c, 0x82c048, 0x82c054, 0x82c064, 0x82c06c, 0x82c074, 0x82c07c, 0x82c084, 0x82c08c, 0x82c098, 0x82c0a4, 0x82c0ac, 0x82c0b8, 0x82c0c0, 0x82c0cc, 0x82c0dc, 0x82c0e4, 0x82c0f4, 0x82c100, 0x82c108, 0x82c110, 0x82c114, 0x82c120, 0x82c12c, 0x82c138, 0x82c140, 0x82c150, 0x82c15c, 0x82c164, 0x82c174, 0x82c17c, 0x82c184, 0x82c190, 0x82c198, 0x82c1a4, 0x82c1b4, 0x82c1c0, 0x82c1cc, 0x82c1dc, 0x82c1e4, 0x82c1f4, 0x82c200, 0x82c20c, 0x82c214, 0x82c21c, 0x82c228, 0x82c234, 0x82c23c, 0x82c248, 0x82c250, 0x82c258, 0x82c264, 0x82c270, 0x82c27c, 0x82c284, 0x82c290, 0x82c29c, 0x82c2a8, 0x82c2b0, 0x82c2b8, 0x82c2c0, 0x82c2d0, 0x82c2dc, 0x82c2e8, 0x82c2f4, 0x82c2fc, 0x82c308, 0x82c314, 0x82c320, 0x82c32c, 0x82c338, 0x82c340, 0x82c34c, 0x82c358, 0x82c360, 0x82c36c, 0x82c378, 0x82c384, 0x82c38c, 0x82c398, 0x82c3a8, 0x82c3b0, 0x82c3bc, 0x82c3c8, 0x82c3d4, 0x82c3e0, 0x82c3ec, 0x82c3f8, 0x82c400, 0x82c40c, 0x82c414, 0x82c420, 0x82c42c, 0x82c434, 0x82c43c, 0x82c448, 0x82c450, 0x82c458 | 0x48442e, 0x6dccb4, 0x6dcca4, 0x6dcc98, 0x6dcc8c, 0x6dcc7c, 0x6dcc74, 0x6dcc6c, 0x6dcc64, 0x6dcc5c, 0x6dcc54, 0x6dcc48, 0x6dcc3c, 0x6dcc34, 0x6dcc28, 0x6dcc20, 0x6dcc14, 0x6dcc04, 0x6dcbfc, 0x6dcbec, 0x6dcbe0, 0x6dcbd8, 0x6dcbd0, 0x6dcbcc, 0x6d3d54, 0x6dcbc0, 0x6dcbb4, 0x6dcbac, 0x6dcb9c, 0x6dcb90, 0x6dcb88, 0x6dcb78, 0x6dcb70, 0x6dcb68, 0x6dcb5c, 0x6dcb54, 0x6dcb48, 0x6dcb38, 0x6dcb2c, 0x6dcb20, 0x6dcb10, 0x6dcb08, 0x6dcaf8, 0x6dcaec, 0x6dcae0, 0x6dcad8, 0x6dcad0, 0x6dcac4, 0x6dcab8, 0x6dcab0, 0x6dcaa4, 0x6dca9c, 0x6dca94, 0x6dca88, 0x6dca7c, 0x6dca70, 0x6dca68, 0x6dca5c, 0x6dca50, 0x6dca44, 0x6dca3c, 0x6dca34, 0x6dca2c, 0x6dca1c, 0x6dca10, 0x6dca04, 0x6dc9f8, 0x6dc9f0, 0x6dc9e4, 0x6dc9d8, 0x6dc9cc, 0x6dc9c0, 0x6dc9b4, 0x6dc9ac, 0x6dc9a0, 0x6dc994, 0x6dc98c, 0x6dc980, 0x6dc974, 0x6dc968, 0x6dc960, 0x6dc954, 0x6dc944, 0x6dc93c, 0x6dc930, 0x6dc924, 0x6dc918, 0x6dc90c, 0x6dc900, 0x6dc8f4, 0x6dc8ec, 0x6dc8e0, 0x6dc8d8, 0x6dc8cc, 0x6dc8c0, 0x6dc8b8, 0x6dc8b0, 0x6dc8a4, 0x6dc89c, 0x6dc894, 0x6dc88c | traits table: the trait names (rows) |
| `GEM_HEAD` | 3 | 0x82cedc, 0x82cee8, 0x82cf0c | 0x6dd224, 0x6dd200, 0x6dd1dc | gem table: column heading lines |
| `GEMS` | 56 (12+13+11+6+6+8) | 0x82cc44, 0x82cc4c, 0x82cc5c, 0x82cc68, 0x82cc74, 0x82cc80, 0x82cc90, 0x82cc9c, 0x82cca8, 0x82ccb4, 0x82ccc4, 0x82ccd4, 0x82cce0, 0x82ccec, 0x82ccf8, 0x82cd04, 0x82cd10, 0x82cd18, 0x82cd20, 0x82cd2c, 0x82cd34, 0x82cd44, 0x82cd50, 0x82cd60, 0x82cd74, 0x82cd7c, 0x82cd84, 0x82cd90, 0x82cd9c, 0x82cda8, 0x82cdb0, 0x82cdb8, 0x82cdc0, 0x82cdc4, 0x82cdcc, 0x82cdd4, 0x82cde0, 0x82cdec, 0x82cdf4, 0x82cdfc, 0x82ce04, 0x82ce0c, 0x82ce14, 0x82ce20, 0x82ce2c, 0x82ce34, 0x82ce48, 0x82ce58, 0x82ce64, 0x82ce74, 0x82ce7c, 0x82ce84, 0x82ce8c, 0x82cea0, 0x82cea8, 0x82ceb4 | 0x6dd49c, 0x6dd48c, 0x6dd480, 0x6dd474, 0x6dd468, 0x6dd458, 0x6dd44c, 0x6dd440, 0x6dd434, 0x6dd424, 0x6dd414, 0x6dd408, 0x6dd3fc, 0x6dd3f0, 0x6dd3e4, 0x6dd3d8, 0x6dd3d0, 0x6dd3c8, 0x6dd3bc, 0x6dd3b4, 0x6dd3a4, 0x6dd398, 0x6dd388, 0x6dd374, 0x6dd36c, 0x6dd364, 0x6dd358, 0x6dd34c, 0x6dd340, 0x6dd338, 0x6dd330, 0x6d5f48, 0x6dd32c, 0x6dd324, 0x6dd31c, 0x6dd310, 0x6dd304, 0x6dd330, 0x6dd324, 0x6dd2fc, 0x6dd31c, 0x6dd2f4, 0x6dd2e8, 0x6dd2dc, 0x6dd2d4, 0x6dd2c0, 0x6dd2b0, 0x6dd2a4, 0x6dd294, 0x6dd28c, 0x6dd284, 0x6dd27c, 0x6dd268, 0x6dd260, 0x6dd254, 0x6dd244 | gem table: the gem names, one list per value class |
| `IOUN_TITLE` | 1 | 0x82cf38 | 0x6dd950 | Ioun stone table: title |
| `IOUN_HEAD` | 2 | 0x82cf4c, 0x82cf74 | 0x6dd928, 0x6dd900 | Ioun stone table: column heading lines |
| `IOUN` | 18 (1+1+1+1+1+1+1+2+1+1+2+2+2+1) | 0x82cf9c, 0x82cfd0, 0x82d004, 0x82d038, 0x82d06c, 0x82d0a0, 0x82d0d4, 0x82d110, 0x82d148, 0x82d174, 0x82d1b0, 0x82d1e4, 0x82d218, 0x82d248, 0x82d27c, 0x82d2ac, 0x82d2d4, 0x82d2e8 | 0x6dd8cc, 0x6dd898, 0x6dd864, 0x6dd830, 0x6dd7fc, 0x6dd7c8, 0x6dd78c, 0x6dd754, 0x6dd728, 0x6dd6ec, 0x6dd6b8, 0x6dd684, 0x6dd654, 0x6dd620, 0x6dd5f0, 0x6dd5c8, 0x6dd5b4, 0x6dd580 | Ioun stone table: the row texts (one or two lines per row) |
| `IOUN_DEAD` | 1 | 0x82d31c | 0x6dd54c | Ioun stone table: the burned-out stone row |
| `IOUN_MAX` | 1 | 0x82d354 | 0x6dd53c | Ioun stone table: the note on the most stones |
| `IOUN_ABSORB_4` | 2 | 0x82d364, 0x82d398 | 0x6dd508, 0x6dd4d8 | Ioun stone table: absorption note (4 levels) |
| `IOUN_ABSORB_8` | 2 | 0x82d3c8, 0x82d3fc | 0x6dd4a4, 0x6dd4d8 | Ioun stone table: absorption note (8 levels) |
| `WILD_DEVOTION_HEAD` | 2 | 0x82ee70, 0x82ee84 | 0x6df9d8, 0x6ddd94 | wild talent devotions: column headings |
| `WILD_SCIENCE_HEAD` | 2 | 0x82ee98, 0x82eeac | 0x6df9c4, 0x6ddd94 | wild talent sciences: column headings |
| `POWER_HEAD` | 2 | 0x82a474, 0x82a484 | 0x6dbbf0, 0x6dbbe4 | intelligent weapon powers: column headings |
| `PURPOSE_HEAD` | 2 | 0x82a4f0, 0x82a500 | 0x6db93c, 0x6dbc28 | intelligent weapon purpose: column headings |
| `SPECIAL_POWER_HEAD` | 2 | 0x82a510, 0x82a52c | 0x6dbbc8, 0x6dbbac | intelligent weapon special powers: column headings |
| `PAGES` | 1 | 0x82b004 | 0x6de628 | magic book pages: lead text |
| `PAGES_HEAD` | 2 | 0x82b00c, 0x82b034 | 0x6de600, 0x6de5d8 | magic book pages: column headings |
| `BEADS` | 3 | 0x82b1a4, 0x82b1bc, 0x82b1cc | 0x6de478, 0x6de468, 0x6de44c | bead item: the bead row texts |
| `BEAD_HEAD` | 2 | 0x82b1f0, 0x82b20c | 0x6de430, 0x6de414 | bead item: column headings |
| `PATCHES` | 1 | 0x82b380 | 0x6de2a8 | patch item: lead text |
| `PATCH_HEAD` | 2 | 0x82b394, 0x82b3ac | 0x6de290, 0x6de778 | patch item: column headings |
| `RESULT` | 2 | 0x82e198, 0x82e1a8 | 0x6dcdc8, 0x6dcdb8 | Roll Results column headings (roll, result) of a follow-up table |
| `ITEM_HEAD` | 2 | 0x829648, 0x829678 | 0x6db794, 0x6db764 | magic item tables: column headings (roll, item, XP) |
| `JEWEL_HEAD` | 2 | 0x8299a0, 0x8299d4 | 0x6deb04, 0x6dead0 | jewelry table: column headings |
| `HORSE_HEAD` | 2 | 0x8258b4, 0x8258d8 | 0x6dbb70, 0x6dbb4c | horse table: column headings |
| `HIGH_SPIRITED_HEAD` | 2 | 0x829524, 0x829548 | 0x6dbb28, 0x6dbb04 | high-spirited horse table: column headings |
| `PUNCH_HEAD` | 2 | 0x825964, 0x825994 | 0x6dbad4, 0x6dbaac | punching table: column headings |
| `WRESTLE_HEAD` | 2 | 0x8259bc, 0x8259ec | 0x6dba7c, 0x6dba54 | wrestling table: column headings |
| `SUBTABLE_89` | 3 | 0x8296c8, 0x8296d8, 0x8296e8 | 0x6de87c, 0x6de86c, 0x6de85c | magic item table 89: the sub-table branch labels |
| `WIZARD_SCROLL_HEAD` | 2 | 0x829720, 0x829788 | 0x6de9a8, 0x6de950 | wizard scroll table: column headings |
| `PRIEST_SCROLL_HEAD` | 2 | 0x829754, 0x829788 | 0x6de97c, 0x6de950 | priest scroll table: column headings |
| `SPELL_HEAD` | 2 | 0x8297bc, 0x8297cc | 0x6de940, 0x6de930 | spell result: column headings |
| `SUBTABLE_90` | 2 | 0x82970e, 0x8297ee | 0x6de9e6, 0x6de91e | magic item table 90: sub-table branch labels (addresses point inside longer strings) |
| `SCROLL_ITEM_HEAD` | 2 | 0x829800, 0x829830 | 0x6de8dc, 0x6de8ac | scroll item table: column headings |
| `SUBTABLE_91` | 2 | 0x829874, 0x829894 | 0x6debe0, 0x6debc0 | magic item table 91: sub-table branch labels |
| `SUBTABLE_96` | 2 | 0x829964, 0x829990 | 0x6deb80, 0x6deb54 | magic item table 96: sub-table branch labels |
| `SUBTABLE_104` | 2 | 0x829ea3, 0x829ec7 | 0x6dea9f, 0x6dea7b | magic item table 104: sub-table branch labels (addresses point inside longer strings) |
| `ARMOR_HEAD` | 2 | 0x829f58, 0x829f64 | 0x6ded60, 0x6dbbe4 | magic armor table: column headings |
| `AC_HEAD` | 2 | 0x829f98, 0x829fc8 | 0x6ded0c, 0x6decdc | armor class adjustment table: column headings |
| `ARMOR_TYPE_HEAD` | 2 | 0x82a018, 0x82a048 | 0x6dec90, 0x6dec60 | armor type table: column headings |
| `WEAPON_HEAD` | 2 | 0x82a078, 0x82a088 | 0x6dea58, 0x6dcdb8 | magic weapon table: column headings |
| `SUBTABLE_108` | 2 | 0x82a0b4, 0x82a0e4 | 0x6dea44, 0x6dea14 | magic item table 108: sub-table branch labels |
| `SWORD_HEAD` | 2 | 0x82a0fc, 0x82a12c | 0x6dba24, 0x6db9f4 | sword table: column headings |
| `OTHER_WEAPON_HEAD` | 2 | 0x82a184, 0x82a1b4 | 0x6db9c4, 0x6db994 | other weapon table: column headings |
| `SUBTABLE_110` | 4 | 0x82a28d, 0x82a2a1, 0x82a2b5, 0x82a2c9 | 0x6db731, 0x6db71d, 0x6db709, 0x6db6f5 | magic item table 110: sub-table branch labels |
| `RESULT_88` | 2 | 0x82e204, 0x82e214 | 0x6dcdc8, 0x6dbbe4 | Roll Results column headings of table 88 |
| `INTELLIGENCE_HEAD` | 2 | 0x82a358, 0x82a380 | 0x6dbcb0, 0x6dbc88 | intelligent weapon intelligence: column headings |
| `ALIGNMENT_HEAD` | 2 | 0x82eb98, 0x82ebb4 | 0x6dec44, 0x6dec28 | alignment table: column headings |
| `ABILITY_HEAD` | 2 | 0x82a428, 0x82a438 | 0x6dbc38, 0x6dbc28 | intelligent weapon abilities: column headings |
| `SPECIAL_POWER_HEAD_B` | 2 | 0x82a490, 0x82a4ac | 0x6dbbc8, 0x6dbbac | intelligent weapon special powers (second table): column headings |
| `HEAD_90001` | 2 | 0x82ad2c, 0x82ad40 | 0x6de848, 0x6de820 | Roll Results column headings of follow-up table 90001 |
| `HEAD_90002` | 2 | 0x82ad54, 0x82ad68 | 0x6de834, 0x6de820 | Roll Results column headings of follow-up table 90002 |
| `HEAD_90003` | 2 | 0x82ad7c, 0x82ad90 | 0x6de80c, 0x6de820 | Roll Results column headings of follow-up table 90003 |
| `HEAD_90004` | 2 | 0x82ada4, 0x82adc4 | 0x6de7ec, 0x6de7cc | Roll Results column headings of follow-up table 90004 |
| `HEAD_90005` | 2 | 0x82ade4, 0x82adf4 | 0x6de7bc, 0x6dbc28 | Roll Results column headings of follow-up table 90005 |
| `HEAD_90006` | 2 | 0x82ae04, 0x82ae18 | 0x6de7a8, 0x6de820 | Roll Results column headings of follow-up table 90006 |
| `HEAD_90007` | 2 | 0x82ae2c, 0x82ae44 | 0x6de790, 0x6de778 | Roll Results column headings of follow-up table 90007 |
| `HEAD_90008` | 2 | 0x82ae5c, 0x82ae70 | 0x6de764, 0x6de820 | Roll Results column headings of follow-up table 90008 |
| `HEAD_90009` | 2 | 0x82ae84, 0x82ae9c | 0x6de74c, 0x6de778 | Roll Results column headings of follow-up table 90009 |
| `HEAD_90010` | 2 | 0x82aeb4, 0x82aed8 | 0x6de728, 0x6de704 | Roll Results column headings of follow-up table 90010 |
| `HEAD_90011` | 2 | 0x82aefc, 0x82af18 | 0x6de6e8, 0x6dec28 | Roll Results column headings of follow-up table 90011 |
| `HEAD_90012` | 2 | 0x82af34, 0x82af48 | 0x6de6d4, 0x6de6c0 | Roll Results column headings of follow-up table 90012 |
| `HEAD_90013` | 2 | 0x82af5c, 0x82af74 | 0x6de6a8, 0x6db35c | Roll Results column headings of follow-up table 90013 |
| `HEAD_90029` | 2 | 0x82af8c, 0x82afa0 | 0x6de694, 0x6de680 | Roll Results column headings of follow-up table 90029 |
| `HEAD_90015` | 2 | 0x82afb4, 0x82afdc | 0x6de658, 0x6de630 | Roll Results column headings of follow-up table 90015 |
| `HEAD_90031` | 3 | 0x82b05c, 0x82b088, 0x82b0b8 | 0x6de5ac, 0x6de57c, 0x6de54c | Roll Results column headings of follow-up table 90031 |
| `HEAD_90016` | 3 | 0x82b0e8, 0x82b104, 0x82b138 | 0x6de530, 0x6de4fc, 0x6de4c8 | Roll Results column headings of follow-up table 90016 |
| `HEAD_90032` | 3 | 0x82b16c, 0x82b17c, 0x82b190 | 0x6de4b8, 0x6de4a4, 0x6de490 | Roll Results column headings of follow-up table 90032 |
| `HEAD_90035` | 3 | 0x82b228, 0x82b240, 0x82b25c | 0x6de3fc, 0x6de3e0, 0x6de3c4 | Roll Results column headings of follow-up table 90035 |
| `HEAD_90036` | 3 | 0x82b278, 0x82b294, 0x82b2b4 | 0x6de3a8, 0x6de388, 0x6de368 | Roll Results column headings of follow-up table 90036 |
| `HEAD_90037` | 3 | 0x82b2d4, 0x82b2f8, 0x82b32c | 0x6de344, 0x6de310, 0x6de2dc | Roll Results column headings of follow-up table 90037 |
| `HEAD_90038` | 2 | 0x82b360, 0x82b370 | 0x6de2cc, 0x6de2bc | Roll Results column headings of follow-up table 90038 |
| `HEAD_90025` | 3 | 0x82b414, 0x82b438, 0x82b450 | 0x6de26c, 0x6de254, 0x6de230 | Roll Results column headings of follow-up table 90025 |
| `HEAD_90026` | 2 | 0x82b474, 0x82b488 | 0x6de21c, 0x6de820 | Roll Results column headings of follow-up table 90026 |
| `HEAD_90027` | 3 | 0x82b49c, 0x82b4d4, 0x82b50c | 0x6de1e4, 0x6de1ac, 0x6de174 | Roll Results column headings of follow-up table 90027 |
| `HEAD_90017` | 2 | 0x82b648, 0x82b674 | 0x6de048, 0x6de01c | Roll Results column headings of follow-up table 90017 |
| `HEAD_90028` | 2 | 0x82b6a0, 0x82b6b0 | 0x6de00c, 0x6ddffc | Roll Results column headings of follow-up table 90028 |
| `HEAD_90030` | 2 | 0x82b6d8, 0x82b700 | 0x6ddfbc, 0x6ddf98 | Roll Results column headings of follow-up table 90030 |
| `HEAD_90048` | 2 | 0x82b76c, 0x82b778 | 0x6ddf48, 0x6ddf3c | Roll Results column headings of follow-up table 90048 |
| `HEAD_90020` | 2 | 0x82b784, 0x82b7b4 | 0x6ddf0c, 0x6ddedc | Roll Results column headings of follow-up table 90020 |
| `HEAD_90022` | 2 | 0x82b844, 0x82b874 | 0x6ddeac, 0x6db764 | Roll Results column headings of follow-up table 90022 |
| `HEAD_90024` | 2 | 0x82b904, 0x82b918 | 0x6de7a8, 0x6de820 | Roll Results column headings of follow-up table 90024 |
| `HEAD_90041` | 2 | 0x82b98c, 0x82b9b4 | 0x6dde84, 0x6dde5c | Roll Results column headings of follow-up table 90041 |
| `HEAD_90042` | 4 | 0x82b9dc, 0x82b9f8, 0x82ba18, 0x82ba48 | 0x6dde40, 0x6dde20, 0x6dddf0, 0x6dddc0 | Roll Results column headings of follow-up table 90042 |
| `HEAD_90043` | 2 | 0x82ba78, 0x82ba90 | 0x6ddda8, 0x6ddd94 | Roll Results column headings of follow-up table 90043 |
| `HEAD_90044` | 2 | 0x82baa4, 0x82bab4 | 0x6ddd84, 0x6dcdb8 | Roll Results column headings of follow-up table 90044 |
| `HEAD_90045` | 2 | 0x82bac4, 0x82bae4 | 0x6ddd64, 0x6de7cc | Roll Results column headings of follow-up table 90045 |
| `GIANT_NOTE` | 6 | 0x82b548, 0x82b578, 0x82b5a8, 0x82b5d8, 0x82b608, 0x82b638 | 0x6de144, 0x6de114, 0x6de0e4, 0x6de0b4, 0x6de084, 0x6de074 | follow-up table 90027: footnote lines |
| `PURSE` | 1 | 0x82b6c0 | 0x6ddfe4 | follow-up table 90030: title text |
| `PURSE_NOTE` | 2 | 0x82b728, 0x82b750 | 0x6ddf70, 0x6ddf54 | follow-up table 90030: footnote lines |
| `TRACK_SUBTABLES` | 2 | 0x82b3c4, 0x82b3d4 | 0x6ddd54, 0x6ddd44 | tracking: sub-table labels |
| `TRACK_HEAD` | 2 | 0x82b3e4, 0x82b3fc | 0x6ddd2c, 0x6ddd14 | tracking table: column headings |
| `FREQUENCY_HEAD` | 2 | 0x82e0f8, 0x82e108 | 0x6df760, 0x6df6c8 | frequency table: column headings |
| `GENERAL_TRAIT_HEAD` | 2 | 0x82e134, 0x82e148 | 0x6df770, 0x6ddd94 | general traits table: column headings |
| `SEASONS` | 3 | 0x826e24, 0x826e30, 0x826e38 | 0x6dbe34, 0x6dbe2c, 0x6dbe24 | parameter options: seasons |
| `SEASON_HEADS` | 6 (2+2+2) | 0x825a18, 0x825a38, 0x825a58, 0x825a70, 0x825a88, 0x825aa0 | 0x6db91c, 0x6db8fc, 0x6db8e4, 0x6db8cc, 0x6db8b4, 0x6db8cc | per-season tables: column headings, one pair per season |
| `VALUE_HEAD` | 2 | 0x82e1d4, 0x82e1e0 | 0x6df784, 0x6dbbe4 | value table: column headings |
| `RESULT_111` | 2 | 0x82eb10, 0x82eb20 | 0x6dcdc8, 0x6dcdb8 | Roll Results column headings of table 111 |
| `HEIGHT` | 2 | 0x8257a4, 0x825794 | 0x6dbd74, 0x6dbd68 | height and weight result: height label lines |
| `WEIGHT` | 2 | 0x8257e8, 0x8257d8 | 0x6dbd34, 0x6dbd28 | height and weight result: weight label lines |
| `SEXES` | 2 | 0x826da4, 0x826dac | 0x6c2614, 0x6c260c | parameter options: the two sexes |
| `STARTING_AGE` | 1 | 0x825830 | 0x6dbdd8 | age result: starting age label |
| `MAXIMUM_AGE` | 1 | 0x825864 | 0x6dbda4 | age result: maximum age label |
| `LEADER_HEAD` | 2 | 0x82de90, 0x82dea0 | 0x6df6b8, 0x6dcdb8 | followers: leader table column headings |
| `TROOPS_HEAD` | 2 | 0x82deb0, 0x82ded8 | 0x6df690, 0x6df668 | followers: troops table column headings |
| `ELITE_HEAD` | 2 | 0x82df00, 0x82df14 | 0x6df654, 0x6de820 | followers: elite unit table column headings |
| `FOLLOWERS_HEAD` | 2 | 0x82df48, 0x82df58 | 0x6df6d8, 0x6df6c8 | followers table: column headings |
| `FOLLOWER_HEAD` | 2 | 0x82df84, 0x82dfb4 | 0x6df718, 0x6df6e8 | follower table: column headings |
| `SKILLS_HEAD` | 2 | 0x82e000, 0x82e018 | 0x6df748, 0x6ddd14 | skills table: column headings |
| `EVENT_HEAD` | 2 | 0x82ed4c, 0x82ed58 | 0x6df95c, 0x6dbbe4 | life events table: column headings |
| `SOCIAL_HEAD` | 2 | 0x82edbc, 0x82edd0 | 0x6df968, 0x6de820 | social status table: column headings |
| `KIT_HEAD` | 2 | 0x82ee04, 0x82ee10 | 0x6df9a8, 0x6df99c | kit table: column headings |
| `COMPANION_HEAD` | 2 | 0x82ee38, 0x82ee48 | 0x6df9b4, 0x6df6c8 | companion table: column headings |
| `PHOBIA_HEAD` | 2 | 0x82ef5c, 0x82ef6c | 0x6df9ec, 0x6dcdb8 | phobia table: column headings |
| `LOCATION_HEAD` | 2 | 0x8261e0, 0x8261f8 | 0x6db374, 0x6db35c | hit location table: column headings |
| `LOCATION_TARGETS` | 3 | 0x827044, 0x82705c, 0x827078 | 0x6dc1b0, 0x6dc194, 0x6dbf70 | parameter options: hit location target types |
| `WEAPON_TARGETS` | 5 | 0x827488, 0x8274a0, 0x8274b4, 0x8274cc, 0x8274f4 | 0x6dc1b0, 0x6dc030, 0x6dc018, 0x6dbff0, 0x6dbfd4 | critical hits (weapons) parameter options: target types |
| `WEAPON_TYPES` | 3 | 0x82743c, 0x827458, 0x827470 | 0x6dc074, 0x6dc05c, 0x6dc044 | critical hits (weapons) parameter options: weapon damage types |
| `WEAPON_SIZE_ADVANTAGE` | 3 | 0x827510, 0x827528, 0x827544 | 0x6dc15c, 0x6dc140, 0x6dc124 | critical hits (weapons) parameter options: size advantage |
| `WEAPON_SIZES` | 4 | 0x827560, 0x827580, 0x8275a0, 0x8275c0 | 0x6dbfb4, 0x6dab34, 0x6dbf94, 0x6daaf4 | critical hits (weapons) parameter options: weapon sizes |
| `CRITICAL_TITLE` | 1 | 0x826c4c | 0x6daad0 | critical hits (weapons): entry title |
| `CRITICAL_LIST` | 1 | 0x82db3c | 0x6df180 | critical hits (weapons): list label |
| `STRIKE_HEAD` | 2 | 0x826c74, 0x826c90 | 0x6db438, 0x6db41c | critical hits (weapons): strike table column headings |
| `CALLED_HEAD` | 2 | 0x826cac, 0x826cbc | 0x6db40c, 0x6db3fc | critical hits (weapons): called shot column headings |
| `SEVERITY_EFFECT_HEAD` | 2 | 0x826cd0, 0x826ce4 | 0x6db3e8, 0x6db3d4 | critical hits (weapons): severity and effect column headings |
| `SPELL_TARGETS` | 3 | 0x827304, 0x82731c, 0x827338 | 0x6dc1b0, 0x6dc194, 0x6dc174 | critical hits (spells) parameter options: target types |
| `SPELL_SIZE_ADVANTAGE` | 3 | 0x827358, 0x827370, 0x82738c | 0x6dc15c, 0x6dc140, 0x6dc124 | critical hits (spells) parameter options: size advantage |
| `SPELL_MAX_DAMAGE` | 4 | 0x8273a8, 0x8273c8, 0x8273e4, 0x827404 | 0x6dc104, 0x6dc0e8, 0x6dc0c8, 0x6dc0a4 | critical hits (spells) parameter options: maximum damage |
| `SPELL_CRITICAL_TITLE` | 1 | 0x8262b4 | 0x6db454 | critical hits (spells): entry title |
| `SPELL_CRITICAL_LIST` | 1 | 0x82dd30 | 0x6db454 | critical hits (spells): list label |
| `SPELL_STRIKE_HEAD` | 2 | 0x8262d8, 0x8262f4 | 0x6db438, 0x6db41c | critical hits (spells): strike table column headings |
| `SPELL_CALLED_HEAD` | 2 | 0x826310, 0x826320 | 0x6db40c, 0x6db3fc | critical hits (spells): called shot column headings |
| `SPELL_SEVERITY_HEAD` | 2 | 0x826334, 0x826348 | 0x6db3e8, 0x6db3d4 | critical hits (spells): severity and effect column headings |
| `EXTRA_LOCATION_HEAD` | 3 | 0x82635c, 0x826364, 0x826380 | 0x6db3cc, 0x6db438, 0x6db3b0 | extra hit location column headings |
| `RANDOM_LOCATION` | 1 | 0x829018 | 0x6ddcec | called shot option: the random location choice |
| `INSANITY_HEAD` | 2 | 0x825fd0, 0x825ff8 | 0x6db6a0, 0x6db678 | insanity table: column headings |
| `INSANE` | 2 | 0x82602c, 0x826034 | 0x6db670, 0x6db668 | insanity result: the two outcome texts |
| `INSANITY_NOTE` | 5 | 0x826040, 0x82606c, 0x826098, 0x8260c4, 0x8260f4 | 0x6db63c, 0x6db610, 0x6db5e4, 0x6db5b4, 0x6db5a0 | insanity table: footnote lines |
| `CHART_HEAD` | 3 | 0x826150, 0x82615c, 0x826170 | 0x6db574, 0x6db560, 0x6db54c | chart table: column headings |
| `CHART_NOTE` | 2 | 0x826184, 0x8261ac | 0x6db524, 0x6db514 | chart table: footnote lines |

## Missing or different program

| user | when the program is missing | when it is present but not the known build | when a string is not there |
|---|---|---|---|
| NPC generator (`Installation::npc_rules` → `Data::load`) | error `Program/Tsrp3.exe is missing: some texts are read from the program`; no NPC can be made | error `<path>: not the program version whose texts this app knows (<n> bytes)`, or `<path>: not a Windows program` when the section table cannot be read; a read failure is an I/O error | `the program has no text at <va>` for a text; `the program's <what> were not found where the NPC generator reads them` when a push list is short; a missing stack, club or id table is empty |
| Encounter NPC packages | the encounter loads; each NPC package fails with `The NPC generator is not available: <error>` | same | same |
| Intelligent weapons | each line reads `(an ability line of Program/<exe>, which is missing)` | same text (the open error is dropped) | same text |
| Character sheet, Turning Undead | the undead labels read `Undead <n>:` and each legend line `(legend text: the program file is missing)` | same | same, per string |
| Import of a Core Rules 2.0 part with old class flags | the import fails with the open error | the import fails with the open error | the import fails: `the program has no class names` |
| Custom-data editor lists | the list is empty, no error | not detected: the strings at the known addresses are used if they are printable; otherwise error `<path>: unexpected program (list <name>)` | the whole list fails with the same error (address outside every section, no NUL, a byte outside 0x20..0x7e, or 200 bytes or more) |
| Table Rolls | text left blank; the result gets the note "Some text of this result is read from the original program, and `Program/<exe>` is missing; it is left blank." | same note with `is not the build this program knows`, `is not a Windows program`, or `cannot be read (<error>)` | blank, with the note ending `has no such text` |

The NPC generator itself runs only for the Expansion: `npc_rules` refuses a Core Rules 2.0
installation, so the Evermore.exe addresses of npc/program.rs and npc/texts.rs are not
reached by the app today.

## Notes

* Three PE readers exist with different rules (table under
  [The three readers](#the-three-readers)). Only `exe::Image` checks that a string lies in a
  data section; only `userdb::program` skips the build check and caps the length at 200.
* exe.rs takes a section's extent as `max(vsize, rsize)`. Tsrp3.exe's `.data` has vsize
  0x5a5c8 and rsize 0x53000, so an address in 0x860000..0x8675c8 (the uninitialized tail)
  would map to file bytes of `.rsrc`. No address the app uses is in that range.
* `pushed_texts` is a byte scan, not a disassembly; the fixed lengths and counts guard it.
  The backgrounds are read from 0x54f66a, ten bytes into what npc/program.rs and the old
  notes call 0x54f660 (the load site).
* Evermore.exe address 0x48442e (the first entry of `TRAITS`, a one-character string) is in
  `.text`; the Table Rolls reader does not check the section kind, so it reads it there.
  `SUBTABLE_90` and `SUBTABLE_104` point inside longer strings in both programs and so read
  a string's tail.
* `userdb/program.rs` lists the tight weapon groups by address (ids 1..35 in order) for the
  editors, while the NPC generator reads the same table from the constructor's code
  (`id_tables`, member 0x248) with the ids the code passes.
* The old notes (`/root/re/spec/npc.md`, section 6) list 0x4d7d99 and the club kit 0x4d8040 as not decoded;
  the app now reads both, and the club kit's replacements at 0x4d81ad. They also say the
  capacity-by-Strength table of 0x4290f5 is read from the exe; the app keeps it as a constant
  (`npc::program::CAPACITY`).
