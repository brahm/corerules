# The extraction pipeline: architecture and reproducibility

Type: grilling
Status: resolved
Blocked by: 01, 03, 04

## Question

How the 13 books become JSON. By this point three tickets have supplied the inputs: ticket 01 says
what each kind's source yields and which bucket it falls in, ticket 03 says whether anyone has done
it before, and ticket 04 says what LLM-assisted extraction can and cannot promise.

## What has to be decided

**1. The architecture.** Deterministic parser, model-driven, or hybrid — and if hybrid, *where the
seam is*. Ticket 01's three buckets are the natural partition: mechanical to a parser,
regular-but-ambiguous to a parser with a written rule, judgement to a model or a human. Confirm or
reject that mapping rather than inheriting it.

**2. Whether the corpus may be sent to a third-party API at all.** Not a technical question. The
books are licensed material the user owns — unlike a Character, which §6.5 calls the user's own work.
Sending 1.27M words of Wizards of the Coast text to a hosted model is a decision that deserves to be
made explicitly and recorded, with the local-model option (M1 Max, 32 GB) as the alternative that
avoids the question entirely. **Ticket 04 is forbidden from doing this; this ticket decides it.**

**3. How reproducible the pipeline must be**, which is where
[ticket 07](./07-identity-and-id-stability.md)'s answer lands. Re-running is not an edge case: the
pipeline will be re-run every time a parser improves, for years. Decide whether identical input must
give byte-identical output, and if not, what is allowed to vary.

**3b. The shape of its configuration.** [Ticket 02](./02-where-the-corpus-lives.md) settled that the
pipeline lives in the public repository, the corpus in a private one, and the sources on disk outside
git — connected by **configured paths, never a submodule**. So the pipeline takes **three paths**:
sources, corpus, and its own output location. Decide whether that is an env var, a config file, or
CLI arguments, and where the **source hash manifest** is verified — ticket 02 put the manifest in the
public repository precisely so the pipeline can check its inputs before running.

**4. What language it is written in.** The schema (ticket 05) has two consumers — this pipeline and
the Engine — and §7.2's one-evaluator rule means the expression language must not be implemented
twice from a description. TypeScript would share code with the Engine; Python has the better
extraction ecosystem. This is a real trade and it should be made on the schema-and-evaluator
argument, not on preference.

**5. Incrementality.** The corpus is transcribed over years, book by book, with corrections
throughout. A pipeline that only runs whole-corpus is unusable; one that runs per-record needs to
keep the manifest in step, which §7.1 requires and hand-editing can desynchronise.

**6. Where human judgement enters the loop**, mechanically. Not "someone reviews it" —
*which artifact does a human edit, and how does that edit survive the next run?* This is the
question that makes or breaks a re-runnable pipeline, and it is closely tied to ticket 07: an edit
that is overwritten on re-run is not a correction, it is a rehearsal.

## ⚠ Ticket 01's constraint was measured wrong — re-read this before item 1

The block below said the pipeline **must** read both renditions because whole books have kits only in
one. **The first half of that is void.** Re-measured in Python, the HTML carries **one titled page
per kit in every book** and at least the RTF's count everywhere, including the three books where the
RTF exposes nothing. The Complete Thief's "24 RTF against 7 HTML" was ugrep skipping binaries.

So the question this ticket now owns is the **opposite** of what was written: not *how do we join two
renditions*, but **is the RTF needed at all?**

Its remaining candidates are narrow and worth testing rather than assuming:

- **Field contents.** The HTML wins on *boundaries*; whether its `<P>`/`<FONT>` markup preserves
  within-record structure as well as the RTF's tabs is unmeasured.
- **A second independent extraction to diff against.** [Ticket 04](./04-llm-assisted-extraction.md)
  established that a check only works when its reference is not the model — and two renditions of the
  same book are exactly such a reference, free of charge.

**If the answer is "HTML only", the alignment stage disappears** and this ticket gets substantially
smaller. Settle it before choosing a seam.

The untitled-file precondition is **gone**: there are no untitled files.

## ~~What ticket 01 forces on the architecture~~ (see correction above)

[Ticket 01](./01-what-the-source-yields.md) measured both renditions and the result constrains item 1
harder than ticket 04 did:

