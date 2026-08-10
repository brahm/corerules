# Reproducible LLM-assisted extraction of structured records

Type: research
Status: resolved
Blocked by: —

## Question

[v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md) made a claim and nobody checked it:

> **LLM-assisted extraction is the natural 2026 route** from RTF to structured JSON, which makes the
> bulk far less manual than "somebody types the books in" implied, and reduces a graphical editor's
> value proportionally.

That sentence did real work — it is part of why the v1 map decided corerules would not author packs
at all. It is also an unverified assumption about a fast-moving field, and
[ticket 09](./09-extraction-pipeline.md) cannot choose a pipeline architecture without it.

The interesting half is not "can a model extract a kit". It obviously can. The interesting half is
**everything that makes an extraction trustworthy enough to build a rules engine on**.

## What to find out

1. **Constrained/structured output as it actually stands.** Schema-constrained decoding, JSON-mode
   guarantees, and where they still fail. This matters directly: the pipeline's output must satisfy
   [ticket 05](./05-pack-schema.md)'s schema, and a model that *usually* emits valid JSON is a
   different engineering problem from one that cannot emit anything else.
2. **Reproducibility.** Whether the same input yields the same output across runs and across model
   versions, and what the field does when it does not. This is not a nicety —
   [ticket 07](./07-identity-and-id-stability.md) turns on it: pack-scoped IDs are referenced live by
   Character files, and an extractor that renumbers on re-run breaks every Character silently. Find
   out what determinism is actually available (temperature, seeds, versioned endpoints) and what it
   is *not*.
3. **Verification patterns that are known to work.** Self-consistency across runs, extract-then-verify
   with a second pass, round-tripping the JSON back to prose and diffing, adjudicating two
   independent extractions. Which of these detect the error class this map fears — a **plausible
   wrong number** rather than a malformed record. See [ticket 10](./10-mechanical-verification.md).
4. **Long-document handling.** The v1 tier is ~1.27M words; the PHB alone is ~249K. Chunking that
   does not sever a record, keeping a table's header with its rows, and what context windows make
   possible now that they did not in 2024.
5. **Cost and practicality at this scale.** Order-of-magnitude for ~1.27M words of input, run more
   than once (because the pipeline will be re-run as parsers improve). Include the local-model option
   honestly: the machine is a Fedora workstation, and there is an M1 Max with 32 GB on hand.
6. **Where deterministic parsing plainly wins.** The map's charting found tab-delimited tables with
   consistent column counts. A regex gets those exactly right, every time, for free. Establish what
   the field says about **hybrid** designs — deterministic for structure, model for judgement — since
   [ticket 01](./01-what-the-source-yields.md) is producing exactly that partition.

## What the answer must not do

**Do not send book content anywhere as part of this research.** This ticket reads about techniques;
it does not run an extraction. Trying one on real text is
[ticket 09](./09-extraction-pipeline.md)'s business, and whether the corpus may be sent to a
third-party API at all is a live question that ticket 09 must decide explicitly — the content is
licensed material the user owns, not the user's own work.

Findings go in `research/04-llm-assisted-extraction.md`.

## Answer

Findings: [`research/04-llm-assisted-extraction.md`](../research/04-llm-assisted-extraction.md)
(720 lines, 20 sources). No extraction was run and no book was opened.

**[v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md)'s claim is half true, and the
false half is the half that did the work.**

**Verified — the capability is real and cheap.** Schema-constrained decoding is a hard syntactic
guarantee; 1M-token context is standard; a full corpus pass costs tens of dollars.

**Refuted — "from RTF to structured JSON" names the wrong problem.** The corpus's dominant volume is
*already* tab-delimited with header lines and consistent column counts. The bulk is less manual
because the tables are already structured, **not** because a model reads them.

**Refuted as an argument.** Ticket 13 used the claim to conclude that a pack editor's value drops.
But what LLM assistance actually accelerates is the **judgement** half — precisely the half that
needs human review and spot correction forever. One input to
[ticket 12](./12-how-much-tool.md) is withdrawn.

### Reproducibility: none is available on a hosted API, by vendor admission

