# The human review protocol for the judgement half

Type: grilling
Status: resolved
Blocked by: 08

## Question

The expensive half of the map's hybrid verification decision, and the one that does not scale. The
mechanical checker ([ticket 10](./10-mechanical-verification.md)) never opens the book; it proves
internal consistency. **Whether a kit's transcription says what the book says is a reading, and only
a person can do it.**

Charting already conceded the shape of the problem: reviewing ~100 kits against the source is a full
re-read, measured in weeks, with attention decaying throughout. So this ticket is not "design a
thorough review". It is **decide how little review buys how much confidence**, and write the protocol
down so it survives being done in pieces over months.

## What has to be decided

1. **What gets reviewed.** Every judged record, a sample, or a risk-ranked subset. If a sample: what
   makes the sample representative, given ticket 01 will have shown the corpus is regular *but not
   uniform* — the label vocabulary alone splits by book family.
2. **What the reviewer is actually comparing.** Reading JSON against a book is a poor diff. Options
   include rendering the record back to prose and comparing that, presenting source paragraph beside
   extracted fields, or reviewing the *Engine's behaviour* rather than the record — building a
   character with the kit and checking the sheet. The last is the strongest test and the slowest.
3. **What counts as reviewed**, recorded where. A per-record marker, a per-book sign-off, nothing at
   all. This interacts with [ticket 07](./07-identity-and-id-stability.md): if re-extraction changes
   records, a review marker attached to a record has to survive that, or the review is lost every
   time the pipeline improves.
4. **What happens to a disagreement.** Sometimes the extraction is wrong; sometimes the book is
   ambiguous and the transcription is an interpretation. `spec.md` §7.1 requires book and page on
   every record but has no place to record *"the book is unclear and this is my reading"*. If that
   field is needed, it is a schema change and belongs to
   [ticket 05](./05-pack-schema.md) — say so rather than inventing it here.
5. **How this doubles as v1 spec known unknown #4.** The spec says the six-operation vocabulary may
   prove insufficient for some PHBR kit, and that it is checkable now because the books are in hand.
   **Human review of the judgement half is that check.** Structure the protocol so a reviewer who
   finds an inexpressible rule has somewhere to put it — that finding is worth more than the review
   itself.

## Estimate the hours — ticket 04 found this is the real cost

[Ticket 04](./04-llm-assisted-extraction.md) priced a full corpus pass at ~$10–$100 and then said the
figure is a rounding error: **human review hours dominate every dollar in this effort, and no ticket
on this map has estimated them.**

That makes an estimate part of this ticket's deliverable, not an afterthought. It is also what makes
the sampling decision above a real decision rather than a preference — "review everything" has a
number attached, and the number is what rules it in or out.

## The slice this protocol is designed against

[Ticket 08](./08-which-slice-proves-the-format.md) resolved: **~40 records** — 24 Complete Thief's
kits, 6–8 Deity, 5 Subrace, plus 5 PHB tables. Small enough that the protocol can be tried in full
rather than sampled, which is the one chance this ticket gets to calibrate before the corpus makes
full review impossible.

Two properties of the slice shape this ticket directly:

- **It was chosen for representativeness, not extremity**, precisely so it can serve as a permanent
  regression test. So reviewing it measures the *ordinary* cost per record, which is the number
  ticket 04 said nobody has estimated — not a worst case.
- **The tables are mechanical and the Attachables are judgement.** The 5 PHB tables belong to
  [ticket 10](./10-mechanical-verification.md); the ~37 Attachable records are this ticket's whole
  subject. That split is the hybrid decision arriving concretely rather than in principle.

## Answer

### Decision 1 — the reviewer compares the record against its own source passage

Rejected: **the Engine's behaviour**, which the ticket called the strongest test — **it is
unavailable**. The Engine is months away and the slice is transcribed now, and designing a protocol
around a tool nobody has run produces exactly what this ticket warns against: a procedure nobody has
tried.

Rejected: **the printed book**, on the cost that makes review not scale. Without page numbers the
reviewer navigates by section path, and per record that search is the difference between weeks and
months across ~146 kits.

**Side-by-side is already paid for.** [Ticket 05](./05-pack-schema.md) put an `anchor` on every
record for a different purpose; it now delivers the source passage next to the extracted record
automatically, with nothing to find.

**The scoping this forces, stated rather than left implicit: the review verifies extraction fidelity,
not corpus fidelity.** It catches what the pipeline can introduce. It does not catch an error TSR's
own digitisation made, nor the pre-errata problem [ticket 03](./03-prior-art-core-rules-extraction.md)
found. That is the correct separation — a corpus-level fact is pinned by ticket 02's hashes and
recorded by ticket 03, and making per-record review carry it would load the review with a problem it
cannot solve anyway.

