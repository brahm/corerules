# The extraction pipeline: architecture and reproducibility

Type: grilling
Status: open
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
