# Which spec does the Engine implement?

Type: grilling
Status: resolved — the schema is the format, the spec is the product

## Question

`spec.md` is the specification. It is also **wrong in 42 places, and we know which**.

The [corpus map](../../corpus-v1/map.md#corrections-owed-to-the-v1-spec) collected every conclusion
that did not survive contact with the books. Nine were fixed in the schema as they were found; **33
are owed to `spec.md` and unwritten there**. So the document an implementation session reads still
says the superseded thing, and the corrected thing lives in a different repository's map, in a list
ordered by discovery rather than by section.

This ticket decides **what an implementation session is supposed to read**, and it has to be decided
first, because every other ticket in this map inherits the answer.

## Why this is a decision and not a chore

The obvious move — *"update `spec.md` from the list, then start"* — is one option, not the answer.
Three things argue against taking it reflexively.

**The corrections are not all the same kind of thing.** Reading the 42: some are outright reversals
(§4.3 needs a clamp between `adjust` and `set`), some are widenings the schema already carries
(`scalar` gained a third arm), some are *measurements* rather than corrections at all (the Engine's
field vocabulary is far larger than the schema's, and nothing checks it). Folding a measurement into
a specification as though it were a rule change would make the spec worse.

**A spec is written to be implementable, and the list is written to be complete.** They are ordered
by different things. Correction 16 became 16b through 16e because one item turned out to be four —
useful history in a map, noise in a specification.

**The spec may not be the artifact that should absorb them.** Nine corrections were absorbed by
`pack-0.1.schema.json` instead, and that turned out well: a schema change is testable the moment it is
made, and `validate.py` proves the corpus still conforms. A correction landed in prose is checked by
nobody. **The question is not only *when* to update the spec but *which corrections belong in a
document at all*.**

## What would settle it

- **A pass over all 42, classifying each**: reversal, widening already carried, measurement, or
  product decision. The classification is the work; the update policy falls out of it.
- **A judgement on §13's known unknowns.** The spec lists nine; the corpus map answered #2 and #4 and
  left #1 open with far more evidence than it had. Whatever happens to the corrections list, §13 is
  stale in a way that matters, because it is the section a reader consults to find out what is still
  risky.
- **A statement of what an implementation session reads on day one.** One document, or a document plus
  a list, or a rewritten spec. Whichever it is, it should be nameable in a sentence.

## The classification pass

Done over the 44 as they stand. **Ten are already RESOLVED in the schema**; of the 34 owed, the
headline count is misleading and the useful number is much smaller.

| | | |
|---|---:|---|
| **superseded or retracted** | 8 | 5, 9, 10, 16, 16c, 16d, 17, 22 |
| **method — belongs to the maps, not to a specification** | 9 | 4, 8, 12, 20, 28, 29, 30, 31, 39 |
| **the spec says something the corpus disproved** | 12 | 1, 2, 3, 6, 7, 11, 13, 14, 15, 21, 37, 38 |
| **the spec is silent about something that turned out to matter** | 5 | 23, 25, 33, 34, 41 |

**Seventeen touch `spec.md`.** The other seventeen are history or method: a progress report that a
later correction overtook (16 → 16c → 16d → closed at 99.9%), a retraction (10), or a fact about the
extractor and the checker that a reader of the specification has no use for.

Two are worth calling out because they look like corrections and are not:

- **17 is superseded by 38.** It said the best-evidenced missing piece was *a second subject*; reading
  the eighteen markers one at a time showed the subject was never the hard part. **Folding 17 into the
  spec would write down a diagnosis this effort disproved.**
- **5 is superseded by 13.** *Six operations cannot express the corpus* became *six operations
  suffice, and the shortfall is elsewhere* over 1,910 effects.

**This is the argument for the classification made concrete**: a list ordered by discovery contains
its own dead ends, and folding it in wholesale would put two retracted conclusions into the
specification.

## What is NOT on the list, and matters more than most of it

Three things stand between the corrections list and a closeable spec, and none is a correction:

1. ~~**The spec has no concept of an `UNMODELLED` marker.**~~ **Decided.**
   [Ticket 02](./02-what-the-engine-does-with-an-unmodelled-effect.md) resolved it: the operation
   decides, not the category, and a marked effect never reaches the total. §5 gains a **third kind of
   answer** beside a value and a refusal — *withheld from the total, shown in full, attributed* — and
   §5.2's validation/computation line is restated at the grain of an effect, where `grant`/`forbid`/
   `require`/`except` against `adjust`/`set` already draws it. **This was the biggest gap between spec
   and pack and it was not on the corrections list at all**, because the list was written by the effort
   that invented the markers. It produced correction 45, ticket 03 produced 46 and 47, and ticket 05
   produced 48, 49 and 50 — so **the list has gone from 44 to 50 in four tickets and is still not
   frozen**, which is the standing argument against updating `spec.md` before the Engine runs.
2. ~~**Contradiction between Attachables is unresolved.**~~ **Decided, and it was the wrong question.**
   [Ticket 03](./03-precedence-when-two-attachables-contradict.md) measured that **there is no
   precedence to find**: only kits restrict anything, 14,910 sheets produce zero contested values, and
   the three precedence statements in 4,500 pages point in two directions, so any fixed arm order
   contradicts one of them. §4.3 gains **no ordering** — precedence is declared by one record about
   another (`target`, `except`) or the value is refused with both books named. **§13's known unknown
   #1 is therefore answered**, which changes what §13 has to say. What was underneath it is now
   [ticket 05](./05-an-operand-that-lives-in-another-layer.md) — which is **also decided**, and
   needed no change to §4.3's six operations: a bound is a record with `members`, `forbid` applies it,
   `except` lifts it, and the set operations commute. **Correction 13 holds: six operations suffice.**
3. **§13 itself is stale.** Nine known unknowns: **#1, #2 and #4 are now answered**, #3 (psionics) is
   untouched, and #5–#9 are packaging details the v1 spec map partly settled
   by experiment. It is the section a reader consults to find out what is still risky, and it is
   currently wrong about two thirds of its contents.

## What it must not do

**Do not turn this map into a documentation effort.** The destination is an Engine that runs. If the
answer is *"rewrite the spec first"*, that is a decision with a cost, and the cost should be stated
before it is paid rather than discovered halfway through.

---

## The judgement on §13

The ticket named this as a settling item and it turned out to be the more useful half. Nine entries,
judged one at a time, come to **four statuses — not the "two thirds stale" this ticket estimated
before reading them**:

| | | |
|---|---:|---|
| **closed** | 3 | #1 kit prior art, #2 no shipping precedent, #4 six operations |
| **stands, and is now the only substantive one inherited** | 1 | #3 psionics |
| **never visited, because nothing has been packaged** | 5 | #5–#9: SmartScreen, Workflow labels, Intel Macs, `node:sqlite`, the unsigned build |

"Never visited" is a real status and it was worth separating out. Five of the nine are not stale at
all — they are build-and-release questions that neither the corpus map nor the Engine map went
anywhere near, and they will be answered by the first release, not by more reading.

**And the three that closed did not close the way they were asked.** #1 reads *"the kit mechanism has
no prior art anywhere"* and #2 reads *"has no shipping precedent"*, and **neither was answered by
finding a precedent**. Both were answered by building the thing and measuring it: §4.1's one-shape
claim held across 238 records and three arms, and a table now declares `supplies`. An entry phrased
as an anxiety cannot close, because nothing counts as evidence against it. **An entry has to be a
question with a test.**

§13 is rewritten on that principle: four entries, each with the test that would settle it — psionics,
bounds over item properties, the corpus the pack cannot contain, and whether a marker can expire.

*(A correction to this ticket's own earlier text: it said ticket 03 answered known unknown #1. It
answered the **residue** corpus ticket 13 left standing after the one-shape claim held. The prior-art
question itself was never re-asked and did not need to be.)*

## The classification, re-measured over 50

The list was 44 when this ticket was charted. Four Engine tickets later it is **50**, and the six new
ones do not fit the four categories the first pass produced:

| | | |
|---|---:|---|
| already RESOLVED in the schema | 10 | |
| superseded or retracted | 8 | 5, 9, 10, 16, 16c, 16d, 17, 22 |
| method — belongs to the maps | 9 | 4, 8, 12, 20, 28, 29, 30, 31, 39 |
| **the spec says something the corpus disproved** | 12 | 1, 2, 3, 6, 7, 11, 13, 14, 15, 21, 37, 38 |
| **the spec is silent about something that matters** | 7 | 23, 25, 33, 34, 41, **47, 50** |
| a schema change, owed and not yet made | 1 | **48** |
| **a defect in the pack** | 3 | **45, 46, 49** |

**"A defect in the pack" is a category the corpus list could not have.** That map *was* the pack: a
defect found was a defect fixed in the same session, and only the surviving *conclusions* were written
down as corrections. The Engine reads the pack from outside and finds defects it has no standing to
repair, so it must record them. **Nineteen touch `spec.md`.**

## The measurement that decided it

Reading the nineteen for **which section they land in**:

- **Eighteen are about the format** — §3 object kinds, §4 the layer model, §7 content packs. Page
  citation, expressions-as-strings, the clamp, `require.from`, the dice grammar, id ordering, the
  scalar vocabulary, markers, the predicate diagnosis, proficiency scores, `limitation.members`,
  item properties.
- **One is not.** Correction 6: **mandatory provenance forbids a house-rule pack, which §5.1
  explicitly supports.** §5.1 says the escape hatch *is* the pack; §7.1 requires every record to cite
  a book. A house rule has no book.

And the format already has a second description:

| | |
|---|---:|
| commits to `pack-0.1.schema.json` | **22** |
| commits to `spec.md` | **1** |

**The document that is checked was corrected twenty-two times against the books. The document that is
not was written once and never touched.** Every one of the eighteen has a counterpart that is already
in the schema, or belongs there. That is not an argument for updating the spec faster. It is an
argument that **the spec stopped being the definition of the format some time ago and nobody said so.**

## The decision

**`spec.md` §§3, 4 and 7 are non-normative. `pack-0.1.schema.json` is the format.**

Day one reads, in a sentence: **the spec for what the product is and what it refuses, the schema for
what a pack may contain, and the Engine map's four decisions for what the Engine does with what it
loads** — and where the spec and the schema disagree about the format, the schema is right.

This is not a demotion of the spec. §§3, 4 and 7 are kept in place as **the reading guide that says
why the schema is shaped as it is**, which is a thing the schema's own `description` fields do only in
fragments. What changes is which one a reader believes when they differ.

**Eighteen of the nineteen corrections stop being owed**, not because they were fixed but because the
sections they correct no longer claim to be right. The corpus map's own instinct was already this: it
resolved ten corrections in the schema rather than in prose, and noted why — *"a schema change is
testable the moment it is made, and `validate.py` proves the corpus still conforms. A correction
landed in prose is checked by nobody."*

### What was done in this session

- **§13 rewritten** — nine entries to four, each with a test.
- **§§3, 4 and 7 marked non-normative**, with the schema named as the authority and the reason given.
- **§5.4 added — a third kind of answer.** This one *had* to land in the spec: §5 is the behaviour
  half, which stays normative, and ticket 02's decision about 381 marked effects has no other home.
  It also carries ticket 03's result, that a value may be withheld for three distinct reasons.

### What is left, and its size

- **Correction 6**, the one spec correction that survives the decision. A contradiction between §5.1
  and §7.1, one paragraph, and it needs a judgement about whether a house-rule pack cites nothing or
  cites itself.
- **Correction 48** — `limitation` gains `members`. A schema change, testable the moment it is made,
  and the prerequisite for corrections 46 and 49.
- **§§9–12** were never in question and were not touched.

## Why this ticket says do not rewrite the spec now

The ticket asked for the cost to be stated before it is paid. Stated:

**A full rewrite is seventeen-to-nineteen edits across thirteen sections, roughly half a day — and the
evidence is that it would be stale again within a week.** The list grew from 44 to 50 in four Engine
tickets, and every one of the six came from *running a program*, not from reading. Correction 45 was
found by a script counting markers. Correction 47 was found by a sweep looking for something else
entirely. Correction 41 was found by the first program that ever computed a character sheet.

**A specification for a format nothing has implemented yet is a document that is wrong at a rate
proportional to how much you use it.** The right response is not to update it faster; it is to stop
having two of them, and keep the one a checker reads.

## What this ticket must not do — kept

The destination is an Engine that runs. This session wrote three edits and one rewritten section, and
**closed eighteen corrections by deciding they were owed to the wrong document.** The map goes back to
building.
