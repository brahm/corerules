# What each pack kind actually yields at source

Type: task
Status: open
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

## Two things already known to be waiting here

- **The Complete Priest's Handbook's ~59 records do not have a kit's shape.** Ten different fields.
  Whether they are Deity, Class, or a priest specialty class the spec does not name is fog on the
  map, and this ticket produces the characterisation that graduates it.
- **`Nonweapon Proficiencies` splits across two labels** (52 + 54 `Bonus Nonweapon Proficiencies`,
  summing to exactly the 106 of `Weapon Proficiencies`). That is the shape of the whole problem in
  miniature: regular, but not uniform, and the naive regex undercounts by 44%.
