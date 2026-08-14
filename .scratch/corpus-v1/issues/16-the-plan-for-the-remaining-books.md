# The plan for the remaining books

Status: resolved — all four decisions taken in ticket 13 session 61

## Why this exists

The [map's destination](../map.md#destination) says it is reached when *"a proving slice has been
transcribed end to end, the schema and the expression language are fixed at `0.x`, and **the remaining
books are mechanical work with no decisions left inside them**."*

The first two clauses are done. This ticket answers the third, and it answers it by **measuring what is
left rather than asserting that it is easy** — because the whole method of this map has been that a
claim about the corpus is worth what its measurement is worth.

The short answer: **what remains outweighs what is done — 469 spells and 48 Deities against a pack of
461 records — most of it is more mechanical than the slice was, and four decisions are left inside
it.** Three of them are cheap. One of them —
what a spell record is *for* — is the largest open question in the effort and has never been asked.

## What is transcribed, measured

461 records, from 15 books, against a v1 tier of **3,603 WebHelp pages**.

| | at source | in the pack | |
|---|---:|---:|---|
| kits | 174 | **174** | ✅ one had been lost to a name collision |
| Deities (CPRH) | **59** | **59** | ✅ three were being dropped |
| Subraces | ~6 | 6 | |
| PHB nonweapon proficiencies | 65 | 65 | ✅ |
| proficiencies the handbooks add | ? | 54 | six books read, five unread |
| weapons (Table 44) | 79 | 79 | ✅ |
| races, abilities, alignments, thieving skills | 29 | 29 | ✅ |
| lookup tables | 67 PHB + 34 DMG in scope | **101** | ✅ |
| **spells** | **469** | **12** | |
| **classes** | **~15** | **0** | |

### The slice has one hole in it, and a name collision made it

`cbd:DD04648` — the **Patrician** of the Complete Book of Dwarves — is extractable and **is not in the
pack**. The extractor finds 24 CBD kits; 23 were transcribed.

`cwh:DD06130` is *also* called Patrician, and *was* transcribed. §7.3 says a name is never identity and
the schema enforces it perfectly — **and the transcriber checked by name anyway**, saw a Patrician in
the pack, and moved on. The rule protected the data and not the person applying it. **Step 0 of this
plan is to transcribe the CBD Patrician**, and the general lesson is that the extractor's output is
the checklist, never the pack's own names.

## The four decisions

Everything else in this ticket is volume. These are the parts where somebody has to choose.

### Decision 1 — what a spell record is for

**469 spell pages, and they are the most regular thing in the corpus.** Every one carries `Range`,
`Components`, `Duration`, `Casting Time`, `Area of Effect` and `Saving Throw`; all 167 priest spells
carry `Sphere`; the level and the caster class are in the page title. **100% field coverage across 469
pages** — the kits never came close to that.

So the *extraction* is trivial. The decision is what the record is **for**:

- **As a label.** Id, name, level, caster class, sphere or school — which is exactly what the pack's
  twelve spells already are. A kit that grants a spell can point at it, a reviewer can find it, and the
  Engine never reads a spell's mechanics because character generation does not cast anything.
- **As a mechanic.** Add the six printed fields, and the Engine can answer *"what is my range?"* — but
  the printed fields are `Range: 10 yards/level`, `Duration: 1 round + 1 round/level`, which is the
  expression language again, aimed at a population three times the size of the kit slice and with none
  of their regularity in the *values*.
- **As a full effect model.** What the spell *does* is a paragraph of prose. This is out of the
  question for v1 and should be said so explicitly rather than left as an implied ambition.

**TAKEN: label plus the six printed fields as opaque strings.** 470 records. What the recommendation
did not foresee is that the *fields* are perfectly regular and the *markup* is not — four variants of
the school parenthetical, values that wrap into the next row, five singular labels
([finding 121](./13-transcribe-the-proving-slice.md#finding-121--the-spell-corpus-is-regular-in-its-fields-and-irregular-in-its-markup)) —
and that the corpus names its own schools two different ways
([finding 122](./13-transcribe-the-proving-slice.md#finding-122--the-corpus-names-its-own-schools-two-different-ways)).
The reasoning as written: The strings are exactly what
the book prints, A3 makes an unparsed string honest, and nothing forces the expression language to grow
across 469 records to serve a question the Engine has not been asked. **This is the single largest
decision left and it is one ticket's worth of argument, not one session's.**

### Decision 2 — `classes` is a missing kind, and it is the most referenced thing in the pack

**171 of the pack's 288 unresolved references are a class.** Every Attachable names the class it
attaches to — `phb:fighter` 37 times, `phb:thief` 32, `phb:priest` 22 — and **no class record exists.**

[Finding 117](./13-transcribe-the-proving-slice.md#finding-117--the-most-repeated-reference-in-the-pack-was-never-counted)
is why nobody noticed: the checker walked effects and prerequisites and **never walked `target`**.

The decision is not *whether* — it is **what a class carries**. A class is not an Attachable: nothing
attaches it, it is what a character *is*, which is the shape [finding 107](./13-transcribe-the-proving-slice.md#finding-107--a-race-carrying-effects-is-not-a-new-idea-it-is-one-the-schema-withheld)
gave races. But a class owns **tables** — XP progression, THAC0, saving throws, spell progression — and
`tableValue` already reads one *of* a class (`{supplies: "thac0", of: "phb:fighter"}`). So the likely
shape is **a Race-like record that owns lookup tables**, and the four multi-class combinations
(`phb:fighter-thief` and friends) are a second question §6.1 already has an opinion about.

**Cheap, high value, and it is what makes the pack loadable.** It should be next.

**Taken in session 60.** 19 records — four groups, nine classes, five multi-class arrangements — and
the four experience tables. Reference resolution went **72% → 89%**, and every `target` in the pack now
lands. What the decision did *not* finish is the tables: a class indexes a COLUMN of its group's
progression, and Table 14's columns are `Fighter`, `Paladin/Ranger` and `Hit Dice (d10)` — two
vocabularies at once, with one column naming two classes. See
[finding 120](./13-transcribe-the-proving-slice.md#finding-120--a-printed-table-can-have-two-column-vocabularies-at-once).

### Decision 3 — how much of the DMG is in scope at all

**965 pages, untouched, and the map's destination names it.** Its chapters are not one thing:

| | pages | |
|---|---:|---|
| chapters 1–8 — ability scores, races, classes, alignment, proficiencies, money, magic, experience | 200 | **the same subjects as the PHB** |
| chapters 9–15 — combat, treasure, encounters, NPCs, vision, time, miscellany | 765 | the DM's table |

**TAKEN: chapters 9–15 are out of scope; chapter 2's Table 7 is in.** The Engine ships character
generation and advancement, so **765 of the DMG's 965 pages are the DM's table** and are declared out
of scope in [the map](../map.md#out-of-scope) rather than left as a debt.

One correction to this ticket's own wording: the manifest is **not** the place for that statement.
`manifest.declares` is A3's rule-set enumeration — §3.4's single exception to open enumerations — and a
scope statement is not a rule-set name. Scope lives in the map, which is where this methodology keeps
it.

One page is not optional: **`DD00223`, Table 7, Racial Class and Level Limits.** The PHB states the
human's unlimited advancement, contrasts it with every other race, and then
tells the player to ask the DM.
The numbers are in the DMG. **A race's level limits are character generation, and they are the one thing
the PHB cannot supply.**

### Decision 4 — the permit lists, which no amount of transcription closes

About **25 unresolved ids are placeholders**, not records: `weapons-outside-explorer-list`,
`armor-other-than-leather`, `all-except-concealable`, `thief-weapon-restriction`. They are the shape
[finding 43](./13-transcribe-the-proving-slice.md#finding-43--two-thirds-of-kits-give-examples-not-enumerations)
named — a book bounding a choice rather than naming its members — and **transcribing every remaining
book will not remove a single one.**

**TAKEN, and it had a clean answer the plan did not see**: a complement is not a record, it is the
record's own sentence, so it becomes a `defines` and the id disappears — 33 of them. The eleven reached
by `except` are different, because `except` alone has no `defines`: its meaning is to lift a
restriction that exists elsewhere, and those elsewheres are real PHB rules that get a small
`limitations` kind. **The operation's shape told us which repair each reference needed.** See
[finding 124](./13-transcribe-the-proving-slice.md#finding-124--a-complement-is-not-a-record-it-is-a-definition).

## The mechanical remainder, in order of value

Each of these is volume with a known method and no decision inside it.

1. ~~**The CBD Patrician**~~ — **done in session 68.** The slice is whole at 164 kits.
2. ~~**Classes**~~ — **done in session 60**: 19 records, 171 references closed.
3. ~~**Secondary skills**~~ — **done in session 61**: Table 36's 23, and seven more hand-minted slugs
   corrected on the way (finding 125).
4. ~~**The 48 remaining CPRH Deities**~~ — **done in session 62**, and there were **51**: three were
   being dropped for one unmarked label (finding 127). Modelled by a program, which this book alone
   earns (finding 129).
5. ~~**The five unread proficiency books**~~ — **read in session 63, and four of them have no
   proficiency chapter at all** (finding 132). The Fighter's has one, and what is in it is a
   weapon-group system and 46 new weapons, not proficiencies. This item was
   a block of five and was a block of one.
6. **The remaining lookup tables** — the PHB prints 76 and the DMG 110; 6 are transcribed. `extract_tables.py`
   emits `supplies` and the row keys empty for a human, deliberately.
7. ~~**Terrains and creature vocabulary**~~ — **done in sessions 64 and 66**. The terrains were druid
   BRANCHES and became classes with `variantOf`; the creatures became a kind, and the fourteen names
   came out of the field paths into conditions (finding 141). It was a kind question, and the answer
   was already in the schema.
8. ~~**Spells**~~ — **done in session 61**: 470 records, the largest single block in the corpus.

## So: is the rest mechanical?

**Mostly, and more so than the slice was.** The slice was hard because the *format* was moving —
**fifteen commits have touched the schema and all of them came out of transcription**. What remains is dominated by two blocks — 469 spells and 48
deities — that are **more regular than any kit**, plus a long tail of small closed tables.

**But the destination's wording is not yet satisfied.** Four decisions remain inside the remaining
books, and one of them (spells) is big enough to deserve its own ticket. The honest statement is:

> The remaining books are mechanical work **once four decisions are made**, three of which are one
> session each, and one of which is a design question the effort has never asked.

That is a materially different claim from *"no decisions left"*, and it is the one the measurement
supports.


## Where this leaves the map's third clause

All four decisions are taken, so the sentence the ticket was opened to test can now be answered.

**The remaining books are mechanical work.** What is left is item 4 (48 Complete Priest's Deities),
item 5 (five unread proficiency books), item 6 (the remaining lookup tables), item 7 (terrains and the
creature vocabulary) and the CBD Patrician — volume with a known method, no decision inside any of it.

Two things are *not* mechanical and are not in the remaining books either: **the format's own gaps**,
which [ticket 13's verdict](./13-transcribe-the-proving-slice.md#the-verdict) measures and the map's
corrections list carries, and **the 34 references that still dangle** — a third of them the creature
and terrain vocabulary that no kind holds, which is a kind question, not a volume one.
