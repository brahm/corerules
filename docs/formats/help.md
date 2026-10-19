# Help: `Help/*.hlp`, `Help/*.cnt`

The Expansion's help is a set of WinHelp 4 files in `Help/`: the Online Manual
(`Manual.hlp`), the program help the Help… buttons open (`Program.hlp`), the help versions
of the rule books (`Players.hlp`, `Dungeon.hlp`, …) and a few smaller files. Each `.hlp` may
have a contents file with the same name and `.cnt`. The original hands them to the system's
WinHelp (`WinHelpA`); the app has its own reader and help window.

Rust:

* [`winhelp.rs`](../../crates/cr-formats/src/winhelp.rs) (`HelpFile`, `Topic`, `Paragraph`,
  `Item`, `Run`, `Link`, `Macro`, `context_hash`, `lz77`): the `.hlp` reader, read only.
* [`winhelp_bitmap.rs`](../../crates/cr-formats/src/winhelp_bitmap.rs): the `|bmN` pictures.
* [`help.rs`](../../crates/cr-store/src/help.rs) (`Contents`, `Installation::help_file`,
  `Installation::help_contents`, `topic`, `Installation::screen_help`,
  `Installation::item_help`, `Installation::original_about`): the `.cnt` reader, the
  lookup in `Help/`, the screens' topics, the right-click help and the About box's texts.
* [`cr-gtk/src/help.rs`](../../crates/cr-gtk/src/help.rs): the help window (behaviour only,
  summarised at the end).

Every installed file reads (32 in the Expansion), every topic its `.cnt` lists opens, every
picture decodes, and every link and `JumpId` macro to another file opens a topic there except
five, whose targets are not in the file they name (WinHelp cannot open them either). The tests
in [`tests/help.rs`](../../crates/cr-store/tests/help.rs) check this against an installation.

The layout below is the subset these files use: WinHelp 3.1 and 4 files (`|SYSTEM` minor
version above 16). All integers are little endian.

## Internal file system

| offset | size | field |
|---|---|---|
| 0 | 4 | magic `0x00035F3F` |
| 4 | 4 | offset of the directory's internal file |
| 8 | 4 | offset of the first free block (unused here) |
| 12 | 4 | file size |

Each internal file starts with a 9-byte header (`reserved` u32, `used` u32, `flags` u8);
its content is the `used` bytes after it. The directory is a B+ tree whose leaves map
NUL-terminated names (`|SYSTEM`, `|TOPIC`, `|bm0`, …) to the internal file's offset.

A B+ tree (the directory, `|CONTEXT`, `|TTLBTREE`, …) has a 38-byte header and then its
pages:

| offset | size | field |
|---|---|---|
| 0 | 2 | magic `0x293B` |
| 4 | 2 | page size |
| 26 | 2 | root page |
| 32 | 2 | number of levels |

An index page starts with `unused` u16, `entries` u16 and the first child page u16 (offset
4); the reader descends through the first child to the first leaf. A leaf page starts with
`unused` u16, `entries` u16, `previous` u16 and `next` u16 (−1 after the last leaf), then
its entries.

## `|SYSTEM`

| offset | size | field |
|---|---|---|
| 0 | 2 | magic `0x036C` |
| 2 | 2 | minor version (33 for WinHelp 4) |
| 4 | 2 | major version (1) |
| 6 | 4 | generation date |
| 10 | 2 | flags: 0 not compressed; 4 LZ77, 4 KB topic blocks; 8 LZ77, 2 KB topic blocks |

Then records of `type` u16, `size` u16 and `size` bytes, to the end. The app reads type 1
(the title, a NUL-terminated string), 2 (the copyright) and 3 (the contents topic, a
`TOPICOFFSET` u32). Without type 3 the contents topic is the first one.

## `|TOPIC`

