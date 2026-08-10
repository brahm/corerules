# Prior art: has anyone already extracted the Core Rules 2.0 corpus?

Research ticket: `.scratch/corpus-v1/issues/03-prior-art-core-rules-extraction.md`
Researched 2026-08-10 against live repositories, archived forum threads and the corpus files themselves.

Primary sources used: the `FG--AD-D-Rules-Core-Importer` Perl source and its README and commit log;
the `AD-DCoreRule2.0Errata` repository tree and release assets; the archived Fantasy Grounds thread
in which the importer's author describes his own results; Internet Archive item metadata for both
CD-ROM uploads; the Wayback CDX index for a fan-site mirror; the shipped data files of four
adjacent 2e projects; and a small set of **new** measurements taken directly on the corpus (§3.3),
distinct from those the map already recorded. Where a claim rests on inference it says so.

**Headline, in three parts.**

1. **Nobody has parsed the RTF.** No script, dump, database or wiki anywhere is traceable to
   `phbbk.rtf`, `dmgbk.rtf` or the `*bk.rtf` handbooks. The ticket's expected negative holds for the
   question as literally asked.

2. **But the underlying question has a much better answer than "nothing".** The same CD ships the
   same books in **a second rendition — `CoreCD\WebHelp\*` — that is already structured**: one HTML
   file per record, a `<TITLE>` that names the record *and its type* *and its book*, and **real
   `<TABLE>`/`<TR>`/`<TD>` markup**. Everything that has ever been extracted from this product was
   extracted from that rendition, never from the RTF — and a **working parser for it exists**
   (Perl, 2017–2019, public, readable).

3. **Consequently the map's hardest measured obstacle is not an obstacle.** Map measurement 1 —
   "No table is a table", zero `\trowd`, every table flattened to tab-delimited paragraphs — is a
   property of *the RTF rendition*, not of the corpus. The same rows exist as marked-up tables in
   another rendition of the same product. That is simultaneously the strongest cross-check
   available to [ticket 10](../issues/10-mechanical-verification.md) and a candidate better input
   for [tickets 01](../issues/01-what-the-source-yields.md) and
   [09](../issues/09-extraction-pipeline.md).

Nothing here is directly reusable *as a corpus*. The value is in points 2 and 3 above, in a
catalogue of exactly how the one serious prior extraction corrupted its output (§3), and in four
independent 2e datasets that are usable as cross-checks (§4).

---

## 1. Extractions of these specific files

### 1.1 The confirmed negative

Searched by filename (`phbbk`, `dmgbk`, `fightrbk`, `thiefbk`, `gnmhlfbk`, `*bk.rtf`), by product
name plus format ("Core Rules 2.0" + RTF/extract/parse/convert), and across GitHub, GitLab,
SourceForge, the Internet Archive, Dragonsfoot, EN World, RPG.net, Reddit, the Roll20 and Foundry
ecosystems and old fan-site archives. **No project parses the RTF.** The filenames appear in public
only in one place, and that project patches the bytes rather than reading them (§1.3).

This is a *strong* negative for the RTF but it is not exhaustive; see §6 for the search I could not
run.

### 1.2 The one real extraction — and it reads the HTML, not the RTF

**`69bossmustang/FG--AD-D-Rules-Core-Importer`** (formerly `CelestianGC/…`), by "celestian", the
author of Fantasy Grounds' AD&D 2E ruleset.
<https://github.com/69bossmustang/FG--AD-D-Rules-Core-Importer>

- Two Perl scripts, 803 and 647 lines: `corebook-html.pl` (whole books → Fantasy Grounds reference
  manual / Story XML, plus `*_items.xml` and `*_skills.xml`) and `corebook-spells.pl` (spell records
  → FG spell XML).
