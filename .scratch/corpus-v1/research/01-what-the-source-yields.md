# What each pack kind yields at source

Measurement for [ticket 01](../issues/01-what-the-source-yields.md). Covers the **13 v1-tier books**
in **both renditions** — the RTF in `books/` and the RoboHELP HTML in `/WEBHELP/` inside
`add_core_rules_2_expansion.iso`.

**No book text appears in this file.** Counts, field labels, headings and table titles only.

---

## ⚠ CORRECTION — the HTML measurements below were wrong, and in one direction

Re-measured before [ticket 09](../issues/09-extraction-pipeline.md), in Python. **Every HTML figure
in this file that came from a shell `grep` was an undercount**, because `grep` here is **ugrep**,
which silently skips files it classifies as binary — and these HTML files carry cp1252 high bytes.
On the Complete Fighter's Handbook it skipped **47% of the files**.

The caution written in the Method section below turned out to apply to this file's own HTML numbers.
The Python-based RTF numbers are unaffected.

**What was wrong:**

| Claim | Truth |
|---|---|
| "124 of CFH's 264 files carry no `<TITLE>`"; coverage 28%–94% | **All 3,603 v1-tier files carry a non-empty `<TITLE>`.** The coverage range is void. |
| "Only one kit — Myrmidon — has a titled page of its own" | **Every kit has one.** Verified by title: CFH yields Amazon, Barbarian, Beast Rider, Berserker, Cavalier, Gladiator, Myrmidon, Noble Warrior, Peasant Hero, Pirate/Outlaw, Samurai, Savage, Swashbuckler, Wilderness Warrior — 14. CTH yields 18, likewise one per page. |
| "Neither rendition wins, the split is per book" | **The HTML carries at least the RTF's kit count in every book**, including the three where the RTF exposes none. Kit records: CBD 26→32, CBE 0→13, CBGH 29→30, CBH 18→21, CDH 22→27, CFH 16→34, CPAH 0→21, CPRH 11→78, CRH 0→20, CTH 24→32, CWH 3→12. |
| Table counts (§2) | All undercounts. PHB 578→**692**, DMG 133→**233**, CFH 7→**40**, CBE 3→**22**, CTH 44→**55**. The conclusion strengthens rather than reverses. |
| "The Complete Priest's has 1 HTML table" | 3. Still almost none — that point survives qualitatively. |

**What survives unchanged**, because it was measured in Python: the four record shapes; the ~146 kit
and 60 priest counts; the label-variant analysis; the PHB's 67 numbered tables and their mapping to
§3.1's kinds; and the RTF's own poverty in the Complete Paladin's, Ranger's and Book of Elves.

**Page numbers, re-verified:** the finding stands with a refinement. There is **no per-record page
field** and **no `<META>` at all**; what exists is an in-prose cross-reference (`page 42`) in **48 of
2,840 files — 1.7%** — which cannot serve as a record's own provenance. §7.1's requirement remains
unmeetable and [ticket 05](../issues/05-pack-schema.md)'s replacement stands.

**The corrected headline:** the WebHelp is **uniformly better for record boundaries**, giving one
titled page per kit in every book measured — including the three the RTF cannot serve at all. The
RTF's remaining value is narrower than this file claimed and is the open question
[ticket 09](../issues/09-extraction-pipeline.md) should settle.

---

## Method, and one correction to how it was measured

Counts come from a Python census (`^[ \t]*Label:[ \t]`, counting lines) rather than shell greps.
**This matters: `grep` on this machine is `ugrep`**, whose flag combinations (`-oc`, `-c` with no
match) do not behave like GNU grep and silently produced wrong numbers in a first pass. Any later
session re-measuring this corpus should use the Python route or verify the grep first.

---

## 1. The headline: neither rendition wins, and the split is per book

This is the ticket's central result and it contradicts the expectation both earlier tickets carried.
[Ticket 03](../issues/03-prior-art-core-rules-extraction.md) said the HTML solves tables and its
correction said the HTML fails on kits. **Both are true only on average.** Per book:

| Book | dir | Kit records, RTF | Kit records, HTML | Which rendition |
|---|---|---:|---:|---|
| Complete Book of Dwarves | CBD | 26 | 26 (22 one-per-page) | either |
| Comp. Gnomes & Halflings | CBGH | 29 | 29 (27 one-per-page) | either |
| Complete Bard's | CBH | 18 | 16 | RTF |
| Complete Druid's | CDH | 22 | 15 | RTF |
| Complete Fighter's | CFH | 16 | 10 | RTF |
| Complete Thief's | CTH | 24 | 7 | **RTF, decisively** |
| Complete Wizard's | CWH | 3–12 | 8 | mixed |
| Complete Priest's | CPRH | 11 | 75 | **HTML** (different shape — §3) |
| **Complete Paladin's** | CPAH | **0** | **8** | **HTML only** |
| **Complete Ranger's** | CRH | **0** | **13** | **HTML only** |
| **Complete Book of Elves** | CBE | **0** | **1** | **HTML only** |

The three books at the bottom are the finding. In the Paladin's, Ranger's and Elves handbooks the
RTF exposes **no kit field labels at all**, while the HTML carries individually titled record pages —
`Militarist`, `Skyrider`, `Inquisitor`; `Feralan`, `Forest Runner`, `Greenwood Ranger`.

Measured on the RTF side of those three:

- **`paladnbk`** — 70 labelled lines total across **58 distinct labels**, nearly all chapter
  headings. There is no record structure to parse.
- **`rangerbk`** — the only labels are spell fields (9 spells). No kit vocabulary.
- **`elfbk`** — carries a **subrace** shape instead (§3), plus equipment fields.

**A pipeline that reads one rendition loses whole books.** Reading only the RTF loses every Paladin,
Ranger and Elf kit; reading only the HTML loses two thirds of the Thief kits.

---

## 2. Tables: the HTML wins everywhere, but for a better reason than row counts

A first comparison suggested the RTF held *more* table rows in 8 of 13 books. That was an artifact:
counting "lines with more than two tabs" also counts tab-indented prose. Under a strict definition —
**≥3 non-empty columns containing a digit** — the HTML wins in all thirteen:

| | PHB | DMG | CBD | CBH | CRH | CTH | CWH | CFH |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| RTF data rows (strict) | 923 | 203 | 58 | 78 | 439 | 168 | 40 | 79 |
| HTML `<TR>` | 3293 | 1577 | 731 | 1176 | 885 | 431 | 307 | 110 |

**The real argument is not the count, it is ambiguity.** In the RTF a tab-delimited line cannot be
distinguished from indented prose without a heuristic, and any heuristic has both false positives
(prose) and false negatives (header rows with no digits, text-only cells). `<TR>` needs no heuristic.

**But HTML table markup is unevenly applied**, and a pipeline must not assume it: `<TABLE>` counts
run from 578 (PHB) and 133 (DMG) down to **1 in the Complete Priest's Handbook** and 3 in the
Complete Book of Elves. Where the HTML declines to use table markup it falls back to `<P>` + `<FONT>`
runs, which is *worse* than the RTF's tabs.

---

## 3. Four record shapes, not one — and the kinds they map to

The label census over the whole v1 tier exposes four distinct record shapes.

**Kit** — ~**146 records** across 8 books. Five labels are reliable and co-occur:
`Weapon Proficiencies` (149) · `Role` (147) · `Secondary Skills` (146) · `Special Benefits` (146) ·
`Special Hindrances` (146). Weaker: `Equipment` (128), `Description` (104), `Wealth Options` (79).

