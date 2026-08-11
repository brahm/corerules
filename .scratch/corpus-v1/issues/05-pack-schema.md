# The pack schema, version 0.1

Type: grilling
Status: resolved
Blocked by: 01, 04

## Question

`spec.md` §7 describes the Content Pack format **in prose** — directory of JSON, manifest declares
contents, book and page citation on every record, pack-scoped opaque IDs, expressions as strings,
Foundry-style three-way compatibility range. It is precise and it is not machine-readable. **Nothing
anywhere says what a valid kit is.**

This ticket writes it. The map's charting settled that the schema is authored **here** and is
**canonical** — the Engine consumes it — because whoever transcribes first learns most about the
format, and the corpus is the long pole. It is born `0.x`.

## What has to be decided

**1. What the schema is written in.**

JSON Schema, or TypeScript types as the source of truth with a validator derived from them (zod,
valibot, typebox), or a schema language with codegen in both directions. The Engine is TypeScript +
React ([the v1 map](../../v1-spec/map.md)), which pulls one way; the pipeline may not be TypeScript
at all, which pulls the other. Whatever is chosen has to serve **two consumers in different
languages** without a second source of truth — `spec.md` §7.2's warning about PCGen's three live
parsers is about evaluators, but the same disease applies to schemas.

**2. The record shape of every kind in §3.1.** The bulk of the work, and
[ticket 01](./01-what-the-source-yields.md) is what makes it possible: it says what each kind's
source actually yields, which is the difference between designing a schema and guessing one.

**3. How the three-way split of §3 lands in files.** Pack kinds get identity; value types
(dice expression, prerequisite predicate, effect, slot budget, money, physical properties) do not and
are structure inside their owner. The schema has to enforce that, or a value type acquires an `id`
by accident and §3's criterion quietly stops being true.

**4. The manifest.** `id`, `version`, `compatibility: {minimum, verified, maximum}`, dependencies with
their own ranges, **plus the A3 rule-set declarations** — which §5.1 says scope to the subjects the
pack introduces and cannot be derived from contents. And the file list: §7.1 requires declaration
over discovery, with a file present but unlisted **reported**.

**5. Book and page citation — and §7.1's requirement cannot be met.**
[Ticket 01](./01-what-the-source-yields.md) measured it: **page numbers exist in neither rendition.**
Zero `\page`, bookmarks, footnotes or hyperlinks in the RTF; zero page text and zero `<META>` in the
HTML. §7.1 makes book-and-page citation the answer to JSON's lack of comments and requires it on
every record — **so the spec requires a field the corpus cannot supply.**

This ticket has to replace it, and the substitution is not cosmetic: the point of the field is that
provenance be *validatable*. Ticket 01's suggestion is **book plus the record's own source anchor**
(HTML filename, or RTF line offset), which is reproducible and machine-checkable where a page number
would have been neither — but an anchor is only stable while the parser's segmentation is, which ties
it to [ticket 07](./07-identity-and-id-stability.md). Decide the shape here, and decide it early:
it appears on every record in the corpus and changing it later is the most expensive migration
available.

**6. Which kind the Complete Priest's records become.** Graduated from the map's fog by ticket 01,
which characterised them: **60 records, ten fields**, sharing nothing with a kit. §3.1 lists both
Deity and Class; §4.1 makes Deity an Attachable. A third reading — a priest *specialty class* — is
what the fields actually describe. This is now a modelling decision, and the source question behind
it is closed.

**7. Where the schema physically lives**, given the pipeline is published here and the Engine does
not exist yet. It must be findable by a repository that has not been created.

## What this ticket must not do

**Do not re-open §3's kind list or §4's six operations.** Those are settled. If ticket 01 finds
something the six operations cannot express, that is **v1 spec known unknown #4 firing** — record it
as a finding against the v1 spec, do not silently add a seventh operation here.

## Answer

**This ticket writes the spine, not the whole schema.** The per-kind record shapes are deferred to
[ticket 14](./14-record-shapes-for-the-slice.md), which is blocked by the slice choice.

The ticket's own warning decided it: *designing for the books already read*. Twenty-seven record
shapes designed before a single record is transcribed is that error at maximum exposure — and
[ticket 13](./13-transcribe-the-proving-slice.md) exists precisely to report what the books force a
change to, saying a "everything worked" result should be disbelieved. The shock absorber is already
mandated by §7.3: the format carries its own version and a converter path. Meanwhile **the two
tickets blocked on this one — [07](./07-identity-and-id-stability.md) and
[10](./10-mechanical-verification.md) — need only the spine.**

Correction owed to the map: the advice that ticket 05 should precede
[ticket 08](./08-which-slice-proves-the-format.md) holds only in the narrow sense that 08 wants the
spine. Running 08 first would have lost nothing.

### Decision 1 — JSON Schema is canonical; TypeScript is generated

Rejected: TypeScript-first with zod/valibot/typebox and JSON Schema generated.

The discriminator is **when** the schema is written. It is authored *here*, before the Engine exists.
TypeScript-first would make the schema's home a TypeScript project — the one language this authoring
effort has no reason to be in — to serve a pipeline that may be Python and a consumer that has not
been written.