- 15 commits, March 2017 → September 2019. Not archived, not maintained, no licence file.
- Input is explicitly the CD's HTML: *"This 'should' work for any directory of files you find in
  your CoreCD\WebHelp\*"* and *"all you need are the .htm files located in Webhelp/*"* (author,
  Fantasy Grounds forum, 2017-03-06 and 2017-04-17;
  <https://www.fantasygrounds.com/forums/showthread.php?37123-AD-amp-D-Core-Rules-CD-Importer-for-HTML-files>,
  read via the Wayback capture of 2024-07-17).
- Books he names as processed: Player's Handbook, Dungeon Master's Guide, Tome of Magic, Complete
  Thief's Handbook, Complete Paladin's Handbook.

**Why this matters more than the tool itself.** The scripts are a precise, dated description of the
WebHelp rendition's structure, written by someone who had the CD in front of him. Corroborated
independently by the errata project's shipped files (§1.3) and by a fan-site mirror (§1.4):

| Property of the WebHelp rendition | Evidence |
|---|---|
| One `.htm` file per record (a spell, a table, a magic item, a proficiency, a section) | errata repo ships six such files; importer iterates a directory one file = one record |
| `<TITLE>` = `<record name> (<book name>)` | importer regex; verified in all six sample files |
| Spell titles additionally carry level and class: `<name>-- <n>th Level <Class> Spell (<book>)` | importer regex; verified on two sample spell files |
| Record *type* is encoded in the title after `--`: `Nonweapon Proficiency`, `Potion`, `Scroll`, `Magical Item`, `Magical Weapon` | importer's skill/item detection |
| **Real HTML tables** | verified: `Table 22: Wizard Specialist Requirements` = 1 `<TABLE>`, 17 `<TR>`, 85 `<TD>`; `Item Saving Throws-- Table 29` = 1 `<TABLE>`, 15 `<TR>`, 150 `<TD>` |
| Spell stat block is itself a two-column table with `Sphere:` / `Range:` / `Components:` / `Duration:` / `Casting Time:` / `Area of Effect:` / `Saving Throw:` labels | importer's field regexes; verified on two sample spell files (1 table, 3–4 rows, 6–8 cells) |
| Cross-reference hyperlinks between records; index pages enumerate their members | verified: `Protection-- Priest Spells by Sphere` is a link-only page with 42 `<A HREF>` and no body table — i.e. **a machine-readable sphere→spell mapping** |
| Filenames are `DD` + five digits, and **the number orders the pages as they appear in the book** | importer commit `2018-01-06`: *"Discovered that filenames were order of the pages… I was able to use that to order the pages in the ref manual"* |
| Topic IDs are global across the library, not per book | observed range `DD00024` … `DD05368` spanning Arms & Equipment through the Complete Paladin's Handbook (§1.4) |

**Inference, flagged as such:** those `DDNNNNN` numbers are a publisher-assigned, stable, per-record
identifier for the entire library. If that holds, it is directly relevant to
[ticket 07](../issues/07-identity-and-id-stability.md) — an identity scheme that predates us and is
not a display name. See §3.3 for the measurement that constrains how far it can be used.

### 1.3 The errata project — patches the RTF, does not read it

**`Alby1987/AD-DCoreRule2.0Errata`**, GPL-3.0, created 2022-02, last push 2024-09, 5 stars, 4 forks.
<https://github.com/Alby1987/AD-DCoreRule2.0Errata>

It distributes TSR's official errata as **BDiff binary patches** rather than as files, explicitly to
avoid redistributing the books. Its tree is the single public confirmation of the corpus filenames:

```
patch/Books/PhbBk.rtf.patch          patch/Help/Players.hlp.patch
patch/Books/DmgBk.rtf.patch          patch/Help/Dungeon.hlp.patch
patch/Books/DMGBK.RTF.2.patch        patch/Help/DUNGEON.HLP.2.patch
                                     patch/Help/Spells.hlp.patch
patch/WebHelp/PHB/{DD01857,DD02184,DD02316,DD02355}.htm
patch/WebHelp/DMG/{DD00331,DD00823}.htm
```

Three findings from that listing, all load-bearing:

1. **The corpus is pre-errata.** TSR issued official corrections to the PHB and DMG that the CD's
   own files do not contain. A transcription of the RTF is therefore a faithful transcription of a
   *known-defective* text. Direct input to
   [ticket 10](../issues/10-mechanical-verification.md) and to whatever the pack manifest says about
   provenance.
2. **There is more than one RTF variant in circulation.** Two DMG patches exist — `DmgBk.rtf.patch`
   and `DMGBK.RTF.2.patch` — which only makes sense if two byte-different DMG RTFs shipped
   (different pressing, or base CD vs. Expansion). **Our pipeline should record a checksum of every
   input file**, or a future disagreement with anyone else's extraction will be unattributable.
3. **The corrected WebHelp pages are shipped whole, not as diffs** — six complete `.htm` files. They
   are the errata project's own worked example of which topics TSR corrected, and they are what
   allowed me to verify the WebHelp structure above without owning the disc.

Both release assets are live (`ADnD20CoreRulesErrata.zip`, 1,023 downloads; `…1.1.zip`, 77).

### 1.4 A public mirror of the WebHelp rendition

`naggaroth.daerma.com/2e/corerule-page=<PREFIX>_DDNNNNN.htm.php.html` serves Core Rules WebHelp
topics through a PHP wrapper. **The site returned HTTP 502 throughout this research**; its shape is
established from the Wayback CDX index (759 captures, of which 177 are `corerule-page=` URLs) and
from search-result titles. Observed book prefixes: `PHB`, `DMG`, `AEG` (Arms & Equipment),
`CBD` (Complete Book of Dwarves), `CDH` (Complete Druid's), `CPaH` (Complete Paladin's), plus
unprefixed `DDNNNNN`.

Two conclusions: the WebHelp rendition **covers the Complete handbooks, not only the PHB and DMG**
— the same book set as our RTF — and the whole thing has been publicly crawlable at various times.
The Wayback holdings are partial (~177 topics) and are not a usable corpus.

### 1.5 Everything else that came up, and why it is not this

- **`decheine/complete-compendium`** — a genuine, live extraction pipeline (C++ harvester in Docker,
  Node copy step, JSON output, Gatsby static site), 2,000+ AD&D 2e monsters, 31 stars, last push
  2026-05. Its source is **not** the Core Rules CD: it harvests the dead fan site `lomion.de/cmm`
  out of the Wayback Machine. Monsters are out of v1 scope, but this is the closest thing in the
  ecosystem to what [ticket 09](../issues/09-extraction-pipeline.md) will build, and its disclaimer
  ("All intellectual property mentioned is the intellectual property of TSR inc. … archival and
  encyclopedic purposes only") is the posture that WotC-derived projects settle on.
  <https://github.com/decheine/complete-compendium>, <https://www.completecompendium.com/about/>
- **`69bossmustang/ADnD-2e-Framework`** — 19 MapTools `.cmpgn` files, ~300 MB, all binary. The 2e
  rules live inside MapTools macros inside zip blobs. Not a corpus, not readable data.
- **Internet Archive** holds two CD images: `adndcore2rules` (176 MB ISO, "Core Rules 2.0",
  dated 1999-10-24) and `add-core-rules` (441 MB ISO, dated 1996). Both are disc images only — no
  extracted text, no derived dataset. <https://archive.org/details/adndcore2rules>,
  <https://archive.org/details/add-core-rules>
- Community forums (Dragonsfoot, EN World, RPG.net, Reddit) discuss the CD constantly and treat
  "the RTFs are real text, searchable, copy-pasteable" as the product's main virtue — but **no
  thread found describes anyone converting them into structured data.**

---

## 2. What they produced, and whether it survives

| | `corebook-html.pl` | `corebook-spells.pl` | errata patches |
|---|---|---|---|
| Kind | parser | parser | binary diffs |
| Survives | yes, GitHub, unmaintained since 2019 | yes | yes, releases live |
| Output | Fantasy Grounds XML: reference-manual chapters, Story entries, `*_items.xml`, `*_skills.xml` | Fantasy Grounds spell XML | corrected `.rtf` / `.hlp` / `.htm` |
| Reusable as a **pipeline**? | **No** | **No** | n/a |
| Reusable as a **cross-check**? | **No** | **No** | **Yes** (as an errata list) |

**Why neither is reusable as a pipeline.** Both target Fantasy Grounds' XML dialect directly — FG
`<reference>` / `refmanualindex` / `chapters` / `subchapters` blocks, `<librarylink>`,
`windowreference` — with no intermediate representation. The whole middle of both scripts is
HTML-cosmetics-to-FG-cosmetics rewriting (`<h2>` → `<b>`, `<th>` → `<td>`, strip `<font>`, strip
`<a>`, strip `<img>`). There is no data model in them at all: a "skill" is a name plus a blob of
marked-up description, an "item" likewise. **They extract presentation, not mechanics.** Nothing in
either script would survive contact with `spec.md` §3's object kinds.

**Why neither is reusable as a cross-check.** They produce no counts, no tables-as-rows and no
values — only names and HTML blobs. The one exception is `corebook-spells.pl`, which does isolate
the seven stat-block labels; but it emits them concatenated into an FG "short description" string
rather than as fields.

**What *is* reusable, and it is the valuable half:** the scripts are a free, dated, verified
specification of the WebHelp rendition's markup (the table in §1.2). They cost someone two years to
discover and they are readable in an afternoon.

**No dump of any kind survives.** The author says why, and it is a licensing statement, not a
technical one: *"If I could just give the .mod files out I would but right now they don't allow
anything but 5e on dmsguild."*

---

## 3. Where they broke

This is the highest-value section and it divides in three: what the author says broke, what his
source demonstrably corrupts, and what I measured on our own files as a result.

### 3.1 What the author reported

From the archived thread and the README, in his own words:

- **Ordering had no structural source.** *"Right now it's just alphabetically sorted with each entry
  a subchapter. Not really sure I can do more than that since **the html has no real ref that I can
  use to organize it by**."* (2017-09-27). Solved four months later only by the accident that the
  filename numbers happen to follow book order — *"the order they appear in the actual books (90% at
  least)"* (2018-01-06). **Reading order is not recoverable from the record content; it comes from
  outside the records.**
- **~90%, not 100%.** *"It's got a good 90% of everything in them. Pictures obviously are not there
  and **a few of the tables get wonky in translation**."*
- **Files silently dropped.** The script's failure mode is `"Discarding file …, did not find title"`.
  The README's stated cause is malformed source: *"`<TITLE>SectionOfBook (Player's Handbook`,
  notice the missing `)` at the end"*, plus *"some incorrect entries in the html files for
  paragraphs, tables, fonts/etc."* **The publisher's own HTML is not well-formed**; he runs it
  through `HTML::Tidy` first and still loses files.
- **Per-book irregularity.** *"the Paladin's Handbook didn't have chapters labeled properly"*, fixed
  by hand-editing the source HTML. This is the HTML analogue of the map's finding that the kit label
  vocabulary varies per book family: **book-to-book irregularity is a property of the product, and
  it will appear in whichever rendition we choose.**
- **One book family needed special-casing.** Commit 2017-04-13: *"fixed it so it will grab
  rod/staff/wand/rings from the DMG (the format for those was different for some reason)"*.

### 3.2 What his character handling actually does — a warning, not a criticism

`corebook-spells.pl` ends with a run of substitutions over high-byte characters. Two of them are
**wrong**, and the last one is worse:

```perl
$this_string =~ s/\\xBC/1\/2/g;   # U+00BC is ¼, not ½
$this_string =~ s/\\xBD/1\/2/g;   # ½ — correct
...
$this_string =~ s/\\x..//g;       # delete every remaining high byte, silently
```

So a quarter becomes a half, and every character not explicitly listed — including the `†` and `‡`
that anchor table footnotes — is deleted without a warning. **This is the single most useful thing
in the prior art: it names the exact failure mode to guard against, and it is invisible in the
output.** A pipeline that drops or mangles these characters produces text that looks perfect and is
numerically wrong.

That prompted the measurement below.

### 3.3 New measurements on our corpus (not a re-measurement of the map)

Read-only, on `/run/media/brahm/PocketNAS/Projects/corerules/books/`. The map measured tables, kit
records and tooling; these are different properties, taken because §1 and §3.2 made them decisive.

**(a) The RTF carries no cross-reference identity of any kind.** Counts over `phbbk`, `dmgbk`,
`fightrbk`, `priestbk`, `wizardbk`, all zero in every file: `bkmkstart` (bookmarks), `footnote`,
`HYPERLINK`, `\page`, and any `DD` + 5-digit token. There is one `\stylesheet` per file and
`\s<n>` style references (300 in `phbbk`, 2 in `priestbk`).

Three consequences:

- **The RTF and the WebHelp topics cannot be aligned by anything intrinsic.** Any cross-walk between
  the two renditions must be built on heading-text matching. That is the cost of using the WebHelp
  as a cross-check, and it should be priced into ticket 10 rather than discovered inside it.
- **The RTF contains no page-break markup, so page numbers are not recoverable from it.** v1
  research recommended "book + page citation on every record" as the user's means of checking a
  transcription. **From the RTF that recommendation is not implementable**; a citation key will have
  to be structural (book + chapter + heading, or a table number) rather than a page. Input to
  [ticket 05](../issues/05-pack-schema.md).
- Named styles exist but are used very unevenly across books (300 vs. 2), so style names are a weak
  segmentation signal in some books and absent in others.

**(b) The non-ASCII inventory is small, entirely `\'hh`-escaped, and semantically loaded.** The
files declare `\ansicpg1252` and contain **zero raw bytes above 0x7F**; every such character is an
escape. Across the whole 13-book v1 tier: **2,020 escapes, 54 distinct codes, 8 `\uN` escapes.**

| Group | Codes | Count | Why it matters |
|---|---|---|---|
| Middle dot | `b7` | 1,364 | by far the most common; a layout/leader character, needs a decision not a delete |
| Trademark marks | `ae` `a9` `99` | 263 | pure noise, safe to strip — but only if deliberately |
| **Vulgar fractions** | `bd` `bc` `be` | **124** | ½ ¼ ¾ — **numerically load-bearing**, and exactly what §3.2's prior art corrupts |
| Multiplication sign | `d7` | 40 | appears in dimensions and dice expressions |
| **Footnote daggers** | `86` `87` | **19** | † ‡ — **anchors that bind a table footnote to a row**; deleting them orphans the note |
| Degree / acute / yen | `b0` `b4` `a5` | 7 | |
| **Control characters** | `00`–`11` | **119** | escaped C0 control bytes embedded in the text; junk, but they must be classified, not passed through |
| Escaped ASCII | `5c` `20`, letters, `.` `:` | ~84 | ordinary characters that happen to be escaped; harmless if decoded properly |

Per book, the count ranges from 17 (`paladnbk`) to 369 (`elfbk`). **The entire non-ASCII surface of
the v1 corpus is roughly 2,000 characters — small enough to enumerate and eyeball once, per book.**
That is a cheap, high-yield check for [ticket 09](../issues/09-extraction-pipeline.md): decode
`\'hh` against cp1252 explicitly, assert the resulting character set against a declared allowlist,
and fail loudly on anything unlisted — instead of the prior art's silent `s/\\x..//g`.

---

## 4. Adjacent structured 2e data, judged only as corpus or cross-check

v1 ticket 01 surveyed several of these as *modelling* prior art. This section asks the different
question the ticket poses: **can the bytes be diffed against ours?** Two of the five are new since
that research.

### 4.1 EZDM — the best table-level cross-check found

`ajventer/ezdm`, GPL-3.0, last push 2015, 0 stars.
<https://github.com/ajventer/ezdm/tree/master/ezdm_libs/adnd2e>

Thirteen small JSON files that are, essentially, the PHB's core tables in machine-readable form:

| File | Shape |
|---|---|
| `thac0.json` | 5 class groups (`priest`, `rogue`, `warrior`, `wizard`, `creature`), each keyed by level band (`1-3`, `4-6`, … `19-20`) |
| `xp_levels.json` | 4 class groups, ~20 keys each, plus a `dice` entry — **a per-class-group XP table**, the shape v1 research established PCGen cannot express |
| `saving_throws.json` | 4 class groups + a `dwarf` entry + a `names` map of 5 categories: `ppd`, `rod`, `pp`, `breath`, `spell` |
| `ability_scores.json` | all six abilities, 30 rows each |
| `various.json` | `attacks_per_round`, `abilities`, `spell progression` |
| `creature_xp.json`, `attack_mods.json` | monster XP by level; 8 named situational combat modifiers |

**Why it is the best cross-check:** it is small, flat, numeric, independently transcribed (its only
attribution file credits icon artists, not a data source — so it was almost certainly typed from the
printed books, not from our CD), and it covers precisely the tables where the map says a single
wrong number *"poisons every Character that passes through it, silently"*. A diff against our
extracted THAC0, XP and save tables is an afternoon's work and would be the cheapest real error
detector this project can obtain. Its class-group vocabulary and 5-category save split match the
Roll20 sheet's exactly, which is itself weak corroboration that both are right.

Caveats: it is a small hobby project, dormant since 2015, and covers class *groups* only — no
per-class detail, no kits, no proficiencies.

### 4.2 Two independent spell indexes that agree with each other

- **`ChrisSSocha/dnd-2e-data`** — four CSVs, header `type,level,name`. 933 wizard rows over **500
  distinct spell names**, 22 `type` values (schools) and 12 levels; 409 priest rows over **366
  distinct names** and 27 `type` values (spheres); plus 507 "additional" priest rows drawn from the
  Realms deity books. Its README states provenance precisely: Appendices 3 and 4 of *Player's
  Option: Spells & Magic* — **a book we hold as `posmbk.rtf`**. No descriptions, so it is
  copyright-light and safe to diff. <https://github.com/ChrisSSocha/dnd-2e-data>
- **`vodabois.fi/2eSpells`** — an Angular site whose data is two plain JSON assets,
  `assets/WizardSpells.json` (**511 records**) and `assets/PriestSpells.json` (**374 records**),
  fields `name`, `lvl`, `schools`, `spheres`, `range`, `componenets` *(sic)*, `duration`, `aoe`,
  `save`, `casting`, `description`, `done`. Its own description names the source books: Tome of
  Magic, Spells & Magic, Complete Wizard's and Complete Necromancer's handbooks. The `done` flag
  suggests hand transcription with a progress tracker. **It includes full descriptions**, so it is a
  full-text derivative: link it, do not vendor it. <https://vodabois.fi/2eSpells/>

**Why the pair matters:** 500 vs. 511 wizard names and 366 vs. 374 priest names, from two
independent transcriptions of overlapping book sets. That is close enough that a three-way
comparison against our own extraction would localise disagreements to a handful of records — which
is exactly the "free error detector" the ticket is asking for. It also confirms empirically that
school and sphere membership are **many-to-many** (933 wizard rows over 500 names).

### 4.3 Dungeon Craft 2e databases — the richest per-class dataset

`manikus/2e_databases_for_Dungeon_Craft`, GPL-2.0, 1,697 files, last push 2023-03.
<https://github.com/manikus/2e_databases_for_Dungeon_Craft>

Line-oriented text databases for the Dungeon Craft engine, split into `baseclass/`, `classes/`,
`ability/`, `spell_schools/` and a `__2e__/` directory. Its `baseclass/0_header.txt` documents the
field vocabulary, and the fields are startlingly close to what our pack schema needs:

`name`, `BonusXP` (ability name, modifier, 25 values), `THAC0` (40 integers, one per level),
`AbilityRequirement` (ability, min/max), `Allowed Race` (repeatable), `Allowed Alignment`
(repeatable), `ExpLevel` (minimum XP per level, 40 entries), `Base Spell Levels`
(level, school, up to 9 integers), `MaxNumSpellsByPrime` (school, ability, 25 integers).

**As a cross-check** it is the only source found that carries per-class (not per-group) THAC0
arrays, XP thresholds and ability minimums together — i.e. it can be diffed against PHB Tables 14–24
class by class. **As modelling prior art it is a genuine gap in v1 ticket 01**, which concluded that
no project models 2e's per-class XP tables or level progressions as loadable data; this one does,
in a data-driven engine, and it predates the survey. Worth a look before
[ticket 05](../issues/05-pack-schema.md) settles the class shape.

Caveats: coverage is idiosyncratic (a `Crusader` base class, Planescape monsters, specialist-priest
files), provenance is undocumented, and the format is Dungeon Craft's own, so a reader must be
written.

### 4.4 A live, sophisticated 2e spell schema — new since v1 ticket 01

`runecalico-ai/second_edition_spellbook` (Tauri/React/SQLite, last push 2026-07-15) and its sibling
`second_edition_spellscribe` ("extracting and reviewing structured spell data from documents",
PyInstaller + **Tesseract OCR**, last push 2026-07-08).

`apps/desktop/src-tauri/schemas/spell.schema.json` is a 47 KB JSON Schema (draft 2020-12) with 33
top-level properties and 18 `$defs`. Required: `name`, `level`, `description`, `tradition`.
Structured definitions include `RangeSpec`, `DurationSpec`, `AreaSpec`, `SaveSpec`,
`MagicResistanceSpec`, `MaterialComponentSpec`, `SpellDamageSpec`, `DicePool`, `DiceTerm`,
`ScalingRule`, `ClampSpec`, `ApplicationSpec`. `components` decomposes to
`verbal`/`somatic`/`material`/`focus`/`divine_focus`/`experience`. `casting_time` decomposes to
`text`, `unit`, `base_value`, `per_level`, `level_divisor` — **and `raw_legacy_value`.**

Two things to take from it:

1. **`raw_legacy_value` is the pattern this project should copy.** Every parsed mechanical field
   keeps the original source string beside it. That makes a parse error visible and repairable
   without re-extraction, and it is the natural answer to the map's fog about re-extraction cost.
   Its `resources/parser-specification.md` documents the string→structure rules for components,
   range, duration, area and casting time, including the per-N-levels and "Touch"/"Permanent"
   keyword cases — a ready-made checklist for
   [ticket 06](../issues/06-expression-language.md).
2. **It ships no spell data**, and it is OCR-driven — the PDF pipeline the map deliberately
   excluded. It is a modelling and parsing cross-check, not a corpus.

Honesty flag: the repository's `openspec/` and `docs/superpowers/plans/` layout indicates heavily
AI-assisted authorship. The schema is coherent and worth reading; **its correctness against the
books is unverified by me and should not be assumed.**

### 4.5 Roll20 `ADnD_2E_Revised` — large, but the wrong shape for diffing

Still live and still the largest open 2e implementation: `wizardSpells.js` 1,011 KB,
`priestSpells.js` 857 KB, `psionicPowers.js` 286 KB, `weapons.js` 192 KB,
`nonweaponProficiencies.js` 57 KB, `weaponProficiencies.js` 20 KB, `abilityScores.js` 12 KB,
`savingThrows.js` 3.5 KB, plus Player's Option files.

As a cross-check its usable pieces are the small ones — `savingThrows.js` (literal arrays per class
group), the THAC0 formulas and `abilityScores.js` — which overlap EZDM and can be used as a third
opinion. The large spell files are executable JavaScript that mutates module-level tables at load,
so extracting comparable records means running it. v1 ticket 01 already established its record
counts (217 distinct proficiency names over 224 entries) and its modelling; nothing here supersedes
that.

---

## 5. Licence and provenance reality

- **The books are WotC IP and nothing found changes that.** Every project above is a derivative.
  None has permission.
- **A permissive repo licence does not license the game content inside it** — the ticket's warning
  has a concrete example. `AD-DCoreRule2.0Errata` is GPL-3.0 and ships six complete WotC book pages
  as HTML. The GPL covers the project; it cannot and does not cover TSR's text. Its own README says
  as much about why it ships diffs: *"The Help and RTF files contains the whole books, and freely
  distributing them would be a huge copyright infringement."* **The BDiff approach is the most
  legally careful pattern found in this whole survey** and is worth remembering if corerules ever
  needs to ship a correction to something a user already owns.
- **The one prior extraction could not distribute its output.** The FG importer's author shipped the
  *script* and told users to run it on their own disc, because he could not ship the `.mod`. That is
  the same posture `spec.md` §1 fixes for us, arrived at independently.
- **A licensed commercial 2e dataset does exist, and it is closed.** SmiteWorks *"expanded our
  license with Wizards of the Coast to allow us to produce conversions of D&D Classic material"* and
  ships official AD&D 2E Player's Handbook, Monster Manual and Dungeon Master Guide modules for
  Fantasy Grounds, on top of a 2E ruleset built from the same author's community work.
  <https://www.enworld.org/threads/ad-d-2e-core-books-now-on-fantasy-grounds.658665/>
  This refines — it does not overturn — v1 research's conclusion that shipping no content is the
  only configuration under which *public* 2e support exists. Licensed 2e data exists; it is
  encrypted, vendor-locked and unavailable as a corpus or a cross-check.
- **Provenance is undocumented almost everywhere.** Of the datasets in §4, only `dnd-2e-data` names
  its source book and appendix. If we adopt any of them as a cross-check, our own record of *what we
  compared against* has to supply the provenance the source does not.

---

## 6. What this changes for the map

Stated as inputs to existing tickets, not as decisions.

1. **[Ticket 01](../issues/01-what-the-source-yields.md) and
   [09](../issues/09-extraction-pipeline.md) should look at the WebHelp rendition before committing
   to RTF-only.** It supplies, for free, three things the RTF does not have: record boundaries, a
   record *type* per record, and tables as tables. The map's scoping decision — "the corpus target
   is the RTF tier only" — was about *which books*, and is untouched by this; this is about *which
   rendition of the same books*. Whether the WebHelp is obtainable is
   [ticket 02](../issues/02-where-the-corpus-lives.md)'s question, and it is now a sharper one.
2. **[Ticket 10](../issues/10-mechanical-verification.md) gains four concrete cross-checks**: the
   WebHelp tables (same product, different rendition — strongest), EZDM's THAC0/XP/save/ability
   tables, the two spell indexes against each other and against us, and the Dungeon Craft per-class
   tables. Note the alignment cost measured in §3.3(a): no intrinsic key joins the renditions.
3. **The pipeline must decode `\'hh` explicitly and assert its character set** (§3.3(b)), because
   the only prior art we have demonstrably corrupts fractions and deletes footnote daggers in
   silence.
4. **Record input checksums** — at least two DMG RTF variants exist (§1.3).
5. **The corpus is pre-errata** (§1.3). Decide deliberately whether packs transcribe the CD as
   shipped or as corrected, and record which.
6. **"Book + page" citation is not implementable from the RTF** (§3.3(a)). Input to
   [ticket 05](../issues/05-pack-schema.md).
7. **Two corrections to carry back from v1 ticket 01**, neither fatal to its conclusions: Dungeon
   Craft *does* model per-class XP tables and per-level THAC0 as loadable data (§4.3), and a live
   project *does* model 2e spells with real mechanical decomposition (§4.4). The headline —
   "no project models AD&D 2e's rules as loadable data" — is right about *kits* and about the
   *whole* ruleset, and remains the right basis for the map.

---

## 7. Things I could not establish

- **An exhaustive code search.** GitHub's code search needs authentication and `gh` is not logged in
  here; grep.app returned 429 behind a bot check on every attempt; searchcode.com's API is dead
  (404). The negative in §1.1 rests on web search plus targeted inspection of every candidate
  repository, **not** on a full-text scan of public code. A single authenticated
  `gh search code phbbk` would close this properly and costs one command.
- **Whether the WebHelp folder shipped on the base Core Rules 2.0 disc, on the Expansion, or both.**
  Three independent sources place it on "the CD" (the importer author, the errata project, an EN
  World thread); none distinguishes the pressings. Related and also open: whether `DDNNNNN` topic
  IDs are stable across pressings.
- **Whether the WebHelp is a complete rendition of every book we hold as RTF.** Evidence covers PHB,
  DMG, Tome of Magic, Arms & Equipment and at least four Complete handbooks. I did not verify the
  Player's Option books, the Monstrous Manual, or the remaining handbooks.
- **How the WebHelp handles the kit records specifically.** The six sample pages I could verify are
  two spells, two tables and two DMG items. **Whether a kit is one topic per kit, with its labels as
  markup or as inline text, is the single most valuable unanswered question here**, because the map
  measured ~100 kit records as the corpus's most regular structure. `naggaroth`'s mirror would have
  answered it; the site was down (502) for the duration.
- **The two Dragonsfoot threads** most likely to contain community extraction attempts — the
  "MASSIVE AD&D Spell Database" thread (`t=6533`) and the Core Rules CD threads — are behind
  Cloudflare and returned 403 to every method tried; no Wayback capture was retrievable in time.
  There may be a spell database there worth the trip.
- **Pages 2 and 3 of the Fantasy Grounds importer thread.** Only page 1 is in the Wayback capture;
  later breakage reports may exist.
- **Whether any of §4's datasets are actually correct.** I verified their shape, size and stated
  provenance. I did not verify a single value against a book, and neither did their authors publish
  a method. Their worth as a cross-check is that they are *independent*, not that they are *right* —
  a disagreement flags a record to inspect, it does not say who is wrong.