**Priest specialty** — **60 records**, one book (`priestbk`), a ten-field shape that shares nothing
with a kit: `Duties of the Priest` · `Followers and Strongholds` · `Possible Symbols` · `Powers` ·
`Weapon and Armor Restrictions` · `Other Limitations` (60 each), plus `Spheres of Influence` (68),
`Minimum Ability Scores` (68), `Races Allowed` (77), `Alignment` (62). **This is the map's fog patch
about the Priest's Handbook, and the measurement graduates it**: the shape is a *priest specialty
class*, and whether it becomes Deity, Class, or something §3.1 lacks is a modelling decision for
[ticket 05](../issues/05-pack-schema.md), not a source question.

**Subrace** — **5 records**, one book (`elfbk`): `Ability Score Adjustments` · `Languages` ·
`Infravision` · `Special Advantages` · `Special Disadvantages` · `Additional Experience Cost`. Maps
cleanly onto `spec.md` §4.1's **Subrace** attachable.

**Spell** — the largest population. `Range` 572, `Duration` 573, `Area of Effect` 572 across 6 books;
`Casting Time` 87, `Components` 86, `Saving Throw` 84; `Sphere` 207.

Two cautions on spells, both load-bearing:

- **The PHB lays spells out differently from the Complete handbooks.** `phbbk` has 488 `Range:` lines
  but only **2** `Casting Time:` lines — in the PHB those fields are not line-start labels. Same
  kind, two layouts, one book apart.
- **`Range` / `Duration` / `Area of Effect` are a shared sub-shape**, not a spell signature: 572
  occurrences against 87 `Casting Time`. Other kinds — magic items, class powers — reuse the same
  three fields. Keying spell extraction on them over-collects by roughly 6×.

**Spell school is not a field.** `School:` appears twice in the whole PHB; school names occur as
section text (`Abjuration` 53, `Conjuration/Summoning` 57). School must be derived from position, not
read from a label.

---

## 4. The label vocabulary is regular, not uniform — quantified

The charting measurement said this; here is the size of it.

- **`Nonweapon Proficiencies` splits three ways**: `Nonweapon Proficiencies` (102) ·
  `Bonus Nonweapon Proficiencies` (54) · `Recommended Nonweapon Proficiencies` (53). The Complete
  Priest's Handbook merges the field entirely as `Nonweapon and Weapon Proficiencies` (60).
- **A naive single-label regex undercounts by 44%** on that field, and by **100%** on three whole
  books (§1).
- Books carry 58–120 *distinct* labels each, most appearing once or twice. **Label frequency is the
  signal**: a label appearing ~n times where n is the book's record count is a record field; a label
  appearing once is a heading.

---

## 5. The PHB's 67 numbered tables map almost one-to-one onto §3.1 kinds

Extracted from the RTF; the titles are the book's own. This is the kind inventory for everything that
is not a kit or a spell.

| §3.1 kind | PHB tables |
|---|---|
| **Ability** | 1–6 (one per ability) |
| **Race** | 7 Racial Ability Requirements · 8 Racial Ability Adjustments · 10 Height/Weight · 11 Age · 12 Aging Effects |
| **Class** | 13 Class Ability Minimums · 22 Wizard Specialist Requirements |
| **Class group** | 14 Warrior · 20 Wizard · 23 Priest · 25 Rogue Experience Levels — **the tables are group-keyed, confirming §3.1's claim that class group is a modelling entity** |
| **Class ability** | 15 Warrior Attacks/Round · 18 Ranger Abilities · 33 Bard Abilities · 16/19/31 Followers · 61 Turning Undead |
| **Lookup table** (spell progression) | 17 Paladin · 21 Wizard · 24 Priest · 32 Bard |
| **Thieving skill** | 26 Base Scores · 27 Racial · 28 Dexterity · 29 Armor Adjustments · 30 Backstab Multipliers |
| **Proficiency slot type** | 34 Proficiency Slots |
| **Non-weapon proficiency** | 37 (the list) · 36 Secondary Skills |
| **Proficiency group** | 38 Group Crossovers |
| **Coin** | 42 Standard Exchange Rates — **the convertible-currency kind, as a table** |
| **Generation method** | 43 Initial Character Funds |
| **Gear** | 44 Equipment |
| **Weapon** | 41 Weapon Construction · 45 Missile Ranges · 52 weapon vs. armour |
| **Armour** | 46 Armor Class Ratings |
| **Encumbrance category** | 47 Character Encumbrance · 48 Modified Movement · 49/50 capacities |
| **Saving throw category** | 60 Character Saving Throws · 9 Constitution Saving Throw Bonuses |
| **Lookup table** (THAC0) | 53 Calculated THAC0s · 54 THAC0 Advancement |

