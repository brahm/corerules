# How much tool, and where it lives

Type: grilling
Status: open
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

## The prediction, recorded now so it can be checked later

Written at charting, before any measurement: **the answer will be less tool than the original
request imagined, and the interface that survives will be for reviewing rather than for authoring.**
Ticket 01's mechanical/judgement partition is why — the mechanical half needs no interface at all,
and the judgement half needs to be *read*, not typed.

If the measurements contradict this, that is a genuine finding and the prediction was wrong. It is
written down so that it can be, rather than being quietly retrofitted to whatever the tickets happen
to produce.