- **The pipeline must read both renditions.** Not as an optimisation — the Complete Paladin's,
  Ranger's and Book of Elves have no kit structure in the RTF at all, and the Complete Thief's has
  three times more in the RTF than in the HTML. Reading one loses whole books.
- **So alignment is a first-class stage.** The two renditions share **no identifier**; joining a
  table read from the HTML to kit prose read from the RTF is by heading text, and that stage owns its
  own error modes.
- **Rendition choice is per book and per kind**, not global. The research file carries the table.
- **Settle the untitled-file question first.** Ticket 01's highest-value open item: HTML `<TITLE>`
  coverage ranges from 28% (Paladin) to 94% (Priest), and it is unknown whether untitled files are
  continuation pages of a titled record or independent records. That answer decides how much of the
  HTML is usable, so it comes before the seam is chosen — not after.

## What ticket 04 already supplied

[Ticket 04](./04-llm-assisted-extraction.md) answers **item 1** and supplies the evidence for
**item 3**:

- **Architecture: hybrid, with the seam at ticket 01's bucket boundaries** — and one correction to
  the obvious mapping: the **"regular but ambiguous" middle bucket belongs on the parser side**, not
  the model side. Its whole point is that a rule is decided once and applies identically forever,
  which is the opposite of what a model does per run.
- **Reproducibility: unavailable on a hosted API, by vendor admission.** Locally there is a version
  *pin* — GGUF hash, commit, sampler, batch params — which works because you own every variable.
- **Measure the token count once** with `count_tokens` on a single chunk before trusting any cost
  figure; every number in ticket 04 is derived from word counts times published ratios, and
  tab-delimited tabular text is the shape most likely to deviate.

