# Which slice proves the format

Type: grilling
Status: open
Blocked by: 01

## Question

The map's destination ends at a **proving slice** transcribed end to end — not a book, and not the
corpus. This ticket chooses it.

The charting argument for a slice over a book: **a book is volume, a slice is information.** The PHB
has hundreds of spells and a full equipment catalogue; after the fiftieth spell nothing further is
learned about the format. What a slice must maximise is the number of format questions it answers
per record transcribed.

## The criterion

**Pick for stress, not for coverage.** The slice should be the smallest thing that exercises every
part of the format that has no precedent. `spec.md` §13 names three of those, and all three are
closable only by real data:

- **#1 — the kit mechanism has no prior art anywhere.** §4.1 is invention.
- **#2 — "Engine computes, user supplies the tables" has no shipping precedent.**
- **#4 — the six-operation vocabulary may prove insufficient** for some PHBR kit, and the spec says
  this is checkable now because the books are in hand.

## What ticket 01's measurement does to the candidates

[Ticket 01](./01-what-the-source-yields.md) supplies the counts this ticket was waiting on, and it
sharpens the candidate list rather than just filling it in:

- **~146 kit records** across 8 books, and **60 priest specialty records** with a ten-field shape
  sharing nothing with a kit. A slice of "one handbook's kits" is 16–29 records — a session's work,
  not a season's.
- **A slice must now name its rendition.** Since the split is per book, the Complete Dwarves or
  Gnomes & Halflings (kits in *both*, ~1:1) prove the format while proving nothing about the
  alignment problem; the Complete Paladin's or Ranger's (HTML only) or Thief's (RTF-dominant) prove
  the format *and* exercise the harder pipeline. That is a real trade, and the ticket must choose it
  deliberately.
- **Subrace is nearly free**: 5 records in the Complete Book of Elves, mapping onto §4.1's third
  Attachable. Adding them tests the claim that Kit, Deity and Subrace are one shape used three times
  — which no other slice would touch — at a cost of five records.

## Candidates, with the case for each

- **One Complete handbook's kits plus the PHB tables they modify.** The charting hunch. Exercises
  Attachables, all three effect natures, the layer stack, prerequisites, and the pack-to-pack
  reference from a handbook into the PHB — which is where §5.1's A3 union rule lives.
- **That, plus a sample of the Complete Priest's records.** The map's charting found ~59 entries with
  a **completely different ten-field shape** — `Spheres of Influence`, `Duties of the Priest`,
  `Followers and Strongholds`. If those turn out to be Deity, then the slice above never tests the
  second Attachable at all, and §4.1's claim that Kit, Deity and Subrace are one shape used three
  times goes untested until v2.
- **A vertical slice: one complete Character's worth of records.** Everything needed to build a
  single legal character end to end. Tests the pipeline of §9.1 rather than the format's edges.
- **The hardest kit in the corpus**, whatever ticket 01 flags as beyond the six operations.

## What the answer must produce

1. **The slice, named concretely** — which books, which sections, which records.
2. **What it is claimed to prove**, mapped to the three known unknowns above. A slice that does not
   name what it tests is just a small amount of transcription.
3. **What it deliberately does not prove**, so the map does not later mistake a passed proof for a
   validated format. Psionics is already the standing example — v1 spec known unknown #3 says
   deferring it entirely risks discovering in v2 that the format cannot express it, *which is how
   PCGen failed*. That risk was raised and overruled for v1; this ticket should say plainly whether
   the slice changes it or leaves it standing.
4. **A size estimate**, so [ticket 13](./13-transcribe-the-proving-slice.md) is a session's work and
   not a season's.

## A second job for the slice, from ticket 04

The slice does not end its life once it has proved the format. Adopted from
[ticket 04](./04-llm-assisted-extraction.md): it is **retained permanently as the gold standard and
regression test** (see [ticket 13](./13-transcribe-the-proving-slice.md)), because a hand-verified
reference is the only kind of check that works on this error class.

That adds a dimension to the choice. A slice picked purely to stress the format once may be the
wrong thing to regression-test against for years — the ideal slice is both **maximally
format-stressing** and **representative enough that a later pipeline change failing against it means
something**. Say which of the two the chosen slice favours, and why.

## The tension to resolve

Between **breadth** (touch every kind once, prove the schema is complete) and **depth** (transcribe
one area properly, prove the format is *correct*). They are different proofs and the slice cannot
maximise both. Ticket 01's bucket partition — mechanical, regular-but-ambiguous, judgement — is the
input that makes this decidable rather than a matter of taste.
