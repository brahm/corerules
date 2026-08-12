# How much tool, and where it lives

Type: grilling
Status: resolved
Blocked by: 09

## Question

**The question this map was opened with**, deliberately answered last:

> uma ferramenta de criação de pacotes de conteúdo para o corerules a partir de pdf, rtf, html, txt
> ou qualquer outro formato, incluindo edição manual, pode ser uma aplicação independente ou uma
> funcionalidade do corerules

By the time this ticket is takeable, eleven tickets have measured the thing the tool would serve.
That is the whole reason it is last: charting chose the corpus as the destination precisely so that
**how much tool is justified becomes a measurement rather than a taste**.

## What has to be decided

1. **How much interface, if any**, beyond what [ticket 09](./09-extraction-pipeline.md)'s pipeline
   already needs. The live range: scripts and a text editor · scripts plus a review interface for the
   judgement half · a full authoring application.
2. **Standalone application or a corerules feature.** Note this reopens
   [v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md), which decided **corerules does
   not author packs** and rejected both an embedded editor and the apparently-contained middle of
   "edit existing records only". Reopening is legitimate — Wagner opened this map by explicitly
   putting both options back on the table — but ticket 13's reasoning has to be answered, not
   ignored. Its strongest arguments were that a directory of JSON gets version control for free and
   an embedded editor would fight it, and that under closed kinds an editor means a form for each of
   twenty-odd kinds.
3. **What "manual editing" means concretely**, which was in the original request and has not been
   pinned down anywhere. Editing extracted records before they become a pack; correcting a pack in
   place; authoring a record with no source at all (a house rule, which §5.1 says is expressed by
   what a pack declares and contains). These are three different features and they do not have to get
   the same answer.
4. **Whether the input side generalises**, given the map ruled the PDF-only books out of scope and
   fixed the pipeline as published-but-not-generalised. The original request said "pdf, rtf, html,
   txt or any other format"; the corpus decision narrowed that to RTF **for v1's corpus**, not
   forever.

## What is already settled and not open here

- **Published, not generalised** (the map's Notes). Publishing is free; designing for a stranger is
  the cost. Whatever this ticket decides, it does not add packaging, installers, or an interface for
  someone who does not know what a kit is.
- **corerules v1 ships with a declared usability hole.** The Engine is FOSS, arrives empty, and packs
  cannot circulate. If this ticket's answer changes that, it is a significant improvement to the
  product and should be recorded as one — but it is not this map's obligation to fix it.

## One input withdrawn by ticket 04

[v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md) argued that LLM-assisted extraction
"reduces a graphical editor's value proportionally", and that argument fed the decision this ticket
may reopen. **[Ticket 04](./04-llm-assisted-extraction.md) withdrew it.**

The bulk is less manual because the tables were *already structured*, not because a model reads them.
What LLM assistance actually accelerates is the **judgement** half — which is precisely the half that
needs review and spot correction forever. So the premise is true and the conclusion does not follow
from it. Do not re-import that argument on either side of this decision.

## Answer

### What eleven resolutions left for "tool" to do

| piece | surface | settled by |
|---|---|---|
| Extraction | CLI, three configured paths, no interface | [09](./09-extraction-pipeline.md), [02](./02-where-the-corpus-lives.md) |
| Mechanical checking | a report; a broken invariant fails the run | [10](./10-mechanical-verification.md) |
| Correction | text editor and `git add -p` | [09](./09-extraction-pipeline.md) |
| **Review** | record and source passage side by side, plus attestation carrying a **hash** | [11](./11-human-review-protocol.md) |

Only the last asks for something that does not exist, and it asks for a mechanical reason:
**nobody types a SHA-256 by hand.**

### Decision 1 — review gets a generated static page, not an application

The surviving tool set is four commands and one template: `extract`, `check`, `review` — which
*generates* a page — and `attest`.

**The page is nearly free, by a happy accident of ticket 09.** Making the WebHelp the only parse
target means every record's source passage **is already an HTML file**, and ticket 05's `anchor`
points straight at it. Side-by-side is a template, not an application: record on one side, the CD's
own page on the other, opened in a browser. No framework, no state, no server.

Rejected: **no interface at all** — two editor panes read fine, but the record is JSON and the source
is tag soup, so the reviewer would spend attention decoding markup rather than checking rules, and
attention is exactly what ticket 11 says decays across a review.

Rejected: **an interactive application** — the one thing here that would require designing for a
stranger, which the map's charting ruled out with *publishing is free, generalising is the cost*. A
generated page is publishable without being generalised; an application is not.

### Decision 2 — a separate tool, and the input side does not generalise

**corerules does not author packs**, and nothing in twelve resolutions reopened the case
[v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md) made — the surface shrank at every
step. A Python CLI plus a generated page does not belong inside an Electron/TypeScript application;
putting it there would make it a different thing.

**And the original request's premise did not survive measurement.** It asked for a tool reading
"pdf, rtf, html, txt or any other format". The answer is **one format**: ticket 09 made the WebHelp
HTML the only parse target, and the map ruled the PDF-only books out of scope. Not because generality
is hard, but because the corpus does not need it.

### Decision 3 — a pack declares its provenance mode

**This map closed a door the v1 spec had deliberately left open, and did not notice.**
[Ticket 05](./05-pack-schema.md) made `section` and `anchor` mandatory on every record and had the
manifest name its sources by hash. **A house rule has none of the three** — and §5.1 says plainly:

> A3 subsumes the house-rule escape hatch. **The escape hatch is the pack.**

So the pack **declares its provenance mode** — extracted, or hand-authored — and the per-record
requirement follows from the declaration. This is the project's standing pattern, **declare rather
than infer**: it is A3 applied to provenance. An extracted pack owes an anchor on every record; a
hand-authored one does not, and *says so*, so absence is never ambiguous.

Rejected: **an anchor whose value is "hand-authored"** — an anchor that does not anchor.
[Ticket 10](./10-mechanical-verification.md) has just made *the anchor resolves in the source* an
invariant that fails the run, and it would need a special exception. That is how invariants die.

Rejected: **leaving house rules out of this map's scope.** The schema is authored here and is
canonical, so excluding them from the map does not exclude them from the schema — it only guarantees
the schema forbids them by omission.

**Recorded as the map's sixth correction owed to the v1 spec**, and the second against something the
spec had settled rather than assumed. Requirement passed to
[ticket 14](./14-record-shapes-for-the-slice.md).

### The prediction, checked

It was written into this ticket before any measurement, so that it could be wrong:

> *the answer will be less tool than the original request imagined, and the interface that survives
> will be for reviewing rather than for authoring.*

**It held, and it undershot.** The surviving interface is for reviewing — and it is not even a review
application, but a generated page. What made that possible was not restraint; it was three
measurements, none of which was aimed at this ticket: the HTML marks up its own field labels
(ticket 09), the source is already a browsable page (ticket 09), and the anchor points into it
(ticket 05).

## The prediction, recorded now so it can be checked later

Written at charting, before any measurement: **the answer will be less tool than the original
request imagined, and the interface that survives will be for reviewing rather than for authoring.**
Ticket 01's mechanical/judgement partition is why — the mechanical half needs no interface at all,
and the judgement half needs to be *read*, not typed.

If the measurements contradict this, that is a genuine finding and the prediction was wrong. It is
written down so that it can be, rather than being quietly retrofitted to whatever the tickets happen
to produce.
