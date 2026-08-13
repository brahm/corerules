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

### Still not done

The judgement pass on the remaining **24 of 63** Attachables · the measured cost per record against
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
