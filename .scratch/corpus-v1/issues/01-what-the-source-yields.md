# What each pack kind actually yields at source

Type: task
Status: resolved
Blocked by: —

## Question

Nothing to decide. A fact has to be produced before six other tickets can decide anything, and it is
the fact this whole map was scoped around: **for each pack kind, what does the RTF actually give
up?**

The map's charting measured the corpus from the *source* side — file inventory, table markup, kit
label frequencies. This ticket measures the *mapping* side: for every kind in
[`spec.md` §3.1](../../v1-spec/spec.md), locate where it lives in the 13 v1 books and characterise
what extraction yields.

Take the kinds as given. §3.1 already fixed them, and this ticket does not reopen that list:

> Race · Class · Class group · Kit · Deity · Alignment · Ability · Language · Weapon proficiency ·
> Non-weapon proficiency · Proficiency group · Proficiency slot type · Thieving skill · Class
> ability · Spell · Spell school · Sphere · Weapon · Armour · Gear · Saving throw category ·
> Encumbrance category · Coin · Generation method · Lookup table · Rule-set · Content Pack

## What the answer must produce

For each kind, four things:

1. **Where it lives.** Which book, which section, and how a parser recognises the boundary. "The
   thieving skill table is one tab-delimited block under a numbered heading" is an answer; "in the
   PHB somewhere" is not.
2. **Which of three buckets it falls in.** This is the ticket's central output and the line
   [v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md) explicitly pushed out of the
   spec and onto this pipeline:
   - **mechanical** — a parser gets it right with no judgement (tab-delimited tables look like this)
   - **regular but ambiguous** — structure is reliable, content needs a rule someone writes. The
     proficiency lines are the type case: they follow a parseable grammar of the form
     `Bonus Proficiency: <name>. Recommended: <group>-<name>, <name>, (<group>) <name>, ...`, but
     somebody has to decide that "Recommended" is advice and not a grant, and that a parenthesised
     group name changes the slot cost of everything after it
   - **judgement** — the book states it as prose and a human has to model it
3. **A count.** How many records of this kind exist across the 13 books. The map needs magnitudes to
   size tickets 08 and 13, and counts are also what [ticket 10](./10-mechanical-verification.md)
   will check extraction against.
4. **What it needs that the format may not have.** Flag anything whose expression looks beyond
   `spec.md` §4.3's six operations. This is the early-warning signal for **known unknown #4** of the
   v1 spec, which says the vocabulary may prove insufficient and that this is *"checkable before
   code is written, since the books are in hand"*.

## This ticket now covers two renditions, not one

[Ticket 03](./03-prior-art-core-rules-extraction.md) established that the same books exist twice on
this machine: the RTF in `books/`, and a RoboHELP **HTML** export inside
`add_core_rules_2_expansion.iso` under `/WEBHELP/`, covering the whole v1 tier with **real
`<TABLE>` markup**.

They are good at opposite things, and this ticket must measure **both**:

| | RTF | WebHelp HTML |
|---|---|---|
| **Tables** | destroyed — flattened to tabs | **real `<TABLE>`/`<TR>`/`<TD>`** |
| **Record boundaries** | kit field labels are regular (~100 records, five reliable labels) | 70% of text in files with **no `<TITLE>`**; only one CFH kit has its own page |
| **Page numbers** | **none** — zero `\page`, verified | unknown — measure it |

So bucket 2 of the four outputs above gains a column: for each kind, say **which rendition yields it
better**, not just which bucket it falls in. A kind that is mechanical in one rendition and
judgement in the other is the most valuable thing this ticket can find, because it decides the
pipeline's shape in [ticket 09](./09-extraction-pipeline.md).

**Also measure whether the WebHelp carries page numbers at all.** `spec.md` §7.1 requires book *and
page* on every record and the RTF cannot supply the page. If neither rendition can, that requirement
is unmeetable from this corpus and [ticket 05](./05-pack-schema.md) has to replace it — which is a
schema change, so the earlier it is known the cheaper it is.

## Where to start

[`tools/dertf.py`](../tools/dertf.py) converts RTF to text preserving `\par` as newline and `\tab`
as TAB, which is what makes the tables recoverable. It is a charting instrument, not the pipeline —
[ticket 09](./09-extraction-pipeline.md) decides what the real thing is. Improve it here if that
helps the measurement; do not design the pipeline in it.

The map's Notes carry the measurements already taken. Do not repeat them.

## Constraints

**No book text in this repository.** Field labels, headings and counts only — never a paragraph of
body text, never a table's contents. See the map's hard constraint. Findings go in
`research/01-what-the-source-yields.md`, under the same rule.

## Answer

Full measurement: [`research/01-what-the-source-yields.md`](../research/01-what-the-source-yields.md).
Both renditions, all 13 v1 books.

> **⚠ Corrected after the fact.** Every HTML figure in the original answer came from a shell `grep`,
> and `grep` here is **ugrep**, which silently skips files it classifies as binary — which these
> cp1252 HTML files are. It skipped 47% of the Complete Fighter's Handbook. The caution this ticket
> itself recorded turned out to apply to its own numbers. See the Correction block at the top of the
> research file; the corrected headline is at the end of this answer. **The RTF numbers, measured in
> Python, are unaffected.**