**Not found as tables, and therefore prose:** Alignment, Language, Spell school, Sphere (as a
catalogue), Deity, Weapon proficiency (as a catalogue distinct from the Weapon list).

**Parsing caution:** **4 of the 67 table titles are not on the number's line** (7, 37, 51, 52) — the
extractor picks up the following sentence instead. Table-title extraction needs a fallback.

---

## 6. Bucket assignment

| Bucket | What falls in it |
|---|---|
| **Mechanical** | All 67 PHB numbered tables and their equivalents in other books, **read from the HTML** where `<TABLE>` markup exists. Table titles (with the 4-case fallback). Record *boundaries* in the HTML where `<TITLE>` exists. |
| **Regular but ambiguous** | Kit records in the 8 RTF-bearing books: the five reliable labels give the boundary, but the field *contents* need a written rule — proficiency lines carry a grammar mixing grants, recommendations and group-scoped slot costs. Spell records, given the PHB/handbook layout split. The `Nonweapon Proficiencies` three-way variant. |
| **Judgement** | Every kit's `Special Benefits` / `Special Hindrances` prose. All Paladin, Ranger and Elf kits, which have no machine-readable structure in either rendition beyond the HTML page boundary. Alignment, Language, Sphere and Deity, which are prose everywhere. |

Per [ticket 04](../issues/04-llm-assisted-extraction.md), the middle bucket belongs on the **parser**
side — a rule decided once and applied identically forever.

---

## 7. Two findings that change other tickets

**Page numbers do not exist in either rendition.** Verified: zero `\page`, zero bookmarks, zero
footnotes, zero hyperlinks in the RTF; zero `page N` text and zero `<META>` in the HTML.
[`spec.md` §7.1](../../v1-spec/spec.md) requires **book and page citation on every record** and calls
it the answer to JSON's lack of comments. **That requirement is unmeetable from this corpus.**
Ticket 05 must replace it — the natural substitute is book plus the record's own source anchor
(HTML filename, or RTF line offset), which is reproducible and machine-checkable where a page number
would have been neither.

**The renditions share no identifier**, so a pipeline reading tables from the HTML and kit prose from
the RTF must align them by heading text. That alignment is a real component of
[ticket 09](../issues/09-extraction-pipeline.md), not a detail.

---

## Things I could not establish

1. **Whether the HTML's untitled files matter.** 124 of the Complete Fighter's Handbook's 264 files
   carry no `<TITLE>` and hold ~70% of its text, but title coverage ranges from **28%** (Paladin) to
   **94%** (Priest) across the tier. I did not determine whether untitled files are continuation
   pages of a titled record — which would make them harmless — or independent records.
   **This is the highest-value follow-up** and it decides how much of the HTML is usable.
2. **The exact spell count.** 488 `Range:` lines in the PHB is an upper bound that includes magic
   items and powers sharing the sub-shape; I did not separate them.
3. **Alignment and Language as records.** `Lawful Good` occurs once in the PHB text, which is too few
   for the alignment chapter and suggests a casing or layout difference I did not chase.
4. **The DMG's record shapes.** I measured its tables (203 strict data rows, 1,577 HTML `<TR>`) but
   not its record vocabulary. v1 scope needs less from the DMG, but "less" is not "none".
5. **Whether HTML `<TABLE>` contents are clean.** I verified the markup exists and counted it; I did
   not verify that cells align with the RTF's columns for the same table. Ticket 10's mechanical
   checks want that comparison, and it is a free cross-rendition diff nobody has run.
