# Mechanical verification: what the checker checks

Type: grilling
Status: resolved
Blocked by: 05

## Question

The cheap half of the map's hybrid verification decision. **Valid against the schema is not faithful
to the book**, and the dangerous error is the one that passes every existing check:

A pack can be well-formed JSON, manifest in order, IDs unique, book and page cited on every record,
A3 declared — and say `THAC0 18` where the book says `19`. Nothing detects it. Under `spec.md` §4
nothing is stored; every value is recomputed by walking the layer stack. So **one wrong number in a
lookup table silently poisons every Character that passes through it**, and surfaces months later at
the table as "that doesn't match the book".

This is worse than the failures the v1 spec already planned for. A malformed pack does not load at
all (§7.5) — loud, high, unmissable. A pack that is merely *wrong* loads perfectly.

## Promoted by ticket 04

This ticket was written as the *cheap* half of verification.
[Ticket 04](./04-llm-assisted-extraction.md) found it is the **load-bearing** half.

Every model-internal check fails on exactly this error class: self-consistency correlates with
accuracy at only ρ 0.10–0.30 (a model can be confidently and consistently wrong), intrinsic
self-correction *degrades* performance, and semantic round-tripping cannot distinguish `18` from
`19` because both round-trip to near-identical prose. **A check only works if its reference is not
the model.**

More than that: per Tyen et al. (ACL Findings 2024), *finding* the error is the bottleneck while
*fixing* it given the location is robust. So mechanical localisation is not a nice-to-have running
alongside human review — **it is the precondition that makes any repair work at all**, model or
human. Design it as the primary instrument.

## What has to be decided

**What a mechanical checker can assert without reading the book.** Candidates, to be confirmed,
rejected or extended:

- **Referential integrity.** Every reference resolves; every kit's target exists; every sphere named
  by a priest entry is a sphere that exists.
- **Structural invariants of tables.** A saving-throw matrix has a known shape; THAC0 progressions
  step by a constant; level-indexed tables are monotonic where the rules make them so. A checker that
  complains about an off-pattern step catches exactly the error class that never shows up in play.
