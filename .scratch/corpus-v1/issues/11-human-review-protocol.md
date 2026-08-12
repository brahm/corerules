# The human review protocol for the judgement half

Type: grilling
Status: open
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

## Why this is blocked by ticket 08 and not by 10

The protocol has to be designed against real reviewing, and the proving slice is the first real
reviewing that happens. Designing it earlier would produce a procedure nobody has tried.

## The honest expectation

**This protocol will be applied to the proving slice and then applied unevenly forever.** That is not
a reason to skip it — a written protocol applied unevenly still beats an unwritten one — but the
answer should be designed for intermittent use by one person over years, not for a review board.
Anything requiring sustained discipline will not survive book seven.