Item 2 was deliberately left to this ticket. Note what ticket 04 adds to it: **the local option
removes the licence question entirely**, and costs a night rather than a capability — the M1 Max is
the only viable local host (the workstation has no discrete GPU; see the map's Notes).

## Answer

### The measurement that closed the inverted question

The HTML does not merely win on record boundaries — **it marks up the field labels themselves**.
Measured across the v1 tier: `<I>Weapon Proficiencies:</I>`, `<B>Special Benefits:</B>` and so on,
with the markup convention varying by book (`<I>` dominant in the Complete Thief's, `<B>` in the
Complete Book of Dwarves) but always *markup* rather than typographic convention.

The counts land within a few percent of the RTF's Python census, **and they disagree in both
directions**:

| field | HTML markup | RTF census |
|---|---:|---:|
| Weapon Proficiencies | 146 | 149 |
| Role | 143 | 147 |
| Special Hindrances | 138 | 146 |
| Minimum Ability Scores | 68 | 68 |
| Spheres of Influence | 68 | 68 |
| Followers and Strongholds | 60 | 60 |
| Description | **131** | 104 |
| Equipment | 97 | **128** |

### Decision 1 — the HTML is the only parse target; the RTF becomes a cross-check

Rejected: parsing both and merging, and discarding the RTF.

Marked-up labels beat labels-by-convention, because the parser never has to infer what is a heading.
That closes the first of the two candidate roles the RTF had left.

**The second role survives, and the table above is why.** The two renditions disagree by a few
percent per field, in both directions — and that disagreement is exactly the check
[ticket 04](./04-llm-assisted-extraction.md) established as the only kind that works: *a check only
works if its reference is not the model*. Two independent renditions of the same book are such a
reference, already on disk, at no cost. Discarding the RTF throws away the only free non-model
reference this project will ever have, and after transcription comparison becomes expensive again.

Merging was rejected because it buys nothing: it costs the alignment stage — the expensive part —
while the HTML needs no help producing records.

**So the alignment stage disappears.** One parse target, no join by heading text, and cross-rendition
comparison becomes a count-and-boundary diff that belongs to
[ticket 10](./10-mechanical-verification.md), not here.

### Decision 2 — local models only; the corpus is never sent to a third-party API

Deliberately deferred by [ticket 04](./04-llm-assisted-extraction.md) and decided here.

- **[Ticket 02](./02-where-the-corpus-lives.md) already drew this line for a weaker act.** It kept
  verbatim book text off every third-party service and called that the stronger claim its
  packs-may-be-backed-up decision deliberately did not make. **Inference is a stronger form of egress
  than storage** — transmitted, processed, possibly retained. If storing in a private repository was
  judged too strong, sending for inference is not weaker.
- **Ticket 04 measured that reproducibility exists only locally.** A pin — GGUF hash, commit,
  sampler, batch params — works because you own every variable. There is no `seed` on a hosted API
  and no model snapshot survives this transcription's horizon, while the pipeline will be re-run for
  years.
- **Decision 1 removed the cost objection.** With labels marked up and boundaries titled, the
  deterministic parser takes almost everything; the model's share is the judgement half only —
  hundreds of field values across the tier, dozens for the proving slice. "Local is too slow" no
  longer has a factual basis.

**Accepted risk, to be measured rather than assumed:** a local 7–30B model against a frontier one, on
exactly the half that needs judgement. The counterweight is that
[ticket 11](./11-human-review-protocol.md) reviews the judgement half regardless, so the model
produces a draft rather than a verdict — but if the draft is bad enough, reviewing costs more than
writing from scratch and this decision pays for itself in human hours.
**[Ticket 13](./13-transcribe-the-proving-slice.md) must measure draft quality on the slice**, since
that is the first and cheapest place the risk becomes visible.

### Decision 3 — Python

Rejected: TypeScript.

**[Ticket 05](./05-pack-schema.md) already removed the reason to be TypeScript** by making JSON
Schema canonical precisely so the schema would be language-neutral. A Python pipeline and a
TypeScript Engine validate against the *same* schema with off-the-shelf libraries on each side —
that is not two implementations of the language, it is two stock validators, so §7.2's drift warning
does not apply. And [ticket 06](./06-expression-language.md) removed the rest: with predicates as
structure there is no evaluator to share.

What is left points one way: the extraction ecosystem, the local-inference path on the M1 Max, and
the fact that this map's existing instruments — [`dertf.py`](../tools/dertf.py) and
[`census.py`](../tools/census.py) — are already Python.

### Decision 4 — git is the overlay; the pipeline never merges

The ticket named this the make-or-break: *an edit overwritten on re-run is not a correction, it is a
rehearsal.*

**The mechanism already exists and [ticket 02](./02-where-the-corpus-lives.md) installed it.** The
corpus repository *is* the Engine's content folder, with history. A correction is a commit.
Re-extraction writes over the working tree and `git diff` shows exactly what the new parser changed —
**including where it undid a human correction, which is the case that matters**. The human
adjudicates with `git add -p`, a tool they already have.

Rejected: a separate overlay of corrections re-applied after each extraction. That is a second
artifact with its own drift, recreating the defect §8 rejected when it refused a persistent index —
*a second source of truth able to go quietly stale* — and worse, overlay corrections are invisible to
the Engine, which reads the folder.

**Accepted cost, an operating rule rather than code:** re-extraction runs against a clean tree, or
the diff mixes parser change with work in progress. Anyone using git already follows it.

### Consequences for the pipeline's shape

- **Incrementality falls out.** Git-as-overlay means the pipeline can run per book or per record and
  the diff stays legible; a whole-corpus-only run was never compatible with this.
- **Three configured paths**, per ticket 02: sources, corpus, output. The pipeline verifies the
  source hash manifest before running — that manifest lives in this public repository so it can.
- **The seam is ticket 01's buckets**, with the *regular-but-ambiguous* middle on the **parser** side
  per ticket 04, because a rule decided once and applied identically forever is the opposite of what
  a model does per run.

## Why this is not "build the tool"

[Ticket 12](./12-how-much-tool.md) decides how much tool. This ticket decides the pipeline's shape —
the stages, the seam between deterministic and judged, the reproducibility contract. Tooling is what
gets wrapped around that afterwards, and the map deliberately puts it last so the answer is measured
rather than assumed.

## The failure this ticket exists to prevent

**A pipeline that works once.** The tempting design extracts everything in one heroic run and leaves
a directory of JSON that nobody can regenerate — at which point the JSON is the source of truth, the
books are decoration, and every future correction is hand-editing forever. That is the outcome
[v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md) accepted as the *cost* of not
building an editor. It is a much worse outcome if it arrives by accident.
