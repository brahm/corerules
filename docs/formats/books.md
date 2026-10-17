# Rule books: `Books/*.rtf`

The on-line rule books are Rich Text Format files written by Word 97, one file per book, in
`Books/`. They are not CArchive files (see [archive.md](archive.md) for those). Neither original
program reads them: the Expansion hands the chosen book to WinHelp, a browser or the RTF viewer
of the system (`ShellExecute "open"`), and 2.0 only to the RTF viewer. The app has its own reader.

Rust:

* [`rtf.rs`](../../crates/cr-formats/src/rtf.rs) (`Document`, `Paragraph`, `Inline`, `Format`,
  `Picture`): the RTF subset reader, read only.
* [`books.rs`](../../crates/cr-store/src/books.rs) (`BOOKS`, `CORE20_BOOKS`,
  `Installation::books`, `outline`, `table_captions`, `search_fold`): which books an installation
  has, their titles and shelves, the outline, and the table captions used by Table Rolls
  ([tables.md](tables.md)).
* [`cr-gtk/src/books.rs`](../../crates/cr-gtk/src/books.rs): the reader window (behaviour only,
  summarised at the end).

Original code (TSRP3.EXE unless marked 2.0), from the reverse-engineering notes:

| what | address |
|---|---|
| Books screen `CBooksDlg` (BooksDlg.cpp, dialog 0x8a), seven spines | Expansion |
| List of Books OnInitDialog (reads `Titles\count`, `Book<n>\type`, `Book<n>\name`) | 0x414304 |
| List of Books Open (reads `Book<n>\help`, `html` or `rtf` and opens it) | 0x41455c |
| browser launch for the HTML rendering | 0x563e89 |
| 2.0 Books screen (TSRP2\BooksDlg.cpp): one stack, file names fixed in code | 2.0 |
| Books menu of the character creation screen (menu 343, ids 0x8059..0x8063) | 0x496f7b..0x49718a, both programs |

Both programs write nothing to these files. The 2.0 set is the same text as the Expansion's for
the nine books it has: `ArmsBk.rtf` and `TomeBk.rtf` are byte-identical, the other seven differ in
one publisher paragraph on the credits page.

## The books

### Expansion: registry `Book1`..`Book20`

The Expansion's installer writes `HKCU\Software\Evermore\TSRP3\Titles\count` (20) and, for each
book `n`, `HKCU\Software\Evermore\TSRP3\Book<n>` with the values `name`, `type`, `rtf`, `help` and
`html`. The Books screen has five shelves (`type` 1..5); a shelf's List of Books shows the
`name`s of that `type`, sorted, and Open reads the value of the rendering chosen (Help = WinHelp
`HELP_FINDER`, HTML = `WebHelp\<file>` in the browser, Rtf = `Books\<file>` with the shell).