`|TOPIC` is a sequence of blocks (4 KB, or 2 KB when the flags are 8). A block has a 12-byte
header (last topic link, first topic link, last topic header) and then its data, LZ77
compressed when the flags are 4 or 8. Decompressed, the blocks are 16 KB each, and a
`TOPICPOS` is `block × 0x4000 + offset`, where `offset` counts from the block header's start
(so data starts at 12).

**LZ77.** A flag byte gives the next eight items, least significant bit first: a 0 bit is a
literal byte; a 1 bit is a u16 `w`, which copies `(w >> 12) + 3` bytes from `(w & 0xFFF) + 1`
bytes back.

### Topic links

The decompressed stream is a chain of topic links, starting at `TOPICPOS` 12:

| offset | size | field |
|---|---|---|
| 0 | 4 | block size: the whole link, header included |
| 4 | 4 | `LinkData2` size, after phrase expansion |
| 8 | 4 | previous link (`TOPICPOS`) |
| 12 | 4 | next link (`TOPICPOS`) |
| 16 | 4 | `LinkData1` size, header included |
| 20 | 1 | record type: 2 topic header, 0x20 display text, 0x23 table |

`LinkData1` follows the 21-byte header; `LinkData2` (text) fills the rest of the block size.
`LinkData2` is phrase compressed when its stored size is less than the size at 4.

**`TOPICOFFSET`.** Links to topics (`|CONTEXT`, `|CTXOMAP`, jump hotspots) give a
`TOPICOFFSET`: `block × 0x8000 + characters`, where `block` is the `TOPICPOS` block of the
record and `characters` counts the text characters (each display or table record's
`TopicLength`) since the start of that block. A topic header's offset is the value at its
record; a topic is found as the last header at or before an offset.

**Topic header (type 2).** `LinkData1` holds seven i32: size of the topic, next topic and
previous topic headers, topic number, the first non-scrolling record (offset 16, −1 for
none), the first scrolling record and the next topic in the browse sequence. `LinkData2`
starts with the NUL-terminated title (empty for topics without one), then the topic's entry
macros.

### Display and table records (0x20, 0x23)

`LinkData1` starts with a compressed long `TopicSize` and an unsigned compressed short
`TopicLength` (the characters this record adds to `TOPICOFFSET`). An unsigned compressed
short is one byte `b >> 1` when its low bit is clear, otherwise a u16 `>> 1`; the signed one
subtracts 0x40 (one byte) or 0x4000 (two). A compressed long is a u16 `>> 1` minus 0x4000
when the low bit is clear, otherwise a u32 `>> 1` minus 0x4000_0000.

(These are read only to skip them.) A table record then has `columns` u8, `type` u8 (an i16 minimum width follows when the type
is 0 or 2) and 4 bytes per column (gap width, column width).

Then paragraphs, one in a display record and one per cell in a table record. A table cell
starts with the `column` i16 (−1 ends the row) and 3 unused bytes. Every paragraph has 4
fixed bytes, then a u16 of bits saying which fields follow:

| bit | field |
|---|---|
| 0x0001 | a compressed long (unused) |
| 0x0002 | space before, signed compressed short (× 10 for twips) |
| 0x0004 | space after |
| 0x0008 | line spacing |
| 0x0010 | left indent |
| 0x0020 | right indent |
| 0x0040 | first-line indent |
| 0x0100 | border: 3 bytes (sides and width), not shown |
| 0x0200 | tab stops: a signed compressed short count, then for each an unsigned compressed short position in its low 14 bits; its 0x4000 bit says an unsigned compressed short type follows (1 right, 2 centre) |
| 0x0400 | right aligned |
| 0x0800 | centred |

The paragraph's content interleaves the NUL-separated strings of `LinkData2` with the
command bytes that follow the format in `LinkData1`: each string is the text before the next
command, and the commands are:

| byte | meaning | operands |
|---|---|---|
| 0x20 | vfld | 4 bytes |
| 0x21 | dtype | 2 bytes |
| 0x80 | font change | u16 index into `|FONT` |
| 0x81 | line break | |
| 0x82 | paragraph end | |
| 0x83 | tab | |
| 0x86, 0x87, 0x88 | picture as character, left, right | `type` u8, compressed long size, a u16 when `type` is 0x22; for types 3 and 0x22 the data's bytes 2..4 are the `|bmN` number |
| 0x89 | end of hotspot | |
| 0x8B | non-breaking space | |
| 0x8C | non-breaking hyphen | |
| 0xC8, 0xCC | macro hotspot | u16 length, macro string |
| 0xE0, 0xE1 | jump to a `TOPICOFFSET` (popup, jump; WinHelp 3.0) | u32 |
| 0xE2, 0xE3, 0xE6, 0xE7 | jump to a context hash (popup for E2 and E6) | u32 hash |
| 0xEA, 0xEB, 0xEE, 0xEF | jump to a context hash, maybe in another file or window (popup for EA and EE) | u16 size, then in those bytes `type` u8, u32 hash, a window byte when `type` is 1, a NUL-terminated file name when `type` is 4 or 6 |
| 0xFF | end of the paragraphs | |

The macros help files use as links are `JumpId`/`JI` and `PopupId`/`PI` (file, context
string), `JumpContents` (file) and `Inet` (address); `Macro` in `winhelp.rs` reads them.

### Phrases

WinHelp 3.1 files keep phrases in `|Phrases`: a u16 count `n`, a u16 (0x0100), the
decompressed size u32, `n + 1` u16 offsets and the LZ77-compressed phrase text; the
offsets count from the start of the offsets table (`2 × (n + 1) + 4` before the text). In
`LinkData2`, a byte `a` in 1..15 and the byte `b` after it make `m = (a − 1) × 256 + b`,
which stands for phrase `m / 2`, followed by a space when `m` is odd.

WinHelp 4 files (Hall compression) keep them in `|PhrIndex` and `|PhrImage`. `|PhrIndex`
has a 28-byte header (u32 count at 4, u32 image size at 12, u32 compressed image size at
16, and at 24 a u16 whose low 4 bits are `bits`), then the phrase lengths as a bit stream,
least significant bit first: each length is 1, plus `2^bits` for every 1 bit before the
first 0 bit, plus `bits` more bits worth 1, 2, 4, 8 and 16. `|PhrImage` is the phrases one
after another, LZ77 compressed when the compressed size is less than the image size. A
`LinkData2` byte `b` is:

| `b` | meaning |
|---|---|
| even | phrase `b >> 1` |
| `b & 3 == 1` | phrase `128 + (b >> 2) × 256 + next byte` |
| `b & 7 == 3` | `(b >> 3) + 1` literal bytes follow |
| `b & 15 == 7` | `(b >> 4) + 1` spaces |
| `b & 15 == 15` | `(b >> 4) + 1` NULs |

## `|CONTEXT`, `|CTXOMAP`

`|CONTEXT` is a B+ tree of (`hash` u32, `TOPICOFFSET` u32) leaf entries, sorted by hash. A
context string's hash starts at 0 and, for each byte, is `hash × 43 + value` (wrapping u32),
where `value` is 1..9 for `1`..`9`, 10 for `0`, 12 for `.`, 13 for `_`, `upper − 0x30` for
letters and `(i8)b − 0x30` for anything else. `context_hash` computes it; it matches every
context of every `.cnt` file installed.

`|CTXOMAP` is the help project's `[MAP]` section: a u16 count, then (`number` u32,
`TOPICOFFSET` u32) pairs. These are the numbers a program passes with `HELP_CONTEXT`.

## `|FONT`

| offset | size | field |
|---|---|---|
| 0 | 2 | number of face names |
| 2 | 2 | number of descriptors |
| 4 | 2 | offset of the face names |
| 6 | 2 | offset of the descriptors |