- **No `seed` parameter exists** in the Messages API. `temperature` is documented as not fully
  deterministic even at `0.0`, and on Claude 4.7+ a non-default value returns a **400** outright.
  *(Verified independently against the `claude-api` skill, not taken on the agent's word.)*
- **The cause is batch invariance, not floating point.** Thinking Machines (Sep 2025): server-side
  batch size varies with *other users' load*. Qwen3-235B at temperature 0 produced **80 unique
  completions in 1,000 runs, diverging at token 103** — well inside a single kit record.
- **No hosted snapshot survives this corpus's horizon.** Published retirement windows run 12–20
  months with 60 days' notice; the transcription runs for years.
- **Locally there is a *pin*, not determinism**: GGUF hash + commit + sampler + batch params.
  llama.cpp CPU is deterministic; the CUDA determinism PR is an unmerged draft.

**This lands directly on [ticket 07](./07-identity-and-id-stability.md): IDs must never be a
function of model output.** The deterministic half can carry source-position IDs safely; the judged
half needs a deterministic anchor or the minted-and-recorded option.

### Verification: every model-internal check fails on exactly the error class this map fears

The literature is specific, not general. Self-consistency correlates with accuracy at only ρ
0.10–0.30 on relation extraction — *a model can be confidently and consistently wrong*. Intrinsic
self-correction **degrades** performance (Huang, ICLR 2024), because *finding* the error is the
bottleneck while *fixing* it given the location is robust (Tyen, ACL Findings 2024). Two-model
adjudication suffers correlated failures that rise with capability (Goel, ICML 2025). And semantic
round-tripping is simply the wrong instrument: a record saying `18` instead of `19` round-trips to a
~0.999-similar sentence.

**The rule that falls out: a check only works if its reference is not the model.**

**This promotes [ticket 10](./10-mechanical-verification.md).** It is not the cheap consolation half
of verification — per Tyen it is the only thing that reliably *localises* this error class, and
localisation is the precondition that makes any repair, model or human, work at all.

### Deterministic parsing wins on every axis this map cares about

Accuracy, reproducibility, cost, auditability, and — decisively — **failure mode**: a loud
column-count mismatch versus a silent plausible number. [Ticket 01](./01-what-the-source-yields.md)'s
three-bucket partition is confirmed with one correction: **the "regular but ambiguous" middle bucket
belongs on the parser side, not the model side.** Its whole point is that somebody decides the rule
once and it applies identically forever — the opposite of what a model does per run.

A consequence worth carrying: if the parser owns the tables, **the model never sees a long table**,
and long-context degradation stops mattering.

### Constraint on [ticket 06](./06-expression-language.md), found here rather than there

**Recursive schemas are unsupported by structured outputs.** If the expression language is recursive
— parenthesised sub-expressions — the decoder **cannot enforce it at all**. Numeric ranges and string
lengths are likewise unenforceable and belong to the validator.

### Cost cannot bear this decision's weight

~$10–$100 per full corpus pass, halved by the Batch API; order $600–$3,000 for twenty re-runs over
five years. **The dominant cost is human review hours, which no ticket on this map has estimated** —
and §3's finding makes that number load-bearing rather than incidental.

### Hardware correction

The map called the workstation "a Fedora workstation". Measured: **14 GiB RAM (~9.3 GiB available),
Radeon 860M integrated, no discrete GPU, no inference runtime installed** — practical ceiling ~7–9B
at Q4. *(Verified independently.)* **The M1 Max 32 GB is the only viable local inference host**, at
roughly one overnight run for a full pass. Choosing local removes two of this map's three genuine
constraints — reproducibility, and ticket 09's licence question — at the cost of a night rather than
a dollar.

### Suggestion adopted into [tickets 08](./08-which-slice-proves-the-format.md) and [13](./13-transcribe-the-proving-slice.md)

Retain the hand-transcribed proving slice as the **permanent gold standard and regression test**.
Converts a one-off demo into ongoing verification at no extra cost.

### Left unestablished, and two of them matter

Nine items are listed in the findings. The load-bearing ones: **the corpus's real token count was
never measured** (ticket 09 should measure once with `count_tokens` on a single chunk before
trusting any cost figure); **no study exists of LLM extraction from already-delimited digital text**,
so the 3.3% cell-error prior comes from a harder task (scanned images) and the direction of
difference is favourable but the magnitude unknown; and **human review hours dominate every dollar
figure** and remain unestimated.
