# Map: the corerules Engine, v1

## Destination

**A corerules v1 that runs** — the Electron desktop application specified by
[`../v1-spec/spec.md`](../v1-spec/spec.md), creating and persisting AD&D 2nd Edition characters from
Content Packs the user supplies.

The map is done when a character can be **created, advanced and persisted** against the transcribed
corpus, and the application ships to Wagner's own machine.

This effort is the sequel to [the corpus map](../corpus-v1/map.md), which produced the pack this
Engine loads and closed with the sentence this map exists to falsify: **nothing has ever loaded this
pack.**

## Notes

**Domain.** AD&D 2nd Edition character generation. The lineage is three maps deep: the
[v1 spec map](../v1-spec/map.md) specified an Engine that ships no content; the
[corpus map](../corpus-v1/map.md) built the content that Engine would load; this one builds the
Engine. Each closed where the next had to start.

**Language.** Artifacts in English. Conversation with Wagner in Portuguese.

### What this map inherits, and what is wrong with it

The unusual thing about this map's starting position is that **its specification is known to be
wrong in 42 specific places, and the list is written down.** The corpus map's
[corrections owed to the v1 spec](../corpus-v1/map.md#corrections-owed-to-the-v1-spec) is not a
backlog of bugs; it is 42 conclusions that did not survive contact with the books, collected so that
whoever implements has one thing to read. **Nine were already fixed in the schema itself**; 33 are
owed to `spec.md`.

That is a gift and a hazard. The gift is that no implementation session has to rediscover them. The
hazard is that **`spec.md` still says the superseded thing**, and a session that reads it without the
corrections list will build the wrong Engine. Ticket 01 exists for exactly this.

| inherited | state |
|---|---|
| [`spec.md`](../v1-spec/spec.md) — 13 sections, product surface and technical shape | **written before any content existed**; wrong in 42 places |
| `pack-0.1.schema.json` — 20 kinds, six operations, layer model | settled, changed 22 times by transcription, **nothing open pushing on it** |
| the pack — 1,233 records, 1,910 effects, 99.9% of references resolving | **loaded once**, by [ticket 04](issues/04-first-light.md); eight ids collide and the sheet has no combat numbers |
| `validate.py`, `verdict.py` | run both before believing anything written in either prior map |

### Settled during charting — do not re-litigate

These come from the v1 spec map and are **not reopened by this map**. They were charting decisions
there and remain charting decisions here.

- **FOSS engine, content supplied by the user.** corerules never bundles licensed 2e content. The
  boundary is that **packs do not circulate**; a character is the user's own work.
- **Single-user.** No accounts, no authentication, no permissions.
- **Desktop application.** Electron, TypeScript + React, cross-platform. Electron over Tauri so there
  is one rendering engine rather than three.
- **Unsigned builds**, with the per-OS posture the v1 spec map measured: Linux fine, Windows mostly,
  macOS only if *"open Terminal and run `xattr -dr com.apple.quarantine`"* counts as a way past.
- **Release via GitHub Actions**, with literal step-by-step checklists rather than pointers to
  documentation — Wagner has never used Actions.
- **Native to AD&D 2e, not a generic RPG engine.** Closed set of object kinds, open enumerations: the
  Engine owns the shape, the pack owns the contents.
- **The corpus is never sent to a third-party API**, and the private corpus repository is never
  mirrored into this one.

### Permanent constraints

Excluded from v1, but no decision here may foreclose them: **sync between clients** (so: stable
global identifiers, a diffable persistence format) and **v2/v3** — campaign settings, psionics,
Player's Option (so: enumerations stay open, the character records which packs it was built against).

### What the corpus map proved that changes how to build this

Four results are load-bearing for implementation and are easy to miss in 149 findings:

1. **Six operations suffice.** All six used across 1,910 effects, none ever found missing. The
   evaluator can be written against a closed set.
2. **§4.1's three Attachable arms are one shape** — held for 238 records without a per-arm exception.
   One `attachable` code path, not three.
3. **The layer model absorbed everything that looked like it needed a new feature** — offsets,
   cancelling exceptions, conjunction-of-disjunction. Order-independence held everywhere it was
   tested.
4. **21% of effects carry an `UNMODELLED` marker**, and the spec has no concept of one. See ticket 02;
   this is the biggest gap between what was specified and what was built.

## Decisions so far

<!-- one line per closed ticket: enough to judge relevance, then follow the link for detail -->

- **[04 — First light](issues/04-first-light.md)** — **resolved.** A program loads the pack and
  computes a character. The prediction held: **no gap was in the six operations, every gap was a
  join.** Eight ids are defined twice because the namespace carries no kind and `validate.py` built a
  set; two `set` layers contradict *inside one record* because the Complete Book of Dwarves prints
  Navigation twice; **the sheet has no combat numbers at all**, because 15 of 19 class records are
  untranscribed and the class is what supplies them; the pack holds deltas and nothing holds the base;
  a table read returns `"+4"` where the contract says an integer; and two mutually exclusive marked
  bonuses summed into a wrong number on the first character anyone builds.

**Every ticket in this map is resolved** — the four charted at the start and the one they
produced. The next work is building, not deciding.

Running the grillings to decisions has been worth more than the charting predicted, and mostly by
**subtraction**: 02 and 03 were both smaller than they looked and both sitting on top of something
larger; 05 — the something larger — needed **no new operation at all**; and 01 closed eighteen of its
nineteen corrections by deciding they were owed to **the wrong document**. Six corrections came out of
these four tickets, every one found by *running a program* rather than by reading, which is the
evidence 01 used.

_Resolved:_

- **[01 — Which spec does the Engine implement?](issues/01-which-spec-does-the-engine-implement.md)**
  **`spec.md` §§3, 4 and 7 are non-normative; `pack-0.1.schema.json` is the format.** The list reached
  **50**, of which nineteen touch the spec — and **eighteen of the nineteen are about the format**,
  which already has a second description: **22 commits to the schema against 1 to the spec.** The
  document that is checked was corrected twenty-two times against the books; the one that is not was
  written once. So the sections stay as the reading guide and stop being the definition, and eighteen
  corrections stop being owed. Day one reads **the spec for the product, the schema for the format,
  and this map's decisions for what the Engine does with what it loads.** §13 was rewritten — nine
  entries to four, each with a test — and §5.4 added, because ticket 02's decision about 381 marked
  effects has no other home. **Corrections 6 and 48 are both applied, so nothing is owed to `spec.md`.**
  Correction 6 turned out to be **half done for years**: `provenanceMode` was already required in the
  manifest and the slice already declared `extracted`, and **nothing enforced a consequence** — a
  declaration nothing acts on is not A3, it is a field. The checker now enforces the mode, and
  `fixtures/house-rules/` is **the pack §5.1 promised, validating**.
- **[02 — What the Engine does with an UNMODELLED effect](issues/02-what-the-engine-does-with-an-unmodelled-effect.md)**
  **The operation decides, not the category — and a marked effect never reaches the total.** On a
  structural operation (222 of 381) the thing is applied and the marker rides on that entry; on a
  numeric one (159) the value is withheld from the sum and printed as a named situational line. The
  declared category is prose — **32 labels, twelve used once, `CONDITION` and `CONDITIONS` both
  present** — and the Engine never reads it, because `grant`/`forbid`/`require`/`except` against
  `adjust`/`set` is §5.2's own line expressed in a field the schema already enforces. Exposure was
  measured over all 36,126 combinations: **median three markers a sheet, worst fourteen** — few
  enough to print, too many to prompt about. Sent back one correction (45).
- **[03 — Precedence when two Attachables contradict](issues/03-precedence-when-two-attachables-contradict.md)**
  **Precedence is never inferred; it is declared by one record about another, or the value is
  refused.** There was nothing to rank: **every restricting operation in the pack lives in a kit** —
  all 96 `forbid`s, all 13 `except`s, 120 of 121 `require`s — and a Deity, with 690 grants, cannot say
  no about anything. 2,969 kit pairs can meet on a multi-class character and none contradicts;
  **14,910 sheets produced 0 contested values and 2,130 collisions that a declared refinement
  resolved.** A fixed arm order is **refuted**: of three precedence statements in 4,500 pages, the CBD
  puts the kit over the mythos and the PHB puts the mythos over the class. The two declarations needed
  already exist — `target` and `except`, the latter with seven `limitation` records, every one pierced.
  **This closes §13's known unknown #1** — the residue corpus ticket 13 left standing when §4.1's
  one-shape claim held across 238 records and three arms.
  Sent back corrections 46 and 47, and opened ticket 05.
- **[05 — An operand that lives in another layer](issues/05-an-operand-that-lives-in-another-layer.md)**
  **A bound is a record with `members`, and no seventh operation is needed.** The sweep found **142**
  bounded-set assertions in four encodings, not the 62 the ticket was opened with — and they are two
  concepts: **125** bound things the pack holds, **17** bound categories it does not contain at all
  (totem animals, terrains, undead types), which is correction 23's boundary arriving from a new
  direction. The mechanism was already in the pack: **20 CFH weapon groups carry `members`**, 2 to 26
  ids each. `limitation` gains the same field, `forbid` applies a bound, `except` lifts one, and
  `permitted = (∩ bounds) \ (∪ forbids)` **commutes**, so §4.3's guarantee is inherited. Prototyped:
  an Agriculture priest goes from 117 weapons to 5, a second bound to 1, and an `except` back to 117.
  **72 predicate-shaped bounds are explicitly NOT solved** — the pack records no material. Sent back
  corrections 48, 49 and 50, and **48 is applied**: the schema has the field, the thief's twelve
  weapons and the wizard's five are in the pack, and a thief now computes **12 permitted of 117**
  while the Assassin computes 117 with the lifted rule named. No pack effect was needed —
  **`imposedBy` was the imposition all along.** Two limitations declined members on purpose, which is
  correction 50's boundary met on the second record tried. **49 is applied too** — and it was
  **sixteen** empty groups rather than six, with the membership sitting in the source as **three
  spaces of indentation**. A permit-list reading *"swords (all)"* went from **0 of 117 to 8**. Sent
  back corrections 52 and 53, and **52 is applied** — in the extractor as well as the data.
  `extract_phb.py` was throwing the indentation away in its first line of parsing while
  `extract_cfh_weapons.py` had been reading it all along; it now reproduces the pack exactly, so
  49 and 52 survive a re-extraction instead of being undone by one. **53 is applied**: eleven of
  Table 44's weapons carried no damage, because in 2e the launcher has the speed factor and the
  ammunition has the damage — **112 of 117 weapons now state one.** And **54**, which came out of 53
  and is applied: `isGroup` spelled **three** relations in two directions, so `groupKind` became the
  whole statement for weapons and `isGroup` is now refused there outright. **And 46, the one this
  ticket existed for**: 60 limitation records, 58 marked grants replaced by a `forbid` and an
  `except`, **markers 381 → 323**, and a priest of Agriculture now computes the book's six weapons
  exactly. **62 effects of cross-layer parameterisation became 3** — the CBD kits, which need only
  the ability to name *"the limitation my Deity imposes"*. **And 50**: of its 34 remaining predicate
  bounds, **16 are answerable** — melee-versus-missile lives in the range tables, now transcribed,
  and the split is three-way (launcher 11, hurled 21, melee 87) — while **material is measurably
  absent from every table in twelve books**, which makes 8 of them the edge of the corpus rather
  than a gap in the format. **And 51**: `weapon` against `weaponProficiency` is a real distinction
  the books mark with a verb, the pack had **five of twelve wrong**, and the Spy's `except` turned
  out to lift nothing — it was restating the general non-proficiency rule. `limitation.bounds` makes
  it checkable, and the checker found the Spy on its first run. **And 55**, which the Engine's own
  work produced by tripping over it twice: the pack now has a canonical serialisation, a
  `normalise.py` that applies it and a checker that reports drift — and **every extractor was
  already writing it.** The drift was in the hand edits, not the tools. **And 56**, the last one owed: a record carries
  the other names its books print, and the checker refuses an alias that resolves to two things —
  which it did on the first run, catching `Polearms` claimed by both Table 44's heading and the
  Complete Fighter's purchasable group.

**Of the corrections list's 56 entries, 21 are now applied and two remain as work** — 45, the 37
stale disjunction markers, and 47, the roll-under against the percentage. The other 33 are history,
method, or corrections to the `spec.md` sections ticket 01 made non-normative.

## Not yet specified

<!-- in-scope fog you can't ticket yet; graduates as the frontier advances -->

- **Whether the schema is the Engine's internal model or only its wire format.** A pack is JSON shaped
  for transcription and review; a character sheet is computed by walking layers. Whether the Engine
  keeps records in the pack's shape or normalises on load cannot be asked sharply until something has
  loaded a pack once and the cost of either is visible.
- **What the review page of [corpus ticket 12](../corpus-v1/issues/12-how-much-tool.md) becomes.** That
  ticket built tooling for a human checking a transcription; the Engine has to show provenance to a
  *player* who is not checking anything. Whether those are the same surface is a product question that
  needs a running app to look at.
- **Whether psionics can be expressed at all.** The v1 spec's known unknown 3, raised and overruled:
  deferring psionics entirely risks discovering in v2 that the format cannot hold it, which is how
  PCGen failed. This map does not implement psionics, but the first Engine that computes anything is
  the first thing that could cheaply *test* the format against one psionic power.

## Out of scope

- **Transcribing more of the corpus.** [The corpus map](../corpus-v1/map.md) closed with a plan and an
  empty mechanical list; adding books is that map's work resumed, not this one's. This Engine must run
  against what exists.
- **v2 and v3** — campaign settings, psionics, Player's Option. Inherited from the v1 spec map's
  roadmap and unchanged.
- **Rewriting the spec.** Ticket 01 decides *whether and when* `spec.md` is updated; it does not turn
  this map into a documentation effort.
- **A generalised authoring product for third parties.** Preserved, not built — as in both prior maps.
- **Anything the v1 spec put out of scope** in its §12, unchanged: campaign management, encounter
  running, monsters, the DM's side of the table.
