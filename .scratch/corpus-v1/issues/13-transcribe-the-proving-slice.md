# Transcribe the proving slice end to end

Type: task
Status: open
Blocked by: 05, 06, 08, 09, 10, 14, 15

## Question

Nothing to decide. **This is the map's execution ticket** — the one that makes this effort override
wayfinder's plan-don't-do default — and the point at which the destination is reached or found to be
wrong.

By the time it is takeable: the schema exists (05), the expression language exists (06), the slice is
chosen (08), the pipeline has a shape (09), and the mechanical checker knows what it asserts (10).
This ticket runs all of it against real books and produces real packs.

## What it must produce

1. **The slice, transcribed**, in the corpus's home — never in this repository.
   [Ticket 02](./02-where-the-corpus-lives.md) resolved: that home is a **private repository cloned
   to the `spec.md` §8 content path on `/home`**, which *is* the Engine's content folder. So the
   slice is transcribed straight into the directory the Engine reads, and the correction loop is
   live from the first record.
2. **A pack that passes the mechanical checker**, with the failures found along the way recorded
   rather than quietly fixed. What the checker caught is the evidence that
   [ticket 10](./10-mechanical-verification.md) chose the right checks.
3. **A verdict on the schema and the expression language.** Both are `0.x` and both were designed
   before anything was transcribed. What did the books force a change to? A ticket that reports
   "everything worked" after transcribing real 2e kits should be disbelieved and re-read.
4. **A verdict on the three v1 known unknowns.** #1 (the kit mechanism has no prior art), #2
   ("Engine computes, user supplies the tables" has no shipping precedent), #4 (six operations may
   not suffice). This is the first evidence any of them has ever had.
5. **A measured cost per record**, by bucket, so the remaining books can be estimated instead of
   guessed. This is the number [ticket 12](./12-how-much-tool.md) needs and the one that turns "the
   rest is mechanical work" from a hope into a plan.
6. **A verdict on local-model draft quality**, required by
   [ticket 09](./09-extraction-pipeline.md)'s decision 2. That decision accepted a 7–30B local model
   against a frontier one on exactly the half that needs judgement, on the ground that
   [ticket 11](./11-human-review-protocol.md) reviews it regardless so the model produces a draft
   rather than a verdict. **If the draft is bad enough, reviewing costs more than writing from
   scratch** — and this slice is the first and cheapest place that becomes visible. Measure it; do
   not assume it.

## The slice, named

[Ticket 08](./08-which-slice-proves-the-format.md) chose it: **the Complete Thief's Handbook's 24
kits** (7 in the HTML — the widest rendition gap in the corpus), **the five PHB thieving-skill tables
those kits adjust** (26–30), **six to eight Complete Priest's Deity records** sampled for
representativeness, and **all five Complete Book of Elves subraces**. About 40 records and 5 tables.

Two things that ticket asks this one to honour. **All three Attachable arms are present on purpose** —
§4.1's claim that Kit, Deity and Subrace are one shape is the largest invention in the spec and is
untestable with one arm. And the slice **deliberately proves nothing about psionics, spells,
equipment breadth, or the HTML-only failure case**; do not let a passed slice be reported as a
validated format.

## Keep the slice — it becomes the gold standard

Adopted from [ticket 04](./04-llm-assisted-extraction.md). The hand-transcribed slice must be
**retained permanently as the gold standard and regression test**, not discarded once it has proved
the format.

The reason is ticket 04's central finding: **a check only works if its reference is not the model.**
A hand-verified slice is the only reference of that kind this effort will ever produce cheaply, and
keeping it converts a one-off demonstration into ongoing verification at no extra cost. Every later
pipeline change can be re-run against it.

## Progress — the mechanical half of the kits, and three findings

**Not resolved.** What follows is the first execution session; the ticket stays open.

### A gap this ticket exposed before doing any work

The map's Notes say this effort carries execution and name tickets 01, 09 and 10 as producing running
code. **Ticket 09 produced a decision, not a pipeline** — so this ticket depended on code no ticket
owned writing. [`tools/extract.py`](../tools/extract.py) is that code, written here rather than
chartered as a fifteenth ticket.

### Done

**The Complete Thief's kits, mechanically extracted: 20 records, 0 schema errors.** The extractor
reads the WebHelp, finds records by their marked field labels, and emits identity, provenance, target
and cardinality.

### Finding 1 — record boundary detection is **not** mechanical, correcting ticket 01

[Ticket 01](./01-what-the-source-yields.md) put *record boundaries in the HTML where `<TITLE>` exists*
in the **mechanical** bucket. Running it says otherwise.

The extractor returns **20 records where 18 are kits**. The two extras are `Kits and Thief Types` and
`Creating New Kits` — chapter apparatus. And they are not sloppy matches: **their field sets are
identical to a real kit's.** `Kits and Thief Types` carries the same ten labels as `Assassin`,
because it is the chapter that *explains* each field.

**No field-based heuristic can separate them.** They are structurally identical and semantically
different, which is the *regular but ambiguous* bucket in its purest form — landing on record
**boundary**, which ticket 01 had assumed was the safe half. The rule that separates them is written
once by a human, per [ticket 04](./04-llm-assisted-extraction.md)'s finding that the middle bucket
belongs on the parser side.

### Finding 2 — the schema accepted a semantically empty record; **fixed**

The first run validated **cleanly**, which was the bad outcome. A mechanically extracted kit carrying
`effects: []` and no modelled rules **validated as a complete kit with no effects**.

That is **A3's own distinction missing one level down.** §5.1 put it on the pack — *does not
restrict* versus *not yet transcribed* — and a record needs it for the same reason, because
extraction produces the incomplete state **by construction** and the corpus lives in it for years.

Fixed in the schema: `effectsModelled` is now **required** on every Attachable. The extractor emits
`false`, and the 20 records are now honest rather than merely valid. Declared rather than inferred,
exactly as A3 is.

### Finding 3 — expressing §4.1 structurally costs the ability to close the object

Closing the kind objects with `unevaluatedProperties: false` — to stop the extractor smuggling a
`_fields` key into a pack — **rejected `id`, `name` and `effects` as well**. It does not see through
the `allOf` → `$ref` chain that [ticket 14](./14-record-shapes-for-the-slice.md) chose in order to
make §4.1's one-shape-three-times claim load-bearing.

So there is a real trade, discovered by running it: **the shared Attachable base and a cheaply closed
object do not compose in JSON Schema.** Decision 1 of ticket 14 is worth its price, but the price is
that extra properties fall to the validator — [ticket 10](./10-mechanical-verification.md)'s tier
two — rather than being refused by the schema. Recorded rather than worked around.

Field prose is now deliberately **not** carried into the record: it is book text, and the record
already points at it through ticket 05's anchor, which is how ticket 12's review page fetches the
source. Carrying it would duplicate the corpus inside the pack — the shape §6.5 forbids.

### Session 2 — all three Attachable arms extracted and committed

**The slice's mechanical half is in the corpus repository**: 18 Complete Thief's kits, 8 Complete
Priest's Deity records, 5 Complete Book of Elves subraces. **31 records, validating whole against
`pack-0.1`.**

### Finding 4 — the ordinal works, and §4.1's three arms share one extractor

[Ticket 07](./07-identity-and-id-stability.md)'s decision 2 is exercised for the first time:
`cbe:DD04777#1` … `#5`, five subraces in one file, ordinals carried into the anchor as well as the
id. It works.

More useful: **one extractor handles all three arms**, differing only in which label marks a record
and whether the file holds one or many. That is the first *external* evidence for §4.1's claim —
[ticket 14](./14-record-shapes-for-the-slice.md) showed the three fit one schema shape, and this
shows they fit one parser. Both are still short of the real test, which is whether they fit one set
of **effects**.

### Finding 5 — record names carry no markup at all

The subrace names are in **plain text**: no heading, no bold, not even a larger font. Measured on the
page — `<FONT SIZE>` is uniform, there are no non-label `<B>` runs.

So the rule is positional, and it took two attempts to state: the first record's name is the proper
noun before its first field label, and every later one follows the *previous* record's final field.
That is a rule a human writes once and a parser applies forever —
[ticket 04](./04-llm-assisted-extraction.md)'s middle bucket on the parser side, arriving concretely.

**It came out 4 of 5 correct.** `Half-Elf` could not be separated from the prose that follows it,
because English pluralisation turns *elf* into *elves* and no stem rule catches it. **Polishing
stopped there deliberately** — teaching the regex English morphology is the over-fitting ticket 04
warned about, and *one name in five needed a human* is the measurement rather than the defect.

### The two human interventions, which is the number that matters

Across 31 records the mechanical pass needed exactly two:

1. **Excluding two chapter-apparatus pages by name** — finding 1, and unavoidable, since their field
   sets are identical to a real kit's.
2. **Correcting one subrace name** — finding 5.

Both are *boundary and naming*, not content. Neither is a rule that generalises to the next book, and
both belong to the written-once-by-a-human layer.

### Session 3 — the tables, and the first record modelled by hand

**The slice's mechanical half is complete: 36 records, 0 schema errors** — 18 kits, 8 Deity,
5 Subrace, and the 5 PHB thieving-skill tables.

### Finding 6 — the corpus duplicates its own tables, and one copy disagrees

Each PHB table appears **twice**: once in the chapter (`Thieving Skill Base Scores-- Table 26`) and
once in an appendix (`Table 26: Thieving Skill Base Scores`). Same table, two files, two title
formats — **two anchors, and therefore two ids under
[ticket 07](./07-identity-and-id-stability.md)'s source-derived scheme.**

Worse: **Table 29's two copies differ.** Compared cell by cell, the difference is confined to the
**header rows** — one carries a footnote asterisk and breaks a column label differently — and all
eight data rows are identical. So the corpus is internally inconsistent in presentation but not in
data, this time.

Two consequences. **A human picks the canonical copy** — the chapter one was taken, being in reading
order. And [ticket 10](./10-mechanical-verification.md) gains a check nobody had considered: a
**within-rendition duplicate detector**, keyed on the table number the book itself prints. Every
cross-check the map had designed compares *renditions*; this one compares a rendition against itself.

### Finding 7 — the predicate vocabulary has no negation, and ticket 06 never measured it

Modelling the Acrobat by hand hit it on the first record. *"+2 if the Acrobat is wearing no armor
(and, under the optional encumbrance rules, is unencumbered)"* — **"no armour" and "unencumbered" are
negations**, and [ticket 06](./06-expression-language.md)'s closed vocabulary has `compare`, `member`
and `has`, with no `not`.

Measured across the eight RTF kit books, inside effect fields:

| form | count | covered? |
|---|---:|---|
| `not` / `never` / `cannot` — mostly prohibitions | 464 | **yes**, §4.3's `forbid` |
| **`no <thing>` / `without` / `unencumbered`** | **33** | **no — predicate negation** |
| `ought not` / `should not` — race exclusions | 5 | yes, inverted to an A3 permit-list |

**Thirty-three, the same order as the 31 occurrences that fired known unknown #4.**

And the reason it was missed is precise rather than careless: ticket 06 asked *"does the predicate
admit boolean combination?"*, measured **disjunction** at 4 occurrences, and closed the question on
that evidence. **Negation was never measured.** The question was answered correctly for the data it
looked at.

Carried as text and not computed for now, per the standing posture — but recorded here because
33 is above the line the map has been using.

### Finding 8 — half the fields produce no effects at all

Of the Acrobat's six populated fields, **three yield nothing computable**: `Weapon Proficiencies`
says the kit uses what thieves normally use, `Nonweapon Proficiencies` lists *Recommended* — advice,
not a grant, exactly as [ticket 01](./01-what-the-source-yields.md) predicted — and
`Skill Progression` is pure counsel about which skills to raise.

**The judgement pass therefore has less to do than the field count suggests**, and what it does have
is concentrated in `Special Benefits` and `Races`. That should move
[ticket 11](./11-human-review-protocol.md)'s hour estimate down, and it is the first evidence either
way.

### What the modelling confirmed

Three decisions paid off on contact:

- **`interpretation` earned its place on the very first record.** The book says *+1, rising to +2*;
  the layer model wants **two additive adjusts**, which produces the same numbers. That is a reading,
  and now it says so.
- **Order-independence held.** The halfling/gnome carve-out — *may take the kit but do not gain the
  jumping and tightrope bonuses* — models as a **cancelling `adjust`** conditioned on race. It works
  precisely because §4.3's operations commute.
- **Race exclusion inverts into an A3 permit-list**, which requires the full race enumeration and so
  cannot be checked by the pack alone. That confirms ticket 10's split putting cross-pack referential
  integrity on the Engine.

### Session 4 — the Deity arm, and a measurement that was simply wrong

Modelling a **Deity** by hand, because §4.1's claim that Kit, Deity and Subrace are one shape is
only tested when all three arms carry **effects** rather than schema shape.

### Finding 9 — the chapter apparatus recurs in a second book and a second kind

The first Deity record in the committed slice was **`Priesthoods`** — the *Designing Faiths*
template page, whose fields read *"This paragraph describes the usual alignment of such a god…"*.
It carries every field label a real deity carries, **including the marker**, so it parses perfectly
and is not a record. It was record #1 of 8, so the sample was taken straight through it.

Finding 1 found two of these by hand in CTH. This makes it **two books and two kinds**, which
promotes it from a CTH quirk to a property of the WebHelp: *a chapter that documents a record
format does so using the format*.

The interesting part is the attempt to mechanise it. Apparatus pages have a giveaway — they
**describe** their fields instead of filling them — and a detector keyed on that opening scores
**1 hit, 0 false positives across CPRH's 57 pages**. Run against CTH it catches **neither** known
apparatus page.

**So the detector is a hint for a human reading a new book, never a gate**, and the exclusion list
in `extract.py` is human-maintained by measured conclusion rather than by default. Finding 1 is
strengthened, not solved.

### Finding 10 — ticket 06's disjunction measurement was wrong by an order of magnitude

The Agriculture deity says *"Wisdom **or** Constitution 16 means +5% experience"*.
[Ticket 06](./06-expression-language.md) measured genuine disjunction at **4 occurrences** and
closed the question on that evidence — and the schema's `condition` still carries that number.

The regex required *ability → number → `or` → ability*. The corpus mostly writes
*ability → `or` → ability → number*. Re-measured across the eight RTF kit books:

| form | count |
|---|---:|
| `<Ab> N … or … <Ab>` — what ticket 06 measured | 4 |
| **`<Ab> or <Ab> N` — never measured** | **38** |
| `<Ab> or <Ab>`, any continuation | 56 |

Where it lands matters more than the count. **37 are effect conditions** — the prime-requisite
experience bonus — and **the `effect` union does not reference `condition` at all**; its `when` is
a `predicate`, the same flat conjunction. But the rest are worse:

> The halfling **Homesteader** must have a Strength of at least 12 **and** an Intelligence **or**
> Wisdom of at least 12.
> A **Bandit** PC should have a Charisma of at least 12 **and** a Strength **or** Constitution of at
> least 13.

That is `A ∧ (B ∨ C)` — conjunction *and* disjunction in one prerequisite, in v1-tier kits, in the
Subrace book the slice already draws from. **A flat list where "every condition must hold" cannot
express it**, so these two kits are not transcribable as the schema stands.

**Repaired, by one clause type** — a `predicate` entry is *either* a condition *or* an `anyOf` of
conditions, keeping the top level a flat AND. Verified to accept both kits, and to reject nested
`anyOf`, single-term disjunctions and clauses with extra keys. The 36 existing records revalidate
untouched, because a pure conjunction is still a clause.

Worth naming why the repair was cheap: ticket 06's stated objection to `or` was **grammar, nesting
and precedence**, not disjunction itself. A one-level clause has none of the three, so the argument
survived its own evidence being wrong. The decision was right about what it was protecting.

A coda from the same sweep: *"Intelligence or Wisdom, whichever is higher"* (**5 occurrences**) is
not disjunction at all — it is **max() over two scalars**, a computed operand absent from
[ticket 15](./15-computed-operands.md)'s closed set of halving and rounding.

### Finding 11 — two shapes the Deity arm has and the Kit arm did not

Agriculture produced **17 effects**, and three of them cannot be said:

- **The permit-list has no operation.** *"Weapons Permitted: bill, flails (both), hand-throwing
  axe, scythe, sickle"* means **only those**. §4.3 has `forbid` and `except`, but `forbid` takes a
  single `ref` and there is no wildcard, so "everything, minus these" cannot be written. `set` does
  not rescue it either: `operand` is an integer or a computed operand, never a list of ids. Recorded
  as grants, which **loses the prohibition on everything unnamed**.
- **Armour is worse than a list.** *"All non-magical non-metal armour"* is a **predicate over
  items**, not an enumeration — there is nothing to enumerate against.
- **`grant` has no count.** Followers arrive as *one 5th-, one 3rd-, one 2nd- and ten 1st-level
  priests*. `require` has a `count`, but it counts a **choice the player has yet to make**, which
  this is not.

Note that the first two are the same gap the Acrobat met from the other side. There, race exclusion
**inverted into an A3 permit-list** and that was recorded as a confirmation. Here the permit-list is
an **effect**, not a prerequisite, and A3 is a manifest-level declaration — so the inversion is not
available and the gap is real.

### What the Deity arm did to §4.1

**The claim held where it was doing work.** Target, prerequisite and an ordered list of effects fit
a deity with no strain: the whole record validates unchanged against the shared `attachable`, and
the four-condition prerequisite — alignment, two ability minima, a race permit-list — is the same
machinery the Acrobat used.

**`interpretation` earned its place a second time, and for the same reason.** The XP rule reads
*+5% for one prime requisite at 16, +10% for both*; the layer model wants **two additive +5
adjusts**, which produces the same numbers — the identical decomposition the Acrobat's *"+1, rising
to +2"* required. Twice on two records is a pattern, not a coincidence: **the book states totals,
the layer model wants increments.**

The same field also forced a second reading: major access to Summoning is granted as its **three
named spells** rather than the sphere minus everything else, because the subtraction cannot be
written (finding 11).

**What §4.1 did not get tested on is `cardinality`.** All three arms are `one-per-target` in this
slice, so the field is present and unexercised.

### Session 5 — the Subrace arm, from the one book the pipeline cannot read

### Finding 12 — one v1 book carries no field markup at all

[Ticket 09](./09-extraction-pipeline.md) made WebHelp the only parse target because field labels are
carried as `<I>Label:</I>` markup rather than typographic convention. Running the subrace extractor
over **CBGH** — *The Complete Book of Gnomes and Halflings* — returns **0 records from 112 pages**.

Measured across all twenty book directories, as pages carrying at least one label:

| | |
|---|---|
| every other *Complete* handbook | **15–42 %** |
| **CBGH** | **3 %** — 13 labels total, and they are *adventure-hook titles*, not fields |

The Homesteader's page has three `<B>` tags: `Table of Contents` and two empty. Its fields are
typographic — *"Roads to Adventure:"* sits in running prose — and **its prerequisite is a sentence
in the middle of a paragraph**.

So ticket 09's decision holds for twelve of thirteen v1 books and fails completely, not partially,
for the thirteenth. What still holds there is the *page* rule: `<TITLE>` is present and correct, one
record per titled page. **It is the field layer that is missing, not the record layer** — which is
the difference between "write a second parser" and "start over".

And the irony is worth recording rather than smoothing away: **the two kits that forced the schema
repair in finding 10 live in the only book the parser cannot read.** They were found by a text census
of the RTF, not by the pipeline — which is a point in favour of ticket 09's *rejected* arm, the one
that kept both renditions.

### Finding 13 — disjunction in an *effect* has no operation, and the clause repair does not reach it

The Stout's ability line reads:

> Ability Score Adjustments: **-1 to Strength; +1 to either Dexterity or Constitution**

The second half is **one adjust the player chooses between**. Finding 10's repair does not touch
this: a `clause` is a **test**, and this is a **choice**. Nor does `require`, which counts a choice
of **ids** — *pick two weapon proficiencies* — not a choice of **effects**.

**Modelled deliberately wrong**, as +1 Dexterity carrying an `UNMODELLED CHOICE` marker, so the
record errs in a visible direction instead of silently dropping a bonus the character is owed.

This is the third gap of the same family — finding 11's permit-list and follower counts, and now
this. All three are **§4.3 lacking an operation**, and all three were invisible until an arm other
than the Kit was modelled.

### Finding 14 — an embedded table collides with its host record

`Table 6: Stout Ability Scores` is real `<TABLE>` markup **inside the subrace's page**. Run the table
extractor on it and it produces a `lookupTable` with `id: cbgh:DD04891` — **the same id the subrace
record takes**, because [ticket 07](./07-identity-and-id-stability.md) derives identity from source
position and both records are at the same position.

It also takes the **wrong name**: `Stout`, the page title, rather than `Table 6: Stout Ability
Scores`, the caption sitting in a `<B>` just outside the `<TABLE>`. Finding 6's tables each owned a
page, so page title *was* caption; that coincidence is what the extractor encoded.

Both are consequences of the same unstated assumption — **one page, one record** — which ticket 07
already knew was false for subraces and handled with an ordinal. The ordinal disambiguates records of
the *same* kind. Nothing disambiguates two records of *different* kinds from one page.

Here it was resolved by **not emitting the table**: its two columns are the subrace's own prerequisite
and caps, so a table record would duplicate the pack's own data. That is a reading, and the record
says so.

### What the Subrace arm did to §4.1

**All three arms now carry effects, and §4.1 held on all three.** The Stout validates against the
shared `attachable` with no strain: a target, six prerequisite conditions, thirteen effects.

But the arm was only reachable by hand. **Every other record in the slice was extracted and then
judged; this one was authored end to end**, because finding 12 left nothing to extract. That makes it
the slice's only evidence about the hand-authoring path — and it validates identically, which is what
[ticket 12](./12-how-much-tool.md)'s `provenanceMode` was designed to allow.

`cardinality` remains unexercised: all three arms are `one-per-target`.

### Session 6 — the second parser, and what it cannot reach

**Finding 12 is resolved: CBGH goes from 0 records to 38** — 28 kits and 10 subraces — with the other
three books extracting identically to before.

It is **one pluggable layer, not a second program**, which is what finding 12 predicted: the page
layer survived, so only the field layer needed replacing. Both strategies return the same thing —
the paragraphs before the first label, then ordered `(label, value)` pairs — and names, ids,
provenance and records keep one code path.

Three things the writing turned up, each a correction to an assumption rather than a bug:

**The convention is the paragraph, so the parser splits on paragraphs.** The obvious implementation —
one regex over the raw HTML anchored on `<P>` — was written first and **disagreed with a plain-text
census by eleven occurrences**, because arbitrary `FONT`/`B`/`A` tags interleave between the
paragraph break and the first letter. Matching offsets in markup to model a convention that is not
markup is the wrong shape.

**The field vocabulary belongs to the BOOK, not to the kind.** CBGH's subraces are delimited by
`Infravision`; CBE's by `Additional Experience Cost`. The kind is the same and the marker is not.
Packing too: CBE puts five subraces on one page, CBGH gives each its own — so `multi` is a book's
habit, not a kind's nature. **Assuming otherwise is exactly what returned 0 records from 112 pages**,
and the marker had looked like a property of *subrace* only because one book had been read.

**The page title is not always the record's name.** CBGH gives the first kit of each class section
the section's own page, so `DD04865` is titled *Fighter Kits* and is the **Breachgnome**, and
`DD04917` is titled *Fighter Kits* and is the **Archer**. Taking `<TITLE>` produced two records named
*Fighter Kits* and lost both real names. The page is regular about it — chunk 0 is the title line,
chunk 1 repeats it, chunk 2 is either the record's heading or the first line of prose — and a heading
is short and does not end a sentence. **Nine of the 28 kits are named this way**, and they were
initially miscounted as chapter apparatus, because a section-titled record and a section page look
identical until you read the fields.

### Finding 15 — recovering the fields does not recover the prerequisites

The Homesteader parses to nine clean fields and **not one of them contains its prerequisite**, which
sits in the descriptive prose above them. Counted across both books, over records that state one:

| | prerequisite in a labelled field | in prose only |
|---|---:|---:|
| CTH | 10 | **0** |
| CBGH | 5 | **17** |

So CBGH differs not only in *how a label is marked* but in **what is a field at all**. Seventeen of
its twenty-eight kits keep the prerequisite outside the field structure entirely.

This bounds the second parser honestly: **it recovers the field layer and stops there.** For those
seventeen the judgement pass must read the page, not the record's fields — which is the same posture
[ticket 04](./04-llm-assisted-extraction.md) already takes, arriving now from a second direction.

The two kits from finding 10 are the specific case: **the Homesteader and the Bandit now come out of
the pipeline**, and their prerequisites still have to be read off the page by a human.

### Session 7 — the Homesteader, which closes the loop

The kit that forced finding 10's schema repair, now **extracted by the pipeline it was invisible to**
and modelled by hand. Its prerequisite is the first real use of the clause repair:

```
Strength ≥ 12  AND  (Intelligence ≥ 12  OR  Wisdom ≥ 12)
```

It validates. The repair works end to end on the record that demanded it.

**Finding 13 is narrower than it looked.** Four of the Homesteader's five effects are choices —
*either short bow or sling*, *either knife or short sword*, *Agriculture or Animal Handling*,
*either Hunting or Fishing* — and **`require` expresses all four**, because they are choices between
**ids**. The Stout's gap is specifically a choice between **effects**: *+1 to either Dexterity or
Constitution* is two `adjust`s, and `require` counts things, not operations. So §4.3 handles the
common case and fails the rarer one, which is a much better position than finding 13 suggested.

One small gap remains: these are **bonus** proficiencies, granted at no slot cost, and nothing in
`require` says a choice is free. Carried as text.

### Finding 16 — a third of kits have no numeric mechanics at all

The Homesteader's `Special Benefits` reads *"is likely to end up a very wealthy halfling, having
earned the respect of the new community"*, and its `Special Hindrances` that *"he or she has to do
just about everything on his or her own"*. **The two fields that exist to carry a kit's mechanical
identity carry none.**

Measured over both books' kits, as fields containing any numeric mechanic — a signed modifier, a
percentage, dice, a saving throw, an armour class:

| | kits | `Special Benefits` | `Special Hindrances` | **neither** |
|---|---:|---:|---:|---:|
| CTH | 18 | 9 | 3 | **8 (44 %)** |
| CBGH | 28 | 18 | 8 | **7 (25 %)** |

**Fifteen of forty-six kits — a third — are mechanically just a prerequisite plus proficiency
choices.** The Homesteader is not an outlier; it is in the *less* empty of the two books.

Two consequences, and both are good news. **The judgement pass is cheaper than
[ticket 11](./11-human-review-protocol.md) priced it** — for a third of kits there is nothing to
model past the fields the parser already isolates. And **a modelled record with a short effect list
is normal rather than suspect**, which is precisely why `effectsModelled` had to be declared rather
than inferred from an empty array: without it, this third of the corpus is indistinguishable from
untranscribed.

