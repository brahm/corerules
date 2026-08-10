# Reproducible LLM-assisted extraction of structured records

Research for [ticket 04](../issues/04-llm-assisted-extraction.md). Conducted 2026-08-10.
Decisions belong to tickets 07, 09, 10 and 12; this file supplies the evidence they were blocked on.

**Nothing was extracted.** No book was opened, converted, sampled, or sent anywhere. Every number
about the corpus below is derived from the map's charting measurements and published tokenizer
ratios, never from reading the source. No book text appears in this file.

---

## Verdict

The v1 ticket 13 sentence under test:

> **LLM-assisted extraction is the natural 2026 route** from RTF to structured JSON, which makes the
> bulk far less manual than "somebody types the books in" implied, and reduces a graphical editor's
> value proportionally.

**Half verified, half refuted — and the refuted half is the load-bearing one.**

Verified: the capability is real and cheap. Schema-constrained decoding is a hard syntactic
guarantee, not a prompt-and-hope; million-token context windows are default and unpriced-extra;
a full pass over the v1 tier costs tens of dollars, not thousands. Nothing about 2026 makes
model-assisted extraction difficult or expensive.

Refuted: **"from RTF to structured JSON" describes the wrong problem.** The corpus's dominant
volume is not prose needing interpretation — it is tab-delimited rows with a header line and a
consistent column count (map, measurement 1). A `split('\t')` transcribes those exactly, every
time, for free, byte-reproducibly. The best-documented comparable pipeline in the literature —
a Federal Reserve Bank of Philadelphia working paper digitising heterogeneous historical numeric
tables with a multimodal LLM — reports **96.7% exact-match on the cells it extracts and a 4.64%
total error rate**, and its authors explicitly warn that "applications that turn on an exact value
in every cell warrant further verification" ([FRB Philadelphia WP 25-28][fed]). corerules is precisely
such an application: `spec.md` §4 stores nothing and recomputes every value through the layer
stack, so one wrong cell poisons every Character silently. Routing already-delimited rows through
a model converts a 0%-error operation into a ~3%-error operation and buys nothing.

Refuted more sharply as an *argument*: the sentence's real work in v1 ticket 13 was "the bulk is
far less manual, therefore an editor's value drops." But the bulk is less manual because the tables
**are already delimited**, not because a model reads them. That half was always going to be a
script. What model assistance actually accelerates is the *judgement* bucket — prose-stated kit
benefits and hindrances, the Complete Priest's records, effects that must become predicates — and
that is exactly the half the map already says needs human review forever, and exactly the half a
correction tool would serve. **The premise supports the conclusion far less than it appears to.**
This does not settle [ticket 12](../issues/12-how-much-tool.md); it removes one unearned input to it.

The three things that actually constrain this pipeline are, in order: **reproducibility over years
(§2), verifiability against a wrong-but-plausible number (§3), and the licence question that
[ticket 09](../issues/09-extraction-pipeline.md) owns.** Cost is not on the list.

### How to read this file

