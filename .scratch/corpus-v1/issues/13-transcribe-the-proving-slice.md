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

### Still not done

The five PHB thieving-skill tables the kits adjust · the judgement pass turning field prose into §4.3
effects · the measured cost per record against [ticket 11](./11-human-review-protocol.md)'s 5–15
minute prediction · local-model draft quality.

**The remaining work is the expensive half.** Everything above is the mechanical pass, and it took
one session for 31 records. The judgement pass is what ticket 11 priced at 3–9 hours for this slice,
and none of it has been done — so the prediction it exists to check is still unchecked.

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
