# Which slice proves the format

Type: grilling
Status: resolved
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

## A fourth candidate, from ticket 05

[Ticket 05](./05-pack-schema.md) resolved the Complete Priest's records as **Deity**, and in doing so
corrected v1 ticket 11's expectation that Deity would enter thin: **60 records of ten fields each.**

That makes them a serious candidate this ticket did not have. **Deity will exercise §4.3's six
operations harder than any kit** — spheres of influence, granted powers, weapon and armour
restrictions, follower rules and duties, all on one record. If the point of a proving slice is to
stress the format where it has no precedent, the heaviest Attachable in the corpus is a strong
answer, and it tests §4.1's claim that Kit, Deity and Subrace are one shape from the direction most
likely to break it.

Against it: the Complete Priest's Handbook has **1 HTML table** in the whole book, so a Deity slice
proves almost nothing about the table pipeline — and ticket 01 showed table handling is where the
two renditions differ most.

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

## Answer

### The slice

| Part | Source | Records |
|---|---|---|
| **Kits** | Complete Thief's Handbook, both renditions | 24 (RTF) / 7 (HTML) |
| **Lookup tables the kits adjust** | PHB tables 26–30 — thieving skill base scores, racial, Dexterity and armour adjustments, backstab multipliers | 5 tables |
| **Deity** | Complete Priest's Handbook, sampled for **representativeness, not extremity** | 6–8 |
| **Subrace** | Complete Book of Elves, exhaustive | 5 |

**Roughly 40 records and 5 tables — a session's work, not a season's.**

### Why three arms, not one

**§4.1 is the largest single piece of invention in the whole spec.** [v1 ticket 11](../../v1-spec/issues/11-engine-object-kinds.md)
records that "Kit, Deity and Subrace are one closed shape used three times" was Wagner's proposal, and
that it **shrank ticket 10 from designing a kit mechanism to designing one applicable-modifier
mechanism**. Half the Attachable architecture rests on that claim.

**And it is untestable with one arm.** A slice of kits proves kits work. It proves nothing about the
assertion that Deity and Subrace are the *same shape* — which is the part without precedent, and the
part whose failure invalidates what was built on top of it. **Subrace costs five records**: the
cheapest test of a structural claim this project will ever get, and
[ticket 05](./05-pack-schema.md) has just shown the second arm is far larger than the spec assumed,
which is exactly the kind of surprise that should make "same shape" suspect.

### Why the Complete Thief's, and the axis the ticket thought was a trade

The ticket called format-stress versus pipeline-stress "a real trade". **For kits it is not one** —
kits exist in 8 books with different rendition difficulties, so *what* to transcribe and *which book
to take it from* are independent axes. Deity and Subrace have no such freedom; each exists in one
book. So the kit book is where rendition difficulty gets chosen deliberately, and the rest of the
slice already covers opposite sides: the Elves subraces are the **only** structure that book exposes
in the RTF, and the Complete Priest's is **HTML-strong** (211 of 225 pages titled).

The Complete Thief's was chosen not for its rendition gap alone — 24 RTF against 7 HTML, the largest
measured — but for what thief kits **drag in with them**. They adjust thieving-skill percentages,
which pulls the *thieving skill* kind and the PHB's five tables into the slice, and that makes it
test three things no other choice tests together:

- **the pack-to-pack reference** — a Complete handbook pointing into PHB tables, which is where
  §5.1's A3 union rule lives;
- **`adjust` on a lookup table**, the most mechanical path through §4's layer model;
- **tables at all**, in a book that has them — which answers the objection that sank the
  Deity-only slice.

Rejected: the Complete Paladin's or Ranger's (HTML-only, 0 records in the RTF). That is the *harder*
pipeline case — no alternative source at all rather than a lossy one — and choosing against it is
deliberate. It costs the thieving-skill tables and the PHB link, and those buy more.

### What it is claimed to prove

- **Known unknown #1** — the kit mechanism has no prior art. 24 kits with real prerequisites and
  effects.
- **Known unknown #4** — whether six operations suffice. Deity stresses this hardest: ten fields
  covering spheres, powers, restrictions, followers and duties on one record.
- **Known unknown #2** — "Engine computes, user supplies the tables", tested end to end by kits that
  `adjust` PHB lookup tables the Engine must then compute through.
- **§4.1's three-arm claim**, the only part of this list that no smaller slice reaches.
- **§5.1's A3 union**, via the cross-pack reference.
- **The rendition split**, at its widest measured point.

### What it deliberately does not prove

- **Psionics.** v1 known unknown #3 stands **unchanged** — deferring it entirely still risks
  discovering in v2 that the format cannot express it, which is how PCGen failed. This slice does not
  touch that risk in either direction.
- **Spells**, the largest record population in the corpus, untouched.
- **Equipment and gear breadth.**
- **The HTML-only failure case**, chosen against above.
- **Table handling at scale** — five tables against the PHB's 578.

Recorded so a passed proof is not later mistaken for a validated format.

### Gold standard: this slice favours representativeness

[Ticket 04](./04-llm-assisted-extraction.md) makes the slice a permanent regression test, and the
ticket asks which of the two properties it favours. **Representativeness.** The kit portion is *all*
24 Complete Thief's kits rather than the hardest ones; Subrace is exhaustive; and the Deity sample is
explicitly drawn for representativeness rather than extremity — because a set chosen only for hard
cases makes a poor regression test: when it breaks in two years it cannot say whether the pipeline
regressed or that case was always pathological.

Format stress is bought instead from **Deity's ten fields** and the **adjust-on-table** path, neither
of which required picking outliers.

### What this hands ticket 14

The record shapes needed are roughly **ten kinds, not twenty-seven**: Kit, Deity, Subrace, Thieving
skill, Lookup table, Weapon proficiency, Non-weapon proficiency, Class, Race, Ability. That is the
deferral in [ticket 05](./05-pack-schema.md) paying off — the slice choice is what makes ticket 14 a
bounded piece of work.

## The tension to resolve

Between **breadth** (touch every kind once, prove the schema is complete) and **depth** (transcribe
one area properly, prove the format is *correct*). They are different proofs and the slice cannot
maximise both. Ticket 01's bucket partition — mechanical, regular-but-ambiguous, judgement — is the
input that makes this decidable rather than a matter of taste.