- **[V]** — verified against a primary source, cited inline.
- **[I]** — my inference from the cited facts. Reasoning shown so it can be attacked.
- **[E]** — arithmetic extrapolation from published rates. Assumptions stated.
- Everything I could not pin down is in [Things I could not establish](#things-i-could-not-establish).

---

## 1. Constrained / structured output as it actually stands

**[V] The guarantee is real and it is syntactic only.** Anthropic's structured outputs
(`output_config.format` with a `json_schema`) enforce the schema through constrained decoding.
The documentation is unambiguous about the boundary: it guarantees "schema-compliant responses
through constrained decoding" — "Always valid: No more `JSON.parse()` errors" — and equally
unambiguous that "there is no guarantee of semantic correctness. Claude's response will be valid
JSON matching your schema, but the *content* may still be incorrect or nonsensical."
([Anthropic, structured outputs][so])

This is the single most important framing for ticket 05. **A model that cannot emit anything but
valid pack JSON is a solved engineering problem. A model that emits valid pack JSON containing the
wrong number is the same problem the map already fears, wearing a schema.** Constrained decoding
moves the entire failure surface from "does it load" to "is it true", which is the surface
[ticket 10](../issues/10-mechanical-verification.md) has to cover.

**[V] Where the schema itself cannot help — and this bites ticket 05 and ticket 06 directly.**
The following are not supported and return a 400 ([Anthropic, structured outputs][so]):

| Unsupported | What corerules wanted it for |
|---|---|
| **Recursive schemas** | The [ticket 06](../issues/06-expression-language.md) expression language. If the grammar is recursive — and an expression grammar with parenthesised sub-expressions is — **the decoder cannot enforce it at all.** |
| `minimum` / `maximum` / `multipleOf` | Ability minima in 3–18, THAC0 ranges, level bounds — exactly ticket 10's "range plausibility" checks. |
| `minLength` / `maxLength` | Non-empty name fields. |
| Array constraints beyond `minItems` of 0 or 1 | Fixed-arity rows (a saving-throw matrix has a known shape). |
| `additionalProperties` other than `false` | — (this one is fine; `false` is what you want). |
| External `$ref` | Splitting the pack schema across files. |
| Most regex features | ID format enforcement (`phb:set-snares`). |

**[I] Consequence for ticket 05:** the pack schema has two tiers whether it wants them or not — the
subset a decoder can enforce (shape, field presence, types, enums) and the rest, which must be a
validator that runs *after* generation. Since `spec.md` §7.6 already requires a validator that names
file, record and field, and §7.2 forbids implementing the same thing twice, the sane reading is:
the decoder enforces shape, the §7.6 validator enforces everything else, and the extraction
pipeline runs the §7.6 validator on its own output before writing anything. The decoder is a
convenience, not a checkpoint.

**[V] Operational costs of the feature that are easy to miss.** Grammar compilation adds latency
on first use of a schema; compiled grammars are cached 24 hours from last use; changing the schema
structure or the tool set invalidates that cache; and **changing `output_config.format` invalidates
the prompt cache for the conversation thread**. Structured outputs also inject an additional system
prompt, so input token counts rise slightly. ([Anthropic, structured outputs][so])

**[I]** The prompt-cache interaction matters for a multi-pass design (§3): if the extract pass and
the verify pass use different schemas, the chunk is not shared between them at cache-read rates.
Use one schema per chunk-pass family, or accept the extra input billing.

**[V] Constrained decoding can degrade the content it constrains.** "Let Me Speak Freely?"
(Tam et al., 2024) found "a significant decline in LLMs reasoning abilities under format
restrictions", and that "stricter format constraints generally lead to greater performance
degradation in reasoning tasks" ([arXiv:2408.02442][lmsf]). The standard mitigation is to separate
reasoning from formatting: reason free-form, then emit under constraint.

**[I]** On current Claude models this is substantially mitigated for free, because thinking is on by
default on Claude Opus 5 and runs before the constrained output block — the unconstrained reasoning
happens in a place the schema does not reach. I did not find a study measuring the residual effect
on a thinking model with adaptive thinking enabled; treat the mitigation as plausible, not proven.

**[V] Coverage across engines is a known weak spot.** JSONSchemaBench (Geng et al., 2025;
10,000 real-world schemas, six engines including Guidance, Outlines, llama.cpp, XGrammar, OpenAI
and Gemini) exists precisely because "coverage of diverse constraint types" varies materially
between implementations ([arXiv:2501.10868][jsb]). If ticket 09 picks a local runtime, the schema
it can actually compile is an engine-specific question, not a JSON Schema question.

**[V] Locally, the same guarantee is available and self-hosted.** llama.cpp's GBNF grammars are a
first-class constraint mechanism with JSON-Schema-to-GBNF conversion, supported in `llama-cli`,
the completion tools and `llama-server` ([llama.cpp grammars README][gbnf]). This is worth
noting for ticket 09: the structured-output guarantee is **not** a reason to prefer a hosted API,
because you can own it.

---

## 2. Reproducibility — the priority question

Short answer: **on a hosted API there is none, by design and by vendor admission; locally there is
a version-pinned approximation that is good enough if you treat it as an engineering discipline.
Byte-identical output across years is not available from any endpoint you do not host.**

### 2a. What Anthropic actually offers [V]

- **There is no `seed` parameter.** The Messages API request body accepts `max_tokens`, `messages`,
  `model`, `cache_control`, `container`, `inference_geo`, `metadata`, `output_config`,
  `service_tier`, `stop_sequences`, `stream`, `system`, `temperature`, `thinking`, `tool_choice`,
  `tools`, `top_k`, `top_p`. Nothing else. No determinism control exists.
  ([Anthropic, Messages API reference][msgs])
- **`temperature` does not do what you would want even where accepted.** The parameter's own
  documentation says: *"Note that even with `temperature` of `0.0`, the results will not be fully
  deterministic."* ([Anthropic, Messages API reference][msgs])
- **On current frontier models you cannot set it at all.** `temperature`, `top_p` and `top_k`
  "return a 400 error when set to a non-default value on Claude 4.7 and later models"
  ([Anthropic, model deprecations][dep]).
- **Adaptive thinking adds a further unreproducible internal step.** On Claude Opus 5 thinking is on
  by default and Claude "determines its thinking allocation dynamically, so thinking token usage
  varies from request to request" ([Anthropic, context windows][ctx]).

For comparison, OpenAI is the vendor that tried hardest: `seed` plus `system_fingerprint`, and the
documented promise is that the system "will make a best effort to sample deterministically." Their
own guidance adds that determinism "isn't guaranteed", that variability is "not uncommon" even with
matching seed and fingerprint, and that the fingerprint changes whenever they update infrastructure
([OpenAI cookbook, reproducible outputs][oai]).

### 2b. Why, mechanically — and why it is worse than it looks [V]

Thinking Machines Lab (Sep 2025) identified the actual cause, and it is not floating-point
non-associativity. Non-associativity explains *why* different reduction orders give different
results; it does not explain *when* orders differ. The cause is **lack of batch invariance**:
inference kernels produce different results for the same input depending on the batch they were
executed in, and *"the primary reason nearly all LLM inference endpoints are nondeterministic is
that the load (and thus batch-size) nondeterministically varies"* ([Thinking Machines][tml]).

Their measurement is the one number that settles this question. Qwen3-235B, 1,000 completions of
the same prompt at temperature 0:

> "we generate **80 unique completions**, with the most common of these occurring 78 times."

Divergence began at **token 103**. With their batch-invariant kernels, all 1,000 completions were
identical — at a cost of 1.6×–2.1× throughput ([Thinking Machines][tml]).

**[I] The operational reading for corerules: on a shared endpoint, your extraction output is a
function of other people's traffic, and the divergence starts about a hundred tokens in — which is
well inside a single kit record.** There is no request parameter that fixes this because the cause
is on the server side of the request.

### 2c. Across model versions: the horizon is shorter than the project [V]

The pipeline "will be re-run — many times, over years" (ticket 07). The endpoints will not be there.
Anthropic commits to "at least 60 days' notice before model retirement for publicly released
models", and the published table shows what that means in practice ([Anthropic, model
deprecations][dep]):