Accepted cost: you verify the extraction against the CD and trust the CD against the book. Closing
that is a **separate, cheap sampling exercise** — a handful of records per book against paper, enough
to detect a systematic digitisation problem — and it belongs to
[ticket 13](./13-transcribe-the-proving-slice.md), not to the routine protocol.

### Decision 2 — a review ledger keyed by record id and content hash

**A review that finds nothing leaves no trace in git.** A correction is a commit; a confirmation is
silence. So under [ticket 09](./09-extraction-pipeline.md)'s git-as-overlay there is no record of
what has been *checked and found correct* — and review is this project's scarcest resource, so in two
years "has the Complete Thief's been reviewed?" would have no answer.

Rejected: **a field in the record.** The pipeline generates records, so a `reviewed` flag would
appear as removed on every re-extraction and have to be re-added by hand — noise in the one artifact
that has to stay legible.

**The hash matters more than the ledger.** A review goes stale when the record changes, and keying on
a content hash makes that **mechanically detectable** — which becomes another check for
[ticket 10](./10-mechanical-verification.md). Without it the ledger lies confidently.

**The tension, raised because this map twice rejected exactly this shape** — an overlay in ticket 09
and a persistent index in §8, both as *a second source of truth able to go quietly stale*. This is
not that: those duplicated data that already existed, while a ledger records **human attestation,
which exists nowhere else in the corpus**. And the hash makes staleness explicit rather than silent,
which was the actual defect.

**Referred to [ticket 14](./14-record-shapes-for-the-slice.md):** the ticket's item 4 — *the book is
ambiguous and this transcription is a reading* — is not attestation but a **property of the record**,
so it is a schema change. Recorded as a requirement there rather than invented here.

### Decision 3 — risk-ranked, with a mandatory floor, and the risk signal is never the model

**The proving slice is reviewed in full** — ticket 08 sized it for that. The permanent rule is
risk-ranked.

**The signal must not be the model.** [Ticket 04](./04-llm-assisted-extraction.md) measured
self-consistency correlating with accuracy at ρ 0.10–0.30 — a model can be confidently and
consistently wrong — so "review where the model was unsure" is *worse than random*, concentrating
effort where the model thinks it erred rather than where it did.

The usable signals all exist already, as by-products of decisions taken elsewhere:

- **Cross-rendition divergence** ([ticket 09](./09-extraction-pipeline.md)) — an external reference,
  already being computed.
- **Presence of a computed operand** ([ticket 15](./15-dice-and-generation-methods.md)) — by
  construction the records known unknown #4 touched, the hardest to transcribe.
- **Effect count** — crude but honest: more effects, more chances to be wrong.

**The floor is structural, not prudence.** The judgement half has almost no mechanical checks — ticket
10 covers tables, where exact redundancy exists, and prose fields carry no invariants at all. So
"review what the checker flags" fails precisely here, because the checker is nearly blind on this
half. Without a floor, risk-ranking degenerates into reviewing no judgement records at all.

**Recorded as revisable, with the trigger written down:** if ticket 13's measured cost per record is
low enough, promote to reviewing everything. Choosing "everything" *now* would be choosing without
the number.

### The hours estimate — bounded here, measured by ticket 13

Ticket 04 said human review hours dominate every dollar in this effort and nobody had estimated them.
This is the estimate, and it is a **prediction to be checked, not a measurement**.

A judgement record is a kit or Deity entry carrying roughly three to four prose fields against a
source passage of a few hundred words. Reviewed side by side, with no searching, a careful pass is
plausibly **5–15 minutes per record**.

- **The proving slice: ~37 records → about 3–9 hours.** A weekend, and it produces the real number.
- **The v1 tier's Attachables: ~206 records → about 17–52 hours.** Weeks of evenings, not months.

**If that holds, "review everything" is affordable for the Attachables and decision 3's caution is
unnecessary** — which is exactly why the trigger above is written down. If it does not hold, the
gap will be large and visible immediately.

### Known unknown #4 already fired — this changes item 5's job

The ticket assigned human review the job of *checking* whether the six operations suffice.
[Ticket 15](./15-dice-and-generation-methods.md) already answered it: **31 occurrences of division,
halving or explicit rounding inside kit effect fields**, none expressible in §4.3.

So the protocol's job shifts from finding the first gap to **catching the next one**. The reviewer
still needs somewhere to put "this rule cannot be expressed" — but it is now a known category with a
known first instance, not a hypothesis. A protocol that produced zero such findings across the
Attachables should be disbelieved, on the same reasoning ticket 13 applies to itself.

## Why this is blocked by ticket 08 and not by 10

The protocol has to be designed against real reviewing, and the proving slice is the first real
reviewing that happens. Designing it earlier would produce a procedure nobody has tried.

## The honest expectation

**This protocol will be applied to the proving slice and then applied unevenly forever.** That is not
a reason to skip it — a written protocol applied unevenly still beats an unwritten one — but the
answer should be designed for intermittent use by one person over years, not for a review board.
Anything requiring sustained discipline will not survive book seven.