The measurement is deliberately conservative — it detects *numeric* mechanics, so a benefit like
*"may use thieves' cant"* would be missed. The Homesteader's own two fields were read directly and
contain nothing mechanical at all.

### Session 8 — the Bandit, twice

Second and last of finding 10's kits: `Charisma ≥ 12 AND (Strength ≥ 13 OR Constitution ≥ 13)`.
Three effects, all `require`. Both kits that broke ticket 06 are now transcribed and validate.

### Finding 17 — the corpus states non-restriction explicitly, which is A3 in the source

`Thieving Skill Emphasis: Any.` `Special Hindrances: None.` *"any secondary skill is acceptable."*

Counted across CTH, CBGH and CPRH — **67 of 1,057 fields consist of nothing but `Any.` or `None.`**,
with 5 more saying it in a sentence. **6.8% of all fields exist solely to say that they do not
restrict.**

The book is not being redundant. It is making exactly the distinction
[A3](../v1-spec/spec.md) was invented for — *"does not restrict"* against *"nobody has said"* — and
it makes it **per field**, by printing a field whose entire content is its own emptiness. A record
whose `Special Hindrances` reads `None.` is complete. A record with no such field is not the same
thing, and the corpus knows it.

This is the first evidence that A3 describes the source rather than only the format. It also settles
why `effectsModelled` had to be a declaration: **a modelled record with no effects is a thing the
corpus produces sixty-seven times**, and an empty array cannot tell you which of the two states it is.

### Finding 18 — the first name collision, and §7.3 was right

The slice now holds **two kits called Bandit**: `cth:DD05809` and `cbgh:DD04924`. They are not the
same kit reprinted. Compared field by field, their `Role` text is **1.4% similar**, they share three
field labels, and their characterisations are opposites — *"vicious characters, desperate, cunning,
and cruel"* against *"Bandits consider themselves equalizers of wealth."*

Nothing had to be decided to absorb this. [Ticket 07](./07-identity-and-id-stability.md)'s
source-position ids separate them without a tiebreaker, and §7.3's *"name is presentation only, never
identity"* is what makes two records with one name unremarkable rather than a conflict. **This is the
first time either rule has been load-bearing** — both were arguments before this record, and now one
of them is holding something up.

The practical warning is for the review page ([ticket 12](./12-how-much-tool.md)): it must never
identify a record to a human by name alone.

### What the two CBGH kits added, and what they did not

Both are **prerequisite plus proficiency choices and nothing else** — finding 16's third of the
corpus, twice. And both hit the same small gap twice more: a `require` whose choice is bounded by a
**category the corpus never enumerates** — *"at least one melee weapon"*, *"any dwarven tongue"* —
so the `from` list is dropped and the constraint survives only as text. Three occurrences now, across
two kinds and two books.

### Session 9 — the Tumbler, and the first record that touches a table

Seven effects, and the densest record the slice holds. It is also **the first record to interact with
the PHB tables**, which is the reason [ticket 08](./08-which-slice-proves-the-format.md) put them in
the slice — they had sat there unexercised through nine sessions.

### Finding 19 — `Special Hindrances` reaches into a table, and the layer model handles it

> their base scores to Open Locks and Detect Noise **begin at 0**, not at the levels (10% and 15%)
> listed on Table 26 … **The bonuses and penalties specified on Tables 27–29 are not affected.**

Two `set` effects, and the book's own scoping sentence is **the layer model stated in prose**: replace
the base, leave the modifiers standing. §4.3's operations commute, so this needs no special handling —
`set` lands on the base layer and Tables 27–29 keep applying above it. **The strongest confirmation
of order-independence the corpus has offered**, because it is the book insisting on it rather than us.

### Finding 20 — a lookupTable has no declared role, and its rows are keyed by prose

This is the finding that matters, and it only became visible because a record finally addressed a
table.

The Tumbler's effect names the field `thiefSkill.openLocks`. The table is `phb:DD01501`, keyed by
`Skill` with a row `["Open Locks", "10%"]`. **Nothing in the pack connects them.**

- **The table declares no role.** `lookupTable` carries `keyedBy`, `columns` and `rows` — and an id
  and a name. The id is source-derived (`DD01501` is a *filename*), so it identifies a passage, not a
  purpose. The name is `Table 26: Thieving Skill Base Scores`, and [§7.3](../v1-spec/spec.md) says a
  name is **presentation only, never identity**. So the Engine has no supported way to find *the*
  thieving-skill base table among a pack's tables.
- **The rows are keyed by book prose.** `"Open Locks"` is a string, not `phb:open-locks`. Even once
  the table is found, matching its rows to the Engine's vocabulary is string-matching English.

This is **v1 known unknown #2 firing** — *"Engine computes, user supplies the tables" has no shipping
precedent* — and this ticket exists partly to give it its first evidence. The evidence is negative in
a specific, fixable way: **supplying a table is not enough if nothing says what it is a table of.**
The manifest's `declares` is A3's rule-set list and does not cover this.

Not repaired here. It is a schema decision of the same weight as finding 10's, and it wants its own
ticket rather than an edit made in passing.

### Finding 21 — two more operand and condition shapes

**Level scaling.** *"+10% to Climb Walls at first level; this bonus increases by +2% per level
thereafter."* **> RETRACTED IN PART by session 22's finding 50: this said ticket 15's set does not
scale by level, and `computedOperand` carries `multiplyBy` — it does. What actually defeats these is
the OFFSET, and only when it sits inside a division. The measurement below stands; the diagnosis did
not.** Measured over **138 kits across nine books**, counting only the
effect-carrying fields: **12 kits (9%)** scale something by level — the Breachgnome's proficiency
slots, the Treetender's languages, the Tunnelrat's sight range, the Urchin's Pick Pockets, the
Samurai, the Witch, the Wu Jen. Nine percent is far above the threshold that misled finding 10, and
the measurement was taken on the record population rather than on raw book text for exactly that
reason.

**Combat-round conditions have no subject.** *"−6 to AC in rounds in which the Tumbler wins
initiative and elects to forgo all other actions."* The predicate names abilities, level, class, race
and membership — **all character state**. This is round state, and there is nothing to name it with.
Carried as text.

### Finding 22 — the field name is not evidence

*"The Tumbler **must** take Jumping, Juggling, and Tightrope Walking"* — in the field called
**Recommended** Nonweapon Proficiencies.