| Model | Deprecated | Retired |
|---|---|---|
| `claude-3-5-sonnet-20241022` | 2025-08-13 | 2025-10-28 |
| `claude-3-7-sonnet-20250219` | 2025-10-28 | 2026-02-19 |
| `claude-sonnet-4-20250514` | 2026-04-14 | 2026-06-15 |
| `claude-opus-4-1-20250805` | 2026-06-05 | 2026-08-05 |

Retirement dates for currently active models are given as "not sooner than" a date **12 to 20
months out** (`claude-opus-5`: not sooner than 2027-07-24; `claude-sonnet-5`: 2027-06-30;
`claude-opus-4-5-20251101`: 2026-11-24). Anthropic is candid about the cost — the page names
"Researchers lose access to models for ongoing and comparative studies" as an acknowledged downside
— and points to weight-preservation commitments, but preserved weights are not a callable endpoint.

**[I] Therefore: no hosted model snapshot survives the transcription horizon of this corpus.** A
pipeline whose output depends on a hosted model is not re-runnable in year three; it is
re-*executable* against a different model, which is a different thing and produces different JSON.

### 2d. What determinism is available locally [V]

- **llama.cpp CPU inference is already deterministic**; the open work is about GPU backends.
- **CUDA deterministic mode exists only as a draft PR** (`#16016`, `-DGGML_DETERMINISTIC=ON` +
  `--deterministic`), unmerged, covering RMSNorm, matmul, attention and KV-cache layout. The
  maintainer response is the important part: *"I don't want to maintain guarantees for bit-for-bit
  identical results as the batch size is varied."* Metal and other backends are not covered
  ([llama.cpp PR #16016][pr]).
- Reproducible sampling requires `temperature=0, top_k=1, top_p=1` — knobs the Anthropic API no
  longer exposes.

**[I] The local reproducibility contract you can actually write** is not "the model is
deterministic" but a pin, enforced by the pipeline and recorded in the pack manifest:

```
model      = sha256 of the GGUF file
runtime    = llama.cpp commit hash + build flags
sampler    = temperature 0, top_k 1, top_p 1, fixed seed
batching   = fixed n_batch / n_ubatch / context size
backend    = named (CPU is the only one with a determinism claim today)
prompt     = hash of the rendered prompt template
```

Under that pin, re-running gives byte-identical output — because you own every variable. It breaks
the day you upgrade the runtime, and it should: that is a visible, dated, diffable event, not
silent drift.

### 2e. What this hands to ticket 07 [I]

Ticket 07 asked whether reproducible model output is achievable, saying that if not, content-derived
and source-position-derived IDs become "much more attractive than option 3." The answer is
conditional, and it splits along the same seam as everything else in this file:

- **For the deterministic half** (tab-delimited tables), output is byte-reproducible with no model
  involved and no pin required. Source-position-derived IDs are safe here, because segmentation is
  driven by the header line and column count — stable properties of the source, not of a parser's
  taste.
- **For the judged half**, output is *not* reproducible on a hosted API at all, and locally only
  under a pin that you will deliberately break every time you improve the prompt or the model. So:
  **an ID must never be a function of model output.** That rules out content-hash IDs on
  model-extracted records (they churn on every prompt improvement, turning corrections into
  delete-plus-insert, which is the failure mode ticket 07 already named) and it rules out ordinals
  assigned by a model-driven segmenter.
- Which leaves, for the judged half: **mint once and record** (ticket 07's option 3), or an anchor
  derived from something deterministic near the record — a heading, a label, a book-and-ordinal
  computed by the deterministic pass rather than the model.

I am not deciding this; ticket 07 is. But "reproducible model output is not available on the
horizon this project runs on" is the input it was waiting for.

---

## 3. Verification patterns — which ones catch a plausible wrong number

This is the map's stated fear: not a malformed record, which fails loudly at load (§7.5), but
`THAC0 18` where the book says `19`, which loads perfectly and poisons every Character.

**The finding is uncomfortably clean: every verification pattern that compares the model to itself
fails on this error class, and the literature says so specifically. Only checks that compare against
something outside the model catch it.**

### 3a. The evidence, pattern by pattern

**Self-consistency across runs — does not work here. [V]**
The most directly relevant study is on entity and relation extraction (plant-health corpus,
GPT-4o-mini / DeepSeek-V3 / Kimi / Qwen3-32b, 5 runs per document, Fleiss' Kappa across runs). The
finding: *"All four models demonstrate weak correlations between 0.10 and 0.30 in all tests,
indicating a limited association between accuracy and consistency"*, that *"high consistency does
not strongly correlate with high accuracy"*, and — the sentence that matters — *"a model can be
confidently and consistently wrong."* The authors conclude that *"consistency alone should not be
relied upon as a proxy for prediction quality in relational IE tasks"* ([Genomics & Informatics 2025][cons]). Separately, self-consistency's returns have been shrinking as base models
improve, because its premise was high sampling variance ([arXiv:2511.00751][scedge]).

**[I]** This is the expected result and it should have been predictable: a misread digit is not a
sampling accident, it is a stable consequence of how the model reads that span. Resampling five
times reproduces it five times. Self-consistency detects *uncertainty*; a plausible wrong number is
characterised by the absence of uncertainty.

**Second-pass self-verification — does not work here. [V]**
Huang et al. (ICLR 2024) find that LLMs "struggle to self-correct their responses without external
feedback", and that intrinsic self-correction can *degrade* performance
([arXiv:2310.01798][selfcorr]). Tyen et al. (ACL Findings 2024) locate the mechanism: models
"generally struggle" at mistake-*finding* "even in highly objective, unambiguous cases", while
their correction ability, given the error location, is robust across five reasoning tasks
([arXiv:2311.08516][tyen]).

**[I] This is the most actionable finding in the whole ticket, and it inverts the obvious design.**
Do not ask the model to check its work. Build a mechanical checker that *localises* — and then hand
the located error to the model or the human for repair, where both are good. Ticket 10's list
(row and column sums, monotonic THAC0 steps, saving-throw matrix shape, record counts against the
book's own index, referential integrity) is not merely a set of cheap checks; per Tyen it is the
missing input that makes model-assisted correction work at all. **Ticket 10 is the enabler for
ticket 04, not a consolation prize for it.**

**Adjudicating two independent extractions — weaker than it sounds. [V]**
"Great Models Think Alike and this Undermines AI Oversight" (Goel et al., ICML 2025) introduces
CAPA (Chance Adjusted Probabilistic Agreement) and finds that model mistakes become *more* similar
as capabilities increase, and that LLM-as-judge scores favour models similar to the judge — a
generalisation of self-preference ([arXiv:2502.04313][alike]). Zheng et al.'s LLM-as-judge work
already established position and self-preference biases.

**[I]** So "two independent extractions" is only as good as the independence, and independence is
declining. Two frontier models from the same generation reading the same ambiguous digit are not
two witnesses; they are close to one witness counted twice. If ticket 09 wants adjudication as a
detector, the second extractor must be *structurally* different — a regex, a different modality, or
the book's own arithmetic — not merely a different vendor's model of similar capability.

**Round-trip to prose and diff — depends entirely on the diff. [V/I]**
The published form of this technique reconstructs a sentence from the extracted structure and
compares via sentence embeddings for semantic equivalence ([arXiv:2508.03438][roundtrip]).
**[I] Embedding similarity is precisely the wrong instrument for this error class**: a record
that says 18 where the book says 19 round-trips to a sentence that is ~0.999 cosine-similar to the
original. Semantic round-tripping catches dropped fields and invented ones. It does not catch
digits.

The variant that *does* work is a round-trip to the source's own delimited form followed by an
**exact byte diff** — regenerate the tab-delimited row from the JSON and compare it character for
character against the source line. **[I] But notice what that is: if you can regenerate the source
line exactly, you have a total, invertible mapping between the row and the record, which is a
deterministic parser. The check that works is the one that proves you did not need the model.**

**Mechanical invariants and external counts — these work. [V/I]**
These are the only detectors in the set that consult something other than the model: the source
bytes, an arithmetic redundancy, the book's own table index, the schema's referential graph. They
are ticket 10's remit and the map has already decided they are cheap and objective.

**Human review against the book — works, and stays scoped. [V]**
The map already scopes this to the judgement half and accepts the resulting asymmetry. The
Philadelphia Fed pipeline reached its reported accuracy only because the authors hand-built a
50,653-cell gold standard across 364 tables and evaluated against it; their standing recommendation
is that *"creation of gold standard data and evaluation of the pipeline using such data as key
ingredients for the success of our (or any) pipeline"*, with the mitigating note that *"gold
standard data need not be costly… Reliable evaluation can often be achieved with moderately sized
datasets"* ([FRB Philadelphia WP 25-28][fed]).

**[I] Read against [ticket 08](../issues/08-which-slice-proves-the-format.md) and
[ticket 13](../issues/13-transcribe-the-proving-slice.md): the proving slice should double as the
gold standard.** Transcribe it by hand, keep it, and make every future pipeline run report its
delta against it. That converts ticket 13 from a one-off demonstration into the permanent regression
test for the pipeline, at no extra cost.

### 3b. Summary table

| Pattern | Catches malformed record | Catches plausible wrong number | Basis |
|---|---|---|---|
| Schema-constrained decoding | Yes, by construction | **No** — explicitly disclaimed | [so] |
| Post-hoc schema validator (ranges, arity) | Yes | **Partly** — only implausible values | ticket 10 |
| Self-consistency over N runs | Partly | **No** — ρ 0.10–0.30; "consistently wrong" | [cons] |
| Model self-verification pass | Weak | **No** — finding errors is the failure | [selfcorr], [tyen] |
| Two model extractions + adjudication | Partly | **Weakly** — correlated failures rising | [alike] |
| Round-trip to prose + semantic diff | Yes | **No** — embeddings ignore one digit | [roundtrip], [I] |
| Round-trip to delimited source + byte diff | Yes | **Yes** — but this *is* a parser | [I] |
| Arithmetic invariants (sums, monotonic steps, shape) | Yes | **Yes**, where redundancy exists | ticket 10 |
| Record counts vs the book's own index | Yes | No (counts, not values) | ticket 10 |
| Human review against the book | Yes | Yes | [fed] |

**The rule that falls out: a check is only useful against this error class if its reference is not
the model.** Source bytes, arithmetic, the book's own index, a human. Everything else is theatre
with respect to the failure the map actually fears.

---

## 4. Long-document handling

**[V] What genuinely changed since 2024.** Claude Opus 5, Opus 4.8/4.7/4.6, Sonnet 5, Sonnet 4.6
and Fable 5 all have a **1M-token context window as the default** — no beta header — and
long-context requests are "billed at standard pricing", with a 900k-token request billed at the same
per-token rate as a 9k one ([Anthropic, context windows][ctx]; [Anthropic, pricing][price]). Max
output is 128k tokens. In 2024 this was a beta with a long-context premium; it is now the plain
default.

**[E] The corpus against that window.** Anthropic's own rule of thumb is ~0.75 words per token
([Anthropic, pricing][price]), so the v1 tier's ~1.27M words ≈ **1.7M tokens** on the Sonnet-4.6-era
tokenizer. Claude 4.7 and later "use a newer tokenizer… [that] produces approximately 30% more
tokens for the same text" ([Anthropic, pricing][price]) → ≈ **2.2M tokens**. Tab-delimited tables,
abbreviations and numerals fragment worse than prose, so I carry **2.0M–2.7M input tokens** as the
working figure for one full pass. The PHB alone (~249K words) is ≈ 330K–430K tokens — **it fits in
one window; the corpus does not.**

**[V] Fitting is not the same as working, and the vendor says so.** Anthropic's context-windows page:
*"As token count grows, accuracy and recall degrade, a phenomenon known as context rot. This makes
curating what's in context just as important as how much space is available."* ([Anthropic, context
windows][ctx])

The independent measurement is harsher. NoLiMa (Modarressi et al., ICML 2025) removes the literal
lexical overlap that makes needle-in-a-haystack easy, and evaluates 13 models that all claim ≥128K
support. At **32K tokens, 11 of 13 fall below 50% of their own short-context baseline**; GPT-4o,
the strongest, drops from 99.3% to 69.7%; Llama 3.3 70B from 97.3% to 42.7%. The authors' account:
*"In the absence of strong surface-level cues (e.g., literal matches), locating relevant facts
becomes challenging for the model"* ([arXiv:2502.05167][nolima]).

**[I] So: do not put a book in the window and ask for its kits.** Chunk small. The 1M window's value
here is not bulk throughput.

**[I] Chunking that cannot sever a record — and the corpus makes this easy.** The map's charting
already found the two boundary markers a chunker needs, and both are deterministic:

- **Kit records** are delimited by their own label vocabulary (`Weapon Proficiencies`, `Role`,
  `Special Benefits`, `Special Hindrances`, `Secondary Skills`, …), and the Complete Priest's
  records by a different ten-label set. A chunker that splits on label-set boundaries cannot cut a
  record in half. A chunker that splits on a token budget will, roughly every time the budget
  lands mid-record.
- **Tables** have a header line and a consistent column count. Keeping the header with its rows is
  therefore not a heuristic — the chunker prepends the known header to every chunk of that table
  as a mechanical operation, with zero reliance on the model noticing.

**[I] But the strongest point about §4 is that §6 mostly dissolves it.** If the deterministic parser
owns the tables, the model never sees a long table at all. What remains for chunking is the
judgement bucket — kit-sized records, hundreds of tokens each — where long-context degradation is
not a factor. The long-document problem is largely an artefact of trying to make the model do the
table work.

**[I] Where the 1M window is genuinely worth paying for: disambiguation, not extraction.** "Here is
the whole PHB; here is this one record; does anything elsewhere in the book modify it?" is a query
the 2024 context window could not express and this one can. At ~330K–430K tokens of book context,
that costs roughly $1.65–$2.15 per query on Opus 5 at standard input rates — but with prompt caching
the book becomes a cached prefix at 1.25× to write and 0.1× to read ([Anthropic, pricing][price]),
so repeated queries against the same book drop to roughly **$0.17–$0.22 each**. That is a real,
new, affordable capability, and it belongs to the judgement half.

---

## 5. Cost and practicality at this scale

**Headline: cost is not a constraint on this decision, at any model tier, at any plausible re-run
count. Anyone arguing for or against the API route on cost grounds is arguing about the wrong
thing.**

### 5a. Hosted API [E, from V prices]

Prices below are Anthropic first-party rates as published on 2026-08-10 ([Anthropic, pricing][price]).
Note Sonnet 5's introductory $2/$10 expires **2026-08-31**, three weeks from this writing.

Assumptions, all stated so they can be attacked: **2.4M input tokens** (mid-range of the §4
estimate) and **1.5M output tokens** per full pass over the 13 books. Output volume is the softest
assumption — extraction JSON with field names can exceed the source it describes; I assume roughly
0.6× input.

| Model | Input $/MTok | Output $/MTok | One full pass | With Batch API (−50%) |
|---|---|---|---|---|
| Haiku 4.5 | $1 | $5 | ~$10 | ~$5 |
| Sonnet 5 (intro, to 2026-08-31) | $2 | $10 | ~$20 | ~$10 |
| Sonnet 5 (from 2026-09-01) | $3 | $15 | ~$30 | ~$15 |
| Opus 5 | $5 | $25 | ~$50 | ~$25 |
| Fable 5 | $10 | $50 | ~$100 | ~$50 |

The Batch API gives "a 50% discount on both input and output tokens", most batches complete within
an hour, maximum 24 hours ([Anthropic, pricing][price]) — a natural fit for a corpus pipeline with
no latency requirement.

**Multi-pass verification.** A three-pass design (extract → independent second extraction →
adjudicate) roughly triples output cost, but prompt caching cuts the repeated input: writing the
chunk once at 1.25× then reading it twice at 0.1× costs 1.45× instead of 3.0×
([Anthropic, pricing][price]). Working figure: **$30–$150 per fully-verified corpus pass**,
Sonnet-to-Opus tier, batch-priced.

**Over the project's life.** Twenty full re-runs across five years: **order $600–$3,000 at the Opus
tier, $200–$600 at Sonnet.** Even multiplying every assumption above by three, this stays below the
cost of a single week of professional time. **[I] Cost cannot bear the weight of this architectural
decision. The reproducibility argument (§2) and the licence question (ticket 09) can.**

### 5b. Local models — the honest version

**[V] Measured on this machine, today.** The map calls it "a Fedora workstation." It is not a
workstation:

```
CPU   AMD Ryzen AI 7 350 (8C/16T, Krackan)
RAM   14 GiB total — 5.5 GiB in use, ~9.4 GiB available at time of measurement
GPU   Radeon 860M integrated (RDNA 3.5, gfx1152) — no discrete GPU, no nvidia-smi
      shares system RAM; no dedicated VRAM
Local inference runtime installed: none (no ollama, no llama-cli, no llama-server)
```

**[I] With 14 GiB total and ~9 GiB free, this machine's practical ceiling is a 7B–9B model at Q4.**
That is below the size class I would trust for the judgement bucket. Community benchmarks report
~8.6 tok/s generation on a Radeon 860M via llama.cpp's Vulkan backend with all layers offloaded
([llama.cpp discussion #10879][vulkan]) — indicative, not verified here, and for a small model.
**[E]** At that rate, 1.5M output tokens is ~48 hours of pure generation per pass. The Fedora box is
a fine place to run the deterministic half and orchestrate; it is not the inference host.

**[V/E] The M1 Max with 32 GB is the local option that actually works.** Unified memory at ~400 GB/s
with no discrete-VRAM ceiling; 30B-class Q4 models "run comfortably" in 32 GB. Since generation is
memory-bandwidth-bound, the arithmetic is straightforward and I give it as arithmetic, not
measurement:

| Model shape | Weights @ Q4 | Bandwidth-bound ceiling | Realistic |
|---|---|---|---|
| Dense ~30B | ~17 GB | ~23 tok/s | **10–20 tok/s** |
| MoE ~30B, ~3B active | ~17 GB resident, ~2 GB active | high | **40–70 tok/s** |

**[E] A full local pass:** at ~50 tok/s on a 30B-A3B-class MoE, 1.5M output tokens is ~8.3 hours of
generation, plus prefill of ~2.4M input tokens. Call it **one overnight run per pass; two to three
days for a fully-verified three-pass run.** Marginal cost: electricity.

**[I] The local route's real advantage is not price — it is §2 and ticket 09's question 2 at once.**
You keep the weights, so the pin in §2d is enforceable and re-running in 2029 is possible. And no
Wizards of the Coast text leaves the machine, which makes ticket 09's third-party-API question moot
rather than answered. That is two of this map's three genuine constraints removed by one choice,
and it costs a night instead of a dollar.

**[V] Local structured output is not a compromise:** llama.cpp's GBNF grammars with
JSON-Schema-to-GBNF conversion give the same hard syntactic guarantee, self-hosted
([llama.cpp grammars README][gbnf]) — subject to the engine-coverage caveat in §1
([arXiv:2501.10868][jsb]).

**[I] What local costs you** is capability on the judgement bucket. A 30B open-weights model is not
Opus 5 at modelling a prose-stated hindrance as a predicate. Ticket 09 has to measure that gap on
the proving slice rather than assume it either way — and note the asymmetry that makes the
measurement cheap: the deterministic half needs no model at all, so the only thing being measured
is the hard half, on a slice.

---

## 6. Where deterministic parsing plainly wins

**[V] The source is the argument.** The map's charting found: zero `\trowd` markup in all 20 files,
every table flattened into tab-delimited paragraphs, the PHB's 161 numbered tables emerging as
"clean tab-separated rows with a header line and a consistent column count", and the corpus not
breaking words mid-line. These are TSR's own digital editions — extraction, not OCR archaeology.

**[I] For that input, a deterministic parser is not merely adequate — it is strictly dominant on
every axis this map cares about:**

| | `split('\t')` | Model |
|---|---|---|
| Accuracy on delimited rows | Exact, by construction | ~96.7% exact-match on linked cells [fed] |
| Reproducibility | Byte-identical, forever | §2: not available on an endpoint |
| Cost per re-run | Zero | §5 |
| Auditability | The rule is 3 lines you can read | The rule is 200B weights |
| Failure mode | Loud — column count mismatch | Silent — a plausible number |
| Correction ergonomics | Fix the rule once, re-run | Fix the prompt, hope, re-verify |

**[V] The strongest empirical case for the LLM route does not transfer to this corpus.** The
Philadelphia Fed pipeline is the best-documented comparable: heterogeneous historical numeric
tables, hybrid architecture (Tesseract + CRAFT for rotation and layout, multimodal LLM for
extraction and header harmonisation under JSON schema enforcement, deterministic post-processing
for name standardisation), validated against a 50,653-cell hand-built gold standard. It reports
critical parsing errors of **0.35%** against a **61.4%** baseline, R² of 98.6%, 98.64% of cells
linked, **96.7% exact-match on linked cells**, MAPE 0.7%, and a **4.64% total error rate**; among
the erroneous cells the median absolute percentage error is 5.52% and the 95th percentile is 25.43%
([FRB Philadelphia WP 25-28][fed]).

**[I] Read that baseline carefully. The 61.4% it beats is AWS Textract on scanned page images.**
corerules has no scan, no OCR stage, no layout inference, and no image. The Fed paper's argument is
"multimodal LLM beats OCR on pictures of tables" — which is true and irrelevant here. What
transfers is the *residual*: even after all that engineering, ~3.3% of the cells it extracted did
not match the gold standard, and the errors' size distribution has a real tail. Under `spec.md` §4,
where nothing is stored and every value is recomputed through the layers, a 3.3% cell error rate in
a lookup table is not a quality issue — it is a corrupted engine. The authors' own caveat lands
exactly on this project: *"applications that turn on an exact value in every cell warrant further
verification."*

**[I] What the field says about hybrid design, stated plainly.** Every credible pipeline in the
literature I found — including the Fed's — is hybrid, and the seam is always in the same place:
**deterministic where the input has exploitable structure, model where meaning must be judged.**
The Fed put OCR/layout on the deterministic side because their input was pixels. corerules' input
is already delimited text, so its deterministic side reaches much further up the stack: it covers
the tables outright.

**[I] Mapping onto ticket 01's three buckets — I confirm the map's partition, with one refinement:**

| Bucket | Owner | Why |
|---|---|---|
| **mechanical** — tab-delimited tables with a header line and consistent column count | **Deterministic parser, exclusively.** No model, not even for verification. | Exact and free; §2 reproducibility for free; ticket 10's invariants apply directly; ticket 07 gets stable source-position IDs here. |
| **regular but ambiguous** — proficiency lines with a parseable grammar; the `Nonweapon` / `Bonus Nonweapon` label split | **Deterministic parser plus a written rule.** Not a model. | The point of this bucket is that *somebody decides once* that "Recommended" is advice and a parenthesised group changes slot cost. A written rule applies that decision identically forever; a model re-decides it on every run, which is the definition of the drift §2 says you cannot fix. The 52+54=106 split is the type case: the naive regex undercounts by 44%, and the fix is a better regex, not a model. |
| **judgement** — prose-stated benefits and hindrances, the ~59 Complete Priest's records, effects that must become predicates | **Model-assisted, human-reviewed.** | No deterministic alternative exists. The map already requires human review here, so the model's error rate is bounded by a reviewer rather than shipped. |

**[I] Two consequences worth carrying forward.** First, the seam is not a compromise between two
techniques — it is a boundary between two *guarantee levels*, and the map already named the
asymmetry ("tables therefore end up with a stronger guarantee than kits"). This ticket confirms
the asymmetry is not an artefact of budget; it is intrinsic. Second, drawing the seam here shrinks
the model's job to roughly the ~100 kit records plus the ~59 priest records plus scattered prose —
hundreds of records, not 1.27M words. **That changes what §5's cost table is even measuring:** the
judgement half is perhaps a few percent of the corpus by volume, so a fully-verified model pass over
*only that half* costs single-digit dollars hosted, or an hour locally. Which makes the local
option, and with it §2's reproducibility and ticket 09's licence question, much easier than the
whole-corpus figures suggest.

---

## What this hands to the blocked tickets

- **[Ticket 05](../issues/05-pack-schema.md)** — the schema has two enforcement tiers whether it
  wants them or not (§1). Recursive structures, numeric ranges, string lengths and array arity
  cannot be decoder-enforced; they belong to the §7.6 validator. If ticket 06's expression language
  is recursive, structured outputs cannot constrain it at all.
- **[Ticket 07](../issues/07-identity-and-id-stability.md)** — reproducible model output is not
  available on a hosted API and only pin-dependent locally (§2). IDs must not be a function of
  model output. The deterministic half can safely carry source-position IDs; the judged half needs
  option 3 or a deterministic anchor.
- **[Ticket 09](../issues/09-extraction-pipeline.md)** — the architecture question (its item 1) is
  answered by §6: hybrid, seam at ticket 01's bucket boundaries, with the middle bucket landing on
  the *parser* side rather than the model side. Its item 2 (third-party API) is not answered here
  by design, but §5b shows the local option costs a night rather than a capability, and §6 shows
  the model's share of the corpus is small enough that the local option is more affordable than the
  whole-corpus arithmetic implies. Its item 3 (how reproducible) has its evidence in §2.
- **[Ticket 10](../issues/10-mechanical-verification.md)** — promoted. §3 shows the mechanical
  checker is not the cheap consolation half of verification; per Tyen et al. it is the *only*
  thing that reliably localises this error class, and localisation is the precondition for the
  model or human being able to repair it.
- **[Ticket 08](../issues/08-which-slice-proves-the-format.md) /
  [13](../issues/13-transcribe-the-proving-slice.md)** — the hand-transcribed proving slice should
  be retained as the permanent gold standard and regression test, per the Fed paper's practice.
- **[Ticket 12](../issues/12-how-much-tool.md)** — one input to it is withdrawn: LLM assistance
  reduces the *judgement* half's manual burden, which is the half that needs correction ergonomics
  forever. The "editor's value drops proportionally" inference does not follow from the premise.

---

## Things I could not establish

1. **The corpus's actual token count.** I was forbidden to open the books, correctly. Every token
   figure in §4 and §5 is derived from the map's word count times published tokenizer ratios.
   Anthropic's own note says the newer tokenizer's ~30% increase "depends on the content and
   workload shape", and tab-delimited tabular text is exactly the shape most likely to deviate.
   **Ticket 09 should measure this once with `count_tokens` on a single chunk before trusting §5.**
2. **Whether the Philadelphia Fed's ~3.3% cell error rate is the right prior for this corpus.**
   Their input is scanned images; ours is clean digital text. The error *mode* differs — theirs
   includes character recognition, ours would not. I found no published study of LLM extraction from
   already-delimited digital text, almost certainly because nobody publishes on a problem a regex
   solves. So the honest position is: I have an upper bound from a harder task and no measurement of
   the easier one, and the direction of the difference is favourable but the magnitude is unknown.
3. **Real throughput on either machine.** No inference runtime is installed on the Fedora box, and
   the M1 Max was not reachable from this session. The Radeon 860M figure is a community benchmark;
   the M1 Max figures are arithmetic from memory bandwidth. Both are marked as such.
4. **The mechanism behind Anthropic's structured outputs.** The docs say "constrained decoding", but
   the implementation is not published, and the documented behaviour of the SDKs stripping
   unsupported constraints and validating them client-side implies not every schema feature is
   decoder-enforced. I could not determine which features are hard guarantees at the token level
   versus post-hoc validation.
5. **Whether the "Let Me Speak Freely" degradation survives on a thinking model.** The paper
   predates adaptive thinking. My argument that unconstrained thinking blocks absorb the effect is
   an inference; I found no study testing it.
6. **Whether any open-weights model in the M1 Max's size class is good enough for the judgement
   bucket.** No benchmark exists for this task and I did not run one — ticket 09 must measure it on
   the proving slice.
7. **Long-term availability and redistribution terms of specific open weights.** Keeping a GGUF file
   mitigates endpoint retirement, but I did not establish licence terms for any candidate model, and
   a licence that forbids retention would silently undo §2d's pin.
8. **Whether the books' own numbered table indices are machine-readable.** If they are, ticket 10's
   count checks are nearly free; if they are prose, they are not. That is ticket 01's measurement and
   I was correctly forbidden from looking.
9. **The cost that actually dominates: human review hours.** Every dollar figure in §5 is a rounding
   error next to reviewer time on the judgement half, and no ticket on this map has estimated it.
   §3's finding — that only non-model references catch the feared error — means that number is
   load-bearing, not incidental.

---

## Sources

Primary vendor documentation (all fetched 2026-08-10):

- [Anthropic — Structured outputs][so] · schema-conformance guarantee, unsupported JSON Schema
  features, grammar compilation and cache behaviour
- [Anthropic — Messages API reference][msgs] · full parameter list (no `seed`); temperature's
  "not fully deterministic" note
- [Anthropic — Model deprecations][dep] · 60-day notice commitment; retirement table; `temperature`
  400 on 4.7+
- [Anthropic — Pricing][price] · per-model rates, Batch 50%, cache multipliers, tokenizer +30% note,
  ~0.75 words/token
- [Anthropic — Context windows][ctx] · 1M default and standard pricing; "context rot"; adaptive
  thinking variability
- [OpenAI Cookbook — Reproducible outputs with the seed parameter][oai] · `seed` /
  `system_fingerprint` "best effort"
- [llama.cpp — GBNF grammars README][gbnf] · local constrained decoding
- [llama.cpp — PR #16016, deterministic CUDA mode][pr] · draft status; maintainer on batch-size
  guarantees

Peer-reviewed and preprint literature:

- [Thinking Machines Lab — Defeating Nondeterminism in LLM Inference (2025-09)][tml] · batch
  invariance; 80 unique completions in 1,000; divergence at token 103
- [Can LLMs Credibly Transform the Creation of Panel Data from Diverse Historical Tables?
  FRB Philadelphia WP 25-28 / arXiv:2505.11599][fed] · hybrid pipeline; 96.7% exact match;
  4.64% total error; gold-standard recommendation
- [Consistency–accuracy correlation in hard-prompted LLMs for entity and relation extraction,
  Genomics & Informatics (2025)][cons] · ρ 0.10–0.30; "confidently and consistently wrong"
- [Huang et al. — Large Language Models Cannot Self-Correct Reasoning Yet, ICLR 2024][selfcorr]
- [Tyen et al. — LLMs cannot find reasoning errors, but can correct them given the error location,
  ACL Findings 2024][tyen] · error localisation is the bottleneck
- [Goel et al. — Great Models Think Alike and this Undermines AI Oversight, ICML 2025][alike] ·
  CAPA; correlated failures rise with capability
- [Tam et al. — Let Me Speak Freely? A Study on the Impact of Format Restrictions (2024)][lmsf]
- [Geng et al. — JSONSchemaBench (2025)][jsb] · engine coverage across 10,000 real schemas
- [Modarressi et al. — NoLiMa: Long-Context Evaluation Beyond Literal Matching, ICML 2025][nolima] ·
  11/13 models below 50% of baseline at 32K
- [Self-Consistency Is Losing Its Edge (2025)][scedge] · diminishing returns as base models improve
- [Knowledge graph construction with enhanced triple extraction (2025)][roundtrip] · round-trip
  reconstruction via embedding similarity
- [llama.cpp discussion #10879 — Vulkan backend performance][vulkan] · indicative Radeon 860M figures

[so]: https://platform.claude.com/docs/en/build-with-claude/structured-outputs
[msgs]: https://platform.claude.com/docs/en/api/messages
[dep]: https://platform.claude.com/docs/en/about-claude/model-deprecations
[price]: https://platform.claude.com/docs/en/about-claude/pricing
[ctx]: https://platform.claude.com/docs/en/build-with-claude/context-windows
[oai]: https://cookbook.openai.com/examples/reproducible_outputs_with_the_seed_parameter
[gbnf]: https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md
[pr]: https://github.com/ggml-org/llama.cpp/pull/16016
[tml]: https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/
[fed]: https://arxiv.org/abs/2505.11599
[cons]: https://link.springer.com/article/10.1186/s44342-025-00063-2
[selfcorr]: https://arxiv.org/abs/2310.01798
[tyen]: https://arxiv.org/abs/2311.08516
[alike]: https://arxiv.org/abs/2502.04313
[lmsf]: https://arxiv.org/abs/2408.02442
[jsb]: https://arxiv.org/abs/2501.10868
[nolima]: https://arxiv.org/abs/2502.05167
[scedge]: https://arxiv.org/abs/2511.00751
[roundtrip]: https://arxiv.org/abs/2508.03438
[vulkan]: https://github.com/ggml-org/llama.cpp/discussions/10879
