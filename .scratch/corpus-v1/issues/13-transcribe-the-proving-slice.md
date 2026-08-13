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

The minimal repair is one clause type — a `predicate` entry that is *either* a condition *or* an
`anyOf` of conditions, keeping the top level a flat AND. Both examples are that shape. **This
reopens a decision the map recorded as settled, so it is not applied here** — but the number it was
decided on is retracted in the schema text.

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

### Still not done

The judgement pass on the remaining **29 of 31** Attachables · the measured cost per record against
[ticket 11](./11-human-review-protocol.md)'s 5–15 minute prediction · local-model draft quality ·
a Subrace modelled by hand, which is the one §4.1 arm still carrying no effects.

**The remaining work is the expensive half**, and two records in, the cost is not yet measurable for
the reason that matters: **both hand-modelled records spent most of their time finding format gaps,
not transcribing.** The Acrobat produced findings 7 and 8; Agriculture produced 10 and 11. That cost
is front-loaded and disappears once the format stops moving — so timing a record now would measure
the wrong thing, and ticket 11's prediction stays unchecked until a record passes through changing
nothing.

**Two of the three §4.1 arms now carry effects.** The Subrace arm is the remaining test, and finding
10 gives it a specific job: the `A ∧ (B ∨ C)` prerequisites are in the gnome-and-halfling book, so
the arm that is untested and the gap that is unrepaired meet in the same pages.

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