- **Range plausibility.** Ability minima in 3–18 (and 5–24 once Dark Sun arrives in v2 — so the check
  must be data-driven, not hardcoded, per §2's open enumerations).
- **Row and column sums** where a table has redundancy to exploit.
- **Record counts against the book's own index.** The books contain their own tables of contents and
  numbered table lists; the PHB has 161 numbered tables. Extracting 158 is a detectable failure and
  ticket 01 is producing the counts.
- **Cross-extraction agreement**, if [ticket 03](./03-prior-art-core-rules-extraction.md) finds an
  independent dump. Two extractions disagreeing is a free error detector and needs no book at all.
- **A3 coherence.** §5.1 already requires a declared-but-empty rule-set to be reported as suspicious;
  decide what else about the declaration is checkable.

**Ticket 07 added a check that costs nothing to run and catches a silent class.** IDs are now
`<pack>:<file-stem>` — a *function* of the anchor — so **`id` and `anchor` must agree on every
record, by construction**. A record where they disagree means either a hand-edit that moved one
without the other, or a parser change that renumbered. Neither is visible any other way, and both are
exactly the silent-corruption class this ticket exists for.

**A whole check arrived from ticket 09, and it is free.** The pipeline now parses the **HTML only**,
keeping the **RTF as an independent second rendition to diff against** — because the two disagree by
a few percent per field, in both directions (HTML finds 131 `Description` to the RTF's 104; the RTF
finds 128 `Equipment` to the HTML's 97). That is a **non-model reference**, which
[ticket 04](./04-llm-assisted-extraction.md) established is the only kind that detects this error
class, and it costs nothing because both renditions are already on disk.

**This ticket owns that comparison.** Per book and per field: record counts, field counts, and record
boundaries. Where the renditions agree, confidence; where they disagree, a location to look. Note
what it is *not* — it cannot check faithfulness to the printed book, only that two independent
digitisations of it tell the same story.

**What the spine already hands this ticket.** [Ticket 05](./05-pack-schema.md) settled that
enforcement is **two-tiered by necessity**: JSON Schema declares what it can, and everything it
cannot — recursive structures, numeric ranges, string lengths, and every cross-field rule — falls to
the validator. **That second tier is this ticket.** The schema is not a checker that happens to be
incomplete; the split is structural, and the checks below live on the far side of it. Two new
checkable facts also arrive with the spine: every record carries a `section` and an `anchor`, and the
manifest names its source files by hash — so *"does this record's anchor resolve in the source this
manifest claims"* is now a mechanical check that nothing else performs.

**Where the checker lives**, and this is not a detail: `spec.md` §7.6 says the Engine's *whole*
contribution to authoring is a validator that names file, record and field. That validator does not
exist yet either. Decide whether this checker **is** that validator, grows into it, or is a separate
authoring-side instrument — noting §7.2's standing prohibition on the same thing being implemented
twice from a description.

## Answer

### Decision 1 — two response classes, because the checks are not one kind

A pack that is merely *wrong* is well-formed, so §7.5's refuse-to-load cannot apply and the natural
output is a warning. **This project has already ruled that warnings do not work**:
[v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md) rejected partial loading precisely
because *"warnings ignored for months would leave characters built against a corpus full of holes"*.

So the checks split by what a failure *means*:

- **Invariants** either hold or are broken, and admit no judgement: `id` must equal a function of
  `anchor`, every reference must resolve, an anchor must exist in the source the manifest names, an
  ability minimum cannot be 25. **A broken invariant fails the pipeline run.** The corpus cannot be
  committed clean with one outstanding.
- **Divergences** are statistical and *expected*: the two renditions disagree by a few percent per
  field ([ticket 09](./09-extraction-pipeline.md)), and blocking on them would stop the pipeline on
  day one. **Known divergence is recorded and committed as a baseline; a *new* divergence fails.**

**The baseline snapshot is the load-bearing part of this decision.** Without it the divergence report
becomes exactly what v1 ticket 13 refused — a list nobody reads because it is never empty. With it,
the list is empty by construction and any growth is a signal. It is the same mechanism as a test
snapshot or a linter baseline, and it is the only one that stops warnings rotting into noise.

Accepted cost: the pipeline becomes stricter to run, and a badly calibrated check blocks legitimate
work. Mitigated by invariants being few and objective — every one is a consequence of a decision
already taken elsewhere, not a new judgement.

Rejected: **the Engine flagging at load**. By then the error is in the corpus and committed; the cheap
point of intervention is extraction.

### Decision 2 — two checkers, and the boundary is visibility

Rejected: one checker in the pipeline (§7.5 still requires the Engine to refuse malformed packs, and
schema alone cannot catch a broken cross-pack reference), and expressing the rules as interpreted
data (building a rule engine to avoid writing two functions, and creating the third language §7.2
warns about).

**§7.2's prohibition does not bite, because the checks divide themselves by what each side can see —
and neither side can do the other's job.**

**The pipeline has the sources; the Engine never will.** A pack does not carry its sources — that
would be the same error §6.5 forbade when it said a Character carrying pack data *is a pack in
disguise*.

**The Engine has the active pack set; the pipeline never will.** A pack is extracted alone;
cross-pack references resolve only at load, and §5.1's A3 union rule does not exist until a second
pack does.

Even referential integrity splits this way: **within**-pack references are the pipeline's,
**between**-pack references are the Engine's.

| Pipeline (has the sources) | class |
|---|---|
| `id` agrees with `anchor` — functionally related by [ticket 07](./07-identity-and-id-stability.md) | invariant |
| anchor resolves in the source file the manifest names | invariant |
| source hashes match the manifest ([ticket 02](./02-where-the-corpus-lives.md)) | invariant |
| manifest file list matches the directory; an orphan file is reported (§7.1) | invariant |
| within-pack references resolve | invariant |
| range plausibility, data-driven rather than hardcoded (§2's open enumerations) | invariant |
| row and column sums where a table carries exact redundancy | invariant |
| record counts against the book's own numbered index | divergence |
| cross-rendition record, field and boundary counts | divergence |
| table progressions — constant steps, monotonicity where the rules impose it | divergence |

| Engine (has the active pack set) | |
|---|---|
| cross-pack references resolve — a failure is §7.5's refuse-to-load | |
| A3 coherence, including §5.1's declared-but-empty rule-set reported as suspicious | |
| plausibility against enumerations another pack extended | |

**The boundary is written as a test, not a negotiated division of labour**: if someone later wants to
move a check, the question is objective — *does that side have the data?*

### What this does not claim, restated because it stays true

**None of this verifies faithfulness.** Every check above is internal consistency, and a perfectly
consistent transcription of the wrong table passes all of them. The cross-rendition check is the
closest thing to an external reference and it still only establishes that **two independent
digitisations of the same book agree** — not that either matches the printed page.

**The asymmetry the map accepted stands and is now visible in the table above**: tables carry
invariants with exact redundancy to exploit, kits carry almost none. **Tables end up with a stronger
guarantee than kits**, and that is a choice, not an oversight.

## What this ticket must not claim

**It does not verify faithfulness.** Every check above is internal consistency; none of them opens the
book. A perfectly consistent transcription of the wrong table passes all of them. The human half is
[ticket 11](./11-human-review-protocol.md), and the map has already accepted that tables end up with
a stronger guarantee than kits. Say so in the answer, so the asymmetry stays visible.
