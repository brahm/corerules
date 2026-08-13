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
thereafter."* [Ticket 15](./15-computed-operands.md)'s closed set covers halving and division;
**scaling by level is not in it**. Measured over **138 kits across nine books**, counting only the
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

### Still not done

The judgement pass on the remaining **29 of 39** Attachables · the measured cost per record against
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