Face names are NUL-padded slots of `(descriptors offset − face names offset) / face names`
bytes (20 or 32). A descriptor is 11 bytes:

| offset | size | field |
|---|---|---|
| 0 | 1 | attributes: 1 bold, 2 italic, 4 underline, 8 strikeout, 0x10 double underline, 0x20 small caps (the app shows all but strikeout) |
| 1 | 1 | half points |
| 2 | 1 | family |
| 3 | 2 | face name index |
| 5 | 3 | foreground RGB; (1, 1, 0) is the automatic colour |
| 8 | 3 | background RGB |

## `|bmN`

A picture file is an SHG/MRB container: magic `lP` (0x506C) or `lp` (0x706C), a u16
count, and a u32 offset per picture. The app shows the first one it can decode. A picture
starts with `type` u8 (5 device-dependent bitmap, 6 device-independent bitmap, 8
metafile) and `packing` u8 (0 none, 1 RLE, 2 LZ77, 3 LZ77 then RLE).

A bitmap then has compressed longs x dpi, y dpi, compressed shorts planes and bit count,
compressed longs width, height, colours used, colours important, data size and hotspot
size, and u32 data offset and hotspot offset, both from the picture's start; a
device-independent bitmap's palette (4 bytes per colour) follows. Device-dependent
bitmaps are word-aligned top-down rows; only monochrome ones are shown. RLE: a count byte
with its high bit set copies `count & 0x7F` bytes, otherwise repeats the next byte `count`
times.

A metafile has a compressed short mapping mode, u16 width and height, compressed longs
size, packed size and hotspot size, and a u32 data offset. The app shows a metafile that
wraps a bitmap (`StretchDIBits` and the like), as the rule books' pictures are read
([books.md](books.md)).

## `.cnt` contents files