Three supporting arguments point the same way. §7.3 says the schema is **published**, which implies a
language-neutral artifact. Generation JSON Schema → TypeScript is mature and **one-directional**,
which eliminates drift by construction. And [ticket 04](./04-llm-assisted-extraction.md) identified a
**third consumer nobody had named** — a constrained decoder, if ticket 09 uses structured output —
which reads JSON Schema natively.

**Recorded so it is not rediscovered:** the schema will never be the whole check. Ticket 04
established that recursive schemas are unsupported by structured outputs and that numeric ranges and
string lengths are not decoder-enforceable. **Two enforcement tiers exist regardless of this
choice** — what the schema declares, and what the §7.6 validator checks.

### Decision 2 — provenance is a section path *and* a source anchor

§7.1 required book **and page**, and [ticket 01](./01-what-the-source-yields.md) established that
page numbers exist in neither rendition. The replacement is not one field but two, because the page
number was quietly doing two jobs:

- **`section` — the heading chain** (*Complete Fighter's Handbook → Warrior Kits → Myrmidon*). For a
  human holding the printed book, findable through its own table of contents. Present in both
  renditions; the HTML nearly hands it over, since `<TITLE>` already reads `Myrmidon (Comp. Fighter's
  Handbook)`.
- **`anchor` — rendition plus file or line offset.** For the machine: relocating the record on
  re-extraction, and diffing.

**The anchor is deliberately the same artifact ticket 07 will weigh as a source-position identity.**
Defined once here, used there — rather than both tickets inventing it separately.

**Rejected: a hand-filled page field.** Wagner owns the printed books, so page numbers are
*obtainable* — by hand, plausibly for ~146 kits and 67 tables, implausibly for ~500 spells. A
provenance field filled for part of the corpus is exactly the **silently incomplete** state this
project rejects everywhere: it is why A3 exists, why YAML was refused, and why a malformed pack does
not load. A field that only sometimes carries provenance is worse than none, because it looks
complete. If page is ever wanted, the coherent form is **optional and declared**, on the A3 pattern,
so it cannot lie.

### Decision 3 — the Complete Priest's records are **Deity**, and Deity enters fat

Rejected: Class, and Kit.

**Kit self-destructs on §6.4.** A kit is bound at creation, never rebound, and may be **abandoned** —
losing benefits and penalties and leaving proficiencies as debt. Applied here that reads *abandon
being a priest of war and remain a priest*, which is not an operation the game has. When a kind's
central operation is meaningless on a record, the record is not that kind.

**Class fails on a measurement:** ticket 01 found no experience progression in `priestbk`. These
records **inherit** the Priest's. Modelling them as Class would invent a class that delegates almost
everything to another one.

**Deity fits §4.1 without forcing anything** — its table already gives Deity the target *a priest
class entry*, referenced by other records, one per target, which is exactly this relationship. And
the fields are portfolio, not class mechanics: symbols, duties, spheres of influence.

**The correction this forces**, recorded rather than buried: [v1 ticket 11](../../v1-spec/issues/11-engine-object-kinds.md)
decided *"Deity enters thin"* and justified it with its own test that a kind added in v2 is a v1
mistake. Sixty records of ten fields are not thin. **Ticket 11 was right that Deity exists and wrong
about its size.** §4.1's shape survives — target, predicate, ordered effects — but a Deity's effect
list is long, and **Deity will exercise §4.3's six operations harder than any kit will**. That is
direct input to [ticket 08](./08-which-slice-proves-the-format.md).

### Decision 4 — the manifest records the source bytes the pack came from

Beyond §7.3's Foundry-style `id` / `version` / three-way `compatibility` / dependencies, §5.1's A3
declarations and §7.1's declared file list, the manifest carries a **source provenance block**: which
source files, **by SHA-256**, this pack was derived from, and from which rendition.

Decision 2 forces it. An anchor points *into a file*, and without knowing which file the anchor means
nothing — and file identity is the hash, because [ticket 02](./02-where-the-corpus-lives.md)
established that **"the DMG" is ambiguous** with two byte-different variants in circulation. A pack
claiming to derive from "the DMG" inherits that ambiguity whole; one naming a hash inherits none.

It also opens drift detection in a direction nothing else covers: if the source is ever replaced — a
better rip, or the errata'd edition this corpus lacks — **the pack knows it is stale.**

### Decision 5 — one file per kind

Rejected: one file per record, and one file per book section.

One file per record makes git's diff perfect and everything else worse: thousands of entries in the
manifest, and record-derived filenames colliding on the **case-insensitive exFAT** mirror ticket 02
put on the card. Mirroring book sections binds the pack to the parser's segmentation, which decision
2 already identified as the unstable part. Per kind is stable, is the unit the schema validates, and
keeps the manifest legible.

### Decision 6 — the schema lives in this public repository

The only repository that exists, and §7.3 says published. How the Engine later consumes it — copy,
package, whatever — is the Engine's problem, not a constraint on where the source of truth lives.

## The thing most likely to go wrong

**Designing for the books already read.** Ticket 01 covers 13 books; the schema will outlive them and
must survive v2's campaign settings and psionics, which the v1 map deliberately excluded but forbade
foreclosing. The relevant discipline is already written down — `spec.md` §2: **closed kinds, open
enumerations.** Every enumeration in this schema stays open except rule-set names.
