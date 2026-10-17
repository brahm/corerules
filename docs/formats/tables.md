# Table Rolls: `Tables/*.TBL`

The DM Tool Kit's **Table Rolls** screen rolls on random tables from the books. The tables are
plain text files in `Tables/`, one file per table (or per subtable). They are not CArchive files
(see [archive.md](archive.md) for the binary formats); nothing in them is versioned, and both
programs ship the same 239 files byte for byte.

Rust:

* [`tbl.rs`](../../crates/cr-formats/src/tbl.rs) (`TableFile`, `TableRecord`): the reader.
* [`tables.rs`](../../crates/cr-store/src/tables.rs) and its modules
  [`data.rs`](../../crates/cr-store/src/tables/data.rs) (id to file map, list entries, handlers),
  [`code.rs`](../../crates/cr-store/src/tables/code.rs) (the tables the programs work out in code),
  [`program.rs`](../../crates/cr-store/src/tables/program.rs) and
  [`strings.rs`](../../crates/cr-store/src/tables/strings.rs) (text read from the program file):
  what the numbers of a record mean and how a roll uses them.

Original code (TSRP3.EXE unless marked 2.0):

| what | Expansion | 2.0 (EVERMORE.EXE) |
|---|---|---|
| table reader (opens the file with `CStdioFile` from registry `Directories\tables`) | 0x5376ab | |
| table id to file name and record count | 0x534d15 | 0x5c1160 |
| roll on a table | 0x538d84 | 0x5c4e20 |
| per-table code of the Roll Results dialog | 0x50c88e, 0x51d5fd | |
| Tables list per book / launchers | 0x52e86d / 0x52f620 | |

There is no document class and no Serialize: the reader is called with a file name and a record
count, both taken from the id map.

## Files

| | Expansion | 2.0 |
|---|---|---|
| files in `Tables/` | 239 | 239 (same bytes) |
| name case | mostly upper case (`DMG100.TBL`, some `.tbl`) | mixed (`Dmg100.tbl`, `POCT1A.tbl`) |
| ids in the map | 256 | 256 |
| distinct file names mapped | 255 | 254 |
| mapped files absent from `Tables/` | 17 | 17 (the same) |
| files in `Tables/` never mapped | `POCT207E.tbl` | `POCT207E.tbl` |

The app opens `Tables/<file>.tbl` with every path component matched without regard to case
(`Installation::path`), so the name case does not matter. A mapped file that is absent makes the
table id "missing" (`TableRolls::missing`); a roll on it reports that the table is not in the
Tables folder. The original rolls show nothing for them.