A plain-text file (Windows-1252, CRLF). Lines starting with `:` are commands: `:Base`
names the help file, `:Title` the window title, `:Include` puts another contents file's
entries in its place (their topics are in that file's `:Base`), and `:Index` and `:Link`
merge other files' indexes and keywords (the app ignores those). `ADandD.cnt`, the Tome of
Help, is a heading and a `:Include` per book; the app splices in the books installed. Other lines are `level title` for a heading or
`level title=context@file>window` for a topic, where `@file` and `>window` are optional.

`Program.cnt` has only `:Base`, so the program help has no Contents list; WinHelp shows its
Index tab instead.

## How the original opens help

Every Help… button, and F1 in a dialog, calls one helper (0x554F41 in the Expansion's
`Tsrp3.exe`) that runs `WinHelpA(Program.hlp, HELP_CONTEXT, n)`, where `n` is the dialog's
own resource id; `Program.hlp`'s `|CTXOMAP` maps each of those ids to its topic (849
entries). The main window's Help menu has Help (topic 101, the main menu), the wizard's
help, Online Manual (`Manual.hlp`, `HELP_FINDER`) and About. The rule books' window adds
Online Manual and Tome Of Help (`ADandD.hlp`, whose contents include every book's).

The app does the same with its own screens: `cr_store::help::topic` names the dialog ids
the app's screens stand for (New Character's pages, the character editor's tabs, every
database editor, the generators, Export and Import, …), and each screen's Help button
(the header's question mark) and F1 open that topic of `Program.hlp`. The abilities page
opens the topic of the roll method chosen; F1 in the main window opens the section's topic.

## Right-click help

The original's screens are pictures with hotspots: `CBitmapDialog` (0x40F036) makes one
for each control of its `DLGTEMPLATEEX` whose style has bit 0x04 or 0x08 set and 0x40 and
0x80 clear, and keeps the item's help id (`0x8000_0000 | dialog << 16 | control`). A
right-click on a hotspot (0x4EB025) runs `WinHelpA(Program.hlp, HELP_CONTEXTPOPUP, id)`.
`Installation::screen_help` reads those controls from the program
([program.md](program.md#resources-and-message-boxes)) and keeps each one's text and its
topic's title with the id, when the id is a topic of `Program.hlp`.

A right-click on a list row or a combo box opens the help of the record it shows
(0x4B3B22, 0x4E673B), by the list's kind of record and the record's help id:

| record | help id | file | shown |
|---|---|---|---|
| weapons, armor, weapon proficiencies, currency | `PartHead::unknown_0x1e4` | `Equip.hlp` | topic |
| equipment, gems and art | `PartHead::unknown_0x1e4` | `Equip.hlp` | popup |
| nonweapon proficiencies | `PartHead::unknown_0x1e4` | `Prof.hlp` | topic |
| racial abilities, languages, traits, disadvantages, class abilities | `PartHead::unknown_0x1e4` | `Prof.hlp` | popup |
| magical items | `PartHead::unknown_0x1e4` | `Magic.hlp` | topic |
| count | `PartHead::unknown_0x1e4` | `Magic.hlp` | popup |
| schools, spheres | `PartHead::unknown_0x1e4` | `Spells.hlp` | popup |
| kits | `PartHead::unknown_0x1e4` | `Kits.hlp` | topic |
| wizard and priest spells | `Spell::id` | `Spells.hlp` | topic |
| races and subraces | `Race::id` | `Charactr.hlp` | topic |
| classes | `ClassData::unknown_0xc` (not `id`) | `Charactr.hlp` | topic |
| a multi-class (class names joined by `/`) | 100 | `Charactr.hlp` | topic |
| psionic powers | `PsionicPower::unknown_0x24` | `Psionics.hlp` | topic |
| monsters | `Monster::unknown_0x3b8`; when 0, its type's | `Monsters.hlp` | topic |
| monster types | the number `MonsterType::picture` starts with (`atoi`) | `Monsters.hlp` | topic |

An id of 0 or below has no help. Encounters, templates, traps, saved treasure and
characters show a summary instead, and the user database's records a box with their
description. The original also has fixed numbers for a few rows found by name (psionic
disciplines, attack and defense modes, fighting styles, weapon groups, three monsters);
the app does not.

`Installation::item_help` reads the shipped records (`Database/`) and keeps, by name, the
help of each kind whose id is a topic of its file; every shipped race, class, part, spell
and psionic power but the kit `None` has one, and nearly all monsters and monster types.
Part names never repeat across part types, but a part and a race can share a name, so
`ItemKind::order` puts the kind of record the screen lists first.

The app does both with one right-click on any screen (the same screens as F1): the first
of the texts under the pointer (the label, the button, the row's labels, the row or group
title, a tooltip, nearest first) that names a hotspot of the screen, in the interface's
language or in English, shows its popup; otherwise the first that names a shipped record
opens its help. A link in a popup opens the help window.

## The About box

The original's About window is dialog 100: five static lines (the game's name, the Expansion's
name, `Version 3.10`, the copyright and the developer), `License.txt` from the installation folder
(Windows-1252) in an edit box, and a Credits button that opens dialog 412, whose sunken
(`SS_SUNKEN`, 0x1000) static texts are the roles and names; a line that starts with spaces
goes on from the one before. `Installation::original_about` reads them from the program
(see [program.md](program.md#resources-and-message-boxes)), and the app's About window adds
them, under the original's name, to its own name, version and licence.

## The help window

One window per installation. The sidebar lists the file's `.cnt` entries indented by
level, or, when there are none, the titled topics alphabetically (like WinHelp's Index
tab); a search field lists the topics whose title or text contains the words typed.
Topics show their fonts, colours, indents, alignment, tab stops, tables (as tab-separated
rows) and pictures (at most 560 pixels wide). Jumps and `JumpId` macros open their topic,
in another file too; popups and `PopupId` open a dialog; `Inet` opens the browser; Back
(Alt+Left) returns. A link whose topic is missing says so.