**The headline contradicts what both earlier tickets assumed: neither rendition wins, and the split
is per book.** Ticket 03 said the HTML solves tables; its correction said the HTML fails on kits.
Both are true only on average. In the **Complete Paladin's, Ranger's and Book of Elves the RTF
exposes no kit structure at all** — `paladnbk` has 70 labelled lines across 58 distinct labels, none
of them record fields — while the HTML carries individually titled kit pages. In the **Complete
Thief's the RTF finds 24 kits to the HTML's 7.** A pipeline reading one rendition loses whole books
either way.

**Tables: HTML everywhere, but the argument is ambiguity, not volume.** A first pass suggested the
RTF held more rows in 8 of 13 books; that was tab-indented prose being counted as data. Under a
strict definition the HTML wins in all thirteen — and the real point is that an RTF tab line cannot
be told from indented prose without a heuristic that has both false positives and false negatives,
while `<TR>` needs none. **But HTML table markup is unevenly applied** — 578 tables in the PHB, **1**
in the Complete Priest's Handbook — and where it is absent the HTML is *worse* than the RTF.

**Four record shapes, where the ticket expected one:** Kit (~146 records, 8 books, five reliable
co-occurring labels), Priest specialty (60 records, ten fields, sharing nothing with a kit), Subrace
(5 records, mapping cleanly onto §4.1's Subrace attachable), and Spell (the largest population).
**This graduates the map's fog patch about the Complete Priest's Handbook**: the shape is a priest
specialty class, and which kind it becomes is now a modelling decision for
[ticket 05](./05-pack-schema.md) rather than a source question.

**The PHB's 67 numbered tables map almost one-to-one onto §3.1's kinds** — the research file carries
the table. Two confirmations fall out: the experience-level tables are **keyed by class group**,
which is §3.1's claim that class group is a real modelling entity rather than a UI grouping; and
**Coin appears as a table of exchange rates**, which is the convertible-currency kind arriving as
data exactly as §3.1 predicted.

**The label vocabulary is regular but not uniform, and now quantified.** `Nonweapon Proficiencies`
splits three ways and the Priest's Handbook merges it with the weapon field entirely; a naive
single-label regex undercounts by 44% on that field and by **100%** on three whole books.

### Two findings that change other tickets

**Page numbers do not exist in either rendition** — zero `\page`, bookmarks, footnotes or hyperlinks
in the RTF; zero page text or `<META>` in the HTML. `spec.md` §7.1 requires **book and page on every
record**. That requirement is **unmeetable from this corpus**, and ticket 05 must replace it. The
natural substitute is book plus the record's own source anchor, which is reproducible and
machine-checkable where a page number would have been neither.

**The renditions share no identifier.** Reading tables from one and kit prose from the other means
aligning by heading text — a real component of [ticket 09](./09-extraction-pipeline.md).

### Bucket assignment

**Mechanical:** the numbered tables, read from HTML where `<TABLE>` exists; record boundaries where
`<TITLE>` exists. **Regular but ambiguous:** kit records in the 8 RTF-bearing books, spell records
given the PHB/handbook layout split, the proficiency-label variants. **Judgement:** all
`Special Benefits`/`Special Hindrances` prose, every Paladin/Ranger/Elf kit, and Alignment, Language,
Sphere and Deity, which are prose everywhere.

### Measurement caution for later sessions

**`grep` on this machine is `ugrep`**, and flag combinations like `-oc` silently produced wrong
counts in a first pass. Every number here comes from a Python census instead. Re-measure that way or
verify the grep first.

### ~~The highest-value follow-up~~ — measured, and it dissolved the question

There are **no untitled files**. All 3,603 v1-tier WebHelp pages carry a non-empty `<TITLE>`; the
28%–94% coverage range was ugrep skipping binaries. The question of whether untitled files were
continuations or records does not exist.

### The corrected headline

**The WebHelp is uniformly better for record boundaries**, not per-book better. It gives **one titled
page per kit in every book measured** — the Complete Fighter's yields Amazon, Barbarian, Beast Rider,
Berserker, Cavalier, Gladiator, Myrmidon, Noble Warrior, Peasant Hero, Pirate/Outlaw, Samurai,
Savage, Swashbuckler and Wilderness Warrior; the Complete Thief's yields eighteen the same way. And
it carries **at least the RTF's kit count in every book**, including the three where the RTF exposes
none.

So the original claim — *neither rendition wins, the split is per book* — was an artifact. What
survives is the **RTF's poverty**, which was measured in Python and is real: the Complete Paladin's,
Ranger's and Book of Elves have no kit structure in the RTF at all. What does not survive is the idea
that the RTF wins anywhere on record boundaries.

**What the RTF is still for is now the open question**, and it belongs to
[ticket 09](./09-extraction-pipeline.md). Its remaining candidates are narrow: field *contents* where
the HTML's markup discards structure the tabs preserved, and a second independent extraction to diff
against — which [ticket 04](./04-llm-assisted-extraction.md) says is the only kind of check that
works.

## Two things already known to be waiting here

- **The Complete Priest's Handbook's ~59 records do not have a kit's shape.** Ten different fields.
  Whether they are Deity, Class, or a priest specialty class the spec does not name is fog on the
  map, and this ticket produces the characterisation that graduates it.
- **`Nonweapon Proficiencies` splits across two labels** (52 + 54 `Bonus Nonweapon Proficiencies`,
  summing to exactly the 106 of `Weapon Proficiencies`). That is the shape of the whole problem in
  miniature: regular, but not uniform, and the naive regex undercounts by 44%.