Mapped but absent (both installs): `PHB43` (10043), `DMG84` (20084), `DMG70B` (22070),
`DMG999A`..`C` (21999, 22999, 23999), `POSP25` (30025), `POSP31` (30031), `POSM14` (40014),
`POSM15` (40015), `POSM44` (40044), `POSM48` (40048), `POSMHit` (40049), `DMO18` (50018),
`DMO29` (50029), `Sub96C` (90033), `Sub101A` (90040). Most of them are tables the programs
work out in code (see [What is not in the files](#what-is-not-in-the-files)); 90040 is reached
from a DMG101 entry and handled in code.

## Text format

A file is Windows-1252 text with CRLF line ends (all 239 files; no bare LF occurs). It is a
sequence of records of four lines each:

```text
<name>                          line 1
<item1>                         line 2
<item2>                         line 3
<lo> <hi> <v3> <v4> <v5> <v6>   line 4
```

Record 0 is a header; the entries follow it. A record:

| # | field | type | in memory (entry object) | meaning |
|---|---|---|---|---|
| 1 | name | line | +0x04 | the entry's text. `Item0` = no text |
| 2 | item1 | line | +0x08 | second column. `Item1` = unused |
| 3 | item2 | line | +0x0c | third column. `Item2` = unused |
| 4 | lo | number | +0x28 | lowest roll of the entry |
| 5 | hi | number | +0x24 | highest roll of the entry |
| 6 | v3 | number | +0x10 | depends on the table, see [The numbers](#the-numbers) |
| 7 | v4 | number | +0x14 | depends on the table |
| 8 | v5 | number | +0x18 | depends on the table |
| 9 | v6 | number | +0x1c | not used by Table Rolls |

The "in memory" offsets are those of the original's entry object (0x2c bytes, ctor 0x50c15e;
+0x20 holds the rolled value), from the reverse-engineering notes.

### Lines

A line is read like MFC `CStdioFile::ReadString`: bytes up to `\n`, which is consumed; a `\r`
right before it is dropped. At the end of the file a line is whatever is left, possibly empty.
The bytes are decoded as Windows-1252 (`\x96` is an en dash).

The placeholders `Item0` (name), `Item1` (item1) and `Item2` (item2), and empty lines, read as
"no value" (`None` in `TableRecord`). The comparison ignores trailing whitespace. The
placeholder only counts in its own line: `Item1` as a name is a name.

### Numbers

The six numbers are read one byte at a time, not as a line. A number is the bytes up to the next
space or `\n` (consumed, not part of the number); `\r` bytes are skipped. The text is converted
with C `atoi`: optional leading whitespace and sign, then digits up to the first non-digit, and 0
when there are none. So:

* a missing number reads as 0 (`" 1"` is an empty token, then `1`);
* `7abc` reads as 7, `x3` as 0;
* past the end of the file every number is 0;
* the reader does not check that line 4 holds exactly six numbers. Extra numbers stay unread and
  become the start of the next record's name line; too few make the reader take the rest from
  the next line(s). See [Defects in the shipped files](#defects-in-the-shipped-files).

### Header record

Record 0 has the table's internal title as its name (for example `DMGTable105 Armor Type`), the
two placeholders, and `0 0 0 0 0 0` (all files). In DMG106 and DMG107 the header's item lines
hold authoring notes on what the numbers mean instead of the placeholders.

The header title is not the title the dialog prints for a list entry (that comes from the program
and the book caption, see below), but the app uses it as the title of a block when a table is
reached by following an entry (`Title::Header`: the sub-subtables 90001..90048 and plain
follow-up rolls), as the subtitle of a critical hit effect, and, cut at `" Spell"`, as the
options of the spell type parameter (headers of 41100..41109). A header named `Item0` gives no
title.

### Continuation lines: `/n1` to `/n9`

A name may contain markers `/n` followed by a digit 1 to 9. They split it into lines: the first
line is printed in the result column of the roll's row, each following line under it
(`TableRecord::lines`, `split_lines`). Each piece has trailing whitespace removed; a trailing
empty piece (a name that ends in a marker) is dropped. `/n0`, and `/n` without a digit, are text.
Example: `Lamp /n1(lit)/n2` gives `Lamp` and `(lit)`. The digit's value has no meaning beyond
being 1..9. 1318 names in the shipped files use markers.

## Record count and the end of the file

The original never looks for the end of the file. The id map gives, for every table id, the
number of records to read (header included), and the reader reads exactly that many. The app does
the same (`TableFile::parse(data, Some(count))`):

* fewer records in the file than the count: the missing records read as empty (no name, no items,
  all numbers 0). With `hi = 0` they are never chosen by a roll, but they still count when rows are
  addressed by position;
* more records in the file than the count: the rest is never read.

`TableFile::parse(data, None)` (for tools) reads records until only whitespace is left.

The counts are compiled into each program (0x534d15, 2.0 0x5c1160) and are listed in
[Table ids and files](#table-ids-and-files). Where a file differs from its count:

| file | records in the file | count | effect |
|---|---|---|---|
| DMG93 | 16 | 15 | the last record (all zeros) is not read |
| Sub91E | 6 | 5 | the last entry (roll 100) is not read; a roll of 100 on 90013 finds no entry |
| DMG47 | 15 as read | 14 | see the spill below |

## Defects in the shipped files

The reader reproduces these as the original read them:

* **DMG47 (20047) spill.** The number lines of the first two entries hold a whole row of the
  turning matrix (14 and 15 numbers) instead of six. The reader takes six numbers; the rest of
  the line becomes the next record's name. From the second entry on every record is shifted by
  one line: its name is the leftover of the previous number line, its item1 is the entry's real
  name, its item2 is the `Item1` placeholder, its lo is read from the `Item2` line (0), and its
  `hi`/`v3`.. are the entry's `lo`/`hi`/... The 14th record ends 3 bytes before the end of the
  file. Nothing reads this table's records: turning undead is computed in code (`Code::TurnUndead`,
  with the code's own matrix).
* **POCT205C (51302) missing line.** Its fourth record has only one placeholder line. The reader
  takes the number line (`5 5 0 0 0 0`) as item2 and the six numbers from the first six words of
  the next record's name (all 0). That record keeps the rest of its name line. So the roll-5 entry
  has `hi = 0` and is never chosen, and a roll of 5 or 6 selects the roll-6 entry with its first
  six words missing. The file still holds exactly 12 records as read.
* **Trailing spaces** after a `/nK` marker (POCT200A and others) are removed with the piece.

## The numbers

What `lo`, `hi` and `v3` to `v6` mean is not in the file: the code of each table decides. The
app's uses, from `tables.rs`, `data.rs` and `code.rs`:

### lo and hi: the roll range

* An entry covers the rolls `lo..hi`. A roll selects the **first entry whose `hi` is at least the
  roll** (`TableFile::lookup`, as 0x50c88e scans them); `lo` is not checked. A roll above every
  `hi` selects nothing (the row shows the roll and no text).
* Ranges are consecutive in the shipped files (from 2 for the 2d6 and 2d10 tables), except for
  the defects above and Sub110G (90047), whose second entry is `2 2` between `1 50` and `51 75`
  and so is never chosen. The largest `hi` is the die the table is made for
  (`TableFile::max_roll`): 4, 6, 8, 10, 12, 20 or 100 in the shipped files (159 tables of 20, 49
  of 100). A table reached without dice of its
  own (a plain follow-up roll) is rolled on one die of that size.
* Tables whose rows are picked by a parameter rather than a roll have `0 0` on every entry: PHB10
  (11010..14010, height and weight by race and sex), PHB11 (10011, age by race) and POSM23 (40023,
  by spell level). They are addressed by position: entry `i` for option `i`.
* POSM24 (40024) goes up to 150: the d100 roll is raised by a modifier from POSM23 first.
* `lo` is also used for an entry that names its own table (below): the reroll uses a die of
  `lo - 1`.

### name, item1, item2

* `name` is the result text, split at `/nK`. Some names contain dice text such as `1d6 rounds`
  that is replaced by a roll for the tables that ask for it (`dice_text`, two pattern lists from
  0x519d97 and 0x52bf8d).
* A few tables read numbers out of the name: spell scrolls (21090, 23090) take the number of
  spells from the leading number of the name; DMG85 (20085) splits its name at the first double
  space into a value and a class; the Book of Infinite Spells pages (90018) test the name for the
  words `Priest` and `Wizard`.
* `item1` and `item2` are printed as extra columns by the tables whose layout has them (the
  punching and wrestling tables, PHB31, DMG90 scrolls, DMG96 (its value column), DMG113, the
  sub-subtables 90015..90047).
* The names of a table's entries serve as the options of a parameter in three places: races
  (11010, 10011), spell levels (40023) and called shots (the entries of the chosen location
  table, whitespace collapsed, each name once).

### v3

One of three things, by table:

* **An experience point value**: the magic item tables (DMG89 to 110 and their subtables) print it
  in an "XP Value" column (`Col::Xp`, blank when 0 or less). Values of 10000 and more occur
  (DMG92, DMG93, DMG104_A, DMG110_D) and are told apart from table ids only because they are not
  in the id map.
* **The id of a table to roll on next** (values of 10000 or more that are in the id map): DMG88
  names the magic item table, DMG26A and DMG26B name each other, DMG105 names 20106, DMG108_A/B
  name 21109 or 22109, DMG115 names itself.
* **A number the table's code uses**: the base height or weight (PHB10) or base age (PHB11) the
  dice are added to; the dice count of objects of art (DMG87, `v3`d`v4` times `v5`); the
  percent chance of insanity (POSM23, with `v4` the modifier to the POSM24 roll); the lowest spell
  level of a scroll (`v3` to `v4`); a number printed as such (DMG113, `Col::V3`); or a location
  code in the critical hit tables (below).

### v4 and v5

* **Sub-subtable ids**: in the magic item tables `v4` (and in DMG107 and DMG110_A..C `v5`) names a
  `Sub*` table (90001..92019) to roll on after the entry, for example the charges or contents of
  an item.
* **Signed bonus**: DMG106 and DMG109S/O print `v4` with a sign (`+1`, `-1`; `Col::SignedV4`)
  next to the XP value in `v3`.
* Dice sides and multiplier (DMG87), the highest spell level of a scroll (21090, 23090), the
  POSM24 modifier (POSM23), and `v4 = 2` on the Robe of Useful Items entries that add two more
  rolls (90039).
* PHB36 (10036) names itself in `v4`.

### v6

Not used by Table Rolls. It is non-zero in the magic item tables (and in PHB31, and in DMG47
through the spill). The reader's documentation calls it the item number of the magic item in the
program's database; the notes mark that as inferred.

### Following an entry

After an entry is printed, `v3`, `v4` and `v5` are checked in that order (`follow_record`):

* a value below 10000, or not an id of the map, is ignored;
* a value equal to the table's own id rolls twice on the same table with one die of `lo - 1`,
  without following those rolls (skipped when `lo - 1 < 1`);
* any other id is rolled on: with that id's handler when it has one (the cases of 0x51d5fd, such
  as the "(D6)" subtable choices and the sub-subtables' dice and columns), else a plain roll on
  one die of the table's largest `hi`, titled with its header. Chains stop at a depth of 8 (a
  limit of the port).

The location tables of PO: Spells & Magic Table 46 hold location codes in `v3` that are not
followed (`follow = false`); the critical hit dialogs use them instead.

### Critical hit tables

The location codes in `v3` are `X * 1000` plus a base, and the effect table is found by adding
the weapon or spell type and the target type to it:

* **PO: Combat & Tactics** (list entry "Critical Hits"): the hit location tables POCT1A..D
  (50900..50903) give `v3 = 50000 + 1000 * X` (X = 1..5). Effect id = `v3 + w + t`, with
  `w` = 200, 300 or 400 by weapon type and `t` = 0, 1 or 2 by target type (the five target types
  use 50900 + 0, 50901 + 1, 50902 + 2, 50903 + 2, 50903 + 1).
* **PO: Spells & Magic** (list entry "Spell Hit Location and Effect"): the location tables
  POSM46A..C (41046..43046) give `v3 = 40000 + 1000 * X` (X = 1..6). Effect id = `v3 + s`, `s` =
  100..109 by spell type. An effect entry whose `v3` is a table id (POSM46D..H, 44046..48046)
  adds a hit on another location: one roll on that table, then its `v3` + `s` is the next effect
  table (at most two extra hits).

## Table ids and files

A table is addressed by an `i32` id (dialog +0x990). The scheme: `B000 + n` for table `n` of a
book, with a higher thousands digit for its subtables A, B, C...:

| ids | book (`book_of`) | example |
|---|---|---|
| 10000..19999 | Player's Handbook | 10019 = PHB Table 19; 11010..14010 = Table 10 by sex and measure |
| 20000..24999 | Dungeon Master's Guide | 20088 = Table 88; 21089, 22089, 23089 = Table 89 A, B, C |
| 30000..39999 | Player's Option: Skills & Powers | 31083, 32083 = Table 83 A, B |
| 40000..49999 | Player's Option: Spells & Magic | 41046..49046 = Table 46 A..I; 4X100..4X109 effects |
| 50000..50899 | DM Option: High Level Campaigns | 50018, 50029 |
| 50900..59999 | Player's Option: Combat & Tactics | 50900..50903 locations; 5X200..5X402 effects |
| 90000..92999 | (DMG) sub-subtables of the magic items | 90001..90048, 91019, 92019 |

The ids 100..109 and 200, 300, 400 are internal ids of the critical hit dialogs, not tables.

The map (`data::ID_MAP`, `id_map`): id, file name without `.tbl`, records read (header
included). It is the same in both programs except for id 55400.

### PO: Combat & Tactics effect tables and the mix-up

The 45 effect ids are `50000 + 1000 * X + 100 * w + t` (X = 1..5, w = 2..4, t = 0..2) and the
files are named `POCT<200 + 3 * (w - 2) + t><letter X>` (A..E): 51200 = `POCT200A`, 51302 =
`POCT205A`, 54401 = `POCT207D`. All have 12 records except 52201, 52202, 52301, 52302, 52401 and
52402 (8). Two ids break the rule:

| id | should be | Expansion reads | 2.0 reads |
|---|---|---|---|
| 55400 | `POCT206E` | `POCT206E` | `POCT207A` |
| 55401 | `POCT207E` | `POCT208A` | `POCT208A` |

So `POCT207E.tbl` is in both installs but never read, and 55401 (both programs) and 55400 (2.0)
show the effects of column A. The app keeps both mix-ups, choosing by the installation's flavor
(`UserDbas/AllowedU.dat` present = Expansion).

### PO: Spells & Magic effect tables

The 60 effect ids are `40000 + 1000 * X + 100 + s` (X = 1..6, s = 0..9), files
`POSM<100 + s><letter X>` (A..F): 41100 = `POSM100A`, 46109 = `POSM109F`. Records: 12 for A..E,
8 for F.

### The other ids

#### Player's Handbook (12 ids)

| id | file | records | id | file | records | id | file | records |
|---|---|---|---|---|---|---|---|---|
| 10011 | `PHB11` | 7 | 10019 | `PHB19` | 26 | 10031 | `PHB31` | 17 |
| 10036 | `PHB36` | 26 | 10043 | `PHB43` | 5 | 11010 | `PHB10HM` | 7 |
| 11016 | `PHB16A` | 6 | 12010 | `PHB10HF` | 7 | 12016 | `PHB16B` | 6 |
| 13010 | `PHB10WM` | 7 | 13016 | `PHB16C` | 8 | 14010 | `PHB10WF` | 7 |

#### Dungeon Master's Guide (60 ids)

| id | file | records | id | file | records | id | file | records |
|---|---|---|---|---|---|---|---|---|
| 20047 | `DMG47` | 14 | 20054 | `DMG54` | 10 | 20084 | `DMG84` | 27 |
| 20085 | `DMG85` | 7 | 20086 | `DMG86` | 7 | 20087 | `DMG87` | 13 |
| 20088 | `DMG88` | 19 | 20092 | `DMG92` | 15 | 20093 | `DMG93` | 15 |
| 20094 | `DMG94` | 21 | 20095 | `DMG95` | 19 | 20097 | `DMG97` | 17 |
| 20098 | `DMG98` | 20 | 20099 | `DMG99` | 14 | 20100 | `DMG100` | 18 |
| 20101 | `DMG101` | 21 | 20102 | `DMG102` | 20 | 20103 | `DMG103` | 21 |
| 20105 | `DMG105` | 14 | 20106 | `DMG106` | 7 | 20107 | `DMG107` | 11 |
| 20111 | `DMG111` | 10 | 20113 | `DMG113` | 7 | 20114 | `DMG114` | 10 |
| 20115 | `DMG115` | 13 | 20116 | `DMG116` | 18 | 21026 | `DMG26A` | 11 |
| 21043 | `DMG43A` | 21 | 21070 | `DMG70A` | 21 | 21079 | `DMG79A` | 8 |
| 21089 | `DMG89_A` | 18 | 21090 | `DMG90_A` | 18 | 21091 | `DMG91_A` | 19 |
| 21096 | `DMG96_A` | 20 | 21104 | `DMG104_A` | 18 | 21108 | `DMG108_A` | 17 |
| 21109 | `DMG109S` | 7 | 21110 | `DMG110_A` | 18 | 21117 | `DMG117A` | 11 |
| 21999 | `DMG999A` | 9 | 22026 | `DMG26B` | 11 | 22043 | `DMG43B` | 21 |
| 22070 | `DMG70B` | 101 | 22079 | `DMG79B` | 8 | 22089 | `DMG89_B` | 20 |
| 22090 | `DMG90_B` | 19 | 22091 | `DMG91_B` | 20 | 22096 | `DMG96_B` | 21 |
| 22104 | `DMG104_B` | 17 | 22108 | `DMG108_B` | 10 | 22109 | `DMG109O` | 7 |
| 22110 | `DMG110_B` | 17 | 22117 | `DMG117B` | 8 | 22999 | `DMG999B` | 9 |
| 23079 | `DMG79C` | 8 | 23089 | `DMG89_C` | 19 | 23090 | `DMG90_C` | 18 |
| 23110 | `DMG110_C` | 12 | 23999 | `DMG999C` | 9 | 24110 | `DMG110_D` | 16 |

#### Player's Option: Skills & Powers (8 ids)

| id | file | records | id | file | records | id | file | records |
|---|---|---|---|---|---|---|---|---|
| 30001 | `POSP1` | 21 | 30025 | `POSP25` | 21 | 30031 | `POSP31` | 21 |
| 30040 | `POSP40` | 5 | 30041 | `POSP41` | 31 | 30042 | `POSP42` | 21 |
| 31083 | `POSP83A` | 58 | 32083 | `POSP83B` | 20 | | | |

#### Player's Option: Spells & Magic, besides the 60 effect tables (18 ids)

| id | file | records | id | file | records | id | file | records |
|---|---|---|---|---|---|---|---|---|
| 40014 | `POSM14` | 21 | 40015 | `POSM15` | 21 | 40023 | `POSM23` | 10 |
| 40024 | `POSM24` | 21 | 40025 | `POSM25` | 11 | 40044 | `POSM44` | 21 |
| 40046 | `POSM46J` | 4 | 40048 | `POSM48` | 21 | 40049 | `POSMHit` | 21 |
| 41046 | `POSM46A` | 8 | 42046 | `POSM46B` | 9 | 43046 | `POSM46C` | 5 |
| 44046 | `POSM46D` | 2 | 45046 | `POSM46E` | 4 | 46046 | `POSM46F` | 5 |
| 47046 | `POSM46G` | 2 | 48046 | `POSM46H` | 3 | 49046 | `POSM46I` | 3 |

#### DM Option: High Level Campaigns (2 ids)

| id | file | records |
|---|---|---|
| 50018 | `DMO18` | 6 |
| 50029 | `DMO29` | 11 |

#### Player's Option: Combat & Tactics, besides the 45 effect tables (4 ids)

| id | file | records |
|---|---|---|
| 50900 | `POCT1A` | 8 |
| 50901 | `POCT1B` | 9 |
| 50902 | `POCT1C` | 9 |
| 50903 | `POCT1D` | 5 |

#### Magic item sub-subtables (47 ids)

| id | file | records | id | file | records | id | file | records |
|---|---|---|---|---|---|---|---|---|
| 90001 | `Sub89A` | 11 | 90002 | `Sub89B` | 7 | 90003 | `Sub89C` | 8 |
| 90004 | `Sub89D` | 9 | 90005 | `Sub89E` | 5 | 90006 | `Sub89F` | 13 |
| 90007 | `Sub90A` | 6 | 90008 | `Sub90B` | 8 | 90009 | `Sub91A` | 8 |
| 90010 | `Sub91B` | 7 | 90011 | `Sub91C` | 8 | 90012 | `Sub91D` | 3 |
| 90013 | `Sub91E` | 5 | 90015 | `Sub93A` | 7 | 90016 | `Sub96A` | 6 |
| 90017 | `Sub100A` | 5 | 90018 | `Sub95A` | 4 | 90020 | `Sub110A` | 6 |
| 90021 | `Sub110B` | 5 | 90022 | `Sub110C` | 5 | 90023 | `Sub110D` | 6 |
| 90024 | `Sub110E` | 11 | 90025 | `Sub98C` | 5 | 90026 | `Sub98D` | 8 |
| 90027 | `Sub99A` | 7 | 90028 | `Sub100B` | 13 | 90029 | `Sub91G` | 9 |
| 90030 | `Sub100C` | 4 | 90031 | `Sub95B` | 5 | 90032 | `Sub96B` | 5 |
| 90033 | `Sub96C` | 6 | 90034 | `Sub96D` | 7 | 90035 | `Sub96E` | 11 |
| 90036 | `Sub96F` | 5 | 90037 | `Sub96G` | 7 | 90038 | `Sub97A` | 6 |
| 90039 | `Sub97B` | 14 | 90040 | `Sub101A` | 5 | 90041 | `Sub102A` | 5 |
| 90042 | `Sub103A` | 5 | 90043 | `Sub104A` | 8 | 90044 | `Sub104B` | 7 |
| 90045 | `Sub107A` | 6 | 90047 | `Sub110G` | 6 | 90048 | `Sub100D` | 4 |
| 91019 | `Sub98A` | 9 | 92019 | `Sub98B` | 9 | | | |

## What is not in the files

* **Which die, which columns, which subtable.** Dice (1d20, 1d100, 2d10 for DMG54, 2d6 for DMG79
  and POSP40, ...), the "(D6)"/"(D10)" choice of subtable A/B/C/D, the number of rolls (PHB19
  2d6 times, PHB31 4d6 times), parameter pages and follow-up handlers are in the code
  (`data::LIST`, `data::HANDLERS`). A roll (0x538d84) sums the dice, rolls again while the sum is
  within 1 of the dialog's previous result, then adds the modifier; see `Session::roll`.
* **Titles and headings.** A list entry's title is "Table N: " and the book's caption for table N,
  found in the book's RTF (`books::table_captions`, see [books.md](books.md)); the column
  headings, footnotes, the rows of computed tables and the parameter options are strings of the
  program, read at run time from `Program/Tsrp3.exe` or `Program/Evermore.exe` (only the builds
  in `program::BUILDS`, checked by size and FNV-1a hash).
* **Tables worked out in code** without a file, or with their file unused: DM Option: High
  Level Campaigns 14, 16, 17; DMG 47, 84 (opens the treasure generator), 112, 118, and the
  specific trait after DMG 70 (strings of the program; `DMG70B` is absent); PHB 43, 61; S&P 25,
  31, 36; S&M 14, 15, 47, 48; the Ioun stones (90040).
* **Tables that use their file plus code**: DMG 85 (the gem is chosen in code), DMG 116 and 117
  (rows that roll again), S&P 83 (rows that roll again), S&M 23 and 24, the critical hit tables,
  and the handlers for 90018, 90034 and 90039.

## Notes

* The old notes say the dialog stores at most 33 (0x21) records (arrays at dialog
  +0xa4c..+0xee8), but the id map asks for 58 records of POSP83A (31083) and 101 of DMG70B. The
  app keeps every record it reads. Whether the original drops records past 33 is open.
* `tbl.rs` says the header title is never shown; the old notes say the same. The app does show it
  for follow-up and sub-subtable rolls (`Title::Header`, after the cases 90001..90048 of 0x51d5fd)
  and for critical hit effects. Which of the two the original does for those cases was not checked
  here.
* The old notes list 15 mapped but missing files; `Sub96C` (90033) and `Sub101A` (90040) are
  missing too (the integration test `every_mapped_table_loads` has all 17). No entry names 90033;
  DMG101 names 90040, which is handled in code.
* The old notes count the DMG47 defect as "record 2 has extra numbers"; in the file the number
  lines of the first two entries are too long (14 and 15 numbers), which shifts every record from
  the second entry on. The test in `tbl.rs` uses a smaller made-up case.
* The old notes list POCT205C as "12 / 11" (count / file). Read as the reader reads it, the file
  holds 12 records, one of them with a missing line.
* The meaning of `v6` is not confirmed.
* Sub110G (90047) is a d100 table (entries up to 100), but its handler rolls 1d20 (`data.rs`,
  after the case in 0x51d5fd), so only its first entry can come up. Whether the original's die is
  really 1d20 there was not rechecked.