`BOOKS` in `books.rs` is this list in registry order. The `rtf` file names, titles and shelves
are the app's data; the `help` and `html` names are from the notes. The heading colours are
the `\cfN` indexes the app uses for the outline (see [Outline](#outline)).

| n | title | type (shelf) | rtf (`Books\`) | help (`Help\`) | html (`WebHelp\`) | heading `\cf` |
|---|---|---|---|---|---|---|
| 1 | Player's Handbook | 1 Core Books | `PhbBk.rtf` | `Players.hlp` | `PHBStart.htm` | 6, 12 |
| 2 | Monstrous Manual | 1 Core Books | `MonBk.rtf` | `Monster.hlp` | `MMStart.htm` | 9 |
| 3 | Dungeon Master's Guide | 1 Core Books | `DmgBk.rtf` | `Dungeon.hlp` | `DMGStart.htm` | 6, 12 |
| 4 | Tome of Magic | 2 Supplements | `TomeBk.rtf` | `Tome.hlp` | `TMStart.htm` | 2 |
| 5 | Arms & Equipment Guide | 2 Supplements | `ArmsBk.rtf` | `Arms.hlp` | `AEGStart.htm` | 9 |
| 6 | Player's Option: Skills & Powers | 3 Option Books | `POSPBk.rtf` | `Posp.hlp` | `SPStart.htm` | 6, 12 |
| 7 | Player's Option: Spells & Magic | 3 Option Books | `POSMBk.rtf` | `Posm.hlp` | `SMStart.htm` | 6, 12 |
| 8 | Player's Option: Combat & Tactics | 3 Option Books | `POCTBk.rtf` | `Poct.hlp` | `CTStart.htm` | 6, 12 |
| 9 | DM Option: High Level Campaigns | 3 Option Books | `DOHLCBk.rtf` | `Dmohlc.hlp` | `HLCStart.htm` | 6, 12 |
| 10 | The Complete Book of Elves | 5 Books of Races | `ElfBk.rtf` | `Cbelv.hlp` | `CBEStart.htm` | 13 |
| 11 | Complete Book of Dwarves | 5 Books of Races | `DwarfBk.rtf` | `Cbdwrv.hlp` | `CBDStart.htm` | 13 |
| 12 | Complete Book of Gnomes & Halflings | 5 Books of Races | `GnmHlfBk.rtf` | `Cbgnmhlf.hlp` | `CBGStart.htm` | 13 |
| 13 | The Complete Wizard's Handbook | 4 Books of Classes | `WizardBk.rtf` | `Hbwizard.hlp` | `CWHStart.htm` | 2 |
| 14 | The Complete Ranger's Handbook | 4 Books of Classes | `RangerBk.rtf` | `Hbranger.hlp` | `CRHStart.htm` | 13 |
| 15 | The Complete Bard's Handbook | 4 Books of Classes | `BardBk.rtf` | `Hbbard.hlp` | `CBHStart.htm` | 13 |
| 16 | The Complete Druid's Handbook | 4 Books of Classes | `DruidBk.rtf` | `Hbdruid.hlp` | `CDHStart.htm` | 13 |
| 17 | The Complete Fighter's Handbook | 4 Books of Classes | `FightrBk.rtf` | `Hbfightr.hlp` | `CFHStart.htm` | 2 |
| 18 | The Complete Paladin's Handbook | 4 Books of Classes | `PaladnBk.rtf` | `Hbpaladn.hlp` | `CPAStart.htm` | 13 |
| 19 | The Complete Priest's Handbook | 4 Books of Classes | `PriestBk.rtf` | `Hbpriest.hlp` | `CPRStart.htm` | 2 |
| 20 | The Complete Thief's Handbook | 4 Books of Classes | `ThiefBk.rtf` | `Hbthief.hlp` | `CTHStart.htm` | 2 |

The shelf titles (the List of Books dialog's title) are `Category::title`: Core Books,
Supplements, Option Books, Books of Classes, Books of Races. Note that type 4 is Classes and 5 is
Races, while the registry numbers put the race books (10..12) before the class books (13..20).
The other two spines of the screen open `Manual.hlp` and a web shop; they are not books.

The registry numbers are also what the Table Rolls code means by a book: High Level Campaigns 9,
Dungeon Master's Guide 3, Player's Handbook 1, Combat & Tactics 8, Skills & Powers 6,
Spells & Magic 7 (`TableBook::book_number`).

### 2.0: the fixed stack

2.0 has no registry list. Its Books screen is one stack of eleven spines with the file names in
the code: the Online Manual (`manual.hlp`) and the Tome of Help (`ADandD.hlp`), which are help
files, then nine RTF books under registry `Directories\books`. `CORE20_BOOKS` gives the nine, top
to bottom, by Expansion registry number:

| place | n | file |
|---|---|---|
| 1 | 1 | `PhbBk.rtf` |
| 2 | 3 | `DmgBk.rtf` |
| 3 | 2 | `MonBk.rtf` |
| 4 | 5 | `ArmsBk.rtf` |
| 5 | 4 | `TomeBk.rtf` |
| 6 | 9 | `DOHLCBk.rtf` |
| 7 | 8 | `POCTBk.rtf` |
| 8 | 6 | `POSPBk.rtf` |
| 9 | 7 | `POSMBk.rtf` |

The same nine files (plus the two help files) are on the Books menu of the character creation
screen in both programs.

### How the app finds the books

* An installation is the Expansion when `UserDbas/AllowedU.dat` exists, else 2.0 (`flavor_of`).
* For each known book the app looks up `Books/<file>`, matching every path component without
  regard to case (`Installation::path`: the exact name first, then a case-insensitive scan of the
  folder), so `BOOKS/phbbk.RTF` is found. A book whose file is absent is left out; nothing else
  in `Books/` is listed.
* Expansion: the books present, sorted by shelf (Core, Supplements, Options, Classes, Races) and
  then by title, as each List of Books sorts. 2.0: the books present among the nine, in stack
  order. A 2.0 installation never lists the eleven Expansion-only books even if their files are
  there.
* `Installation::book(name)` accepts the file name with or without `.rtf` (`PhbBk`), the title,
  or the registry number, ignoring case.

## The RTF subset

All books start `{\rtf1\ansi\ansicpg1252\uc1 \deff0\deflang1033`, have balanced groups (depth up
to 9) and no `\bin`. They use direct formatting rather than styles, tab-aligned tables, Word 95
list numbers, and a few pictures. They do not use `\trowd` tables, `\field`, bookmarks, index or
contents entries (`\xe`, `\tc`), footnotes or sections. Census over the 20 Expansion books (from
the notes): `\par` 117602, `\tab` 132889, `\pard` 10245, `\plain` 3045, `\fs` 93826, `\b` 26370,
`\i` 14596, `\cf` 16195, `\tx` 55662, `\outlinelevel` 912; `\line`, `\page`, `\sect` 0.

`Document::parse` never fails: unknown control words are ignored, unbalanced groups are
tolerated, and text after the last `\par` becomes a last paragraph.

### Tokens

| token | how it is read |
|---|---|
| `{` | pushes the current state (character format, destination, hidden flag, `\uc` count) and marks the start of a group |
| `}` | finishes a font entry or picture that ends here, restores the state saved at the matching `{` |
| `\word` | letters `a`..`z`/`A`..`Z`; then an optional parameter (`-` and digits, read as a number clamped to `i32`; a lone `-` is not a parameter); then one space, if present, is consumed |
| `\'hh` | one byte in hex, decoded as below; malformed hex is dropped |
| `\` + other character | a control symbol, see the table below |
| `\bin N` | the next N bytes are skipped |
| CR, LF | ignored: Word wraps the file in the middle of words, so a line break is not a space |
| other bytes | text in the current destination |

A **destination keyword** is only recognised as the first control word of a group. `\*` before
it does not count as that first word; it marks the group as skippable when the keyword is
unknown.

### Destinations

| group starts with | treatment |
|---|---|
| `\fonttbl` | font table: each `{\fN ...}` gives font N its `\fcharsetN` and its name (the text, cp1252, trimmed, without the final `;`). Inner groups (`{\*\panose ...}`, `{\*\falt ...}`) and other control words are ignored. A font entry written without its own group gets no name |
| `\colortbl` | colour table: each `;` ends an entry; `\redN`, `\greenN`, `\blueN` (0..255) set it; an entry with none of them is the automatic colour (`None`). Index = `\cfN` |
| `\pict` | a picture, see [Pictures](#pictures) |
| `\object` | everything is dropped except a `{\result ...}` group, which is read as body text (it holds the picture Word shows for the object) |
| `\*\shppict` | transparent: its contents are read as if it were not there (Word's PNG picture) |
| `\pntext` | read as body text (the list number or bullet and its tab). The paragraph properties are saved at the group's start and restored at its end, so the `\pard` inside does not reset the numbered paragraph |
| `\stylesheet`, `\info`, `\listtable`, `\listoverridetable`, `\revtbl`, `\header`, `\headerl`, `\headerr`, `\headerf`, `\footer`, `\footerl`, `\footerr`, `\footerf`, `\pn`, `\pnseclvl`, `\nonshppict`, `\filetbl`, `\rsidtbl`, `\objdata`, `\footnote`, `\annotation`, `\atnid`, `\fldinst`, `\xe`, `\tc`, `\bkmkstart`, `\bkmkend`, `\template`, `\generator`, `\latentstyles`, `\themedata`, `\colorschememapping`, `\datastore`, `\xmlnstbl`, `\listpicture`, `\pgdsctbl` | skipped with everything in them |
| `\*` + any other keyword | skipped |
| anything else | not a destination: the group's text is body text (with its own character format) |

So the stylesheet, `\sN` styles and `\outlinelevelN` are never applied: the books' heading styles
are used almost at random, and the outline is found from the formatting instead.

### Character formatting

The character format is part of the group state: a `}` restores it.

| control word | effect on `Format` |
|---|---|
| `\plain` | resets to the default (`\fs24`, font 0, nothing on, automatic colour) and ends hidden text. Font 0 is used, not `\deffN` |
| `\b`, `\i`, `\scaps` | bold, italic, small caps; `\b0` etc. turn it off (any parameter but 0 turns it on) |
| `\ul`, `\uldb`, `\ulw`, `\uld`, `\uldash`, `\ulth`, `\ulwave` | underline (all kinds are one underline); `\ul0` off |
| `\ulnone` | underline off |
| `\super`, `\sub`, `\nosupersub` | superscript, subscript, normal |
| `\upN`, `\dnN` | superscript / subscript when N > 0 (no parameter = 1), else normal |
| `\fsN` | size in half-points (default 24 without a parameter; clamped to 1..4000) |
| `\fN` | font index (into the font table) |
| `\cfN` | colour index; the colour is taken from the colour table read so far (`None` for the automatic colour or a missing entry). The index is kept too (`color_index`), because the books' heading conventions are by index |
| `\vN` | hidden text: dropped from the output (`\v0` ends it). The books have one hidden WinHelp topic id after a double-underlined hotspot (BardBk) |

### Paragraphs

`\par`, `\sect` and `\page` end the paragraph in the body text (not inside a skipped
destination). The paragraph takes the paragraph properties current at that moment.

| control word | effect on `Paragraph` |
|---|---|
| `\pard` | resets the paragraph properties: left alignment, indents and spacing 0, no tab stops, no keep-with-next. Character format is unchanged |
| `\ql`, `\qc`, `\qr`, `\qj` | alignment |
| `\liN`, `\fiN`, `\riN` | left, first-line and right indent, in twips (no parameter = 1) |
| `\sbN`, `\saN` | space before and after, in twips |
| `\keepn` | keep with next (Word uses it on captions); `\keepn0` off |
| `\tqc`, `\tqr`, `\tqdec` | the kind of the next tab stop: centre, right, decimal |
| `\txN`, `\tbN` | a tab stop at N twips (a bar tab is read as a left stop). Stops at N <= 0 are dropped; they still consume a pending `\tq..` kind. Nearly every book paragraph has `\tx-720` |

Not handled (ignored): `\keep`, `\sN`, `\outlinelevelN`, `\lsN`, `\ilvlN`, `\widctlpar`,
`\adjustright`, `\cgrid`, tab leaders and every other control word.

### Text

| input | output |
|---|---|
| `\tab`, `\<TAB>` | U+0009 |
| `\line` | U+000A inside the paragraph |
| `\emdash`, `\endash` | U+2014, U+2013 |
| `\lquote`, `\rquote`, `\ldblquote`, `\rdblquote` | U+2018, U+2019, U+201C, U+201D |
| `\bullet` | U+2022 |
| `\emspace`, `\enspace`, `\qmspace` | a space |
| `\~` | U+00A0 (the books use it as alignment padding) |
| `\_` | U+2011 (non-breaking hyphen) |
| `\-` | nothing (optional hyphen) |
| `\\`, `\{`, `\}` | the character |
| `\` + CR or LF | ends the paragraph |
| other control symbols | nothing |
| `\uN` | UTF-16 unit `N mod 65536` (negative N wraps; a lone surrogate gives U+FFFD), then the next `\ucN` characters (raw bytes or `\'hh`, default 1) are skipped. In the books `\u` only occurs in list templates, which are skipped |
| `\ucN` | the number of fallback characters after `\uN`, for this group |

Raw bytes and `\'hh` are Windows-1252 (with the five unassigned bytes as their C1 controls):
`\'86` is a dagger and `\'99` the trade mark sign, not Latin-1 controls. In a font whose charset is
2 (`\fcharset2`), bytes go through a small map first:

* Symbol (and any other charset 2 font whose name does not contain `Wingdings`): `0xb7` bullet
  U+2022, `0xa7` club, `0xae` right arrow, `0xb0` degree, `0xb1` plus-minus, `0xb4` times,
  `0xb8` divide, `0xa3` <=, `0xb3` >=, `0xb9` not equal, `0xd6` square root, `0x2d` minus, and the
  Greek letters at `A`..`Z` and `a`..`z`;
* Wingdings: `0xa7` small square, `0x6c` circle, `0x6e` square, `0xfc` check mark, `0xd8` arrow;
* any other byte: Windows-1252.

In the books only `\'b7` in Symbol occurs in body text (1014 times).

### Output

`Document`:

| field | content |
|---|---|
| `paragraphs` | `Vec<Paragraph>` in file order |
| `fonts` | `Vec<Font { name, charset }>` indexed by `\fN` (gaps are empty fonts) |
| `colors` | `Vec<Option<Rgb>>` indexed by `\cfN`; entry 0 is the automatic colour in every book (Word's standard 16 colours: 1 black, 2 blue, 6 red, 9 navy, 11 green, 12 purple, 13 maroon, ...) |

`Paragraph`: `align`, `left_indent`, `first_indent`, `right_indent`, `space_before`,
`space_after` (twips), `tabs` (positive stops in file order, each with its kind), `keep_next`, and
`items`: a list of `Inline::Text(text, Format)` runs (adjacent text in the same format merged) and
`Inline::Picture`. `Format`: `size` (half-points), `bold`, `italic`, `underline`, `small_caps`,
`script`, `color`, `color_index`, `font`.

## Pictures

All pictures are embedded as hex in `{\pict ...}`; none refer to files. The books have two forms:

* **DmgBk, PhbBk, GnmHlfBk** (5 pictures): `{\*\shppict{\pict{\*\picprop ...}\picscalex..
  \picscaley..\picwgoal..\pichgoal..\pngblip <hex>}}` followed by `{\nonshppict{\pict\wmetafile8
  ...}}`, the same image as a metafile for older readers.
* **POCTBk** (18 pictures): `{\pict\wmetafile8 ...}`, 17 in the text and one inside
  `{\object ...{\result {\pict ...}}}` (a `Word.Picture.8` object). Each metafile holds a single
  `META_STRETCHDIB` record with an 8-bit DIB.

How the reader takes a `\pict` group:

| control word | use |
|---|---|
| `\pngblip`, `\jpegblip` | the data is a PNG / JPEG file |
| `\wmetafileN` | the data is a WMF; it is turned into a BMP when it holds a bitmap |
| `\dibitmap` | the data is a packed DIB; it is turned into a BMP |
| `\picwN`, `\pichN` | read, not used |
| `\picwgoalN`, `\pichgoalN` | display size in twips |
| `\picscalexN`, `\picscaleyN` | percent applied to the goal size (0 or absent = 100) |
| inner groups (`{\*\picprop ...}`, `{\*\blipuid ...}`) | skipped |
| text | hex digits; anything else (spaces, line breaks) is ignored; an odd last digit is dropped |

When the group closes, the picture becomes an `Inline::Picture` at that point of the current
paragraph, with `width = picwgoal * picscalex / 100` and `height` likewise (0 when the file gives
no goal). It is dropped when it is hidden, has no data, has no format the reader knows (EMF,
Mac PICT, a WMF without a bitmap), or sits in a destination other than the body text. The copy
in `\nonshppict` is never reached, because that destination is skipped, so each image appears
once.

**WMF to BMP** (`wmf_bitmap`): an optional placeable header (`0x9ac6cdd7`, 22 bytes) is skipped,
then the standard header, whose size in 16-bit words is its second word. Records follow, each a
`u32` size in words and a `u16` function. The first `META_STRETCHDIB` (0x0f43, 22 bytes of
parameters) or `META_DIBSTRETCHBLT` (0x0b41, 20 bytes) record gives the DIB: the rest of the
record after the 6-byte record header and the parameters. Records with a size below 3 words or
past the end stop the scan.

**DIB to BMP** (`dib_to_bmp`): a 14-byte `BITMAPFILEHEADER` is put in front: `"BM"`, the file
size (14 + DIB size), 0, and the offset of the bits = 14 + header size (the DIB's first `u32`) +
12 more when the header is 40 bytes and the compression is `BI_BITFIELDS` (3) + 4 times the
number of palette colours (`biClrUsed`, or `2^bits` for 8 bits or fewer when it is 0).

## Outline

The books have no usable outline: headings are direct formatting (a short paragraph in one bold
or large, coloured format), each book has its own colours, and the stylesheet's heading styles
and `\outlinelevel` are applied to unrelated paragraphs. Each book also begins with its own
printed table of contents and list of tables as plain paragraphs (levels by leading tabs, no page
numbers or links). `books::outline(doc, heading_colors)` finds headings by format:

1. **Candidates.** A paragraph whose text, trimmed, is not empty, is at most 90 characters, has no
   tab, starts with an upper-case letter or an ASCII digit, and whose non-blank runs all have the
   same signature (bold, size, colour index). Its title is the text with whitespace collapsed.
2. **Accepted signatures**, tried in three passes; the first pass that finds any heading wins:
   1. size >= 28 half-points (14 pt), or bold in one of the book's heading colours (the `\cf`
      column of the book table; with an empty list, any colour index above 1);
   2. size >= 28, or bold in any colour index above 1 (0 is automatic, 1 black);
   3. any bold candidate.
3. **Levels.** The distinct (size, bold) pairs of the accepted headings are ranked, larger size
   first and bold before regular at the same size; a heading's level is its pair's rank, at most
   3 (0 is the biggest).
4. **"Chapter N:" joins.** A heading that ends with `:` followed within two paragraphs (only
   blank paragraphs between) by the next heading is joined with it into one title (`Chapter 1:`
   and its title), at the smaller of the two levels.
5. A heading with the same title as the heading just before it is dropped.
6. **Printed contents.** The first heading titled "Table of Contents" or "Contents" (compared
   folded to letters and digits, lower case) starts a block that ends at the next heading of the
   same or a higher level. Inside the block, a heading whose folded title occurs again after the
   block is dropped, so the outline leads to the chapter, not to its line in the printed
   contents.

`Heading`: `level` (0..3), `paragraph` (index into `Document::paragraphs`), `title`.

The heading colours measured over the files are red (6) and purple (12) in the core and option
books, blue (2) or maroon (13) in the Complete books, navy (9) in the Monstrous Manual and the
Arms & Equipment Guide. Body text is `\fs24` Times New Roman.

## Table captions

`books::table_captions(doc)` gives the Table Rolls their titles: a paragraph whose trimmed text
is `Table <digits>:` followed by a title (whitespace collapsed, not empty, at most 80 bytes)
gives table number `<digits>` that title. The first paragraph for a number wins, which is the
book's own list of tables. Example: `Table 61:<TAB>Turning Undead` gives 61. Table Rolls reads the
captions of the books with tables (registry numbers 1, 3, 6, 7, 8, 9) when they are present.

## Search folding

`search_fold` folds text for find: lower case, U+00A0 to a space, U+2010 and U+2011 to `-`,
curly single and double quotes to straight ones. Every character folds to exactly one character,
so offsets in the folded text are offsets in the book text.

## The reader (cr-gtk)

Behaviour that depends on the file format:

* Twips are shown as pixels at 96 dpi (twips / 15, times the zoom). Negative indents are limited
  so a hanging indent does not reach left of the margin.
* Tab stops left of the paragraph's left indent are dropped, the rest are measured from the
  indent, and Word's default stops every 720 twips (half inch) follow, up to 14400 twips.
* Fonts are shown by name with a generic fallback (Serif, Sans for Arial-like names, Monospace for
  Courier-like names); charset 2 fonts are shown in the generic family, since their characters
  were mapped to Unicode. Black is shown in the theme's text colour; dark colours are lightened on
  a dark background.
* Super- and subscript are drawn at 70 % size, raised by 35 % or lowered by 15 %.
* Pictures are shown at their goal size, or their pixel size when the file gives none.
* The reading position (paragraph index) and the zoom are kept in `reader-state` under the user's
  data folder, never in the installation.

## Notes

* The old notes call the heading rule "size >= 14 pt or (bold and colour not auto/black)", which
  is the app's second pass. The first pass uses per-book colour lists, and the code adds the
  capital-letter test, the "Chapter N:" join and the printed-contents removal.
* The old notes give Arms & Equipment Guide headings as colour 9 and the Complete books as 2 or 13;
  the code agrees.
* The `help` and `html` registry values and the `HELP_FINDER` call come from the notes only; the
  app does not use them.
* A font table entry outside its own group gets no name (text directly in `\fonttbl` is dropped),
  although a comment in `close_group` speaks of finishing such an entry. The books always group
  their entries.
