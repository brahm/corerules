# corerules

A desktop tool for creating and managing **AD&D 2nd Edition** characters — by the book, with the
rules actually enforced.

> **Status: not built yet.** There is no application to download. What exists is a finished
> [v1 specification](.scratch/v1-spec/spec.md), a [pack schema](.scratch/corpus-v1/schema/pack-0.1.schema.json),
> and the design work behind both. If you came looking for a character generator, come back later —
> or read on, because the interesting part is *why* it does not exist yet.

---

## The idea

**corerules ships no game content.** None. It supplies the rules *logic* and the interface; you
supply the data, from books you own, as **Content Packs**.

That sounds like a limitation. It is the only configuration under which a public AD&D 2e tool can
exist at all. 2e has no SRD and no open licence — the tables, kit descriptions and spell text are
Wizards of the Coast IP. PCGen, a twenty-year-old data-driven engine with a volunteer data team and a
formal publisher-liaison process, still has no AD&D 2e dataset, because it cannot get permission.

So the split is: **the engine knows what a kit *is*; your pack says what the Bladesinger *does*.**

Concretely, the engine validates *hard*. Illegal states are unrepresentable at the point of choice,
not flagged afterwards — you cannot pick a kit your race cannot take, and when it refuses it tells
you which rule refused and which book that rule came from.

## The catch, stated plainly

**You will have to transcribe your own books, and packs cannot be shared.**

A Content Pack is derived WotC content. Yours is yours; it does not circulate. That is not
timidity — it is the condition that lets the engine be public at all. So a fresh install of corerules
opens empty, and the first thing anyone must do is produce a pack.

This is a real usability hole and the project says so rather than letting you discover it. Reducing
it is what the corpus effort below is about.

## What is in this repository

| | |
|---|---|
| [`.scratch/v1-spec/spec.md`](.scratch/v1-spec/spec.md) | **The v1 specification.** Thirteen sections, concrete enough to build from. Start here. |
| [`.scratch/v1-spec/`](.scratch/v1-spec/) | The fourteen decision tickets behind it — *why*, and what was rejected |
| [`.scratch/corpus-v1/`](.scratch/corpus-v1/) | The corpus effort: how books become packs |
| [`.scratch/corpus-v1/schema/`](.scratch/corpus-v1/schema/) | `pack-0.1.schema.json` — what a valid pack is |
| [`CONTEXT.md`](CONTEXT.md) | The project's vocabulary. Words like *Attachable* and *Layer* mean something specific here |

**The spec states what is decided; the tickets state why.** If a decision looks wrong, the ticket
that made it will tell you what was weighed — including the arguments that lost. Several tickets
record decisions that were later contradicted by measurement, and those corrections are collected in
the corpus map rather than quietly edited away.

## Design, in five lines

- **Closed kinds, open enumerations.** The engine owns the *shape* of a class, a kit, a saving-throw
  matrix; packs own the contents. Affordable because 2e is a dead edition — its concept set is closed
  by history.
- **Nothing is overwritten.** A character's view of any value is a stack of layers, and the
  operations commute, so provenance survives computation and a refusal can name its cause.
- **Kit, Deity and Subrace are one shape used three times** — a target, a prerequisite, an ordered
  list of effects.
- **A character is a sequence of level events**, not a snapshot. Hit points are recorded randomness:
  neither a choice nor a derivation.
- **Packs declare what they cover**, so the engine can tell "this rule does not restrict" from
  "nobody has transcribed this yet" — and say which.

## Roadmap

| | |
|---|---|
| **v1** | Core books (PHB, DMG) and the PHBR *Complete* series — class and racial kits |
| **v2** | Campaign settings, psionics, further books |
| **v3** | Player's Option |

Two things are permanently out of scope: **automatic character construction** (rolling dice is a
rule and comes from the book; choosing a kit for you is taste, and no source book contains it), and
**bundled content**, for the licence reason above.

## Platforms

Linux, Windows and macOS via Electron. Builds will be **unsigned** — this is not a commercial
product — and that means something different on each platform, from `chmod +x` on Linux to a
mandatory Terminal command on macOS. Some Windows 11 machines with Smart App Control will not be able
to run it at all. The details are specified and will be repeated at release.

## If you want to help

The most useful contributions right now are not code:

- **Read the spec and argue with it.** It has nine
  [known unknowns](.scratch/v1-spec/spec.md#13-known-unknowns) recorded on purpose. One of them —
  whether six operations can express every kit — has already fired, and the fix is in the schema.
- **Tell us where the model breaks.** The kit mechanism has no prior art anywhere; §4.1 is invention.
  If you know 2e well enough to find a kit it cannot express, that is worth more than a pull request.
- **Psionics is the format's hardest untested load.** v1 defers it deliberately, and that is a
  recorded risk: discovering in v2 that the format cannot express it is how PCGen failed.

Please do not open issues or pull requests containing **book text, tables, or transcribed pack
data.** This repository is public and that content is not ours to publish.

## Licence

Code is **GPL-3.0** ([`LICENSE`](LICENSE)).

Content packs are not covered by it and are not distributed here. AD&D and Advanced Dungeons &
Dragons are trademarks of Wizards of the Coast. This project is unaffiliated with, and unendorsed by,
Wizards of the Coast.