[Finding 8](#finding-8--half-the-fields-produce-no-effects-at-all) read that field as advice, which
it is **25 times out of 27**; the Tumbler and the Cartographer are the exceptions. A rule that holds
93% of the time is worse than one that never holds, because it invites automation and then fails
quietly on the two records that matter. **The sentence is the evidence; the label is a hint.**

Modelled as `require` with `count == len(from)`, which is how the corpus writes a forced selection.

### Session 10 — the Urchin, and the line between what level can and cannot do

Two effects, and they sit on opposite sides of a line worth naming.

### Finding 23 — level-*gated* effects work; level-*scaled* operands do not

> Upon reaching **second level**, the Urchin gains the Disguise proficiency in full.
> … a **+5% bonus per level** to Pick Pockets, starting at second level (+5% at 2nd, +10% at 3rd).

The first is a `grant` with `when: [level ≥ 2]` and it **expresses exactly**. The second is
`5 × (level − 1)` and has no operand. Same subject, same record, one sentence apart.

So the format's level story is precisely half built: **it can say *when* something arrives and not
*how much* of it arrives.** That is a sharper statement of finding 21 than the Tumbler could give,
because the Tumbler's scaling had no gated sibling to contrast with.

**Any repair to [ticket 15](./15-computed-operands.md) must carry a start level.** Re-measured on
finding 21's own population — same books, same fields, same pattern, so the numbers compose:

| | |
|---|---:|
| kits with level scaling | **12 of 138 (9%)** |
| …that declare a starting level | **4 (33%)** |
| …that gloss the series with an example | 3 (25%) |

A bare `perLevel(N)` operand would be **wrong on a third of them**, paying out at first level where
the book starts at second. The corpus is unusually helpful here: a quarter of these spell the series
out — *"+5% at 2nd level, +10% at 3rd level"* — so the intended formula is not in doubt, only
unexpressible.

### Finding 24 — no field name carries force, in either direction

Finding 22 caught the field named *Recommended* stating a requirement. The Urchin gives the mirror
case, in a field with an imperative name:

| | `Weapon Proficiencies` says | read as |
|---|---|---|
| Bandit | *"**should be proficient** in the short bow or hand crossbow"* | requirement |
| Urchin | *"Urchins **tend toward** easily-concealed weapons (dagger, darts, sling, quarterstaff)"* | flavour |

Same field, same book, adjacent pages, opposite force. So the earlier statement was too narrow: it is
not that one field name is unreliable, it is that **the field name never carries force at all**. The
label tells you the subject; only the sentence tells you whether it binds. Every field must be read.

That is a real cost on [ticket 09](./09-extraction-pipeline.md)'s pipeline and a real bound on what a
draft can be trusted to decide — and it is the same shape as finding 1, one level down: structurally
identical, semantically different, no heuristic available.

Two smaller things, both already-known shapes recurring. `Secondary Skills: **Any or none.**` is
finding 17's explicit non-restriction in a new phrasing. And the Urchin's first-level ability to pass
as a human child is a **degraded form of the Disguise proficiency with no id to name it** — carried
as text on the level-2 grant it precedes.

### Session 11 — the Tunnelrat, and the shape of everything the predicate cannot see

Eight effects, and **four of them carry conditions the predicate has no subject for.** Taken
together they stop being separate gaps and become one: the predicate can only name **the character**,
and the corpus routinely conditions on things that are not the character.

| the condition names | example | occurrences |
|---|---|---|
| the **campaign's configuration** | *"if weapon specialization is used in the campaign"* | **13 kits (9%)** |
| the **other party** in an interaction | *"−2 on Reaction Checks made for NPC halflings"* | this record |
| the **combat round's** state | the Tumbler's *"in rounds in which he wins initiative"* | session 9 |
| the DM's **adjudication** | *"at the DM's discretion"*, *"with the DM's permission"* | **14 kits (10%)** |

The last row is the one that should not be repaired. **DM adjudication is not a missing subject; it
is the absence of a rule**, and a format that tried to express it would be claiming to compute
something the book explicitly hands to a person. It is listed here so it stays *deliberately*
unmodelled rather than looking like an oversight.

### Finding 25 — A3 declares rule-sets at the manifest, and effects cannot name them

*"If weapon specialization is used in the campaign, the Tunnelrat must have a melee weapon
specialization by third level."*

That is exactly what [A3](../v1-spec/spec.md) is for — a **declared campaign configuration** — and
§3.4 already calls rule-set names *"the Engine's closed enumeration, the single exception to open
enumerations."* The measurement supports that generosity: across 138 kits the corpus names
essentially **two** optional rule-sets, **encumbrance** and **weapon specialization**.

But `declares` sits on the **manifest** and says which rule-sets the *pack* covers. Here an
**individual effect** is conditional on one, and a `when` clause cannot name a rule-set. So the pack
can say *"I cover weapon specialization"* and cannot say *"this requirement applies only when it is
in use."*

Unlike the other three rows above, this one has an obvious and cheap repair — a condition variant
naming a rule-set from the closed enumeration — and it is left unmade for the same reason as
finding 10: it reopens a settled decision.

**This was already present in the very first record modelled.** The Acrobat's *"under the optional
encumbrance rules, is unencumbered"* was recorded in finding 7 as a **negation** problem, and the
optional-rule half went unnoticed. Two gaps in one clause, one seen.

### Finding 26 — level scaling has three parameters, not one

*"The range of this sight is equal to **10' per level** of the Tunnelrat (**to a maximum of 60'**)."*

Three records now, three parameters:

| record | shape |
|---|---|
| Tumbler | `10 + 2 × level` — a **base** |
| Urchin | `5 × (level − 1)` — an **offset** |
| Tunnelrat | `min(10 × level, 60)` — a **cap** |

A `perLevel(N)` operand would be wrong on all three. Whatever [ticket 15](./15-computed-operands.md)
adopts needs a multiplier, a start level and a ceiling — and the three appeared in the first three
scaling records anyone looked at, which suggests the parameter set is being *observed* rather than
guessed at.

Also here, and worth naming because it is a modelling trap rather than a format gap: the book says
the sight **works exactly like infravision**. That is a comparison, not an identity. Granting
`phb:infravision` would let it stack with a subrace that already grants it — the Stout, in this very
book — so it is granted as a kit-local ability instead.

### Session 12 — the Breachgnome, one thing that works and one that does not

### Finding 27 — the effect list gives conjunction-of-disjunction for free

*"The Breachgnome must have a proficiency in the use of the short sword **and either** the hammer
**or** axe."*

That is `A ∧ (B ∨ C)` — **the exact shape that forced finding 10's schema repair** — and in an effect
it needs no repair at all. Two `require` entries: the array conjoins them, and each `from` list is the
disjunction. The format already had it.

The asymmetry is worth understanding rather than just noting, because it says what the two halves
are. **A predicate entry is a test** — one truth value, so combining tests needs a combinator, which
is why `anyOf` had to be added. **An effect entry is an action with a built-in choice set** — `from`
*is* a disjunction, so the array of effects is already conjunctive normal form.

So finding 10's repair was not a general gap in the format's handling of *or*. It was specific to the
predicate, and the effect side had solved the same problem by a different route before anyone looked.

### Finding 28 — three kits state that a bonus does **not** add, and `adjust` only sums

The book also says the opposite of what §4.3 assumes. Measured across 138 kits:

| | occurrences | kits |
|---|---:|---:|
| the book states an effect **is** cumulative | 10 | 9 |
| the book states an effect is **not** cumulative | **3** | **3** |

The Breachgnome is in the first row — *"cumulative with any applicable size and Dexterity bonuses"* —
and that row is the layer model working for free. The second row is three distinct non-additive
shapes:

- **Assassin** — *"+5% with herbalism… +10% with healing… **These bonuses are not cumulative**."*
  Take the greater, not the sum.
- **Pathfinder** — *"his base chance of getting lost… **will not exceed 20%**. This is not cumulative
  with other benefits."* A **ceiling** on a computed value.
- **Giant Killer** — *"−2 penalty for the infuriating follower. **This effect is not cumulative with
  additional followers**."* Applies once regardless of how many sources supply it.

**§4.3 has `adjust`, which sums, and `set`, which overwrites — and nothing in between.**

One precision that makes this much less alarming than it first reads, and it is the difference
between two properties the design has been treating as one. The layer model's guarantee is
**order-independence**, and `max` is commutative and associative, so a ceiling or a take-the-greater
combiner **preserves it completely**. What these three break is **additivity**, not commutativity. The
gap is a missing *combiner*, not a threat to the model — and at 3 of 138 kits it is small, which is
why it is recorded rather than repaired.

### Finding 29 — "the book states totals, the layer model wants increments" is now a rule

*"−1 to AC if an object is within 3' of either side; if both sides are protected, the bonus is −2."*

Modelled as **two additive −1 adjusts**, and this time **the book confirms the decomposition itself**
by adding that the bonus is cumulative with size and Dexterity bonuses.

Fourth occurrence, across all three Attachable arms: the Acrobat's *"+1, rising to +2"*, Agriculture's
*"+5% for one prime requisite, +10% for both"*, and now this. **`interpretation` has been earned on
every hand-modelled record that states a graded value**, which makes this a property of the corpus's
prose style rather than a series of individual readings — and a candidate for the drafting model to
be told about explicitly.

The Breachgnome also contributes a fourth scaling shape to finding 26: *one slot every two levels,
the first at third* — `floor((level − 1) / 2)`. [Ticket 15](./15-computed-operands.md) **has**
division with rounding down; it is the **offset** that defeats it, since bare `level / 2` pays at
fourth level where the book pays at third and fifth. First scaling record in division form rather
than multiplication.

### Session 13 — the Goblinsticker, and finding 10's count was wrong too

### Correction to finding 10 — four kits, not two

*"A character must have a **Strength or Constitution** score of at least 15 in order to choose this
kit."*

Finding 10 named the Homesteader and the Bandit. The regex that found them required
`<Ab> or <Ab> of at least`, and this sentence puts **`score`** between the ability and the number.
**The third narrow pattern in this ticket to undercount**, after the one that produced finding 10
itself and the one that mismeasured level-scaling offsets in session 10.

Re-measured over the extracted kit records — the right population, prose and fields together, with a
pattern that allows words between the terms:

| kit | shape |
|---|---|
| Goblinsticker | `Str ≥ 15 ∨ Con ≥ 15` — **pure disjunction** |
| Smuggler | `Cha ≥ 13 ∨ Int ≥ 13` — **pure disjunction** |
| Homesteader | `Str ≥ 12 ∧ (Int ≥ 12 ∨ Wis ≥ 12)` |
| Bandit | `Cha ≥ 12 ∧ (Str ≥ 13 ∨ Con ≥ 13)` |

**Four of 138 kits, and all four in CBGH** — the disjunctive prerequisite is one book's house style,
not a corpus-wide habit. The clause repair covers both shapes unchanged; a pure disjunction is a
predicate holding exactly one `anyOf` clause, which is the first time that degenerate form has been
exercised.

The lesson is not about this measurement. **Three times now, a pattern written to find a shape has
found a fraction of it** — and each time the fix was to run against extracted records rather than raw
text, and to allow words between the terms. That is worth carrying to any future measurement in this
map.

### Finding 30 — a required choice whose options no pack contains

*"At first level the Goblinsticker must **declare the hated foe** of his or her life."*

Every other `require` in the slice either enumerates its options in `from` or is bounded by a category
the corpus declines to enumerate — *"any dwarven tongue"*, *"one melee weapon"*. This one is
different in kind: the options are **creature types**, and no v1 pack contains creatures at all. The
Monstrous Manual is [out of scope](../map.md).

So `from` is absent not because the transcription is lazy but because **the referent does not exist in
v1 and will not**. That is a third state beside "enumerated" and "category unenumerated", and it is
the one that cannot be fixed by transcribing more.

### Finding 31 — the book asserts cross-arm stacking

*"This bonus … is cumulative with other gnome benefits, including the **Rock Gnome's** standard +1
attack bonus versus goblins and kobolds."*

A **Kit** effect that the source explicitly stacks with a **Subrace** effect. §4.1's claim is that the
three Attachable arms are one shape; §4.3's is that their effects commute. Here the book itself
composes a kit's bonus with a subrace's and states the result — **the layer model working across
arms, asserted by the source rather than by us.**

It is also a concrete argument for [ticket 10](./10-mechanical-verification.md)'s split: verifying
this sentence requires the Rock Gnome record, which lives in another part of the corpus, so it is
cross-record integrity and belongs to the Engine.

### Session 14 — the Smuggler, and a check that found four more template pages

### Finding 32 — name collisions are systematic, and looking for them found apparatus

Finding 18 recorded one collision (Bandit) and treated it as a curiosity that ticket 07 absorbed.
Measured across nine books, **11 of 138 kits share a name with a kit in another book** — Bandit,
Burglar, Guardian, Patrician, Savage, Smuggler, Squire, Stalker, Swashbuckler, Trader… **8%, not an
oddity.** §7.3's *"name is presentation only, never identity"* is doing real work at that rate.

The useful part was accidental. One collision was **`Kit Subsections`, in CPAH and CRH** — and a
*kit* name appearing in two books is one thing, while a name like that appearing in two books is
another. Both are chapter apparatus. Chasing it turned up **four more template pages in four books**:

| book | page |
|---|---|
| CFH | `Kits and Warriors` |
| CWH | `The Wizard Kits` |
| CPAH, CRH | `Kit Subsections` |

**Twelve apparatus pages in nine books**, where finding 1 found two and finding 9 found a third.
`Creating New Kits` alone appears in **four** books. So the exclusion list was not nearly complete,
and the thing that exposed it was a check written for an unrelated purpose.

It also **corrects finding 9's conclusion.** That finding tested the apparatus detector on two books,
watched it miss both of CTH's, and concluded that each book writes its apparatus differently. Broadened,
the detector finds **5 of the 12 across four books with no false positives** — it fails on CTH and
CBGH, which write their templates as ordinary prose. So it is a genuinely useful **hint generator for
a book nobody has read yet**, which is more than finding 9 credited it with, and still never a gate.

### The exclusion list was keyed wrong, and the same measurement proves it

`EXCLUDE` matched on **name alone**. With 8% of kit names shared across books, that is a live hazard:
an apparatus name in one book could be a real kit in another and would be dropped in silence. Now
keyed by **(book, name)**. No record changed — the bug had not fired yet — which is the only good
time to fix one.

### Finding 33 — `require.count` is a plain integer, and some counts grow

> Smugglers … gain the ability to speak an additional language **every other level, starting at
> second level**.

The growing thing here is a **count**, not a value — and `require` takes `count: integer` where
`adjust` takes an operand that ticket 15 can widen. So a scaling quantity has to be expressed as an
`adjust` on a slot field rather than as the `require` it actually is.

Two useful details. **This particular series is `floor(level / 2)`, which
[ticket 15](./15-computed-operands.md)'s existing division operand covers exactly** — unlike the
Breachgnome's `floor((level − 1) / 2)`. So the obstruction is not the arithmetic; it is *where the
arithmetic is allowed to appear*. And the Smuggler is the **first record in the slice to use a `has`
condition** — *"a Smuggler who possesses the Local History proficiency gains +4 to reaction checks"* —
which means all three condition forms from [ticket 06](./06-expression-language.md) have now been
exercised by real records.

### Finding 34 — scope qualifiers, and the pack has no vocabulary for place

Three of this record's clauses bind an effect to somewhere or something the format cannot name:

- *"Local History **for areas on their regular routes**"*
- *"+4 to reaction checks **in that area**"*
- *"the new language must be one he or she **has had contact with** during the previous two levels"*

The first two are **geography**; the third is **play history**. Both are legitimately outside a
character sheet, and unlike the operand gaps they are not a case of the format being one feature
short — there is no *place* kind in v1 and no record of what happened at the table. Recorded as the
boundary it is rather than as a defect: **an effect can be conditioned on the character, and the
world is not the character.**

### Session 15 — both Burglars, and the field layer turns out to have two levels

*"Model the Burglar"* is ambiguous, which is finding 32 arriving in practice rather than in a table.
**Both were modelled** — `cth:DD05813` and `cbgh:DD04926` — and they coexist with nothing to resolve:
different ids, different targets, different effects, one name. §7.3 held under its first real load.

They are also a study in contrast. The CBGH Burglar carries five effects; the CTH Burglar carries
**one**, its `Special Benefits` and `Special Hindrances` both reading `None.` — finding 16's
mechanically-empty third, now with a modelled example.

### Finding 35 — force is carried one level down, and the extractor was flattening it

The CTH Burglar's proficiencies read:

> **Required:** Alertness, Looting. **Recommended:** Begging, Gather Intelligence, Jumping…

Both sit **inside** the single `Nonweapon Proficiencies` field. Measured across nine books, as kits
having at least one sub-label nested in a field:

| book | kits | with a sub-label |
|---|---:|---:|
| CTH, CWH | 18, 10 | **100 %** |
| CFH | 14 | 93 % |
| CPAH | 12 | 58 % |
| CRH | 14 | 36 % |
| CBD, CDH | 24, 14 | 21 % |
| **CBGH** | 28 | **0 %** |

And the sub-label vocabulary is exactly the force vocabulary: **`Recommended` 46, `Required` 32,
`Bonus Proficiencies` 14.**

**This substantially revises findings 22 and 24.** Those concluded that no field name carries force
and every sentence must be read. That is true of the **top-level** name — and in eight of nine books
the force *is* marked, one level down, in a small closed vocabulary. The extractor has been
**discarding the exact signal it needed**, and CBGH is the outlier that hid it: the one book with no
sub-labels is the book whose fields were read most closely.

The structure of the fix is the pleasing part. **The two strategies are not per-book alternatives;
they are per-level.** `Required:` and `Recommended:` are **plain text inside a marked-up field** — so
the markup books use markup at level one and *typography* at level two, and
[`fields_typographic`](../tools/extract.py) is already the right tool for it. CBGH is simply
typographic at both levels, and flat.

Not implemented here. It changes what every extracted record contains, and the slice is
[the gold standard](#keep-the-slice--it-becomes-the-gold-standard) — re-cutting it belongs in its own
pass with the diff adjudicated, not appended to a modelling session.

A smaller thing found while checking: some sub-labels *are* marked up, **malformed** —
`<I>Skill Progression: The</I>` swallows the following word, so the label regex never sees it. It
costs about **1%** of label candidates and most of those were never labels (`At 8th level`,
`Chapter 8` are cross-references). Perhaps half a dozen genuine field labels are lost corpus-wide.
Small, and recorded as small.

### Finding 36 — the cancelling-adjust technique generalises

> …suffering a **−1 penalty on all attack rolls in melee**. However, attacks made as a **backstab are
> not affected** by this penalty.

Modelled as **−1 on melee attacks and +1 on backstabs**, which nets to zero for a backstab exactly as
written. This is the same move the Acrobat's halfling/gnome carve-out needed, and it works for the
same reason: §4.3's adjusts commute, so an exemption is a cancelling adjust rather than a special
form.

Two independent carve-outs, two arms, one technique. **`except` was never needed for either** — it
pierces prohibitions, and neither of these is a prohibition. That is worth stating positively, since
most of this ticket's recent findings have been gaps: **the operation set is doing real work here that
a naive design would have needed a seventh operation for.**

The CBGH Burglar's benefit is the counterweight, and the sharpest single sentence in the slice:
*"At first level the Burglar gains an additional 10% to Open Locks **or** Move Silently. Each time he
or she gains a new level, the Burglar receives a **+5% bonus that can be applied to either of the
above two talents or to Find and Remove Traps**."* That is finding 13's choice-between-effects and
finding 21's level scaling **in one clause, recurring at every level, allocated by the player**.
Nothing in the format touches it.

### Session 16 — both Squires, and the field vocabulary is not closed

### Finding 37 — two books have a shared spine and a per-*kit* tail

The map's charting notes say *"the label vocabulary is per book family."* That holds for seven of the
nine kit books and **fails completely for two.**

| book | kits | distinct labels | used by exactly **one** kit | fields per kit |
|---|---:|---:|---:|---:|
| CRH | 14 | **99** | **74** | 15–25 |
| CPAH | 12 | 46 | 29 | 7–19 |

Both keep a spine — `Description`, `Requirements`, `Role`, `Secondary Skills`, the proficiency
fields, `Armor/Equipment`. Past that the labels are **the names of the effects themselves**:
`Animal Telepathy`, `Empathic Shock`, `No Fortress`, `Unruly Allies`, `Sanctuary`, `Extra Tithes`,
`Punishment Buffer`. In CRH, **three quarters of all labels appear once in the book.**

**This invalidates finding 16's measurement outside the books it was taken from.** That finding
counted numeric mechanics in `Special Benefits` and `Special Hindrances`. In CPAH those fields are
**empty on 10 of 12 kits**, and in CRH on **13 of 14** — not because the kits have no benefits, but
because the benefits have their own names. Run the same count there and it would report the opposite
of the truth. Finding 16's number stands for CTH and CBGH, where it was taken, **and must not be
extended.**

The consequence for the pipeline is mixed and mostly good. The extractor never needed an allow-list —
it reads whatever labels a page carries — so **nothing breaks**. And a per-kit label is *information*:
`Empathic Shock` names its effect where `Special Hindrances` merely locates it, which is more for the
judgement pass to work with, not less. What it does refute is any plan that keys behaviour off a
fixed set of field names, and any measurement that assumes one.

### Finding 38 — the partner: a fifth subject the predicate cannot name

The halfling Squire is bound to another **player character**:

- *"+2 to NPC Reaction Rolls made for members of **that race**"* — the partner's race.
- *"can reach two levels higher … however, he or she **cannot pass the level of his or her partner**."*
- *"when the halfling goes up in level and at least **half the experience** came from tasks performed
  **with his or her partner**, he can add one proficiency slot, filling it with a proficiency **his or
  her partner holds**."*

Not a transient opponent and not an NPC reaction — a **persistent binding to a second character
sheet**, plus a condition on *how experience was earned*. It joins campaign configuration, the other
party, combat-round state and DM adjudication on the list of things a character-only predicate cannot
see.

The level-limit bonus itself is a genuine `adjust` and worth noting as a positive: 2e racial level
limits are a value the Engine holds, so `+2` lands cleanly. It is only the ceiling that escapes — and
even a *numeric* ceiling would escape, per finding 28.

### What the paladin Squire contributes

Two effects, and the kit is essentially a **social position**: a stipend, a mount, a chain of
obedience. Its `Requirements` field says *"there are no rigid requirements"*, so its prerequisite is
empty **by the book** rather than by omission — which is finding 17's explicit non-restriction
reaching the prerequisite rather than an effect.

It also contributes two shapes nothing else has:

- **A range that is not dice.** *"typically 5–10 gp per month."*
  [Ticket 15](./15-computed-operands.md) settled dice as a string with a pattern; this is neither an
  integer nor dice notation.
- **A weighting rather than a rule.** Mounts: unicorns and pegasi *"rarely"*, medium and light war
  horses *"most likely"*. Not a permit-list, not a prohibition — a **likelihood**, which no operation
  in §4.3 expresses and which arguably should not be modelled at all.

### Session 17 — both Stalkers, and the first evidence *against* growing the operation set

### Finding 39 — after 17 records, five of six operations are used and `except` is not

Every finding since session 9 has been a gap. This one runs the other way. Across the **17
hand-modelled records and their 106 effects**:

| operation | uses |
|---|---:|
| `grant` | 34 |
| `adjust` | 32 |
| `require` | 24 |
| `set` | 13 |
| `forbid` | **3** — all three first used here |
| **`except`** | **0** |

`forbid` waited seventeen records for the gnome Stalker's *"cannot use a shield or wear any type of
metal armor"*. **`except` has still never been needed** — and the two places it looked inevitable were
both better served without it:

- the Acrobat's *"halflings and gnomes do not gain the jumping bonus"*,
- the Burglar's *"attacks made as a backstab are not affected by this penalty"*,

both modelled as **cancelling adjusts** (finding 36). And the gnome Stalker supplies the third case
from the other direction: *"not even elven chain"* is the book **closing** the usual exception to a
metal-armour ban in advance, which a plain `forbid` covers exactly.

So the running verdict on **known unknown #4** — *six operations may not suffice* — is more
interesting than either "yes" or "no". The corpus wants **richer operands and richer conditions**, and
it has not once wanted a seventh *operation*. If anything the set is one too large: **`except` is a
candidate for removal**, and every remaining session should be watched for a case that needs it.

### Finding 40 — VOID. Retracted by finding 50

> *"use half the Intelligence score, **rounded up**"* — CRH Stalker, and again in the Feralan.

**This finding was wrong and is withdrawn in full.** It claimed the operand hard-codes rounding
down. `computedOperand.round` is `{"enum": ["down", "up"]}` — the direction was **already** a
parameter, and had been since ticket 15 was implemented. The measurement it rested on (rounding up in
2 kits, down in 1) is accurate and simply has no consequence.

### Finding 41 — a kit whose target is a multiclass

The gnome Stalker's `<TITLE>` reads `Stalker`; the page's own heading reads **`Stalker
(Fighter/Thief)`**. §4.1 says a kit's target is a class entry or the race, and §6.1 makes a character's
classes a **sum type** — so this is the first record whose target is that sum rather than a member of
it. Nothing in the schema resisted, since `target` is an id; what is untested is whether the Engine
can attach to a multiclass entry at all.

It is also the third place a record's real identity was **outside `<TITLE>`**, after session 6's
section-titled kits and the CTH Burglar's prerequisite living in its `Description`.

### What the ranger Stalker showed

The extreme of finding 37, and a **third structural level**: 18 fields, `Special Benefits` empty, the
benefits carried by `Tracking`, `Stealth Abilities`, `Interrogation` and `Photographic Memory` — each
a named field containing **bulleted sub-items**. Field → sub-label → bullet.

Two of its clauses land cleanly and are worth recording as wins: *"hide in shadows … when wearing
armor of AC 6 or less"* is a plain `compare` on character state, and the 10th-level photographic
memory is a level-gated `grant`, which finding 23 established works. The rest is terrain — and in this
book **terrain is a spine field** (`Primary Terrain`), so finding 34's geography gap is not incidental
here: it is the axis the kit is built on.

### Session 18 — both Traders, and a decision that never reached the artifact

### Finding 42 — ticket 15's dice decision was settled and never implemented; **now it is**

The dwarf Trader starts with **`4d4x10 gp`**. Checking how the schema validates that turned up the
real answer: **the string `dice` does not appear in `pack-0.1.schema.json` at all.**
[Ticket 15](./15-computed-operands.md)'s decision 1 — *dice notation is a string with a validated
pattern* — was decided, argued at length, and never written down in the artifact it was about.

That is a different failure from the ones this ticket keeps finding. Everywhere else the schema was
**wrong**; here it was **silent**, and silence does not fail a validation, so nothing caught it. The
tell was a value the schema had no opinion about.

Implemented as a `$defs/dice` value type — §3.3, no id, never referenced by `record`. It is **not yet
referenced by any kind**, because the kinds that carry dice are outside the slice, and that is stated
in the definition rather than left to be discovered.

**And implementing it corrected the decision.** Ticket 15 states the grammar as **`NdM±k`**. Measured
over the v1 RTF, 2,574 notations:

| form | count | share |
|---|---:|---:|
| `NdM` | 2,006 | 77.9 % |
| `NdM±k` | 447 | 17.4 % |
| **`NdM×k`** | **121** | **4.7 %** |

`4d4x10`, `3d6x5`, `6d4x10`. **A pattern faithful to the stated grammar would reject 121 real corpus
values.** The multiplier is admitted, as is a bare `d10` with no count, for the same reason: the
corpus writes it.

### Finding 43 — two thirds of kits give **examples**, not enumerations

> …a concealable hand weapon **such as** a dagger, knife, or hand axe.

Three options are named and the set is explicitly open. That is neither of the two states finding 30
identified — it is a third: **the corpus enumerates and then declares the enumeration incomplete.**

Measured across 134 kits: **91 (68 %) contain `such as`, `for example`, `e.g.` or `etc.`**

That number reframes `require.from` badly. A `from` list is a **closed** set of options, and in two
thirds of kits the book has signalled that its lists are illustrative. Every `from` written by a
transcriber against an exemplary list is a **false precision** — it will refuse a legal character
choice, and refuse it *confidently*, which is the failure mode this Engine exists to avoid. Recorded
without a repair: the fix is probably a flag on the list rather than a new operation, but that is a
schema decision.

The same record supplies the **inverse of the permit-list problem**, and it is worth putting beside
finding 11. The ranger Stalker's *"limited to blowgun, dagger, dart…"* cannot be said. The dwarf
Trader's *"…and otherwise, they may be proficient in any weapon they choose"* is **said exactly, with
two `require`s and no prohibition at all.** The format handles required-subset-then-free perfectly and
only-these not at all.

### Finding 44 — an effect that a play event rewrites

> A Trader gains a **+1 reaction bonus** from merchants and other traders… **If he cheats on a deal
> and is later discovered, the bonus changes to a −2 penalty.**

Not a condition on character state, and not the DM adjudicating a case. The effect's **own value is
rewritten permanently by something that happened at the table**. Finding 34 recorded play history as a
condition the pack cannot name; this is play history as a **mutation of the record's own content**,
which is further out still.

Worth stating plainly: this one probably should not be modelled even if it could be. A character
sheet that silently flips a bonus because of a past betrayal is a **rule the table remembers**, not a
computation — and the layer model's promise is that a refusal can name its cause.

### Session 19 — the Cartographer, and the slice does not resolve

### Finding 45 — **every** reference in the proving slice is dangling

Counting the ids the slice's effects and prerequisites point at:

| | |
|---|---:|
| records defined | 53 |
| distinct ids referenced | **80** |
| **references that resolve inside the pack** | **0** |

Not one. `phb:` 66, `cbgh:` 8, `crh:` 5, `cprh:` 1.

Two different problems wearing one number. The 66 `phb:` references are **another pack** —
`phb:halfling`, `phb:short-sword`, `phb:tumbling` — and [ticket 10](./10-mechanical-verification.md)
correctly put cross-pack integrity on the Engine, because a pack cannot see the packs it points at.
The PHB has simply never been transcribed; the slice contains five of its tables and none of its
races, classes or proficiencies. The other 14 are ids **minted while modelling** for kit-local
abilities — `cbgh:freeze-in-place`, `crh:photographic-memory` — which no record anywhere defines.

**The slice validates with 0 schema errors and could not be loaded.** That gap is exactly the shape
[ticket 08](./08-which-slice-proves-the-format.md) warned about when it said *"do not let a passed
slice be reported as a validated format"*, and it went unnoticed for nineteen sessions because
**nothing was counting**. The checker reported schema errors and unmodelled records and said nothing
about references.

Now it does — [`validate.py`](../tools/validate.py) reports them grouped by prefix, **without
failing**, because a pack pointing outward is normal and a pack *all* of whose references point
outward is worth knowing about. That is implementing ticket 10's split, not revising it.

What this costs the ticket is honesty about what has been proved. The slice demonstrates that the
**shapes** are expressible. It demonstrates nothing yet about whether the pieces **fit together**, and
the two are easy to confuse when the validator is green.

### Finding 46 — a kit that invents a proficiency

> Upon reaching second level, halflings using this kit gain a **unique proficiency: Cartography**.

The kit does not grant an existing thing; it **defines a new one**, in its own prose, and the
definition is three sentences of what the proficiency can and cannot do. The schema has a top-level
`nonweaponProficiencies` kind, so there is somewhere for it to live — but the pack must now **produce
a record the source never presents as a record**, extracted from the middle of a kit's field.

That is a boundary case for [ticket 07](./07-identity-and-id-stability.md)'s source-position identity:
what is the anchor of a proficiency that has no page? The best available answer is the kit's page, and
then two records share it — which is finding 14's embedded-table collision arriving from a completely
different direction.

The record also lands two clean wins worth noting against the run of gaps: *"Kender Cartographers
specialise in the hoopak"* is a **subrace-conditioned effect** that models exactly, and the
second-level Cartography grant is another level gate, which finding 23 established works.

### Session 20 — the Mercenary, and the predicate/effect asymmetry appears a second time

### Finding 47 — the corpus counts, and only the effect side can count back

> A halfling Mercenary must have ability scores of at least 13 in **two of these categories**:
> Strength, Dexterity, and Constitution.

Measured across 134 kits, **7 occurrences of `k of n`** — and they split cleanly:

| use | kits | expressible? |
|---|---:|---|
| choosing **k things** — *"may choose two of the following skills"*, *"up to four of the weapons"* | 6 | **yes**, `require count: k, from: [...]` |
| a **predicate** counting how many conditions hold | **1** | **no** |

This is **finding 27's asymmetry, exactly repeated one level up**. There, disjunction came free on
the effect side because `from` *is* a choice set, and had to be added to the predicate as `anyOf`.
Here, counting comes free on the effect side because `require` *has* a `count`, and the predicate has
nothing. Two different capabilities, one structural cause: **an effect entry is an action that
already carries a selection; a predicate entry is a single truth value.**

**Modelled by expanding to conjunctive normal form** — `(S∨D) ∧ (S∨C) ∧ (D∨C)` — which is exactly
equivalent and which the clause repair accepts unchanged, verified. But it is a **derivation, not a
transcription**: the record no longer resembles the sentence, and a reviewer comparing the two must
reconstruct the algebra. That is precisely what `interpretation` exists to flag, and it is flagged.

It is affordable only because the corpus asks once, for two-of-three. In general `k of n` needs
**C(n, n−k+1)** clauses — three here, twenty for three-of-six — so the CNF escape is a property of
this instance rather than a general answer.

### Finding 48 — an earmarked grant

> the halfling Mercenary receives **two extra weapon proficiency slots** upon reaching second level.
> **The slots must be used to purchase a weapon specialization**, if he or she does not already have
> one.

The `adjust` lands cleanly — slots are a value, +2 at second level is a level-gated adjust, both
established. What has no form is the **earmark**: the resource is granted *and its spending is
constrained*. `require` says the player must choose something; it does not say that a *previously
granted* resource is the thing that must pay for it.

The escape clause is finding 7's negation again — *"if he or she does not already have one"* — and
worth noting because it shows how that gap compounds: the earmark cannot be expressed, and neither
can the condition under which it lapses.

The same field also modifies a **price**: *"can specialize in the bow at a cost of only two
proficiency slots (not three)"*. Carried as a `set` on a cost field, which works only if the Engine
holds costs as fields — an assumption this record makes and no ticket has confirmed.

### Session 21 — the Sheriff, and "the other party" is the corpus's second condition

### Finding 49 — a third of all kits adjust a reaction roll, and most qualify it

The Sheriff and the Mercenary are the same sentence with the sign reversed, four pages apart:

| | |
|---|---|
| **Sheriff** | *"+2 on all NPC Reaction Rolls made for **halfling NPCs** (as long as the other character **knows of the Sheriff's rank**)"* |
| **Mercenary** | *"a **−2 penalty** on Reaction Rolls"* — from halflings, and only those who know what he does for a living |

That symmetry prompted the measurement, and it is much larger than one book:

| | kits | |
|---|---:|---|
| adjust a reaction roll | **48 of 134** | **36 %** |
| …qualified by **the other party** | **27** | who they are, or what race |
| …requiring the other party to **know** something | 6 | the Assassin, the CTH Bandit, the Beggar, the Berserker, the Cavalier, the Sheriff |

[Finding 25](#finding-25--a3-declares-rule-sets-at-the-manifest-and-effects-cannot-name-them) listed
*"the other party"* as one row of a table, evidenced by a single record. It is not a row. **The
reaction roll is a mechanic in over a third of the corpus's kits, and it is relational by
construction** — *how does this NPC react to you* has no meaning without the NPC. A character-only
predicate cannot condition it, and 27 kits do.

This changes the shape of that gap. Campaign configuration wants a small closed enumeration; the
combat round wants transient state; DM adjudication should never be modelled. **The other party wants
a second subject** — which is a larger thing to add than any of the operand repairs this ticket has
proposed, and is now the best-evidenced of them.

The rest of the Sheriff is finding 16 again: a `require` of three named weapons, and `Special
Hindrances` describing long hours and rude interruptions.

### Session 22 — the Vanisher, and a correction that runs back through four sessions

### Finding 50 — the operand was never as poor as findings 21, 26 and 40 said

Modelling the Vanisher's *"the duration of the spell is **doubled**"* meant checking whether
multiplication existed. It does. `$defs/computedOperand` reads:

```json
{ "of": <scalar>, "divideBy": ≥2, "multiplyBy": ≥2, "round": ["down","up"] }
```

**`multiplyBy` was always there, and `round` was always a parameter.** So:

- **Finding 40 is void in full.** It claimed the operand hard-codes rounding down and that direction
  must become a parameter. It already was one.
- **Findings 21 and 26 are wrong in their diagnosis.** Scaling by level *is* expressible —
  `{of: {level: …}, multiplyBy: 10}`. Their measurements stand; the cause they assigned does not.

**The root cause is the mirror of [finding 42](#finding-42--ticket-15s-dice-decision-was-settled-and-never-implemented-now-it-is).**
There, a decision existed and the artifact was silent. Here, the artifact was **richer than the
prose** — ticket 15's text says *"`half(<scalar>)`, `<scalar>/N rounded down`"* and the schema it
produced is more general than its own description. Both times the mistake was **reading the ticket
instead of the artifact.**

Four records carried wrong `UNMODELLED` markers and are corrected in the slice:

| record | rule | now |
|---|---|---|
| Smuggler | one language every other level from 2nd | `divideBy: 2, round: down` — **exact** |
| Tunnelrat | 10 ft per level, max 60 | `multiplyBy: 10`; **only the cap is unmodelled** |
| Tumbler | +10% at 1st, +2% per level after | **`+8` and `2 × level`** |
| Urchin | +5% per level from 2nd | **`−5` and `5 × level`** |

The last two are the interesting repair, and it is the layer model earning its keep for a fifth time:
**an additive offset does not need an operand, because a constant `adjust` beside the multiple
produces it.** `8 + 2L` is 10 at first level; `−5 + 5L` is 0 at first, 5 at second, 10 at third.

That also sharpens what is genuinely missing, from "offsets" to something much narrower. **An offset
*inside a division* cannot be absorbed that way, because `floor` is not linear** — the Breachgnome's
*one slot every two levels, the first at third* is `floor((L−1)/2)`, and no constant beside
`floor(L/2)` reproduces it. **One record in the slice, not four.** Ticket 15's operand set is in far
better shape than four sessions of this ticket claimed.

### Finding 51 — an effect whose subject is a spell, or the creature you cast it at

The Vanisher's benefits do not land on the Vanisher:

- *"The **duration of the spell** is doubled."* — the subject is a spell being cast.
- *"**Saving throws** against the effects of these spells suffer a −2 penalty."* — the target's roll.
- *"Characters or creatures that could normally see invisible objects **must make a successful
  Intelligence check, with a −4 penalty**."* — an observer's roll.

[Finding 49](#finding-49--a-third-of-all-kits-adjust-a-reaction-roll-and-most-qualify-it) measured
27 kits whose effect is **conditioned by** another party. This is a step further: the effect **lands
on** another party. Measured across 134 kits, **10 (7%)** do it — the Imagemaker, the Anagakok, the
Savage Wizard among them.

Smaller than the reaction case and harder, because a second subject in a *condition* is a lookup while
a second subject in an *effect* means the pack can modify a creature it has never seen. Worth stating
as a boundary the format may simply decline: an Attachable that edits other creatures is not an
Attachable any more.

The record is also the **second and third use of `forbid`** — *"precludes learning any spells from the
greater divination or conjuring/summoning schools"* — and the first on a genuinely closed category,
since 2e's schools of magic are enumerable. `except` remains unused at 23 records.

### Session 23 — the Imagemaker, and `except` finds its one case

### Finding 52 — `except` is needed exactly once in 134 kits

> The Imagemaker gains the Ventriloquism proficiency **(normally available only to rogues)** as a free
> proficiency as soon as this kit is selected.

That is a standing prohibition being **pierced by naming its subject**, which is precisely what §4.3's
`except` is specified to do — and without it the `grant` beside it would be refused by the rogue-only
rule. **First use in 24 modelled records**, after [finding 39](#finding-39--after-17-records-five-of-six-operations-are-used-and-except-is-not)
proposed it for removal.

Measured, so the answer is a number rather than a reprieve: searching all 134 kits for a clause that
grants something the rules otherwise deny returns **three matches, of which two are false** — the
Bounty Hunter's *"despite the fact that…"* and the CPAH Squire's *"Despite their reputation…"* are
narrative. **One real case.**

> **Corrected in session 24 to four.** That pattern required *"normally available only"*, *"otherwise
> forbidden"*, *"despite"* or *"even though"*, and the corpus also writes *"**Unlike most priests**,
> Treetenders can use bows"*. Re-measured: **4 kits of 134 (3%)** — the Imagemaker, the Treetender,
> the Champion (twice) and the Vermin Slayer. **The fourth undercount in this ticket caused by a
> pattern narrower than the corpus.** The conclusion below is unchanged in kind and wrong by 4× in
> degree.

So finding 39's proposal is withdrawn and replaced by something sharper. `except` is not dead weight;
it is **the operation the corpus needs least often**, at roughly 1 in 134 against `grant`'s 44 uses in
24 records. Keeping it is right — a format that could not express *"you may take the rogue-only
proficiency"* would fail on a real kit — but the running tally is now:

| | uses in 24 records |
|---|---:|
| `adjust` | 45 |
| `grant` | 44 |
| `require` | 34 |
| `set` | 16 |
| `forbid` | 7 |
| `except` | **1**, and 4 cases exist |

**All six operations are now exercised, and none has been found missing.** Known unknown #4 asked
whether six operations suffice. On the evidence of 24 hand-modelled records across seven books, the
answer is **yes, and the shortfall was never in the operations** — it is in operands, conditions, and
above all in subjects.

The same record supplies the **third cancelling-adjust carve-out**, and the contrast with the `except`
above it is the useful part. *"Creatures that would not normally be fooled by illusions do not suffer
the −2 penalty"* exempts from a **penalty**, not from a **prohibition** — so it takes the Acrobat's
form, not `except`'s. Two exception-shaped sentences in one record, needing two different mechanisms,
and the difference is exactly whether the thing being escaped is a prohibition.

### Finding 53 — the corpus writes randomness in a second notation

*"Illusions requiring concentration last for **2–12 rounds** after the caster ceases concentrating."*
Also *"a fuse set for 1–10 rounds"*, *"blind for 1–6 rounds"*, *"5–10 gp per month"* (the CPAH
Squire).

Measured: **18 kits, 36 occurrences** of a bare `N–M <unit>` range. Forms include `1-4`, `1-6`,
`1-10`, `2-12`, `1-12`, `6-12`, `30-50`, `6-7`.

The dice pattern implemented in [finding 42](#finding-42--ticket-15s-dice-decision-was-settled-and-never-implemented-now-it-is)
rejects every one of them. Some are dice wearing a different coat — `1-6` is `1d6`, `2-12` is `2d6`,
`1-10` is `1d10` — and **converting them is a derivation, not a transcription**, the same class of
move as the Mercenary's CNF expansion. Others are not dice at all: **`30-50` and `6-7` have no die
that produces them**, so they are a range a person picks or a DM rules within.

So ticket 15's notation question was answered for one of the two notations the corpus actually uses.

### Session 24 — the Treetender, and a fourth narrow pattern

### Finding 54 — `except` has four cases, not one, and one of them carries a cost

> **Unlike most priests**, Treetenders can use bows.

2e bars priests from edged and missile weapons; this names that restriction and pierces it. Which
means [finding 52](#finding-52--except-is-needed-exactly-once-in-134-kits)'s *"one real case"* was
measured with a pattern that did not include the phrasing the corpus actually used. Re-measured:

| kit | clause |
|---|---|
| Imagemaker | Ventriloquism, *"normally available only to rogues"* |
| **Treetender** | *"Unlike most priests…can use bows"* |
| Champion (CBD) | *"Unlike other warrior/priests, he may specialize"* — and *"this is an exception to the restriction against multi-class characters"* |
| Vermin Slayer (CBD) | *"metal armor may be worn, **even though this causes their thief skills to be negated**"* |

**4 kits of 134 (3%), five clauses.** Finding 52's conclusion — `except` earns its place and is the
least-used operation — survives; its number was wrong by 4×.

**This is the fourth time in this ticket that a pattern has undercounted the corpus**, after
finding 10's disjunction, session 10's scaling offsets, and finding 32's `score` between the ability
and the number. The pattern is now itself a finding: *every* count in this ticket taken from a regex
over prose should be read as **a lower bound**, and the recurring cause is the same — English says
the same thing several ways and the first phrasing found becomes the pattern.

The Vermin Slayer's case adds a shape `except` does not have. *"Metal armor may be worn, even though
this causes their thief skills to be negated"* is a **pierce that carries a cost**: the prohibition
is lifted and a penalty arrives with it. Two effects under the layer model — an `except` and an
`adjust` — but the book presents them as one clause with a causal link, and nothing records that the
second is the price of the first.

### What the Treetender modelled cleanly

Worth noting because [finding 50](#finding-50--the-operand-was-never-as-poor-as-findings-21-26-and-40-said)
has just corrected the record on operands: *"one additional woodland language per level, starting at
third"* models as **`−2` beside the bare level scalar**, giving one at 3rd and two at 4th. Note
`multiplyBy` has `minimum: 2`, so *"one per level"* is not a multiple at all — it is
`{of: {level: …}}` on its own, which the operand permits. The offset is absorbed by a constant, as
finding 50 established.

Its `Special Hindrances` supply the counterweight and it is finding 11's oldest gap, unchanged since
Agriculture: *"weapons where **more than 10% of the weight is metal**"* is a predicate over item
properties. There is nothing to enumerate and no operation that takes a property test.

### Session 25 — the Rocktender, and a condition with nowhere to live

### Finding 55 — a gate on the whole record, which the schema has no slot for

> A Rocktender can **only cast spells or use one of his or her special abilities** when he or she is
> in **direct contact with unhewn rock**.

That is not a condition on an effect. It is a switch on **every effect the record has**, and the
schema has two places for conditions and neither is it:

| | asks |
|---|---|
| `prerequisite` | may this character **take** the kit? |
| `when` on an effect | does **this effect** apply? |
| **missing** | does **any of this** apply right now? |

Repeating the condition as a `when` on all six effects is the obvious workaround and it fails anyway,
because the condition names the **environment**. Contact with unhewn rock is not a fact about the
character — it is finding 34's geography and finding 49's second subject arriving in a third guise.

Measured, and read as a **lower bound** per the map's new method note: **2 kits of 134**. The other is
the Cavalier, and the pairing is the useful part — the Rocktender's gate is **environmental**, the
Cavalier's is a **play event** (*"loses all his special benefits until such time as he repents"*).
Same structural need, two unrelated causes, which suggests the shape is real even though the count is
tiny.

### Finding 56 — the book bounds the DM rather than deferring to him

> …the DM can allow **up to a 1 in 10 chance of failure, but no greater**.

[Finding 25](#finding-25--a3-declares-rule-sets-at-the-manifest-and-effects-cannot-name-them) sorted
DM adjudication into the row that *should not be modelled* — 14 kits saying *"at the DM's
discretion"*, which is the absence of a rule and correctly left alone.

This is different. The book hands the DM a decision **and then fences it**: any failure chance he
likes, provided it does not exceed 10%. That is a **rule about discretion**, and it is exactly the
kind of thing the Engine could enforce — a ceiling on a DM-set value — while still leaving the value
to him.

One occurrence, so it changes nothing on its own. Recorded because it means finding 25's fourth row
is not homogeneous: *"the DM decides"* and *"the DM decides, within this bound"* are different
statements, and only the first is unmodellable in principle.

### What else the record carried

All of it already known, which is itself the useful signal — **this is the first record in five
sessions to produce no new gap in its effects**, only new instances of old ones:

- *"will never use metallic weapons"* and *"will only bear weapons of stone"* — finding 11's permit-list
  and an item-property predicate, on one axis, in one record.
- *"1–12 hours of warning… 1–6 hours"* — finding 53's bare ranges, twice.
- *"always gets the largest possible type"* of summoned earth elemental — finding 51's second subject,
  and a value that is the maximum of a distribution rather than a number.

### Session 26 — the Buffoon, and two multiclass kits named by two different rules

### Finding 57 — the extractor named one kit from `<TITLE>` and another from the heading; **fixed**

The Buffoon came out as **`Buffoon (Thief/Illusionist)`** and the Stalker as **`Stalker`** — from the
same book, both multiclass kits, both with the qualifier printed on the page.

The cause is that the two naming rules never met. The Buffoon's page is **section-titled**
(`Multiclass Kits`), so [session 6](#session-6--the-second-parser-and-what-it-cannot-reach)'s
`heading_name` took chunk 2 and got the full name. The Stalker's page is **record-titled**, so
`heading_name` declined and `<TITLE>` won — and `<TITLE>` says `Stalker` where the printed heading
says `Stalker (Fighter/Thief)`. **The qualifier survived by accident in one record and was discarded
by accident in the other.**

Fixed: when the page is record-titled, the heading is preferred if it **extends** the title. It fires
on exactly one record corpus-wide, cannot rename anything (only lengthen), and the slice's Stalker is
renamed to match. That is the third naming rule in one book — `<TITLE>`, section-titled, and now
title-extending — which is the price of finding 12's typographic parser and worth restating: **CBGH
needed a second field parser and it also needs its own naming rules.**

### Finding 58 — an ability granted by citing another class

> This has the **same effect as the bard's ability to influence reactions** (explained on **page 42**
> of the Player's Handbook). Second, he or she can counter the effects of songs and poetry used as
> magical attacks (see **page 44**).

Two of the Buffoon's three benefits are **defined by pointing at another class**. The pack grants an
id it does not define and *cannot* define, because the referent is a paragraph in a book rather than a
record — and the citation is **by page number**, which
[ticket 01](./01-what-the-source-yields.md) established the corpus otherwise does not have.
[Ticket 05](./05-pack-schema.md) replaced page citation with a section chain and an anchor for exactly
that reason; here the *content* cites a page, which is a different thing and outside that decision.

This is the friendliest form of [finding 45](#finding-45--every-reference-in-the-proving-slice-is-dangling)'s
dangling reference: `phb:bard-influence-reactions` is not a transcription gap that more work closes —
it is the corpus **deliberately** reusing a rule by naming it. If the PHB pack ever exists with that
ability as a record, the reference resolves and nothing else changes. That is the cross-pack model
working as designed, and it is the first record where a dangling reference is the *right answer*
rather than a debt.

The remaining benefit stacks three unmodellable conditions in one sentence — the Buffoon must **share
a language** with the spellcaster, must **win initiative**, and the *caster* then rolls a Wisdom check
which the DM may optionally modify by **the difference between the two characters' levels**. Other
party, round state, a roll made by someone else, arithmetic across two subjects, and an optional rule,
in one clause. Nothing new, but it is the densest single example of finding 49's second subject the
slice has produced.

### Session 27 — the Mouseburglar, and the earmark turns out to be a class of effect

### Finding 59 — six kits constrain the spending of a resource they do not grant

> At first level he or she must **divide the 60 discretionary points** between these three abilities.

[Finding 48](#finding-48--an-earmarked-grant) recorded the Mercenary's *"the slots must be used to
purchase a weapon specialization"* as an **earmarked grant** — one record, one shape. It is not one
record, and the framing was slightly wrong.

The 60 discretionary points are the **thief class's** resource, not this kit's. The kit grants
nothing here; it **constrains where an existing resource may go**. Measured:

| | kits |
|---|---:|
| constrain the spending of discretionary points | **5** — Assassin, CTH Burglar, Mouseburglar, Tumbler, Stalker |
| constrain the spending of granted proficiency slots | 1 — Mercenary |

**Six of 134**, and the shape is the same in both rows: *this pool, spent only on these.* So the gap
is not "a grant with an earmark" but **an effect that restricts an allocation the Engine already
owns** — which is a cleaner thing to name and a smaller thing to add, since the resource already
exists on the character.

`require` is the closest form and it is genuinely the wrong one: it means *choose N things* and this
means *distribute a quantity among these*. Written as `require` with `count: 60` and marked, because
the alternative was to invent a grant the book does not make.

### What recurred, which is now the more useful signal

Nothing else in this record is new, and after 28 modelled records that is worth tracking as
deliberately as the gaps:

- **The recurring player-allocated per-level bonus** — *"another 5% that can be applied to any one of
  Open Locks, Move Silently, Find/Remove Traps, or Read Languages"* — is the **second** kit with this
  exact shape, after the CBGH Burglar. Two of 134, and both in one book.
- **The backstab carve-out** is the **fourth** exception modelled as a cancelling adjust, and it is
  word-for-word the Burglar's. The technique has now absorbed every exception-to-a-penalty the slice
  has met, against `except`'s four exception-to-a-prohibition cases. **The split holds cleanly at
  eight instances.**
- The Mouseburglar and the CBGH Burglar are near-duplicates — same class, same book, same benefit
  shape, same hindrance. Which is a small piece of evidence for
  [ticket 11](./11-human-review-protocol.md)'s cost estimate: **the second of a near-duplicate pair
  costs a fraction of the first**, and the corpus has more such pairs than the kit count suggests.

### Session 28 — the Forestwalker, and arithmetic over a collection of other creatures

Twelve effects, the most in the slice, and two shapes nothing else has produced.

### Finding 60 — a modifier whose value is a **count of other things**

> …a successful Tracking check with a **−1 penalty for each non-Forestwalker in the party** (**−2 for
> each horse, mule, or other four-footed animal** accompanying the party).

[Finding 49](#finding-49--a-third-of-all-kits-adjust-a-reaction-roll-and-most-qualify-it) established
that the corpus conditions on **the other party**, singular. This computes a number by **counting a
collection** of them, sorted by kind. Measured, five kits — and what is counted is different every
time:

| kit | counts |
|---|---|
| **Forestwalker** | non-Forestwalkers in the party, and quadrupeds, at different rates |
| Bilker | *"a cumulative +1 for **each previous time** the Bilker has used this ability on him or her"* — **past events**, per target |
| Animal Master | *"+1 for each **slot spent on Training**"* — the character's own spending |
| Patrician | *"−1 for **each dwarf he fails to adequately equip**, until it reaches 0"* — obligations, with a **floor** |
| Noble Warrior | per night per person — pricing, not a character effect |

Only the Animal Master's is plausibly within reach: slots spent is character state, so
`{of: {slotsSpentOnTraining}}` would work if the Engine tracks it. The rest need to count **things
outside the character sheet**, and the Bilker's counts **events in the campaign's past, per victim**.

The Patrician's *"until it reaches 0"* is finding 28's missing combiner again, in its floor form. It
keeps appearing beside other gaps rather than alone.

### Finding 61 — a benefit the player buys with movement, at the table

The Forestwalker's three movement benefits are one mechanic with a dial:

- full speed through tangled undergrowth, **or**
- normal speed and Move Silently through it with **no check**, **or**
- half speed and the party's tracks are concealed.

This is not a condition on state and not a choice made at character creation. It is a **trade the
player elects, per scene, and can elect differently next scene**. `require` chooses once and is
recorded; `when` tests a fact. Nothing represents *"spend this to get that, whenever you like"*.

Carried as text, and worth flagging as probably out of scope rather than missing: a character sheet
that recorded which speed the halfling is walking at today would be tracking play, not rules.

The record also supplies a **seventh earmark** (*"must spend an **initial** proficiency slot on a
missile weapon"* — finding 59), an eighth open list (*"such as a club or a small quarterstaff"*), a
terrain-conditioned bonus, a permit-list, an item-property prohibition, and a reaction penalty that is
**negated membership on the other party** — findings 7 and 49 in one clause. Twelve effects, six
distinct known gaps, and two new ones.

### Session 29 — the Bilker, which is almost entirely procedure

Four effects from ten fields, and two of the four exist only to **name** a procedure so the review
page can reach it. The book closes the benefit itself with *"a Bilker's larger schemes and scams
should be left to the player character to create and enact"* — **the source declining to make a
rule**, which is the cleanest possible answer to what an Engine should do with it.

### Finding 62 — arithmetic across two character sheets

> The Bilker's **Dexterity score is subtracted from the mark's Wisdom score** (to a minimum Wisdom
> of 0). The victim must then roll that number or less on 1d20.

And again for the smoothtalk: *"the Bilker's **Charisma** is subtracted from the victim's
**Wisdom**"*.

This is the third and sharpest step of the same escalation:

| | |
|---|---|
| [finding 49](#finding-49--a-third-of-all-kits-adjust-a-reaction-roll-and-most-qualify-it) | **condition on** the other party — 27 kits |
| [finding 51](#finding-51--an-effect-whose-subject-is-a-spell-or-the-creature-you-cast-it-at) | **effect lands on** the other party — 10 kits |
| [finding 60](#finding-60--a-modifier-whose-value-is-a-count-of-other-things) | **count** a collection of others — 5 kits |
| **finding 62** | **compute across two sheets** — **2 kits** |

Two: this record and the Buffoon's *"the difference between the Buffoon's and the spellcaster's
levels"*. `computedOperand.of` names a scalar **on the character**, and there is no syntax for
*someone else's* Wisdom. Rarest of the four, and the one that would need the most.

### Finding 63 — the clamp now has five records behind it

`(to a minimum Wisdom of 0)`. Measured across the kits, explicit floors appear in **3** — the Bilker,
the Highborn and the Patrician, the last two both *"until it reaches 0"* on a reaction bonus — and
this ticket has already recorded caps in the **Tunnelrat** (*"to a maximum of 60 feet"*) and the
**Pathfinder** (*"will not exceed 20%"*).

So [finding 28](#finding-28--three-kits-state-that-a-bonus-does-not-add-and-adjust-only-sums)'s
missing combiner is no longer a three-record curiosity: **five records want a clamp**, in both
directions, and finding 28 already established the crucial point that **a clamp preserves
order-independence** because `min` and `max` commute. Of everything this ticket has proposed, this is
now the cheapest repair with the most evidence behind it — a bound on a value, not a new subject.

Also here, and already known: the smoothtalk's *"cumulative +1 for each previous time the Bilker has
used this ability **on him or her**"* is finding 60's play-history count, and its *"+1 to +5,
depending on the nature of the suggestion"* is finding 56's bounded DM discretion — a range the DM
picks within, not a value and not free adjudication.

### Session 30 — the Traveler, and a shape regex cannot count

### Finding 64 — effects that land on your **allies**, and a measurement that failed both ways

> A Traveler who has time to tell friends and allies an inspiring story before they enter a combat
> **gives them the same attack and saving throw bonuses as a bless spell**. …his or her music offers
> immunity to song-based charm attacks **to all within earshot**, but **only so long as the Traveler
> can keep playing non-stop**.

[Finding 51](#finding-51--an-effect-whose-subject-is-a-spell-or-the-creature-you-cast-it-at) measured
effects landing on **opponents** — 10 kits — with a pattern built around *"saving throws against…
suffer"*. Neither of these clauses would match it, because helping your friends is written in
completely different words from hurting your enemies.

**And the re-measurement failed in the other direction.** A pattern loose enough to catch *"gives
them…"* returns **32 kits**, of which most are narrative — the Beggar's *"give them whatever meager
scraps can be spared"*, the Trader's *"friends and become acquainted"*, and a Battlerager **spell
list** that happened to contain the word *friends*. Tightening it to require a mechanical token
nearby returns **4**, one of which is a false positive (*"gives **her** a −1 penalty"* — the pronoun
matched).

So the honest answer is **three confirmed and no reliable count** — the Traveler, the Barbarian and
the Savage Wizard.

This is worth adding to the map's method note as its second half. The counts in this ticket that held
up were all keyed on **jargon**: *"reaction roll"*, *"per level"*, *"discretionary points"*, `NdM`.
This shape is written in **ordinary English**, and there regex brackets badly in both directions at
once — a loose pattern drowns in prose and a tight one misses the paraphrases. **Where the corpus uses
plain language for a mechanic, counting it needs reading, not matching.**

### What the record showed besides

Its **+2 to Reaction Rolls is unconditional** — *"to all Reaction Rolls, due to the character's innate
goodwill"* — which is rare enough to be worth naming: of the 48 kits that adjust a reaction, 27
qualify it by the other party, and this one simply does not. The gap in finding 49 is real but it is
not universal, and a plain `adjust` still covers the unqualified case.

The bless-equivalent bonus is also **finding 58's grant-by-citation aimed at a spell** rather than at
another class's ability, which widens that finding's referent: a pack may need to point at spells,
class abilities, and proficiencies it does not define.

### Session 31 — the Healer, and the corpus's own distinction between a rule and a disposition

### Finding 65 — *cannot* and *refuse* are different words and the corpus means them differently

> Healers **refuse** to wear armor in all but the most dire circumstances.

Compare the Forestwalker's *"may **only** wear leather armor"* and the gnome Stalker's *"**cannot**
use a shield or wear any type of metal armor"*. Measured:

| | kits |
|---|---:|
| *cannot / may not / will never* use, wear, carry | **21** |
| *refuse / eschew / disdain / avoid* | **4** — Buccaneer, Stalker, Forestwalker, Healer |

Four is small, and the useful part is not the count but **where the two live**. The soft phrasings sit
in `Equipment` and in narrative asides; the hard ones sit in `Special Hindrances`. The gnome Stalker
carries **both about the same object** — its `Equipment` says it *"disdains the use of a shield"* and
its `Special Hindrances` says it *"cannot use a shield"*.

So a transcriber meeting *"eschews armour"* must not emit a `forbid`, and meeting it **beside** a
`cannot` must not emit two effects. This is not the same as [finding 24](#finding-24--no-field-name-carries-force-in-either-direction),
which said the field name never carries force — here the *verb* carries it reliably, and the field
tells you which of two statements about one object is the binding one.

**A disposition is not a rule**, and the corpus is careful about the difference in a way that the
`forbid` operation cannot record. Modelled as: nothing at all, which is the right answer.

### Finding 66 — a third scaling axis: per die of the spell

> The Healer gains a **+1 hit point bonus per die rolled** to all cure wounds spells he or she casts.

Not per level, and not a count of creatures. The multiplier is **the number of dice the spell itself
rolls** — so `cure light wounds` at 1d8 gains +1 and a bigger cure gains more. **Two kits**: this one,
and the CRH Greenwood Ranger taking *"+1 hit point per die of damage"* from fire, which is the same
axis pointing the other way.

`computedOperand.of` takes a `scalar`, and a scalar is an ability or a class-qualified level. A
property of the spell being cast is neither — it is [finding 51](#finding-51--an-effect-whose-subject-is-a-spell-or-the-creature-you-cast-it-at)'s
missing subject appearing inside an **operand** rather than as the target of an effect.

### A resource reduced rather than granted

*"The Healer **only receives one** weapon proficiency slot at first level."* Worth pairing with the
Mercenary's *"two extra weapon proficiency slots"*: the same field, moved in both directions by two
kits in one book. `set` carries this one because the book states a total, and `adjust` carried the
Mercenary's because the book stated a change — which is the first time in the slice that the choice
between those two operations was made by **how the sentence is written** rather than by what is being
modelled.

### Session 32 — the Leaftender, which confirms rather than surprises

### Finding 65 confirmed inside a single sentence

> Leaftenders **cannot** use metal armor and **will not** use leather armor. They may, however, use
> wooden shields.

[Last session](#finding-65--cannot-and-refuse-are-different-words-and-the-corpus-means-them-differently)
inferred the hard/soft distinction from 25 records across nine books, and it could still have been an
artefact of different authors in different chapters. Here **one author puts both verbs in one
sentence, about two objects, and switches between them deliberately.** The metal is a rule; the
leather is a preference.

Modelled as **one** `forbid`, and that is the whole point of the finding: a transcriber who read
past the verb would have emitted two.

### The Leaftender is the Treetender with a level ladder

The two priest kits — one gnome, one halfling, forty pages apart — grant **the same three
abilities**:

| ability | Treetender | Leaftender |
|---|---|---|
| pass through overgrowth without a trace | from the start | **2nd level** |
| identify plants, animals, pure water | from the start | **3rd level** |
| immunity to charm by woodland creatures | 3rd level | **5th level** |

So the ids are **reused** — `cbgh:pass-without-trace` and its siblings now appear in two records —
which is the first place in the slice where two Attachables deliberately refer to the same thing.
That is what a shared id vocabulary is *for*, and it happened without any mechanism beyond a human
noticing. Worth recording because [ticket 10](./10-mechanical-verification.md)'s cross-record checker
can catch the failure mode — the same ability minted twice under two ids — but nothing can catch it
being **missed**, which is the more likely error at scale.

Three clean level-gated grants, one permit-list, two proficiency grants, and one forbid. **The second
record in the slice to produce no new gap** — after the Rocktender — and by now that is the signal
worth watching: [ticket 08](./08-which-slice-proves-the-format.md) said the slice's job is a verdict,
and a run of records that only instantiate known gaps is what the approach to one looks like.

One small thing with nowhere to go: the weapon list is given *"in order of preference"*. The ordering
carries no mechanical weight and the format has no place for it, which is correct — but it is the
third time (after the paladin Squire's mount likelihoods and the Forestwalker's *"most are
Tallfellows"*) that the corpus has stated a **preference rather than a rule**.

### Session 33 — the Oracle, and an overdue note about scope

### The purest case of finding 16, and a useful one

Nine fields, **one effect**. No prerequisite of any kind. `Weapon Proficiencies` says *"any weapon
allowed for the standard cleric"*; `Equipment` says *"any he or she desires"*. The whole of
`Special Benefits` is:

> When the Oracle performs any spell of the sphere of divination, he or she will receive **(from the
> DM)** unusually reliable and accurate information. **The specifics of this benefit vary by spell, of
> course.**

And the whole of `Special Hindrances` is stubbornness.

This is worth recording precisely because **nothing is wrong with it**. The kit is not badly written
and the format is not failing — the book has handed a benefit to the DM in as many words, and an
Engine that tried to compute it would be inventing a rule its source declines to make. A record with
one grant and `effectsModelled: true` is the *correct* transcription, and it is only legible as
correct because [finding 2](#finding-2--the-schema-accepted-a-semantically-empty-record-fixed) made
that flag a declaration.

### The slice is no longer the slice ticket 08 defined

**27 of CBGH's 28 kits are now modelled**, and CBGH is not in ticket 08's slice at all. That ticket
named the Complete Thief's kits, the five PHB tables, six to eight Deity records and the five Elf
subraces — *"about 40 records"* — and said plainly: **"Do not transcribe beyond the slice."**

What the pack actually holds:

| | records |
|---|---:|
| ticket 08's slice, as defined | ~36 |
| **CBGH kits added since** | **27** |
| CPAH, CRH, CBD records added | 4 |
| **total** | **67** |

Each addition was directed, one record at a time, so this is not drift — but the ticket's own
instruction has been superseded without the ticket saying so, and that is exactly the kind of silent
divergence this map keeps catching elsewhere. **Recorded here rather than left to be discovered.**

The expansion has earned its cost: CBGH is the book that produced findings 12, 35, 37, 57 and most of
the subject findings, none of which the defined slice would have reached, because the defined slice
contains no book that breaks the parser and no kit with a second subject. But the warning in ticket 08
still stands and now needs restating with more force: **a pack of 67 records covering seven books
proves the shapes, and still proves nothing about psionics, spells, equipment breadth, or whether any
of it loads** — [finding 45](#finding-45--every-reference-in-the-proving-slice-is-dangling) having
shown that none of its 119 references resolve.

### Session 34 — the Archer completes a whole book

**All 28 of CBGH's kits are now modelled by hand** — the first complete book in the corpus, and the
one that started as *"the book the parser cannot read"* in [finding 12](#finding-12--one-v1-book-carries-no-field-markup-at-all).

### Finding 67 — a permission that is degraded rather than denied

> The Archer can gain normal proficiency in **only one melee weapon**. He or she **can spend
> proficiency slots on others but will always wield them with a −1 penalty** on all attack rolls.

Neither `forbid` nor `except`. The character **may take the thing, worse** — which is a third
position between permitted and prohibited that §4.3 has no vocabulary for. The `adjust` carries the
penalty; what it cannot carry is the scope, *every melee weapon after the first*, which is per-item
state the predicate cannot reach.

One kit, and per the map's method note that is a lower bound written in ordinary English, so the true
figure is unknown rather than one. Recorded for the shape, not the count: it sits beside
[finding 65](#finding-65--cannot-and-refuse-are-different-words-and-the-corpus-means-them-differently)'s
hard/soft verbs as a **third** grade of restriction, and unlike a disposition this one **is**
mechanical.

### Finding 68 — an election between two effects, made per use

> …the Archer can choose to make a called shot with **increased accuracy or extended range, but not
> both at the same time**.

The called shot is one ability with two mutually exclusive outcomes, chosen **per shot**. That
compounds two known gaps —
[finding 13](#finding-13--disjunction-in-an-effect-has-no-operation-and-the-clause-repair-does-not-reach-it)'s
choice between effects and [finding 61](#finding-61--a-benefit-the-player-buys-with-movement-at-the-table)'s
trade elected at the table — and adds exclusivity, which neither had. **Two kits**: this one and the
CRH Stalker's *"in one (but not both) of the following ways"*.

The trade itself is priced in things the format has no subjects for: forgoing initiative, firing once
that round, and losing the benefit if struck first. And the extended range is *"the weapon's medium
range added to its long range"* — arithmetic over **the weapon's** properties, which is
[finding 51](#finding-51--an-effect-whose-subject-is-a-spell-or-the-creature-you-cast-it-at)'s missing
subject in its fourth guise: after the spell, the creature, and the other character's sheet, now the
item.

Worth ending the book on the positive that came with it: the Archer's *"+2 bonus is **in addition to**
any other attack bonuses the character might have due to Dexterity, weapon specialization, bless
spells, or magical weaponry"* is the corpus stating **additivity across four independent sources** —
the layer model's central claim, asserted by the book, in the last record of the book that produced
the most objections to it.

### Session 35 — into CTH, and finding 35's repair, implemented

Turning to the book [ticket 08](./08-which-slice-proves-the-format.md) actually named. CTH marks
**100% of its kits with sub-labels**, so [finding 35](#finding-35--force-is-carried-one-level-down-and-the-extractor-was-flattening-it)'s
deferred repair was directly in the way: modelling sixteen kits while reading `Required:` and
`Recommended:` off the page by eye is doing sixteen times what the parser should do once.

**Implemented.** `split_sublabels` is the same rule as `fields_typographic` applied one level down —
a capitalised short phrase, then a colon — and `parse` now returns `parts` beside `fields`, so every
existing caller is untouched. Measured across the eight markup books: **CTH 40 sub-labels, CFH 50,
CWH 28, CBD 6, CDH 3, CBGH 0**, and the vocabulary is exactly `Required`, `Recommended`,
`Bonus Proficiencies`.

### Finding 69 — the second level also recovers fields the markup **lost**

An unexpected dividend. `Special Hindrances` and `Optional Rule` do not appear in the Buccaneer's
top-level fields at all, and `Secondary Skills` does not appear in the Beggar's — the `<I>` markup
failed on them, and the sub-label parser found them.

Counting only labels that the **same book** marks up as a top-level field in at least two other
records — so a genuine markup failure rather than a book's habit:

| book | fields lost and recovered |
|---|---:|
| CPAH | 11 — incl. **4 `Special Hindrances`**, 3 `Special Benefits` |
| CBD | 6 — incl. 3 `Wealth Options` |
| CRH | 5 |
| CTH | 4 |
| CFH, CDH | 4 |
| **total** | **31** |

Four `Special Hindrances` fields silently absent is not a cosmetic loss — that is a kit's entire set
of drawbacks vanishing. [Session 15](#finding-35--force-is-carried-one-level-down-and-the-extractor-was-flattening-it)
justified this repair on force alone; it also fixes **31 missing fields**, and neither the schema nor
the validator could ever have detected them, because a record with no `Special Hindrances` is
perfectly valid.

### The first record that is genuinely empty, and correct

**The Adventurer.** Every field reads `Any.` or `None.`, and its `Description` says the kit *"has no
requirements beyond those of the thief class itself"*. Modelled as **no prerequisite, no effects,
`effectsModelled: true`**.

That is the state [finding 2](#finding-2--the-schema-accepted-a-semantically-empty-record-fixed)
invented the flag for, and **this is the first record in the corpus to occupy it**. Thirty-four
sessions after the flag was added to stop an *unfinished* record from looking complete, a genuinely
complete and genuinely empty record has arrived — and it is legible as such only because of it.

### Finding 70 — a fifth `except`, and a fifth undercount

> Assassins, **unlike thieves of other kits, are permitted** the use of any weapon.

[Finding 54](#finding-54--except-has-four-cases-not-one-and-one-of-them-carries-a-cost) re-measured
`except` to four cases using a pattern that required *"unlike most/other …"* followed by
**`can`, `may`, `is able` or `gains`**. The book writes **`are permitted`**. Five cases, and the
**fifth undercount in this ticket from a pattern narrower than the corpus** — now so regular that the
map's method note predicted it.

Two things the Assassin models cleanly and worth recording against the run of gaps: *"the base chance
is the Assassin's **level multiplied by 5%**"* is `{of: level, multiplyBy: 5}` exactly, and the
**Intelligence bands** — 13–15, 16–17, 18 — are three adjusts each gated by a `gte` **and** an `lte`,
which the flat conjunctive predicate expresses with no repair at all.

### Session 36 — CTH complete, and ticket 08's slice is finally transcribed

**All 18 Complete Thief's kits are modelled**, which closes the first item ticket 13 was written to
produce. Two whole books now stand finished, and they are the two extremes of the corpus: the one
with no field markup at all and the one that marks every kit twice over.

### Finding 71 — an effect that swaps one class's table for another's

> With this, the Swashbuckler's *"weapon of choice,"* the thief is able to fight with the **THAC0 of a
> fighter of his experience level**.

`set` names the field. What it cannot name is where the value comes from: **another class's
progression, read at this character's level**. It is not a number, not a computed operand over a
scalar, and not a lookup the pack could supply — it is the Engine's fighter table, indexed by a thief.

That is [finding 20](#finding-20--a-lookuptable-has-no-declared-role-and-its-rows-are-keyed-by-prose)'s
problem arriving from the opposite direction. There, a pack supplied a table and nothing said what it
was a table *of*. Here an effect needs to *read* a table the Engine owns, and nothing lets it say
which.

### Finding 72 — degraded permission is a pattern, not a curiosity

[Finding 67](#finding-67--a-permission-that-is-degraded-rather-than-denied) recorded the Archer's
*"only one melee weapon, others at −1"* as one record. CTH has three more, in two distinct grades:

| kit | the degradation |
|---|---|
| Bounty Hunter | non-thief weapons permitted, but **each costs two slots** |
| Thug | the same, *"to gain proficiency in one requires an extra slot"* |
| **Spy** | **may USE non-thief weapons, but may never take proficiency in them** |
| Archer (CBGH) | may take further melee weapons, **at a permanent −1** |

The Spy's is the sharpest and the one `except` handles worst: **permission and competence come
apart**, and `except` lifts a prohibition whole. Four records across two books, and none of them is a
`forbid` — the corpus's restrictions are far more often *priced* than *closed*.

### The two records that carry a probability

*"Usually **(90%)** none"* (Cutpurse) and *"Most often **(80% of the time, say)** a Swashbuckler has
no secondary skills"* (Swashbuckler). A **likelihood attached to a character property at creation** —
neither an effect nor a prerequisite, and the third distinct place the corpus has stated a
probability rather than a rule, after the paladin Squire's mounts and the Forestwalker's Tallfellows.

### What CTH confirmed rather than added

The book is dense with instances of gaps already recorded, which after two complete books is the
result worth reporting:

- **Nine of eighteen kits have `Special Benefits: None.` and `Special Hindrances: None.`** — half the
  book, and exactly [finding 16](#finding-16--a-third-of-kits-have-no-numeric-mechanics-at-all)'s
  measurement holding on the population it was taken from.
- The **Scout** produced the first adjust over a **category of fields** — *"−5% on all thieves'
  skills"* is eight separate values, written as eight adjusts because `adjust.field` names one field
  and there is no wildcard. That is finding 11's missing wildcard appearing in the *other* half of the
  format.
- The **Thug** needs an earmark and a clamp in one sentence: 40 points to allocate, *"up to 30 of them
  in a single ability"*.
- The **Troubleshooter** has **no `Special Benefits` or `Special Hindrances` field at all** — not
  `None.`, absent — while its `Races` field refers to *"the special benefit/hindrance of this kit"*.
  **The book points at a field it did not print.**

### Session 37 — the judgement pass is complete, and it found a record that was simply wrong

**All 68 records now carry effects.** `effectsModelled: false` appears nowhere in the pack. Ticket
13's judgement pass, which [session 3](#session-3--the-tables-and-the-first-record-modelled-by-hand)
called *"the expensive half"* and which nothing had touched, is done for the slice.

### Finding 73 — a record that was wrong for thirty-five sessions, and validated throughout

The Complete Book of Elves puts all its subraces on one page, which is why
[ticket 07](./07-identity-and-id-stability.md)'s ordinal exists. Reading it to model it:

- The page names **six** subraces — Aquatic, Drow, Grey, Half-Elf, High Elf, Sylvan.
- It carries **five** marker groups.
- The mechanical pass produced five records and named the fourth **`Half-Elf Half-elves`**.

The fourth record is the **High Elf**. The book states that the half-elf *"is not truly a subrace of
elves"* and has *"no adjustments or advantages and disadvantages to note here"* — so it has no
fields at all, and everything the extractor gave record #4 belongs to the high elf. **The record has
carried the wrong name and the wrong provenance since session 2**, through every validation run, and
nothing could have caught it: a subrace named `Half-Elf` with a high elf's `+1 Dexterity, −1
Constitution` is perfectly well-formed.

The same page cost a second field. **The Drow's `Ability Score Adjustments` label is missing from the
markup** — its text sits in the tail of the Aquatic Elf's last field — so the Drow's `+2 Dexterity,
+1 Intelligence, −2 Charisma` had to be read off the page. Five records, one wrong name, one lost
field, and a clean validation the whole time.

This is the strongest evidence the ticket has produced for its own premise. [Ticket 04](./04-llm-assisted-extraction.md)
kept the slice as a **gold standard** on the argument that *a check only works if its reference is not
the model*; here the reference had to be **a person reading the source**, because every mechanical
check the effort owns said the record was fine.

### Finding 74 — the tier-one checker caught its first real authoring error

Modelling the seven deities, ids for the granted powers were built by substituting `-` for `:`,
producing `cprh-DD05531-power`. The validator refused all nine:

```
INVALID  deities/1/effects/10: ... is not valid under any of the given schemas
```

**First time in thirty-seven sessions that the pack-visible checker caught a mistake in a
hand-written record.** [Ticket 10](./10-mechanical-verification.md) exists to make that happen, and
until now it had only ever confirmed things that were already right — which is exactly how a check
looks in the interval before it earns its cost.

Worth pairing with finding 73 deliberately: **the same session shows the checker catching a
malformed id and missing a wrong record entirely.** That is the boundary of what tier one can be
asked to do, drawn by two live examples rather than by argument.

### What the deities confirmed about §4.1

Seven records of ten fields each, all validating against the shared `attachable` unchanged. The arm
is regular to the point of monotony — alignment, two ability minima, a race permit-list, sphere
grants, a granted power, a follower roster — and **that regularity is the result**: after the Kit
arm's 46 records and their sixty-odd distinct gaps, the Deity arm produced **one** new shape, the
Arts priesthood's *"only 6-sided hit dice, not 8-sided"*, which is finding 71's class-table
substitution in its simplest possible form.

### Session 38 — into CRH, the structural extreme

The book [finding 37](#finding-37--two-books-have-a-shared-spine-and-a-per-kit-tail) identified as
the corpus's worst case: **99 distinct labels across 13 kits, 74 used exactly once**, 15–25 fields per
record, benefits as named fields with bulleted sub-items, and terrain as a spine field. If the
pipeline survives CRH it survives v1.

A fourteenth apparatus page found on arrival — `List of Kits`, whose `Description` field is the
**Beastmaster's**, because the page concatenates every kit in the chapter.

### Finding 75 — the follower is a subject with its own sheet

> An attuned falcon receives a **+2 bonus to all attack rolls**, except when fighting **its species
> enemy** (when it receives a +4 bonus). … When fighting on behalf of a Falconer, an attuned falcon
> **never needs to make a morale check**.

The Falconer's kit gives a **falcon** an attack bonus, a species enemy of its own rolled on its own
table, and an exemption from morale. This is past everything the subject findings have recorded:

| finding | subject |
|---|---|
| 49 | the other party, as a **condition** |
| 51 | the other party, as a **target** |
| 62 | another character's **sheet**, as an operand |
| **75** | a creature **this kit creates**, with stats **derived from the kit** |

Findings 49–62 all point at someone who already exists. Here the pack **brings a second sheet into
being** and then modifies it. The effects are written against `follower.attunedFalcon.*` so they stay
visible, and nothing in §4.3 or §3 says such a path means anything.

### Finding 76 — a kit that **reduces** its class

The Explorer's own named fields include `Limited Animal Empathy` — *"an Explorer does not develop
animal empathy to the degree of other rangers"* — `Few Followers`, and `No Fortifications`
(*"he will never build a castle or any other fortification"*).

Across four previous books a kit added, restricted a choice, or priced something. **These take away
abilities the class already has.** `forbid` handles the fortification because it is total; nothing
handles *"to a lesser degree"*, which is [finding 67](#finding-67--a-permission-that-is-degraded-rather-than-denied)'s
degraded permission applied to a **class feature** rather than to equipment.

The Explorer also produced the first **anti-scaling** statement in the corpus: Survival works in all
terrains, and *"assigning additional slots to this proficiency **does not enhance its use in any
way**"*. The format can add to a value; it has no way to say *and no more will be added*.

### Session 39 — CRH complete: the third book, and the one that pays with class features

**All 13 Complete Ranger's kits are modelled.** *(Corrected in session 42: the book has **14**; the Warden was being dropped for a missing marker label.)* The pack holds **80 records** across seven books, and
the structural extreme did not break the pipeline — the same extractor, the same schema, no new
strategy. Finding 37's per-kit vocabulary turned out to cost nothing, because the parser never
depended on knowing the labels.

### Finding 77 — this book's kits pay for their benefits by **giving class features back**

The pattern [finding 76](#finding-76--a-kit-that-reduces-its-class) noticed in the Explorer is how the
whole book works:

| kit | what it surrenders |
|---|---|
| Explorer | animal empathy *"to a lesser degree"*, ≤2 concurrent followers, no fortifications |
| Giant Killer | one nonweapon slot only; **tracking narrowed to giants** |
| Feralan | spells limited to the animal sphere, no fortifications, no money |
| Greenwood Ranger | no armour, **no Dexterity bonus to AC**, spells limited to the plant sphere |
| Justifier | one nonweapon slot, less spell access, a **reduced** species-enemy penalty |
| Mountain Man | fewer spells, no fortifications, one possession over 15 gp |
| **Sea Ranger** | **neither Move Silently nor Hide in Shadows** |
| Seeker | an hour of meditation daily |

Across CTH and CBGH a kit added, chose, or priced. **Here it trades.** And the Sea Ranger states the
trade outright — *"the Sea Ranger has **neither** of these abilities, **replacing** them with Sea Legs
and Aquatic Combat"* — which is the shape the format handles worst: it can write the two `forbid`s and
the two `grant`s, and nothing records that **they are one exchange**.

The Greenwood Ranger's *"he gets **no Dexterity bonus** to his armor class"* is sharper still. That is
not a value adjusted but **a derivation suppressed** — the Engine computes AC from Dexterity, and this
kit switches that computation off. `set` writes values; no operation disables a rule.

### Finding 78 — a prerequisite the pack declares incomplete

> A Seeker must have a Wisdom score of 15 or more. **A particular religion may impose additional
> requirements, as determined by the DM.**

The record states its prerequisite and then says **there may be more, decided elsewhere**. Everything
this ticket has recorded about A3 concerns whether a pack **covers** a rule; this is a pack whose
prerequisite is **open by design**, and a validating record would assert a completeness the book
explicitly disclaims.

### What CRH confirmed, and the `Barred` field

Two of its kits carry a **`Barred` field** — an enumerated list of forbidden proficiencies — which is
**the first natural home `forbid` has found in 80 records**. Every other prohibition in the pack has
been a permit-list written as a placeholder id, because the corpus says *"limited to"* far more often
than it says *"may not take"*. Here it says the second, in a list, and the operation fits without
strain.

The book also settled a question left open in
[finding 53](#finding-53--the-corpus-writes-randomness-in-a-second-notation). The Forest Runner writes
*"2–5 **(1d4+1)** rounds"* — **the book glosses its own bare range with dice notation**, so the two
notations are the same thing written twice and converting one to the other is the source's own move,
not the transcriber's. It does not rescue `30-50` or `6-7`, which no die produces.

### Session 40 — CWH complete: eight books, 89 records

**All nine Complete Wizard's kits are modelled**, and CWH is the opposite of CRH structurally: a tight
16-label spine, only two labels used once, and `Preferred Schools` / `Barred Schools` as first-class
fields. A fifteenth apparatus page found on arrival — `The Kits`, whose `Description` is the
**Academician's**, the same shape as CRH's `List of Kits`.

### Finding 79 — the dice pattern was wrong about grouping; **fixed**

The Amazon Sorceress starts with **`(1d4 + 1) x 10 gp`**, and the pattern implemented in
[finding 42](#finding-42--ticket-15s-dice-decision-was-settled-and-never-implemented-now-it-is)
rejected it. That pattern assumed the grammar `NdM [×K] [±J]` **in that order**; the corpus also
writes **`(NdM ± J) × K`**, which is not a notation variant but **different arithmetic** — the
grouping changes the result.

Measured over the v1 RTF: **14 occurrences**, 0.5%. Small, and one of them is the **wizard class's
standard starting money**, which every kit in this book restates. Widened, and the widening is the
third correction to a value type this ticket has made by meeting a record that used it.

Worth noting what did **not** need widening. The Patrician gets *"an extra 150 gp **in addition to**
the standard (1d4+1) x 10"*, and that composes as a **dice value and a constant beside it** — the
layer model absorbing an offset again, exactly as it did for the scaling in finding 50. The format
needs a richer *notation*, not a richer *expression language*.

### Finding 80 — a kit that restricts character construction itself

The **Witch**:

- *"not allowed an initial Weapon Proficiency, **nor can she acquire a Weapon Proficiency as she
  advances in level**"* — a permanent denial, of which the format can state only the starting value;
- *"Witches **do not earn bonuses to their experience for high ability scores**"* — a **derivation
  suppressed**, like the Greenwood Ranger's Dexterity bonus to AC;
- *"Witches **cannot be multi-classed or dual-classed**."*

The last one is new in eight books. Every other effect in 89 records modifies **a character**; this
one restricts **how a character may be built** — §6.1's class sum type, which is the Engine's own
structure rather than anything on a sheet. An Attachable that constrains the shape of the character it
attaches to is a different thing from an Attachable that changes its values.

### What the wizard book said about spells

The reason for choosing this book was that the slice contains no spell records and CWH is the corpus's
spell-heaviest kit book. The answer is milder than expected:

- **`Barred Schools` is enumerable and `forbid` fits it perfectly** — the second natural home for that
  operation after CRH's `Barred`, and between them **20 of the pack's 27 `forbid`s** now sit on a list
  the book actually printed.
- **`Preferred Schools` is advice** — *"tend to be drawn to"*, *"are among the most frequently
  encountered"* — and yields nothing, which is finding 16 in the spell domain.
- The genuinely hard case is the Wu Jen's *"any one spell that is **three or more levels lower than
  his level**"* — a comparison between **a spell's level and the caster's**, which is
  [finding 62](#finding-62--arithmetic-across-two-character-sheets)'s cross-subject arithmetic with a
  spell on one side.

So the spell domain did not produce a new class of problem. It produced **more instances of the
subject problem**, which after eight books is the only structural gap that has grown every time it was
measured.

### Session 41 — a whole book was missing, and nothing could have said so

Looking for the next book to model, the survey showed **CBE returning 0 kits from 119 pages**. The
Complete Book of Elves has **eleven**, and one of them is the **Bladesinger** — the kit the project's
own public README names as its example of what a pack supplies.

### Finding 81 — a third label convention, and an omission with no symptom

CBE writes its field labels as **`· Role.`** — a Symbol-font bullet, bold, terminated by a **period**.
Eleven books use a colon; [finding 12](#finding-12--one-v1-book-carries-no-field-markup-at-all) found
CBGH using none at all; this is the third convention, and the label regex had required a colon since
session 1.

**The failure mode is what matters.** CBGH's zero was *noticed* because ticket 08's slice named a book
and the extractor returned nothing for it. CBE's zero was never noticed for forty sessions, because
**a book with no kits is indistinguishable from a book whose kits are somewhere else** — the elves
book does have subraces, they extracted fine, and nothing in the pipeline knows how many kits a book
ought to have.

That is a class of error the whole verification apparatus cannot reach.
[Ticket 10](./10-mechanical-verification.md)'s tier one validates what is present;
[finding 45](#finding-45--every-reference-in-the-proving-slice-is-dangling)'s reference counting
reports what is pointed at. **Neither can report what was never extracted**, and this ticket now has
two instances — CBE's eleven kits here, and the CTH `Special Hindrances` fields that
[finding 69](#finding-69--the-second-level-also-recovers-fields-the-markup-lost) recovered. Both were
silent absences found by a person looking at something else.

### The fix is opt-in per book **and kind**, for a reason worth recording

Widening the label to accept a period globally recovered CBE's kits and **broke the same book's
subrace page**, which bolds spell names mid-sentence: `darkness.` became a field, the record split
gained a group, and two records lost their names.

So the period form is anchored on the **bullet** and enabled per `(book, kind)`. One book, two
conventions, and the bullet is what tells them apart — which is the same shape as
[finding 32](#finding-32--name-collisions-are-systematic-and-looking-for-them-found-apparatus)'s
exclusion list, keyed by `(book, name)` because a bare name was not a safe key.

The global attempt also surfaced a **sixteenth apparatus page**, and the first one *inside* a
multi-record page: the elf subrace page opens with a template group whose `Ability Score Adjustments`
reads *"Adjustments to be added to, or subtracted from, the base ability scores when creating
characters."* It has its own marker and would have become a sixth subrace. The scoped fix avoids it;
it is recorded because a template group inside a shared page is a shape finding 9's detector was never
designed for.

### Session 42 — the elf kits, and six more records that were never extracted

**All eleven Complete Book of Elves kits are modelled**, including the **Bladesinger**. The pack
reaches **100 records** across nine books.

### Finding 82 — one bad tag drops a whole record, and six were lost that way

The Spellfilcher has every field a CBE kit has **except `Role`**, which is the marker, so it was
dropped. That prompted a check nobody had written: **pages that share four or more labels with their
book's spine and lack the marker**. Stripping the ten known apparatus pages, six real kits were being
silently discarded:

| book | kit |
|---|---|
| CBD | Outcast |
| CBE | Spellfilcher |
| CPAH | Equerry, Ghosthunter, Militarist |
| **CRH** | **Warden** |

**The CRH is therefore not complete**, and [session 39](#session-39--crh-complete-the-third-book-and-the-one-that-pays-with-class-features)
reported it as such. It has 14 kits, not 13, and CPAH has 15 rather than 12. Corrected here rather
than left standing.

The cause splits in two, and only one half is a heuristic:

- The **Equerry, Ghosthunter, Militarist and Warden** print `Role:` **in their text** and the `<I>`
  markup fails on it. So the marker test now reads **both field levels** — the same repair as
  [finding 69](#finding-69--the-second-level-also-recovers-fields-the-markup-lost), and not a guess:
  it is the label the book printed.
- The **Outcast and Spellfilcher** have **no `Role` field at all**. Those need an `INCLUDE` list,
  human-maintained, the exact mirror of `EXCLUDE` — one drops apparatus that parses, the other keeps
  records that do not. Both are human because
  [finding 1](#finding-1--record-boundary-detection-is-not-mechanical-correcting-ticket-01)
  established record boundaries are not mechanical.

This is the third instance of the class [finding 81](#finding-81--a-third-label-convention-and-an-omission-with-no-symptom)
named: **an omission with no symptom**. A missing record is not invalid, not dangling, and not
counted. The check that found these — *spine overlap without the marker* — is now the only tool in
the effort that can see one, and it should be run against every book before any of them is called
complete.

### Finding 83 — a Kit that points at Subraces

Every CBE kit carries a **`Suggested Elf Subraces`** field: *"Grey elf, high elf, sylvan elf."*

It is advisory — *suggested* — so it models as nothing. But it is the **first field in nine books
where one Attachable arm names another**, and §4.1's claim is precisely that the three arms are one
shape. A kit recommending subraces is the corpus treating them as composable, which is what the shape
predicts, stated by the source rather than inferred.

### What the elf kits added

Little that is new, and after nine books that continues to be the point. The **Archer** compounds a
permit-list with finding 67's degraded permission in one sentence — only three melee weapons, and
*"even then"* a −1 — and gives finding 68's per-use election a second book. The **Windrider** gates
its **whole record by level** (*"no benefits until… typically 4th"*), which unlike the Rocktender's
environmental gate **is** expressible. The **Herbalist** adds a chosen-terrain parameter to the list
that already holds the Feralan's familial species, the Goblinsticker's hated foe, the Seeker's sacred
animal and the Undead Slayer's chosen undead.

### Session 43 — the five recovered records, and four shapes nothing else had

**105 records.** The kits finding 82 rescued from four books are modelled, and for records that were
invisible until yesterday they were unusually productive.

### Finding 84 — a benefit bought with **experience**

> By paying an **additional 10% experience cost** to increase in level, an Outcast may Move Silently
> **as a ranger of the same level**.

An **optional, permanent upgrade priced in advancement**. The character may decline it; if he takes
it, he pays forever. [Finding 61](#finding-61--a-benefit-the-player-buys-with-movement-at-the-table)
recorded a trade elected per scene and [finding 68](#finding-68--an-election-between-two-effects-made-per-use)
one elected per use — this is elected **once, at the start, and binding**.

The format can write the surcharge and it can write the benefit. What it cannot write is that **they
are one purchase**, or that the purchase is refusable. And the benefit itself is
[finding 71](#finding-71--an-effect-that-swaps-one-classs-table-for-anothers)'s cross-class table
read, for the third time.

### Finding 85 — three more subjects, in five records

| record | the subject |
|---|---|
| **Equerry** | *"a **male** Equerry can't choose a unicorn"* — the character's **sex**, conditioned on for the first time in ten books |
| **Militarist** | *"attacks **as if he were one level higher**"* — an offset applied to **the level at which a table is read**, not to a value |
| **Ghosthunter** | *"a **95% immunity** to paralysis"* — a **probability of immunity**, where the format has `grant` and `forbid` and nothing between |

The Ghosthunter's is the one worth dwelling on. `grant` and `forbid` are absolute, and a 95% immunity
is neither — it is the same gap as
[finding 28](#finding-28--three-kits-state-that-a-bonus-does-not-add-and-adjust-only-sums)'s missing
combiner, one level up: **the corpus has partial versions of things the format only has whole.**
Partial permission (finding 67), partial class features (finding 76), partial immunity. Three
findings, one shape.

### Finding 86 — a resource convertible into another, one way only

> He can **substitute weapon proficiencies for nonweapon proficiencies (but not vice versa)**.

The first **exchange rate between two class resources** in ten books, and it is asymmetric. §4.3 can
grant a resource, spend it ([finding 59](#finding-59--six-kits-constrain-the-spending-of-a-resource-they-do-not-grant)'s
earmark), and reduce it — it has no operation that **trades one pool for another**, and no way to
record that the trade runs in one direction.

The same record adds a **conditional forfeit**: an hour of practice daily, *"if he neglects to
practice, **he loses his mounted combat bonuses**"* — the Cavalier's shape from
[finding 55](#finding-55--a-gate-on-the-whole-record-which-the-schema-has-no-slot-for), scoped to two
named effects rather than to the whole record.

### Session 44 — CBH complete: eleven books, 122 records

**All 17 Complete Bard's kits are modelled**, plus an eighteenth apparatus page. The pack is at
**122 records across eleven books**, and every one of them carries effects.

### Finding 87 — every kit in this book carries **per-kit racial level limits**

The `Qualifications` field routinely reads:

> Elves can advance up to **15th level** as Minstrels, while half-elves are limited to **12th**.
> Demihumans can become Heralds of up to **6th level**. Gnomes can rise to **8th** as Riddlemasters.

A **table of (race → maximum level) belonging to the Attachable**, not to the class. 2e's racial level
limits are a *class* rule; here **every one of seventeen kits carries its own**, and no other book in
eleven does this even once.

This is the sharpest instance yet of an Attachable editing something that is not on the character
sheet. [Finding 80](#finding-80--a-kit-that-restricts-character-construction-itself) recorded the
Witch forbidding multi-classing; here a kit rewrites the **advancement ceiling per race**, which the
Engine derives from class and race together.

### Finding 88 — two records define a prerequisite by **transformation**

- **Gnome Professor**: *"The standard qualifications for Intelligence and Charisma are **switched**."*
- **Halfling Whistler**: *"qualifying Intelligence is **lowered to 10**"*, and *"**prime requisites**
  for Whistlers are…"* — reassigning which abilities earn the experience bonus.
- **Dwarven Chanter**: *"they have **no Intelligence qualification** (unlike most bards)"* — a
  standard prerequisite **removed**.

The prerequisite is not stated; it is **derived from the class's own by permuting, lowering, or
dropping a term**. A `predicate` is a list of conditions and has no way to say *"the class's list,
with these two swapped"* — so all three were expanded to absolute values, which is the same
derivation-not-transcription problem as
[finding 47](#finding-47--the-corpus-counts-and-only-the-effect-side-can-count-back)'s CNF expansion,
and it loses the fact that the record is defined **relative to** something.

### What the bard book confirmed

It is the third book after CRH and CWH whose kits **pay in class features**, and the demi-bards pay
hardest: the Dwarven Chanter, Gnome Professor and Halfling Whistler each surrender the bard's
spellcasting outright, and three kits lose the 10th-level written-magic ability by name.

Two smaller confirmations worth recording because they are *positive*:

- The **Meistersinger** has a **`Barred` weapons list** — *"the following weapons are forbidden"* —
  the **third natural home** `forbid` has found, after CRH's and CWH's. And its followers are
  **replaced** rather than removed: *"their animal companions serve as their followers"*, which is the
  Sea Ranger's substitution in a second book.
- The **Skald** cannot take reading/writing *"because the Skald's society does not have a written
  language"* — **the only prohibition in 122 records justified by setting rather than by mechanics**,
  and it models as an ordinary `forbid`. The format does not need to know why.

### Session 45 — CBD complete: twelve books, 143 records

**All 24 Complete Book of Dwarves kits are modelled.** The three race books — gnomes and halflings,
elves, dwarves — are now finished, and CBD has the **tightest spine in the corpus**: 13 labels, two
used once, against CRH's 99 and 74.

### Finding 89 — a kit defined as **another kit plus a delta**

> Vindicators gain **all the special benefits of Battleragers**, but are not allowed to specialize in
> any weapon. … Vindicators have **the same hindrances as Battleragers, except that** members of their
> own religion do not react to them with a negative penalty.

**Inheritance between Attachables.** §4.1 says Kit, Deity and Subrace are one shape used three times;
it says nothing about one record being defined **in terms of another**. The schema has no `extends`,
and modelling it meant **copying the Battlerager's effects by hand and applying two deltas** — which
produces the right character and **loses the relationship entirely**. A reader of the pack cannot tell
that the Vindicator is a Battlerager variant, and a correction to the Battlerager will not reach it.

This is the same loss as [finding 88](#finding-88--two-records-define-a-prerequisite-by-transformation)'s
prerequisites-by-permutation, one level up: **the corpus defines records relative to other records,
and the format only stores absolutes.**

### Finding 90 — a kit that **overrules a Deity**, with the book stating the precedence

The same record: Vindicators must be proficient in battle axe and warhammer **"regardless of the
restrictions imposed by their religion."**

Three CBD kits take their permitted weapons **from the character's deity** — the Temple Guard's must
be *"representative of his religion"*, the Pariah may use *"any weapon not forbidden by his
religion"* — so the Kit arm and the Deity arm speak about the same field. And here the book says
which one wins.

§4.3's operations commute, which is what makes the layer model work — and **commutation is exactly
what has no answer when two layers contradict**. `forbid` from the Deity and `require` from the Kit
are not order-dependent; they are simply inconsistent, and the book resolves it with a precedence rule
the pack has nowhere to put. This is the first place in twelve books where **two Attachables conflict
on the same subject**, and it is worth stating plainly: it is a bigger hole than any missing operand,
because the layer model's central guarantee does not address it.

### Smaller things this book confirmed

- **Finding 84's XP-purchased upgrade appears twice**, word for word, in the Outcast and the Pariah —
  so it is a book-level device rather than one kit's quirk.
- The **Rapid Response Rider's** mount has *"at least 75% of its possible hit points. Roll hit points
  normally; if they come to less than 75%…"* — [finding 63](#finding-63--the-clamp-now-has-five-records-behind-it)'s
  clamp applied to **recorded randomness**, which §6.2 says is neither choice nor derivation.
- The **Pest Controller's** reaction penalty excludes *"Vermin Slayers and Wayfinders"* — a condition
  that names **two other kits**, so an Attachable conditions on which Attachables the other party has.
- Three **OCR artefacts** — `B2` and `B10%` for minus signs, and `miner access` for *minor* in CRH.
  The corpus is not clean, and nothing in the pipeline would notice.

### Session 46 — CFH complete: thirteen books, 157 records

**All 14 Complete Fighter's kits are modelled.**

### Finding 91 — two kits on one page, **interleaved inside every field**

- **Pirate/Outlaw**: *"If the character is a **Pirate**, he must take the following proficiencies…
  If the charact[er is an **Outlaw**]…"*
- **Samurai**: *"The samurai and ronin **have different special hindrances**."*

[Ticket 07](./07-identity-and-id-stability.md)'s ordinal exists because the elf subrace page holds
five records — but those run **in sequence**, each with its own field set. These two hold **two kits
whose divergences are stated inside each shared field**, sentence by sentence.

Splitting a page into records is a boundary problem the ordinal solves. Splitting a **field** into two
readings is not, and nothing in the pipeline or the schema does it. Modelled as one record carrying
both, which is **wrong and deliberately visible**: the pack asserts that a Pirate and an Outlaw have
the same weapon requirement, and the book says they do not.

### Finding 92 — an effect triggered by the result of a die

> Whenever the barbarian character achieves **a reaction roll of 14 or more**, he takes an additional
> [penalty].

Every condition in 157 records tests a **state** — an ability, a level, a race, a membership. This
tests **an outcome**: the effect fires because a die came up a certain way. There is no subject for it,
and there is no moment either; `when` is evaluated against a character, not against a roll in progress.

The Cavalier gives the same shape in a second form: he *"cannot attack an opponent at range **if he
can instead charge**"* — a prohibition conditioned on **what the character could otherwise be doing**,
which is a comparison between two available actions.

### What CFH confirmed

Almost everything, which after thirteen books is the expected and useful result. The **Amazon** is the
CWH Amazon Sorceress's first-encounter bonus in a different class and nearly the same words — a kit
shared across two books and two classes. The **Berserker** conditions on *"any tribe that also has
Berserkers"*, which is the other party's **culture** rather than the party itself. The **Savage**'s
weapon list *"should be defined by the DM"*, so the record's own restriction is delegated. And the
**Swashbuckler** is the eleventh name collision, with the book itself cross-referencing the thief
version in the Complete Thief's Handbook.

### Session 47 — CDH complete: fourteen books, 171 records

**All 14 Complete Druid's kits are modelled.**

### Finding 93 — the rules are told through a **named example character**

> As an Adviser, **Elam** can purchase the rogue's disguise proficiency at normal rather than double
> cost. … **Torrens**, like all Avengers… If **Lasell**, as a Beastfriend, carefully approaches an
> animal…

**12 of 14 CDH kits state their mechanics as sentences about one invented person.** No other book in
fourteen does this even once; every other record says *"the Bounty Hunter is permitted…"* or *"a
Falconer receives…"*.

Every effect in this book is therefore a **generalisation from an anecdote** — the transcriber must
decide that *"Elam can purchase disguise at normal cost"* is a rule about the kit and not a fact about
Elam. That is a judgement the other thirteen books never asked for, and it is invisible in the output:
the resulting records look exactly like every other record.

It also breaks the one heuristic this ticket trusted. The apparatus detector from
[finding 9](#finding-9--the-chapter-apparatus-recurs-in-a-second-book-and-a-second-kind) keys on a
page **describing** its fields instead of filling them; a page filling its fields **with a story** is
a third state it was never built to see.

### Finding 94 — a prerequisite on a **branch of the class**

`Branch Restrictions` is a spine field here:

> **Only forest, plains, and mountain druids** can take this kit.
> **Arctic and jungle druids cannot** take this kit.

Druid branches are a sub-classification **of the class**, the way a Subrace is of a race. So the
prerequisite tests something that is neither an ability, a level, a race, nor an Attachable the
character has — and `member` carries it only because the branches happen to be enumerable and few.

The Hivemaster goes further: *"**Gray druids** with the Hivemaster kit may assume the insectoid form
**instead of any one of their usual shapechanging choices**."* One kit, whose effect **differs by
branch**, and which **substitutes** rather than adds. Finding 89's Vindicator delta and finding 90's
precedence problem meeting inside a single sentence.

### What this book confirmed

- **Finding 86's one-directional resource conversion** — weapon slots spendable as nonweapon slots —
  appears in **three** of these fourteen records. It was one record in CPAH; it is now a device.
- The **Shapeshifter** moves a class feature's **acquisition level** from 7th to 1st, which is neither
  a grant nor an adjust: the content is unchanged and only *when* it arrives moves.
- The **Lost Druid** casts *"only the **reversed versions**"* of heal and cure spells — a
  transformation over a spell set — and *"may never attain Grand Druid status"*, a ceiling on the
  class's own hierarchy.
- The **Pacifist** addresses the player directly: *"**You, the player**, must role-play this druid as
  a strict pacifist."* The only record in 171 that does.

### Session 48 — CPAH complete. **Every kit in the v1 tier is transcribed.**

**182 records.** All fifteen Complete Paladin's kits are modelled, and with them **every kit in every
Complete handbook of the v1 tier**:

| book | kits | | book | kits |
|---|---:|---|---|---:|
| CBGH | 28 | | CBD | 23 |
| CTH | 18 | | CBH | 17 |
| CPAH | 15 | | CRH | 14 |
| CFH | 14 | | CDH | 14 |
| CBE | 11 | | CWH | 9 |

**163 kits**, plus 8 Deity records, 5 Subraces and 5 PHB tables. Ticket 13's first deliverable named
*"about 40 records"*; the pack holds four and a half times that, and the judgement pass is complete
on every one.

### A mistake worth recording, because the checker did not catch it

The first attempt at this book **attached the wrong notes to the wrong kits** — the file numbers were
guessed from reading order and CPAH's stems run `DD05397`–`DD05411` with four already-modelled records
interleaved. Nine records were built with mismatched interpretations before an unrelated crash aborted
the write.

**Nothing would have caught it.** Every record was schema-valid, its provenance pointed at the file it
came from, and only the prose in `interpretation` was wrong. That is
[finding 73](#finding-73--a-record-that-was-wrong-for-thirty-five-sessions-and-validated-throughout)'s
class again — a well-formed record that says something false — and this time it was caught by luck
rather than by a check. The redone version asserts the expected name for every file before writing.

### Finding 95 — three gaps in one clause

> An Inquisitor has an **80% plus 1%/level** immunity to illusion spells of all levels. This immunity
> **has a limit of**…

**Partial** ([finding 85](#finding-85--three-more-subjects-in-five-records)), **scaling**
([finding 26](#finding-26--level-scaling-has-three-parameters-not-one)), and **capped**
([finding 63](#finding-63--the-clamp-now-has-five-records-behind-it)) — three separately-recorded gaps
in a single sentence, in the last book. It is a useful closing datum: the gaps are not independent
features the corpus uses one at a time; **they compose**, and a repair that adds a clamp without a
scaling operand would still fail this record.

### What the paladin book added

- The **True Paladin** is the **second genuinely empty record** in 182, after the CTH Adventurer:
  `Requirements: Standard`, `Special Benefits: None`, `Special Hindrances: None`.
- `Forbidden` **and** `Restricted` both appear as **named fields** — the barred list and the
  permit-list, first-class. This is the fourth book to give `forbid` a natural home, and the only one
  to give the permit-list one too.
- The **Divinate** requires *"membership in an organized religion"*, which is a prerequisite on an
  **institution**, joining the CBE Spellfilcher's guild.
- The **Votary** must **designate a hated faith** — the ninth distinct kind of per-character parameter
  the corpus has asked for, after hated foes, familial species, sacred animals, chosen terrains,
  guarded sites, chosen undead, totem animals and bonded mounts.

## THE VERDICT

Deliverables 3 and 4. **Every figure below is produced by [`verdict.py`](../tools/verdict.py)** and
can be reproduced with `verdict.py ~/corerules/slice`. It was hand-measured at session 48 and was
wrong within two sessions; see [finding 116](#finding-116--the-verdict-was-a-number-that-goes-stale-by-itself).

Two populations, reported separately and never mixed: the **Attachables** ticket 08 defined — kits,
Deities and Subraces — and every record in the pack that carries effects at all, which now includes
races.

### The headline number

| | attachables | whole pack |
|---|---:|---:|
| records carrying effects | 237 | 270 |
| effects | 1,791 | 1,898 |
| effects expressed **without a marker** | **1,386 (77%)** | **1,483 (78%)** |
| effects carrying an `UNMODELLED` marker | 405 (23%) | 415 (22%) |
| **records complete** | **51 of 237 (22%)** | 53 of 270 (20%) |
| references resolving | — | **3,939 of 3,952 (99.7%)** |

The pack is 1,218 records; the 948 in kinds with no `effects` array — spells, proficiencies, weapons,
tables, creatures — are records, never incomplete ones, because they have nothing to express.
**The 77% has not moved in fourteen sessions and three books**, which is the most stable number this
effort has produced.
*Complete* means the record has effects, none is marked, and nobody flagged it unfinished — see
[finding 126](#finding-126--a-record-with-no-effects-was-counting-as-expressed-completely).

**The format says roughly four fifths of what the corpus says, and finishes fewer than a third of its
records.** Those two numbers point in opposite directions and both are true: the *operations* work
almost always, and the *records* rarely close, because one unsayable clause in a fourteen-effect kit
leaves the record incomplete.

For a tool whose promise is *"it tells you which rule refused and which book that rule came from"*,
the second number is the one that binds. A pack of 183 modelled records can answer that question fully
for 55 of them.

**Adding a whole non-Attachable kind moved the completion rate by nothing at all** — 30% either way.
Six races produced 101 effects and six markers, which is a better rate than any Complete handbook, and
the aggregate did not notice. That is what a 163-record majority does to an average, and it is the
reason the two populations are reported apart.

The spread by book is wide and diagnostic:

| book | records | complete | | book | records | complete |
|---|---:|---:|---|---|---:|---:|
| CBH | 17 | **76%** | | CBE | 16 | 50% |
| CDH | 14 | 50% | | CTH | 18 | 50% |
| PHB | 6 | 33% | | CFH | 14 | 29% |
| CPAH | 15 | 27% | | CBGH | 29 | 24% |
| CRH | 14 | 7% | | **CBD, CWH, CPRH** | 40 | **0%** |

A book is not a random sample of difficulty. CBH's kits are proficiencies and named abilities; CBD's
every record carries a reaction penalty qualified by clan, and CPRH's every record carries a
permit-list and a follower roster. **Three books produced no complete record at all**, and they are
the same three after 40 more sessions and two more kinds.

### Known unknown #4 — *six operations may not suffice*. **Answered: they suffice.**

| | uses | share | at session 48 |
|---|---:|---:|---:|
| `grant` | 572 | 46.8 % | 50.8 % |
| `adjust` | 275 | 22.5 % | 21.0 % |
| `set` | 145 | 11.9 % | **7.7 %** |
| `require` | 121 | 9.9 % | 10.8 % |
| `forbid` | 96 | 7.9 % | 8.6 % |
| `except` | 13 | 1.1 % | 1.2 % |

**All six are used, none was ever found missing, and in 1,222 effects not one clause needed a seventh
operation.** `except` waited 24 records for its first case and ends at 13 uses — the least needed and
not redundant, since a format that could not say *"you may take the rogue-only proficiency"* would
fail on real kits.

**The mix is a property of the kind, not of the format.** Six race records moved `set` from 7.7% to
11.9% and pushed `grant` below half, because a Kit hands you things and a Race states what a number
IS. The shares measured over kits alone were never the format's shares; they were the Attachable's.

This is the answer nobody predicted. The known unknown feared the operation set was too small; the
measurement says it is **exactly right**, and that every shortfall is elsewhere.

### Where the shortfall actually is

All 262 markers, by what the format could not say:

| | count | |
|---|---:|---|
| **conditions** | **94** | the other party, terrain, round state, campaign configuration, play history |
| **operands and values** | **61** | bare ranges, dice in effects, per-die scaling, cross-sheet arithmetic |
| **subjects** | **21** | a spell, a follower, an opponent, a second character sheet |
| shapes | 15 | permit-lists, item-property predicates |
| scopes and earmarks | 15 | *this pool, spent only on these* |
| composition | 13 | nesting, precedence, one record pointing into another |
| choices | 13 | *one or the other, but not both* |
| reductions and substitutions | 8 | *replacing them with*, *to a lesser degree* |
| caps and clamps | 7 | *to a maximum of*, *until it reaches 0* |
| frequencies and triggers | 7 | *once per week*, *on a roll of 14 or more* |
| declaring no category | 30 | |

**Conditions and subjects together are 115 of 262 — 44% of everything the format could not say.** Add
operands and the three account for 67%. The session-48 hand pass reached the same ranking with 44
markers it could not classify; the difference is [finding 115](#finding-115--the-markers-were-classifying-themselves-all-along),
not a change in the corpus.

So the verdict on [ticket 06](./06-expression-language.md)'s expression language is the mirror of the
one on §4.3. The predicate's **vocabulary** is adequate — `compare`, `member` and `has` were all
exercised and none was found wanting. What is inadequate is its **reach**: a predicate can name the
character and nothing else, and the corpus routinely conditions on the world.

### The three known unknowns

**#1 — the kit mechanism has no prior art.** §4.1's claim that Kit, Deity and Subrace are one shape
**held for 177 records across three arms**, and held without strain: the shared `attachable` never
needed a per-arm exception. One thing breaks it, and it is
[finding 90](#finding-90--a-kit-that-overrules-a-deity-with-the-book-stating-the-precedence): three
CBD kits take their weapons from the character's **Deity**, and the Vindicator overrides its religion
by name. **Two Attachables can contradict each other**, and §4.3's commutation — the property that
makes the layer model work — has nothing to say about contradiction. That is the single largest hole
the corpus found, and it is a hole in the arms' *composition*, not in their shape.

**#2 — "the Engine computes, the user supplies the tables" has no shipping precedent.** **Fired,
negatively**, at [finding 20](#finding-20--a-lookuptable-has-no-declared-role-and-its-rows-are-keyed-by-prose):
a `lookupTable` has an id derived from its source file, a name that §7.3 says is never identity, and
**no declaration of what it is a table of**. Its rows are keyed by book prose. The Engine cannot find
the table it needs, and [finding 71](#finding-71--an-effect-that-swaps-one-classs-table-for-anothers)
showed the same wall from the other side — the Swashbuckler needs to *read* the fighter's THAC0
progression and cannot name it. **This is unresolved and it is the premise the whole design rests on.**

**#4** — answered above.

### Known unknown #2, **resolved** — session 49

A table now declares **`supplies`**: the **field path it fills**, in the same vocabulary the effects
already use. A table supplying `thiefSkill` answers for `thiefSkill.openLocks`, so the Engine finds
the table it needs by naming the field it is computing.

Chosen over a second closed enumeration deliberately: §3.4 says rule-set names are the Engine's
**single** exception to open enumerations, and a role vocabulary would have made two. It also gives
A3's distinction for free — **a table that supplies no field carries no `supplies`, and the Engine
does not consume it**, which separates *reference data the pack happens to carry* from *a table the
Engine is waiting for*.

**Converting the slice's 37 rows found something the decision did not predict.** The axes are
**heterogeneous**:

| table | keyed by | columns |
|---|---|---|
| 26 Thieving Skill Base | a **skill** (id) | one value |
| 27 Racial Adjustments | a skill (id) | **races** (ids) |
| 28 Dexterity Adjustments | a **Dexterity score** (integer) | skills (ids) — *the transpose of 27* |
| 29 Armor Adjustments | a skill (id) | **armour types** (ids) |
| 30 Backstab Multipliers | a **level band** (`1-4`, `5-8`) | one value |

So `keyedBy` became a `tableAxis` — `kind` of `id`, `integer` or `range`, with the vocabulary named
when it is an id — and `columnsAre` carries the second axis when the table is two-dimensional. **One
key kind would have fitted none of the five.**

A **`tableValue` operand** closes the read half — `{supplies, of, at}`. Two effects that were marked
unmodellable are now expressed:

- the **Swashbuckler** *"fights with the THAC0 of a **fighter** of his experience level"* →
  `{supplies: "thac0", of: "phb:fighter", at: {level: "phb:thief"}}`;
- the **Explorer** learns *"**twice** the normal number of languages allowed by his Intelligence"* →
  adding the table's own value a second time, the layer model absorbing a multiplier exactly as it
  absorbed the scaling offsets in [finding 50](#finding-50--the-operand-was-never-as-poor-as-findings-21-26-and-40-said).

**Four table-dependent clauses remain**, and they are all one shape: *"the weapons listed in Table
47"*, *"Table 6 lists the oppositional schools"*, the Militarist's honours from Table 20. A table that
**bounds a choice** rather than supplying a value — which is
[finding 43](#finding-43--two-thirds-of-kits-give-examples-not-enumerations)'s `require.from` problem
meeting this one, and is left open rather than guessed at.

**The extractor cannot infer either half.** `supplies` and the row-key ids are not in the markup, so
[`extract_tables.py`](../tools/extract_tables.py) emits them empty for a human to fill — the same
posture as the apparatus list, for the same reason: **the source does not say.**

### What the slice proves, and what it does not

**Proved.** The shapes are expressible. The six operations suffice. The three Attachable arms are one
shape. Order-independence held everywhere it was tested, and the layer model repeatedly absorbed
things that looked like they needed new features — offsets as constants beside a multiple
([finding 50](#finding-50--the-operand-was-never-as-poor-as-findings-21-26-and-40-said)), exceptions
as cancelling adjusts ([finding 36](#finding-36--the-cancelling-adjust-technique-generalises)), CNF
for free on the effect side ([finding 27](#finding-27--the-effect-list-gives-conjunction-of-disjunction-for-free)).

**Not proved, and stated as plainly as ticket 08 asked.** Nothing was tested about psionics, spells as
records, equipment breadth, or the DMG. The cost per record was never measured, so
[ticket 11](./11-human-review-protocol.md)'s 5–15 minute prediction is still unchecked, and
[ticket 09](./09-extraction-pipeline.md)'s local-model draft quality was never tried at all. **A pack
that has never been loaded by anything is a demonstration, not a validation**, and the distinction is
the same one A3 exists to keep.

**Referential integrity was the loudest of these and is now half closed.** At session 48 *none of the
pack's 496 references resolved*; today **742 of 1,030 occurrences resolve (72%)**, over 177 of 284
distinct ids. What closed it was transcribing the things the kits point at — the PHB's proficiencies,
weapons, races, thieving skills and alignments.

**171 of the 288 that remain are one missing kind**: every Attachable names the class it attaches to
and `phb:fighter` does not exist ([finding 117](#finding-117--the-most-repeated-reference-in-the-pack-was-never-counted)).
The rest is a long tail of 92 ids, no single one of which is worth a session.

### Session 50 — the PHB's proficiencies, and the first references that resolve

The pack held 182 records pointing at 496 ids that nothing defined. **65 nonweapon proficiencies are
now transcribed from the PHB**, and for the first time in the effort **references resolve**:

| | before | after |
|---|---:|---:|
| records | 182 | **247** |
| distinct ids referenced | 496 | 494 |
| **ids that resolve** | **0** | **59 (12 %)** |
| **occurrences that resolve** | **0** | **261 (24 %)** |

A quarter of the pack's references now land on a record. This is the first work in fifty sessions
aimed at the **PHB** rather than at a Complete handbook, and the first at a kind that is **not an
Attachable**.

### Finding 96 — the resolution check found errors nothing else could

Running the resolution and looking at near-misses turned up **ids minted wrong while hand-modelling**:

| written by hand | the PHB's own slug | uses |
|---|---|---:|
| `phb:riding-landbased` | `phb:riding-land-based` | **9** |
| `phb:blindfighting` | `phb:blind-fighting` | 1 |
| `phb:firebuilding` | `phb:fire-building` | 1 |

Eleven references, invented across five books over dozens of sessions, each a plausible slug for a
proficiency whose real name I had never read. **Every one was schema-valid, and every one pointed at
nothing.**

This is exactly the class [finding 45](#finding-45--every-reference-in-the-proving-slice-is-dangling)
named and could not act on, because there was nothing to resolve against. **The check has now run once
and immediately paid**, which is a stronger argument for building the PHB than any completeness
argument: *the corpus cannot check itself until the things it points at exist.*

### Finding 97 — the extractor's own filter dropped two records on its first run

`Blind-fighting` and `Mountaineering` are scored **`NA NA`** in Table 37 — they have **no ability
check at all**. The first version of the filter required a named ability and **silently dropped both**.

That is [finding 82](#finding-82--one-bad-tag-drops-a-whole-record-and-six-were-lost-that-way)'s class
of error reproduced in a brand-new extractor, on its first run, by the person who wrote finding 82.
The lesson does not transfer by having been learned once; it has to be built in, and the check that
catches it is the same one — **compare what the source lists against what the extractor emitted**.

Two more things the source did to itself: it **misspells one of its own proficiencies** in the scoring
table — `Astology` in one group and `Astrology` in another, with identical scores — and it scores
**15 of 65 proficiencies in two groups at once**.

### Finding 98 — a schema field that was right in principle and wrong in arity

`nonweaponProficiency` was given `group`, `slotCost`, `abilityCheck` and `modifier` by
[ticket 14](./14-record-shapes-for-the-slice.md) **before anything was transcribed**, and those are
exactly Table 37's four columns. The shape was correct on the first guess.

`group` was a **single id**, and 15 of 65 proficiencies are in two groups — a group is the list a class
buys from at normal cost, so belonging to two is *what a crossover is*. Widened to an array.

And the validator earned its keep a second time: the first write produced **195 errors** because the
extractor emitted a bare id where the schema wanted a `scalar`, and `"+1"` where it wanted an integer.
Both were mine; the schema had been right since before the corpus was read.

### Session 51 — the rest of the PHB, and half the pack now resolves

**346 records.** Table 44's 79 weapons, the six races, the six ability scores and the eight thieving
skills — read off Table 26's own row keys so the ids match the table that scores them.

| | session 49 | session 50 | **now** |
|---|---:|---:|---:|
| records | 182 | 247 | **346** |
| reference occurrences that resolve | **0** | 261 (24 %) | **499 (47 %)** |

### Finding 99 — the pack has ten kinds and the corpus references fifteen

Classifying the 570 occurrences that still do not resolve, by the `kind` the citing effect declares:

| | occurrences | has a kind? |
|---|---:|---|
| `sphere` + `sphereMinor` | **64** | **no** |
| `nonweaponProficiency` | 39 | yes — but these are proficiencies **other books** introduce |
| `spellSchool` | 16 | **no** |
| `ability` | 12 | partly — alignments are referenced *through* an ability |
| `spell` | 11 | **no** |
| `follower` | 9 | **no** |
| `armor` | 8 | **no** |

**Five of the kinds the corpus references have no home in the schema**: priest spheres, schools of
magic, spells, armour and followers. [Ticket 05](./05-pack-schema.md) chose ten kinds for the slice
and said so; what 346 records show is that **the ten cover the Attachables and not what Attachables
point at.** Every `grant` of a sphere, a spell or a suit of armour is a reference into a kind that
does not exist — and unlike the proficiencies, more transcription will not fix it, because there is
nowhere to put the result.

The 39 proficiency references are a different and smaller problem, and one I made: they carry a
**`phb:` prefix for proficiencies the PHB does not contain** — `phb:intimidation` is the Complete
Fighter's, `phb:acting` and `phb:poetry` are the Bard's. The id says which book defines a thing, and I
guessed wrong for every proficiency a Complete handbook introduced.

### Finding 100 — the corpus names weapons in pairs, and nothing that cites them does

Table 44 prints **`Dagger or dirk`** and **`Hand or throwing axe`**. Every kit in fifteen books that
requires one says *dagger*, or *hand axe*.

Eighteen references were realigned to the book's own name — and this is the **third distinct way** the
resolution check has caught an id minted by hand, after session 50's `riding-landbased` and this
session's mis-prefixes. The pattern across all three is one sentence: **the id you write while
modelling is not the slug of the heading the book prints**, and only resolution can tell you.

`weaponProficiencies` had been declared by [ticket 14](./14-record-shapes-for-the-slice.md) with **no
fields at all**, like `races`, `abilities` and `thievingSkills`. Its shape is now Table 44's columns —
cost, weight, size, damage type, speed factor, and damage against small-medium and large targets —
which is the **second time** a kind's shape turned out to be a table's columns, after
[finding 98](#finding-98--a-schema-field-that-was-right-in-principle-and-wrong-in-arity). Six of the
79 are **group headings** — `Bow`, `Sword`, `Crossbow` — which score nothing and which the kits cite
far more often than any variant, so they are records carrying `isGroup` rather than weapons with no
cost.

### Session 52 — the five missing kinds, and three quarters of the pack resolves

| | s49 | s50 | s51 | **now** |
|---|---:|---:|---:|---:|
| records | 182 | 247 | 346 | **397** |
| reference occurrences | 1,069 | 1,069 | 1,069 | **853** |
| **resolving** | **0** | 24 % | 47 % | **74 %** |

The occurrence count **fell**, which is the session's main result rather than a rounding artefact.

### Finding 101 — 216 of the pack's references were pointing at nothing that could ever exist

[Finding 99](#finding-99--the-pack-has-ten-kinds-and-the-corpus-references-fifteen) counted five kinds
with no home. Looking at what the homeless references actually **point at** reordered the problem
entirely: the largest group was not spheres but **227 references to abilities and limitations the kits
themselves invent** — `cbgh:blend-into-underbrush`, `cth:identify-poison` — and **216 of them are used
exactly once, by the record that invented them.**

Those are not a transcription debt. **There is nothing to transcribe**: the definition *is* the kit's
own field, and giving it an id creates a reference that can never resolve no matter how much work is
done. So a `grant` or `forbid` may now carry **`defines`** — a name and the source's words — instead
of a `ref`, and the definition lives in the effect that grants it.

**216 dangling references disappeared, and the pack's total reference count fell by a fifth.** That is
the first repair in this ticket that made the problem smaller rather than moving it.

The **eleven** used by more than one record keep their ids and become records — and several turn out
not to be kit inventions at all: `phb:bard-spellcasting` is cited by **three demi-bard kits that take
it away**, and `cbh:influence-reactions` by six. A shared "kit ability" is usually **a class ability
the kits are talking about**.

### Finding 102 — sphere and school are one shape used twice

Five kinds added: `spheres` (16), `spellSchools` (9), `spells` (12), `armor`, `grantedAbilities` (11).

The first two are **the same shape**: a named category a spell belongs to, one for priests and one for
wizards, with nothing distinguishing them but which class reads them. That is §4.1's argument — *one
shape used three times* — reappearing in a corner nobody was looking at, and it is the second
independent confirmation of that design after the Attachable arms held for 177 records.

Only the **twelve spells the pack actually references** are transcribed, out of the PHB's several
hundred. A3 is what makes that legitimate rather than incomplete: the manifest declares coverage, and
**a spell nobody grants is not a debt**.

### What still does not resolve

226 occurrences, and the shape of the remainder is now clear:

- **47 nonweapon proficiencies introduced by Complete handbooks** — Intimidation is the Fighter's,
  Acting and Poetry the Bard's. Each book has its own proficiency table, and none is transcribed.
- **~80 placeholders I minted while modelling** — `cbe:melee-outside-archer-list`,
  `phb:fortification`, `crh:weapons-outside-primitive-list`. These stand where
  [finding 11](#finding-11--two-shapes-the-deity-arm-has-and-the-kit-arm-did-not)'s permit-list has no
  operation, and they will resolve when that gap does, not before.

So the remaining quarter splits cleanly into **work not yet done** and **a gap that no work closes** —
which is exactly the distinction A3 exists to keep, arrived at from the reference side.

### Session 53 — the handbooks' own proficiencies, and a file that ate another

**424 records, 77 % of reference occurrences resolving.** Twenty-seven proficiencies the Complete
handbooks introduce — Camouflage, Falconry, Trail Signs, Distance Sense — and **19 references
repointed** from the `phb:` prefix I had guessed to the book that actually defines them.

Three books print a **compiled table in exactly PHB Table 37's four columns**, abbreviating the
ability: the Bard's, the Paladin's and the Ranger's. The same reader parses all three. Those tables
**restate the PHB's 170 proficiency rows** alongside the new ones, and restating is not redefining —
a row the PHB already owns is skipped rather than minted under a second id, which is the whole point
of the exercise: **one thing, one id, in the book that introduced it.**

### Finding 103 — two files claimed the same kind and one silently ate the other

The pack now has two files contributing `nonweaponProficiencies`: the PHB's 65 and the handbooks' 27.
The loader did `doc.update(...)` per file, so **the second replaced the first** — and the record count
came back **397, exactly what it had been before 27 records were added**. Sixty-five records vanished
and the number stayed plausible.

[Ticket 05](./05-pack-schema.md) settled that the manifest **declares its files** and §7.1 chose
declaration over discovery, and neither says what happens when two of them speak about the same kind.
It is the obvious thing to want — the PHB's proficiencies and a handbook's are the same kind from
different books — and the format had no rule for it.

**Arrays merge; they do not overwrite.** Fixed in the checker, and worth stating as a format rule
rather than a bug fix, because a pack assembled from a dozen books will do this constantly.

That the count *stayed plausible* is the part worth keeping. A loss of 65 records showed up as no
change at all, and only adding 27 and seeing zero growth exposed it. It is
[finding 81](#finding-81--a-third-label-convention-and-an-omission-with-no-symptom)'s omission with no
symptom, this time inside the tooling rather than the corpus.

### What is left

226 occurrences, and the tail is now short and legible:

- **25 proficiencies** from CTH and CBD, whose books do not print a Table 37-shaped list — CTH gives
  three columns of bare names, CBD scores only its own detection proficiencies. Their scores are in
  the descriptions, one page at a time.
- **~80 placeholders** standing where finding 11's permit-list has no operation, unchanged.
- **12 `follower` references**, which finding 99 already showed are not a missing kind: they point at
  classes and at creatures.

### Session 54 — the Thief's own proficiencies, and a modifier that is not a number

**435 records, 79 % resolving.** The Complete Thief's Handbook gives each of its proficiencies a page
whose **first line is the score**, in a form regular enough to parse:

> **Observation** 1 slot, Intelligence, 0 modifier. **Required:** Beggar, Cutpurse, Investigator, Spy,
> Swindler, Troubleshooter. **Recommended:** Assassin, Bounty Hunter, Burglar, Fence, Smuggler.

That second sentence is a **reverse index** — the proficiency listing the kits that need it — which no
other book prints and which nothing in the pack format has a place for. It is the same fact the kits
already state, written from the other end, and it would make an excellent consistency check: *does
every kit this page names actually require it?*

### Finding 104 — the modifier is not always a number

Five of the Thief's proficiencies score **`special modifiers`** rather than a value, and Intimidation
scores **`ability special, special modifier`** — neither the ability nor the modifier is fixed, and
the description sets out the circumstances instead.

The schema had `modifier: integer`, written from PHB Table 37 where every cell is numeric. It now
admits three states, and they are genuinely different:

| | means |
|---|---|
| an integer | the modifier is this |
| `"special"` | the book says the modifier depends on circumstances it describes |
| **absent** | there is no check at all — the PHB's Blind-fighting and Mountaineering |

Collapsing `special` into `0` would have been the natural sloppy move and would have asserted
something the book does not say. This is
[finding 2](#finding-2--the-schema-accepted-a-semantically-empty-record-fixed)'s distinction a third
time: *stated as absent*, *stated as variable*, and *not stated*.

### Finding 105 — two books introduce the same proficiency, and the id must pick one

`Alertness` and `Boating` are defined by **both** the Complete Thief's Handbook and the Complete
Ranger's, with **identical scores** — 1 slot, Wisdom, +1.

[Ticket 07](./07-identity-and-id-stability.md) derives identity from source position, and the pack
prefix says **which book defines a thing**. When two books define the same thing, the prefix is a
**choice, not a fact**: these carry `crh:` because the Ranger's compiled table was parsed first, and
the Thief's Handbook is four years older and has the better claim.

Left as it is, and recorded, because the alternative — renaming after the fact — is exactly the churn
[finding 96](#finding-96--the-resolution-check-found-errors-nothing-else-could) showed is invisible
until something resolves. But it means **a pack's ids encode the order its books were transcribed in**,
which nothing in ticket 07 intended.

The same session also found the corpus using **two names for one proficiency**: the Assassin's list
says *Gather Intelligence* and the page is titled *Information Gathering*. Both were referenced, and
both now point at `cth:information-gathering`.

### Session 55 — the dwarves' proficiencies, and a book that rewrites the core rules

**451 records, 81 % resolving.** Sixteen proficiencies the Complete Book of Dwarves adds, including
its optional **detection** set — Grade or Slope in Passage, Sliding Walls, Stonework Traps — which is
an alternative to the PHB's own dwarf-detection system rather than an addition to it.

### Finding 106 — CBD re-scores **28** of the PHB's proficiencies

Its table restates the PHB's list, and the restatement is **not** a restatement. Comparing the 75 rows
against the PHB's:

| | count |
|---|---:|
| identical to the PHB | 31 |
| **scored differently** | **28** |
| new | 16 |

And the differences are not noise:

| proficiency | PHB | CBD |
|---|---|---|
| Armorer | 2 slots, −2 | **1 slot, 0** |
| Stonemasonry | 1 slot, −2 | **1 slot, 0** |
| Mining | 2 slots, −3 | **1 slot, 0** |
| Gem Cutting | 2 slots, −2 | **1 slot, 0** |
| Riding, Land-Based | 1 slot, **+3** | 1 slot, **−2** |
| Swimming | 1 slot, 0 | 1 slot, **−1** |
| Musical Instrument | 1 slot, −1 | 1 slot, **−2** |

Dwarves are cheaper and better at stone, metal and gems, and worse at riding, swimming and music.
**This is a race book overriding the core rules for its race**, and the pattern is too consistent to be
transcription noise.

The consequence is structural. `phb:armorer` is **one id with one score**, and a dwarf's Armorer costs
and modifies differently — so **the same referenced thing has two values, and which applies depends on
the character's race.** That is
[finding 90](#finding-90--a-kit-that-overrules-a-deity-with-the-book-stating-the-precedence)'s
contradiction between two Attachables, arriving one level down: not two records disagreeing about a
character, but **two books disagreeing about a record**.

Nothing was merged and nothing was overwritten. The 28 are recorded here and the PHB's values stand,
because the alternative — minting `cbd:armorer` beside `phb:armorer` — would say there are two
proficiencies, and there is one. What the format lacks is a way to say *"the same thing, scored
differently for these characters"*, and the layer model is the obvious place for it: a **Race with
effects** would adjust `proficiency.armorer.slotCost` the way a Kit adjusts anything else. `races` is
currently a kind with **no fields at all**.

### The remaining tail, in full

166 occurrences, and none of them is a transcription debt any more:

| | |
|---|---|
| ~80 | placeholders standing where [finding 11](#finding-11--two-shapes-the-deity-arm-has-and-the-kit-arm-did-not)'s permit-list has no operation |
| 12 | `follower` refs pointing at classes and creatures — not a missing kind |
| 8 | `grantedPower` — the Deity arm's Powers field, which is prose |
| 7 | proficiencies from books with no scoring table at all |

**Every proficiency the pack references and any book scores is now transcribed.**

### Session 56 — races get effects, and an effect gets its own provenance

[Finding 106](#finding-106--cbd-re-scores-28-of-the-phbs-proficiencies) left the Complete Book of
Dwarves' 28 re-scorings recorded in prose and absent from the pack. They are now **32 effects on
`phb:dwarf`**, and putting them there took two schema changes that turn out to be one idea each.

### Finding 107 — a Race carrying effects is not a new idea; it is one the schema withheld

§3.1 says **a Subrace is a Race with a parent reference**, and a Subrace is one of §4.1's three
Attachable arms — with a target, a prerequisite and effects. So a Race with effects is **the same idea
the schema never granted the parent**, and `races` had been a kind with no fields at all.

A Race gets **effects and `effectsModelled`, and neither target nor prerequisite**: it is not attached
to a character, it is one of the things a character *is*. Half an Attachable, and the half that was
missing.

What makes it work is §4.4. The dwarves' book does not add a second Armorer proficiency — it says a
dwarf's Armorer costs one slot rather than two — so the effect is
`set proficiency.armorer.slotCost to 1`, **a layer over the PHB's value in exactly the way the layer
model was designed for.** The character's view of a proficiency's cost is a stack, and the race
contributes to it. No new mechanism was needed; the mechanism had simply never been pointed at a race.

### Finding 108 — a record's effects are not all from one book

`phb:dwarf` is the **PHB's** record. Thirty-two of its effects are the **Complete Book of Dwarves**
rewriting the PHB's numbers. Written without qualification, the pack would assert that the core rules
say something they do not.

So an **effect may now carry its own `provenance`**, present only when it differs from the record's.
[Ticket 05](./05-pack-schema.md) put provenance on the record because a record comes from a passage;
what 451 records show is that **a record can be a meeting place for several books**, and the finest
grain that needs an anchor is the effect, not the record.

This also gives [ticket 12](./12-how-much-tool.md)'s review page the thing it would otherwise lack: a
reviewer looking at `proficiency.riding-land-based.modifier = −2` can be sent to the dwarves' book
rather than to the PHB, where that number is +3 and the reviewer would conclude the pack was wrong.

### The five races that now say they are unfinished

`effectsModelled: false` reappears for the first time since [session 37](#session-37--the-judgement-pass-is-complete-and-it-found-a-record-that-was-simply-wrong)
— on the elf, gnome, half-elf, halfling and human, whose PHB racial abilities are not transcribed.

That is the flag doing precisely its job. Giving races effects **created five honestly incomplete
records where there had been six silently empty ones**, and the pack now distinguishes *"this race has
no effects"* from *"nobody has transcribed them yet"* — which is [finding 2](#finding-2--the-schema-accepted-a-semantically-empty-record-fixed)'s
whole argument, arriving in a kind that did not exist when it was made.

### Session 57 — the six races' own entries, and what a race asks for that a kit never did

The five `effectsModelled: false` races are transcribed, and so is the dwarf's **own** entry — which
had been missing. **69 PHB effects: 40 `adjust`, 27 `set`, 2 `grant`.** Table 9 joins the pack as a
sixth lookup table. 452 records, 0 schema errors, no new unresolved references, and the judgement-pass
line is gone.

Races turn out to exercise parts of the format that 1,121 kit effects never touched.

### Finding 109 — `dice` was a value type that nothing was allowed to hold

Every demihuman detection ability is one shape: **`1-5 on 1d6`, `1-7 on 1d10`, `1 on 1d6`** — and the
pack had nowhere to put it. `operand` was `integer | computedOperand | tableValue`, and
[finding 42](#finding-42--a-decision-was-settled-and-the-artifact-was-silent)'s `dice` pattern was a
value type **no kind referenced** — a fact that finding recorded without being able to explain.

This is why. The corpus does not state these as numbers, and **writing 83% where the book writes
`1-5 on 1d6` is inference**, which A3 forbids; the die and the target are what the source supplies. So
`operand` gains a fourth arm, `{rollAtMost, on}`, and **`dice` finally has a consumer.**

**17 occurrences across six records. Zero in 1,121 kit effects** — because a kit's bonuses are flat
modifiers on rolls someone else defines, while a race's abilities *are* rolls. The proving slice was
built entirely out of Attachables, and this is the first thing measured from outside that shape.

### Finding 110 — the field path is where the untyped vocabulary collects

The dwarf hits orcs, half-orcs, goblins and hobgoblins at +1; ogres, trolls, ogre magi, giants and
titans attack him at −4. **`when` cannot say any of that**, and not by oversight: a clause describes
**the character** — his abilities, his level, what he has — and never his opponent. The subject of a
condition is a `scalar`, and a scalar is an ability or a level.

So the target moves into the **field path**: `attackRoll.melee.vsOrc`, `opponent.attackRoll.titan`.
That is not new — `reactionCheck.speciesEnemy` and `thac0.weaponOfChoice` did it already — but the
races make the scale visible. **Fourteen creature names now live inside field-path strings**, and the
pack has no kind that holds a creature, so nothing checks them, nothing resolves them, and a typo is
invisible.

The path is a **string on purpose** — it is how a v1 pack talks about fields the Engine knows and the
schema does not enumerate. What this session shows is that **it is also the escape hatch**, and that
the Engine's real vocabulary is therefore much larger than the schema's: 41 distinct paths from six
records. It is not a defect to repair here; it is a **cost to state**, and correction 15's argument
about a closed field vocabulary now has its strongest evidence.

### Finding 111 — a cascade of chances is not a chance

A halfling has a **15% chance of infravision to 60 feet; failing that, a 25% chance of it to 30 feet.**

A *single* chance has a home — the dwarf's `magicalItem.malfunctionChance` is a field like any other,
and four kits already do this. **A fall-through of two does not.** `when` cannot test a die roll, and
two `set`s on one path would *sum* rather than *fall through*, because that is what the layer model
guarantees. Written as fields it would assert something false.

So it is a `grant` with a `defines` carrying the source's own sentence, marked UNMODELLED — and it is
the **first effect in 1,100+ whose structure, not merely whose scope, the six operations cannot
carry**. It does not reopen [known unknown #4](#known-unknown-4--are-six-operations-enough): no
seventh *operation* is missing. What is missing is a way to **sequence** two effects, and §4.3's
commutativity is exactly the property that forbids it. This is the first case where that guarantee
costs something measurable.

### Finding 112 — `effectsModelled` is one bit per record, and a record can span two books

[Finding 108](#finding-108--a-records-effects-are-not-all-from-one-book) let effects carry their own
provenance. It follows immediately that **the flag cannot mean what it says**: `phb:dwarf` was written
`effectsModelled: true` with 32 effects, **none of them from the PHB** — the book the record is
provenanced to. The pack asserted the dwarf was transcribed while its own entry was untouched.

Corrected by transcribing it — 20 more effects, and the note on the record now says both books are
present. But the flag is still one bit over a record that may draw on several sources, and
[finding 2](#finding-2--the-schema-accepted-a-semantically-empty-record-fixed)'s distinction only
holds **per book**. Recorded rather than repaired: no record outside `races` has this shape yet.

### What the races cost, against the kits

| | kits | races |
|---|---|---|
| records | 163 | 6 |
| distinct field paths | 80 | 41 |
| operations used | 6 | 3 |
| schema changed | — | **3 times in 2 sessions** |

Six records asked for **half as many field paths as 163 kits**, and moved the schema three times in
two sessions — races carry effects, an effect carries provenance, an operand carries a die. Kits are
variations on one shape and the format was designed around them; races are a different shape, and the
proving slice never proved anything about it. **The v1 format's coverage claim should be read as
'Attachables' and not as 'the rules'.**

### Session 58 — alignment, and a kind that is a shape rather than a label

The last of [finding 99](#finding-99--the-pack-has-ten-kinds-and-the-corpus-references-fifteen)'s four
homeless kinds. Nine records, one page. **461 records, 0 schema errors, and reference resolution goes
from 81% to 86% of 853 occurrences** — the single largest jump any one kind has produced.

### Finding 113 — the nine alignments are a product, and the record says so

The obvious transcription is nine labels. The PHB opens the chapter differently: alignment *"is
divided into two sets of attitudes: order and chaos, and good and evil"*, and the nine are what
**combining** them produces. So the record carries **`ethos` and `morality`**, and the label is what
falls out.

The kind has **no `effects` array**, and that is a claim, not an omission: nothing in the alignment
chapter is a mechanic. It is the second kind after `abilities` whose entire purpose is **to be
conditioned on** rather than to do anything — which is why `phb:alignment` appears in 14 prerequisites
as a pseudo-scalar and never once as a target.

`phb:true-neutral` keeps its name. Every other combination is written as its two axes and this one is
not, so the id follows the source rather than the pattern.

### Finding 114 — the axes paid for themselves in one query

With the axes present, every alignment prerequisite in the pack can be **tested against them** rather
than read. Fourteen clauses:

| what the clause actually is | clauses |
|---|---:|
| an axis slice — *any good*, *any chaotic*, *any neutral* | 7 |
| the complement of one — *any non-lawful* | 1 |
| all but one alignment | 2 |
| a genuinely explicit list | 4 |

**Ten of fourteen say something a sentence could say**, and all ten are stored as enumerated ids
because `anyOfIds` is the only membership the format has.

The nuance is what makes this worth recording. **Four of them are the pack's expansion of a rule the
book stated** — the Jester's *any chaotic*, the Feralan's *cannot be lawful*, the Gallant's and
Gypsy-bard's *cannot be X* — and all four already carried an interpretation note saying so, which is
the transcription practice holding up under a check it was never designed for. **The other six are the
book's own enumeration**: the Complete Priest's Handbook writes *"his priests may be chaotic good,
neutral good, or lawful good"* where the Complete Bard's writes *"any chaotic"*. So the pack is
faithful wherever the source enumerates and lossy exactly where the source generalises, and until now
nothing could tell those two apart.

This does not reopen the condition vocabulary — [the verdict](#the-verdict) already counts conditions
and subjects as the format's largest shortfall, 109 of 258 unmodelled clauses, and this is one more
instance of it. What is new is that **the pack can now measure its own lossiness on this axis**, which
is what a shape buys over a label.

### Session 59 — the verdict becomes a program

The `## THE VERDICT` section above is rewritten from
[`tools/verdict.py`](../tools/verdict.py), and every number in it now comes from one command.

### Finding 115 — the markers were classifying themselves all along

Session 48 classified 258 markers by hand into nine buckets and left **44 unclassified**. Rebuilding
that classification as a tool, the first attempt was a keyword regex over the marker prose — and it
was worse: 74 unclassified, and *conditions* over-counted because `only if` appears in explanations of
things that are not conditions.

Then: **the markers already carry a category.** They are written `UNMODELLED CONDITION:`,
`UNMODELLED VALUE AND SUBJECT:`, `UNMODELLED SCOPE:` — **232 of 262 declare one**, in 47 distinct
labels that collapse to ten categories. That label was written *at the moment of the refusal, with the
source open*, which is strictly better evidence than prose read back months later.

The lesson is the one this ticket keeps relearning: **read the artifact, not the prose about the
artifact** — [finding 50](#finding-50--the-operand-was-never-as-poor-as-findings-21-26-and-40-said)'s
mistake and [finding 42](#finding-42--a-decision-was-settled-and-the-artifact-was-silent)'s, arriving
a third time in the measurement layer. A convention nobody designed had been carrying the answer for
fifty sessions.

### Finding 116 — the verdict was a number that goes stale by itself

Session 48's verdict was correct when written and **wrong two sessions later**. It said *none of the
pack's 496 references resolve*; by session 50 a quarter did, and by session 58, 86%. It reported
`set` at 7.7% of effects, a share the races moved to 11.9%. It reported 863 clean effects and 52
complete records, which session 49's own `tableValue` work — the Swashbuckler and the Explorer, both
recorded in this ticket — moved to 865 and 53 **without anyone noticing that the verdict above them
now disagreed**.

None of those drifts was a mistake. Each was **the effort working**, and the number sitting in a
markdown heading could not follow. **A measurement that takes an afternoon and rots by itself is a
tool that was never written**, and the same argument [ticket 12](./12-how-much-tool.md) makes about
checking applies to measuring: if it matters enough to state, it matters enough to re-run.

Running it now also **validates the hand pass** rather than replacing it: on the same population the
tool reproduces session 48 to within two effects and one record, and the entire difference is
explained by work this ticket recorded at the time. The measurement was sound; only its shelf life was
wrong.

### Finding 117 — the most repeated reference in the pack was never counted

Writing [ticket 16](./16-the-plan-for-the-remaining-books.md) meant asking what the pack still points
at. It points, 177 times, at **`target`** — the class a kit attaches to, the race a subrace belongs to
— and the checker **never walked that field**. It walked effects and prerequisites and stopped.

**171 of those 177 resolve to nothing**, because `phb:fighter` does not exist. So the figure this
ticket published two sessions ago, *86% of 853 occurrences*, was measured over a population that
excluded an Attachable's **defining** reference. Corrected: **742 of 1,030, or 72%**, and the verdict
above now says so.

The blind spot has a cause worth naming. `target` is not *in* the effect list — it is the field that
says who the effects apply to — so a walker written by reading the `effect` schema never sees it. The
same one-level-up omission as [finding 35](#finding-35--the-extractor-flattens-away-the-level-at-which-the-corpus-marks-force),
where sub-labels were invisible because the extractor read one level of markup. **A tool that reads a
structure by walking one of its parts will silently be right about that part.**

And the number it was hiding is the plan's headline: **one missing kind accounts for 59% of everything
that does not resolve.**

### Session 60 — classes, and the reference report finally comes back green-ish

[Ticket 16](./16-the-plan-for-the-remaining-books.md)'s decision 2, taken. **19 class records and four
experience tables**, and reference resolution goes **from 72% to 89% of 1,034 occurrences**. Every
`target` in the pack now lands on a record. 484 records, 0 schema errors.

### Finding 118 — a hit die is the die itself, and `dice` still had no arm for that

[Finding 109](#finding-109--dice-was-a-value-type-that-nothing-was-allowed-to-hold) gave `dice` a
consumer as a **threshold** — `1-5 on 1d6`, success at or below. A hit die is the die as a **value**:
*"all warriors gain one ten-sided Hit Die per level"*. Four sentences, one per group, and `operand`
could not hold any of them.

So `operand` gains a fifth arm — the bare `dice` string — and `set hitDice.perLevel to "1d10"` is
sayable. The pattern that [finding 42](#finding-42--a-decision-was-settled-and-the-artifact-was-silent)
found unreferenced is now referenced twice, in the two ways a die is used: **rolled against a target,
and rolled for its result.** Neither was reachable from the kit slice, because a kit adjusts what a
class already establishes.

### Finding 119 — a class is two layers, and the corpus uses both interchangeably

Kits target `phb:priest` 22 times and `phb:cleric` 3. Both are real: the PHB has **four groups**
(warrior, wizard, priest, rogue) and **nine classes inside them**, and a kit attaches to whichever the
book names.

That is **the third appearance of the same group/member split** — Table 44 gives weapons `Bow` and
`Long Bow`, Table 37 gives proficiencies a `group` array, and now classes. Three unrelated chapters,
one shape. It is strong evidence that the split belongs in the *format* rather than being re-derived
per kind, and it arrived only because a kind was transcribed from the PHB rather than from a Complete
handbook.

The layers are not decorative. **The group owns the experience table and the class indexes a column
of it** — Table 14 is the Warrior's, and inside it Fighter and Paladin/Ranger advance on different
numbers while sharing the `d10` hit die. So *"which table"* is a group question and *"which column"* is
a class one, and a design that flattened the two would have to duplicate the table four times or lose
the hit die.

Like a race, a class has **no target and no prerequisite**: it is not attached to a character, it is
one of the things a character is. `phb:multi-class` and `phb:dual-class` are **not** classes and stay
where they were — the Witch forbids them as a `classArrangement`, which is a fact about how classes
combine, not a class.

### Finding 120 — a printed table can have two column vocabularies at once

The four experience tables are transcribed and **carry no `supplies`**, so the Engine will not consume
them. That is deliberate and it is a gap, not a shrug.

`columnsAre` (correction 16b) assumes the columns are one vocabulary — Table 27's columns are races,
Table 29's are armour types. Table 14's columns are **`Fighter`, `Paladin/Ranger`, `Hit Dice (d10)`**:
two of them name classes, one names a derived statistic, and **one column names two classes at once**.
There is no axis kind for that, and inventing one to fit four tables would be guessing.

So the schema's own rule applies — *a table that supplies no field carries no `supplies`, and the
Engine does not consume it* — and the tables sit in the pack as reference data with provenance, which
is what A3 is for. **This is the first table the pack holds and cannot use**, and it is the shape
[ticket 16](./16-the-plan-for-the-remaining-books.md)'s decision 2 will have to finish.

### The classes are records, not yet mechanics

**Fifteen of the nineteen carry `effectsModelled: false`.** The four groups are modelled — their pages
state a hit die, a cap and a per-level remainder in one sentence each — and the nine classes and five
multi-class arrangements are not, because a class's substance is a chapter: spell progressions, turning
undead, thieving skills scored across four tables, followers and strongholds.

That is the flag working exactly as [finding 2](#finding-2--the-schema-accepted-a-semantically-empty-record-fixed)
intended. **171 references now resolve to a record that honestly says it is unfinished**, which is
strictly better than 171 references resolving to nothing, and it is what makes the pack loadable.

### Session 61 — ticket 16's remaining three decisions, taken

**972 records, 0 schema errors, and 97% of 993 reference occurrences resolve.** The pack more than
doubled: 470 spells, 23 secondary skills, 6 limitations, Table 7 and the four experience tables.

### Finding 121 — the spell corpus is regular in its fields and irregular in its markup

470 pages carry the six printed fields without exception. Everything that went wrong went wrong in the
**markup**, and every one of them silently truncates:

| | |
|---|---|
| a long value wraps into the **next row's** cell with no label | `Area of Effect: 1 creature or` / `object per 2 rds.` |
| the school parenthetical has **four** markup variants | `<B>(X)</B>`, `<B> <P></P> (X)`, `<B>(X) <P></P> </B>`, and one page on a different RoboHELP template using `COLOR="RED"` |
| five pages write the field label **singular** | `Component:` for `Components:` |
| one title omits the space | `--2nd Level`, worth one whole spell |

The first regex written for each of these was the one that fits the commonest case, and each missed
between 1 and 12 records. **This is the map's method note arriving for the sixth time**, and it is now
worth stating as a rule: on this corpus, *a first-pass regex is a hypothesis about a convention the
book never promised to keep.*

### Finding 122 — the corpus names its own schools two different ways

The chapter fixes nine schools. **The spell pages abbreviate them**: `Evocation` for
Invocation/Evocation, `Conjuration` for Conjuration/Summoning, `Phantasm` for Illusion. And **43 of
303 wizard spells belong to more than one school**, so the field is a list, not a value.

Two things make this recordable rather than guessable:

- **`Divination` resolves by the book's own rule.** DD01473 says lesser divination is every divination
  spell of 4th level or less and greater divination is 5th or higher. So the spell's own level
  decides which of the two ids it gets, and **nothing is inferred** — the rule is printed.
- **The printed string is kept.** `schoolAsPrinted` holds the parenthetical verbatim beside the ids,
  because the mapping from `Evocation` to `phb:invocation-evocation` is a *reading* and a reviewer has
  to be able to check it. Same for `Elemental (Fire)`, whose four sub-spheres have no records: the
  parenthetical survives and the id is the sphere.

`All Schools` — one spell — maps to nothing, and is left that way.

### Finding 123 — 30 spell names are two different spells

`Know Alignment`, `Detect Magic`, `Gate`: **30 of 440 names are both a wizard and a priest spell**, and
none collides inside a class. Three of them were already in the pack **as bare names**, referenced by
kits — `phb:detect-magic` named two spells and nothing could tell, because neither existed.

So the id is qualified by the caster **always**, not only on collision. Qualifying only on collision
would rename `phb:bless` the day a wizard Bless is transcribed, which is exactly the instability
[ticket 07](./07-identity-and-id-stability.md) exists to prevent. The eleven references were rewritten,
and the caster was **derivable from the referring record's own target** in every case: a ranger and a
paladin cast priest spells.

### Finding 124 — a complement is not a record; it is a definition

[Ticket 16](./16-the-plan-for-the-remaining-books.md)'s decision 4, and it turned out to have a clean
answer. Of 56 dangling effect references, **33 were the complement of a set the record itself
states** — `weapons outside the Explorer's list`, `armour other than leather`, `metal weapons larger
than a knife`. There is nothing in any book to point at, and there never will be. They are now
`defines` — [finding 101](#finding-101--216-of-the-packs-references-can-never-resolve-and-should-not-exist)'s
mechanism, which the transcriber had available and reached for an id instead.

But **`except` alone among the six operations has no `defines`**, and that is not an oversight: its
whole meaning is to lift a restriction that exists *somewhere else*. Eleven effects use it, and the
somewhere-elses are real PHB rules — the thief's weapon list, the wizard's, the two-weapon penalty. So
a small `limitations` kind, six records, owned by the class that imposes them. **The operation's shape
told us which repair each reference needed**, which is the strongest argument yet that the six are
carved at the joints.

### Finding 125 — the resolution check paid for the fifth time

Fixing the references surfaced **seven more hand-minted slugs that pointed at nothing**:
`phb:farming` for `Farmer`, `phb:bowyer` for `Bowyer/Fletcher`, `phb:tailor` for `Tailor/Weaver`,
`phb:limner`, `phb:trapper`, `phb:woodworker`, `phb:staff` for `Quarterstaff`, `phb:war-hammer` for
`Warhammer`. Every one plausible, every one schema-valid, every one wrong —
[finding 96](#finding-96--the-resolution-check-found-errors-nothing-else-could)'s class, five sessions
later, in a fifth kind.

`phb:axe` is **left unresolved on purpose**: Table 44 has a battle axe and a hand axe and no plain axe,
so which one a Gnome Fighter kit meant is not recoverable from the id.

### Finding 126 — a record with no effects was counting as expressed completely

Adding nineteen class records moved the whole-pack completion rate from **30% to 35%** while adding no
expression whatsoever: fifteen of them have an empty `effects` array, carry no marker, and therefore
passed the test for *"no marker anywhere"*.

The metric was measuring **the absence of a complaint** rather than the presence of an answer. Fixed
in [`verdict.py`](../tools/verdict.py): complete now means the record **has** effects, **none** is
marked, and **nobody flagged it unfinished**. The corrected rates are **29% of attachables and 26% of
everything modelled** — and the honest reading of the drop is that the earlier numbers, including
session 48's hand pass, were about two records too kind.

**This is [finding 116](#finding-116--the-verdict-was-a-number-that-goes-stale-by-itself) paying off
two sessions after it was written.** A verdict in a markdown heading could not have caught this; a
verdict that re-runs on every pack change produced a 5-point jump that was obviously wrong on sight.

### Session 62 — the Complete Priest's 51, modelled by a program

**1,023 records, 0 schema errors, 98% of 1,985 reference occurrences resolve.** The Complete Priest's
Handbook is complete: 59 priesthoods, 784 effects.

### Finding 127 — three more records dropped for one unmarked label

The plan said 48 priesthoods remained. There were **51**. `Healing`, `Sun` and `Thunder` print
`Duties of the Priest` in their text with **no `<I>` markup on that one label**, while carrying nine of
the ten fields — so the marker test dropped all three.

This is [finding 82](#finding-82--the-marker-test-drops-a-record-for-one-missing-label)'s class in a
**fourth** book, and the mechanism finding 82 built — `INCLUDE`, keyed by page — is what fixes it.

The same pass found `DD05544`, titled `Druid`, which is **not** a priesthood: it is a cross-reference
paragraph saying the Druid is detailed in the PHB. It was already absent because it carries no labels
at all, and it is now in `EXCLUDE` — **an absence that was accidental is now declared**, which is A3
applied to the extractor rather than to the pack.

### Finding 128 — the markup loses individual labels, not whole pages

Chasing the three led to the general case. Across the 59 records, **26 field instances are missing from
the markup — and every single one is present in the plain text**, in 19 of the 59 records:

| field | lost | in the text |
|---|---:|---:|
| Nonweapon and Weapon Proficiencies | 7 | 7 |
| Weapon and Armor Restrictions | 4 | 4 |
| Powers | 4 | 4 |
| Alignment | 4 | 4 |
| Duties of the Priest | 3 | 3 |
| Possible Symbols, Other Limitations, Races Allowed | 4 | 4 |

A 100% recovery rate is the whole finding: **the `<I>` tag is unreliable per label, not per page**, so
a reader that trusts it loses a scatter of fields across a third of the book and nothing looks wrong.
Four records had **no Alignment field at all** by that route, which would have silently produced four
priesthoods with no alignment requirement.

The modeller therefore reads the plain text first and lets the markup override it. That is not a
heuristic: it is **the same label the book printed**, which is exactly the argument finding 82 made one
level up.

### Finding 129 — the first tool that does the judgement half

[Ticket 09](./09-extraction-pipeline.md) split the pipeline: the extractor does the mechanical half and
turning field prose into §4.3 effects is judgement, done by a human. [`model_deities.py`](../tools/model_deities.py)
does the judgement half for one book, and the reason it can is specific and worth stating:

**the Complete Priest's is the only book whose records share one shape.** 59 entries, the same ten
fields, in the same order, phrased the same way — `Major Access to A, B, C`, `Wisdom 16 means +10%
experience`, `The followers are received at 9th level`. Where a kit's `Special Benefits` is free prose
that could say anything, a priesthood's `Spheres of Influence` is a form.

So the mapping from field to effect is **a rule and not a reading**, and every rule was taken off the
eight records modelled by hand in sessions 44-46 and applied unchanged. Two are worth naming:

- **`Wisdom or Constitution 16 means +5%; Wisdom and Constitution 16 means +10%`** becomes two `+5`
  layers, the second conditioned on both. I checked this expecting the hand pass to be wrong and it is
  **right** — the second layer sums onto the first exactly when the book says +10%, which is §4.4 used
  precisely as designed.
- **`any lawful alignment`** is expanded through the alignment records' own `ethos` and `morality`
  rather than a name list — [finding 114](#finding-114--the-axes-paid-for-themselves-in-one-query)'s
  axes, three sessions later, doing work rather than being measured.

### Finding 130 — what a rule cannot do, it marks

**21 of the 51 carry an interpretation note**, and they are two shapes the format has no room for:

- **13 `Option:` clauses** — *"evil priests can substitute major access to Healing for major access to
  Protection, but can only use the reversed versions"*. A choice between two grants, which is
  [correction 25](../map.md)'s missing sequencing seen from another side.
- **11 accesses restricted in place** — *"Elemental (the priest may only use spells whose names include
  Fire, Flame, Heat, Pyrotechnics)"*. The grant is to a sphere and the restriction is a **predicate
  over the spells inside it**, which is the permit-list problem one level down.

Neither was invented by the modeller and neither is guessed at. **A program that must state its
refusals writes better markers than a human doing it fifty-one times**, because it cannot get bored.

### Finding 131 — a uniform modeller distorts the marker histogram

Adding one book moved `grant` from 46.7% to **61.2%** of all effects and pushed *shapes* to 87 markers
and *composition* to 64 — because the modeller applies the same two markers, the permit-list and the
follower roster, to **all 51 records**.

Those markers are true one at a time. In aggregate they now say more about **how many priesthoods the
book has** than about how often the format fails. The verdict's marker table is a census of the
corpus's fields as much as of the format's gaps, and that was already half true of the hand pass — this
just makes it impossible to ignore.

**The completion rate fell from 29% to 22%** for the same reason, and the fall is honest: the Complete
Priest's was already 0-of-8 complete and is now 0-of-59. **No book in this corpus resists the format
harder**, and it is the one book a program could model.

### Session 63 — the five proficiency books, four of which have none

**1,094 records, 0 schema errors.** [Ticket 16](./16-the-plan-for-the-remaining-books.md)'s item 5 said
five books had unread proficiencies. The measurement says otherwise, and chasing it opened a hole in
the checker that had been widening for thirteen sessions.

### Finding 132 — four of the five books have no proficiency chapter at all

The Complete Book of Elves, the Gnomes and Halflings', the Druid's and the Wizard's: **no proficiency
table, no new proficiency, nothing to transcribe.** The Elves' one page on the subject is advice about
which *existing* proficiencies suit an elf; the Druid's one page expands Agriculture procedurally.

The Fighter's does have a chapter, and what is in it is **not a nonweapon proficiency**: three PHB
proficiencies expanded into crafting procedures, and a **weapon-group system**. So the plan's item 5
was a block of five and is a block of one — and the one contains something the plan did not know was
there.

### Finding 133 — a group you can buy is not a group that is printed

Table 44's `isGroup` is a **printing convention**: `Bow` heads its variants and scores nothing. The
Complete Fighter's makes grouping a **rule** — a Tight Group costs **two** weapon proficiency slots and
a Broad Group **three**, and a third list names the weapons that belong to no group and must be bought
one at a time.

So `weaponProficiency` gains `groupKind`, `slotCost` and `members`: **20 groups over 143 members**,
which cross books — a Fencing Blades group holds Table 44's dagger beside this book's rapier. It is the
group/member split of [finding 119](#finding-119--a-class-is-two-layers-and-the-corpus-uses-both-interchangeably)
a fourth time, and the first time the group is a thing a character can *spend* on.

**46 new weapons** come with it, in Table 44's own eight columns. Two hazards: the footnote marker is a
`<FONT SIZE="1">` span glued to the name, and the table nests **two deep** — `Sword` / `  Katana` /
`    One-handed` — where a single `parent` variable made Rapier a child of Katana. `Sabre` and
`Wakizashi` are printed at the top level beside `Sword` rather than under it; the records follow the
printed indentation rather than correcting the book, and say so.

Four ids the pack had been carrying as `cth:` — stiletto, main-gauche, rapier, sabre — and `phb:net`
are all described **here**. [Finding 105](#finding-105--a-packs-ids-encode-the-order-its-books-were-transcribed-in)'s
problem, fixed by moving five references.

### Finding 134 — every proficiency had a group and nothing defined one

83 reference occurrences to `phb:general`, `phb:rogue`, `cbd:new`, `crh:new`. The five PHB groups are
Table 37's own headings and are now records. **`<book>:new` was an id the extractor invented** for
proficiencies whose group the book does not state; 54 records now carry **no group at all**, which is
A3's distinction rather than a placeholder.

The same sweep found Table 26's row key hand-typed as `phb:hear-noise` where the book prints
**Detect Noise** and the pack's own record says `phb:detect-noise` — a table that did not match the
records it keys, in the pack for thirteen sessions.

### Finding 135 — the checker was seeing 48% of the pack's references

None of the above was visible because **the reference walker was a hand-listed set of paths**: target,
prerequisite, `effects.ref`, `effects.from`, `when`. Every kind added since session 50 put ids
somewhere that list did not mention — `members` on a weapon group, `group` on a proficiency, `schools`
and `spheres` on a spell, `combines` on a class, row keys and column headings on a table.

| | hand-listed paths | every id-shaped string |
|---|---:|---:|
| reference occurrences | 1,985 | **4,052** |
| distinct ids | 246 | 321 |

**The walker now collects every string shaped like an id**, which is mechanical and cannot fall behind
a new kind. `vocabulary` is the one exclusion, because it names a *kind* rather than a record.

This is [finding 117](#finding-117--the-most-repeated-reference-in-the-pack-was-never-counted) again
and it was not learned the first time. The repair there was to add `target` to the list; the repair
here is to **delete the list**. *A tool that reads a structure by walking a list of its parts is a tool
that will be wrong again the next time the structure grows.*

### What the complete sweep says now

**3,813 of 4,052 occurrences resolve (94%), over 289 of 321 distinct ids.** And the remainder is no
longer a long tail — **198 of the 239 unresolved occurrences are two ids**:

`phb:race` (133) and `phb:alignment` (65) are **pseudo-scalars**. §6.1's `scalar` admits
`{ability: id}` and `{level: id}`, so a transcriber needing to condition on a character's race wrote
`{ability: "phb:race"}` — and race is not an ability, and no record answers to that id. Five more do
the same: `phb:subrace`, `cdh:druid-branch`, `phb:spell-duration`, `phb:tracking-base`.

**The scalar vocabulary is too narrow by exactly the things a character most obviously is**, and the
type abuse has been sitting in 200 conditions since the first kit was modelled, unmeasurable until the
walker could see it.

### Session 64 — the pseudo-scalars, and the reference report comes back clean

**1,106 records, 0 schema errors, and 3,835 of 3,854 reference occurrences resolve.** Nineteen
occurrences over sixteen ids remain, none of them more than three.

### Finding 136 — the predicate could not name what a character is, so it lied

§6.1's `scalar` admits `{ability: id}` and `{level: id}` and nothing else. A kit that applies only to
elves has to say so, and the only shape available was **`{ability: "phb:race"}`** — race is not an
ability, and no record has ever answered to that id.

**206 conditions did this**, and it survived because nothing could see it: `phb:race` was referenced
133 times, `phb:alignment` 65, and both were invisible until
[finding 135](#finding-135--the-checker-was-seeing-48-of-the-packs-references) made the walker
complete. **The schema accepted every one of them**, because a `$ref` to `id` cannot say *which* ids.

The third arm is a **field path**, and that choice does more than patch the hole:

- **It makes the format symmetric.** Effects WRITE field paths; predicates now READ them, in one
  vocabulary rather than two. `member {field: "race"} anyOfIds [phb:elf]` reads the same field an
  effect would set.
- **It closes a second abuse with the same shape.** `computedOperand.of` named `phb:spell-duration`
  and `phb:tracking-base` to say *"double the duration"*, *"halve the tracking chance"*. Those are not
  abilities either — they are **the current value of the field the effect is adjusting**, which is
  what *doubled* and *halved* mean. All three become `{field: "spell.duration"}` and
  `{field: "proficiencyCheck.tracking"}`, and in every case the path equals the effect's own `field`.

The cost is [correction 23](../map.md)'s: a field path is a string nothing checks. That is a worse
guarantee than a typed id and a **much** better description of the truth, and the alternative — a
closed enumeration of *race, subrace, alignment, class, branch* — would have been a second closed
vocabulary beside §3.4's single exception, and would have been wrong within one book.

### Finding 137 — a druid branch is a class with a parent, which is §3.1 for the third time

`cdh:druid-branch` was one of the pseudo-scalars, and the branches it names — Forest, Desert, Gray,
Mountain, Plains, Swamp — did not exist either. The Complete Druid's prints **eight**, and they are a
druid with a terrain.

They are `classes` records carrying **`variantOf: phb:druid`**. §3.1 already says a Subrace is a Race
with a parent reference; [finding 119](#finding-119--a-class-is-two-layers-and-the-corpus-uses-both-interchangeably)
found a class inside a group; this is the same idea a **third** time, in a third kind. **A parent
reference is not a feature of races — it is how this corpus says "the same thing, more specific"**, and
the schema has now been asked for it three times by three books that could not have coordinated.

All eight carry `effectsModelled: false`: what a Forest Druid actually gets is a page of the CDH that
nobody has transcribed, and the flag says so rather than implying a branch is an empty label.

### The reference report, five sessions on

| | session 48 | session 58 | now |
|---|---:|---:|---:|
| occurrences | 0 of 496 | 736 of 853 | **3,835 of 3,854** |
| what the walker looked at | five hand-listed paths | five hand-listed paths | every id-shaped string |

The middle column is the honest embarrassment: it looked like 86% and it was measured over **a fifth**
of the references the pack actually holds. The last column is over the complete set.

**Sixteen ids remain**, and the shape of what is left is worth stating because it is no longer a
transcription backlog:

- **four creature names and two subraces** — a vocabulary the pack has no kind for
  ([correction 23](../map.md));
- **three weapons** — `phb:axe` is ambiguous on purpose, `crh:machete` and `cbgh:hoopak` are book
  equipment outside the slice;
- **seven things that are real and simply not transcribed** — strongholds, fortifications, the Grand
  Druid, the holy symbol, the bard's followers, multi-class and dual-class arrangements.

None of them is a format problem. That is the first time this ticket has been able to say that about
the reference report.

### Session 65 — every table the v1 tier prints for character generation

**1,196 records, 0 schema errors.** The lookup tables go from 11 to **101**: all 67 the PHB prints and
all 34 in the DMG's chapters 1-8, the half [ticket 16](./16-the-plan-for-the-remaining-books.md)
decision 3 kept in scope.

### Finding 138 — most of the corpus's tables cannot be consumed, and that is the useful part

[Finding 20](#finding-20--a-lookuptable-has-no-declared-role-and-its-rows-are-keyed-by-prose) called a
table keyed by prose a **defect**. Transcribing ninety more shows it is usually the *truth*:

| the row keys are | tables |
|---|---:|
| **prose the pack has no record for** — a light source, a coin, a tracking condition | **73** |
| an id, in one closed vocabulary | 16 |
| an integer | 8 |
| a range or band | 4 |

Finding 20 was right about the table it was looking at, and the general rule it implied was wrong. The
difference is **declaration**: `keyedBy.kind: "text"` says the key is the book's own string and the
Engine cannot index it, and such a table carries no `supplies` either. **Nine of 101 tables declare a
`supplies`** — those are the ones the Engine is waiting for; the other 92 are reference data a reviewer
consults, and the pack now says which is which instead of leaving them to look alike.

That is A3 applied to a table's axis, and it is what makes transcribing all 101 honest rather than
padding.

### Finding 139 — the axis can be inferred, and the extractor stops guessing

The tool used to emit `supplies: ""` and `keyedBy: {kind: "id"}` for a human to fill. **The keys
themselves say what they are**: all integers is `integer`, `4-6` and `1-4` are `range`, keys that all
resolve to records of **one kind** are `id` with that kind as the vocabulary, and anything else is
`text`.

`supplies` is still never guessed — which field a table fills is a modelling decision — but it is now
**absent** rather than an empty string, which had been claiming a field path called `""`.

Sixteen tables key by id, and they are the ones worth having: Table 38's proficiency-group crossovers
by class, Table 43's starting funds by class group, Table 64's movement rate by race, the DMG's four
sample-character tables by ability.

### Finding 140 — the resolver had to be taught §7.3 the hard way

The first version matched a row key against **one global name map** and Table 7's `Strength` resolved
to `cprh:DD05581` — the Complete Priest's **priesthood of Strength**. `Wisdom` likewise.

A name is not identity, which §7.3 has said from the start and a tool ignored inside a hundred lines of
code that exist to enforce it. Resolution is now **per kind**: a table's keys are drawn from one
enumeration, so the axis is accepted only when a single kind covers all of them — which is both correct
and *why* the vocabulary field exists.

Three smaller things the same pass found: a header rule that took every leading digit-free row made
**Table 38 a record with zero rows**, since all its cells are words; footnote markers glued to a key
(`Paladin*`, `Elf1`) stopped every such key resolving; and **Table 33 has no key column at all** — four
ability names head a single row of base scores — which is the only one in 101 and is recorded as
printed.

### What a table is for, when its facts are already records

Five tables hold facts the pack already carries as records: Table 37's proficiency scores, Table 44's
weapons, Table 36's secondary skills, Table 8's racial adjustments, Table 13's class minimums.

They are **kept**, with a note saying so. A record is the pack's reading of a table; the table is the
book's own artifact, and **the reviewer checks one against the other**. Dropping it would remove the
thing the review protocol compares to. Table 36 makes the point sharpest: the records carry each
skill's name and gloss, and only the table carries **the d100 range it is rolled on**.

### Session 66 — the creature vocabulary comes out of the field paths

**1,218 records, 0 schema errors, and 3,939 of 3,952 reference occurrences resolve.** Ten ids remain.

### Finding 141 — the third scalar arm had already solved the biggest shortfall, in one class of case

[Finding 110](#finding-110--the-field-path-is-where-the-untyped-vocabulary-collects) concluded that a
`when` clause describes the character and never his opponent, so a target-scoped bonus had to hide in
the field path — `attackRoll.melee.vsOrc`, `opponent.attackRoll.titan`. Fourteen creature names in
strings nothing checked, and [correction 23](../map.md) called it a cost to state.

[Finding 136](#finding-136--the-predicate-could-not-name-what-a-character-is-so-it-lied) made a scalar
able to name **any field path**, including one that does not belong to the character. That was written
to fix `{ability: "phb:race"}`, and it turns out to close this too:

```
adjust attackRoll.melee by 1
  when member {field: "opponent.creature"} anyOfIds [orc, half-orc, goblin, hobgoblin]
```

**Fourteen effects became four.** The dwarf goes from 52 effects to 45, the gnome from 18 to 11, and
twelve creature names move out of unchecked strings into references that resolve.

This is worth stating plainly because [the verdict](#the-verdict) names conditions as the format's
largest shortfall — 94 of 415 markers — on the grounds that *the predicate can name the character and
nothing else*. **That is now false for any discriminator the pack can name as a field.** It is not
false for terrain, the round, or the campaign, which have no field either; the shortfall is smaller and
sharper than it was, and it was narrowed by a change made for an unrelated reason.

### Finding 142 — a creature record is a name the rules discriminate on

The `creatures` kind holds **thirteen** — orc, half-orc, goblin, hobgoblin, kobold, gnoll, bugbear,
ogre, ogre magi, troll, giant, titan, duergar — and it is deliberately **not** a monster.

The Monstrous Manual is outside the v1 tier. What a troll's hit dice are is not here and will not be.
What *is* here is that **the dwarf's own entry singles out four species to hit better and five to be
attacked worse by**, and a character sheet has to say so. So the record is provenanced to the page that
discriminates — the dwarf's entry, the gnome's — not to a bestiary the pack does not have.

The duergar is the awkward one and says so in its own note: the CBGH names it as a species a character
may take as an enemy, and the dwarves' book treats it as kin. It is recorded at the grain the referring
rule uses.

### The Complete Gnomes and Halflings' subraces

Nine more `subraces`: **Rock, Svirfneblin, Tinker and Forest gnomes; Hairfoot, Tallfellow, Kender,
Athasian and Furchin halflings** — the two subrace chapters entire, where only Stout had been
transcribed. `cbgh:svirfneblin` and `cbgh:kender` had been hand-minted **by name** while every
Attachable's id is file-derived; both references now point at the records.

### What is left of the reference report

**Ten ids, thirteen occurrences.** Three are book equipment outside the slice (`cbgh:hoopak`,
`crh:machete`, and `phb:axe`, which stays ambiguous on purpose). Seven are real things nobody has
transcribed: strongholds and fortifications, the Grand Druid, the holy symbol, the bard's followers,
and the multi-class and dual-class arrangements.

**None of the ten is a creature, a terrain, a proficiency group, a class, an alignment or a scalar** —
every category that has produced a finding in this ticket is now closed.

### Still not done

~~The judgement pass~~ — **done in session 37; all 68 records carry effects** · the measured cost per record against
[ticket 11](./11-human-review-protocol.md)'s 5–15 minute prediction, which finding 16 says should
come down for a third of them · local-model draft quality.

**The remaining work is the expensive half**, and two records in, the cost is not yet measurable for
the reason that matters: **both hand-modelled records spent most of their time finding format gaps,
not transcribing.** The Acrobat produced findings 7 and 8; Agriculture produced 10 and 11. That cost
is front-loaded and disappears once the format stops moving — so timing a record now would measure
the wrong thing, and ticket 11's prediction stays unchecked until a record passes through changing
nothing.

**All three §4.1 arms now carry effects and the claim held on all three** — which is the single
largest thing this ticket was built to find out, and it came back positive.

## What it must not do

**Do not transcribe beyond the slice.** The temptation at the end of a working pipeline is to keep
going — it is finally fun, and the machinery is warm. The map's destination is the slice plus a plan
for the rest; volume past that point buys nothing this map needs and delays the verdict the other
twelve tickets are waiting on.

**Do not repair the schema silently.** If the books force a change, that is a finding — the most
valuable output this ticket has. Record it, then change it.

## If it fails

A real possibility and worth naming in advance. If the slice cannot be transcribed — the format
cannot express something, the pipeline cannot be made reproducible, the review burden is impossible —
that is the map working, not the map failing. It is the same service
[v1 ticket 12](../../v1-spec/issues/12-verify-adhoc-signed-macos-build.md) performed by building an
app on CI and running it on a real Mac: **one experiment settled a contradiction that no amount of
reading had resolved**, and it proved a primary source wrong.

The failure mode this map most fears is not a failed slice. It is a slice that appears to succeed
because nothing in it was hard enough to hurt — which is why [ticket 08](./08-which-slice-proves-the-format.md)
must state what the slice proves *and what it does not*.
