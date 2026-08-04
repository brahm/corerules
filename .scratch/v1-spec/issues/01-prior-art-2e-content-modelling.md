# Prior art: how existing tools model AD&D 2e content

Type: research
Status: resolved
Blocked by: —
Findings: ../research/01-prior-art-2e-content-modelling.md

## Question

How do existing character tools model AD&D 2e-era rules content, and what should corerules
borrow or deliberately avoid?

PCGen is the closest known prior art: it ships a free engine and distributes licensed data
separately in its own LST format — the exact architecture corerules has committed to. Find out
how it, and any other FOSS tools targeting 2e or its retroclones, express the awkward shapes:

- per-class experience tables and level progressions
- THAC0 and saving-throw matrices
- race-based level limits and class restrictions
- kits that *modify* their parent class rather than standing alone
- weapon and non-weapon proficiency groups
- spell lists split by school (wizard) and sphere (priest)

Where do those formats break down, and what do their maintainers regret?

## Answer

Full findings, with citations: [`../research/01-prior-art-2e-content-modelling.md`](../research/01-prior-art-2e-content-modelling.md)
— researched against the PCGen source tree (commit `ef03350b`, v6.09.08.RC1) and its shipped data
and format documentation, the Roll20 `ADnD_2E_Revised` sheet, the Foundry OSE and Basic Fantasy
systems, Foundry's manifest documentation, and the PCGen mailing-list archive.

**Headline: the architecture is proven; the content model is unprecedented.** PCGen, Foundry OSE
and Basic Fantasy all ship an engine with zero game content, so corerules' central commitment is
validated by three live projects. But **no project anywhere models AD&D 2e's rules as loadable
data.** PCGen has never had a 2e dataset — a maintainer said so on the list in 2017, and its own
OSRIC game mode is a near-copy of the 3.5e mode with iterative attacks and feats, so it is not
prior art for THAC0, save matrices, level limits, kits, proficiency slots or spheres. The two
live Foundry OSR systems deliberately compute nothing: saves and THAC0 are integers the user
types. The Roll20 2e sheet — the largest open 2e implementation in existence — hardcodes
everything in JavaScript and **has no kits at all**.

Two consequences the map must absorb:

1. **corerules is designing into open ground.** The value of this research is in the failure
   modes, which are documented in unusual candour inside PCGen's own shipping source.
2. **"Engine computes, user supplies the tables" has no shipping precedent.** Every successful
   engine-without-content project in this genre chose to be a *recorder*. That is a direct input
   to [ticket 04](./04-validate-or-record.md), which should know it is choosing an unproven
   design rather than picking between two proven ones.

### Borrow

- **A Foundry-style pack manifest**: `id`, `title`, `version`, `authors`, and a three-way
  `compatibility` object (`minimum` / `verified` / `maximum`), with dependencies carrying their
  own nested compatibility. PCGen has **no version field on a pack at all** — its dependency tag
  matches a name with no version constraint.
- **Licence and provenance metadata inside the pack, surfaced at load**, and **book + page
  citation on every record**. A corerules pack is by definition a transcription of a book the
  user owns; the citation is how the user checks their own transcription.
- **Named, typed lookup tables as a first-class construct** — PCGen's `STARTTABLE` carries a
  header row *and a column-type row*, so a loader can validate a pack before the user hits an
  error mid-character. That is the right shape for save matrices, per-class XP tables and the
  race×class level-limit grid.
- **Both table form and formula form for progressions.** 2e needs both: THAC0 is closed-form per
  class group (`warrior: 21-l`, `wizard: 21-ceil(l/3)`, …), saving throws are irregular and must
  be literal arrays. Pad index 0 and clamp the level before indexing.
- **Class *group* as a real modelling entity** (warrior / wizard / priest / rogue / psionicist).
  Saves, THAC0 and proficiency budgets are all keyed on the group, not the class.
- **Data-defined typed fields rather than a closed schema.** PCGen retrofitted `FACTDEF` because
  a closed token set did not survive contact with new books. The PHBR series will apply exactly
  that pressure. Pair it with typed default values — half-authored packs are corerules' normal
  state, because a human is typing books in by hand.
- **Declarative predicates for prerequisites** (PCGen's `PRExxx:` family: composable, negatable,
  n-of-m, attachable inline). A 2e kit is a prerequisite predicate plus a bundle of effects; this
  is the right shape for the predicate half.
- **The character records which packs it was built against.** The Roll20 sheet's 21 book toggles
  show that "which books are in play" is a *per-character* setting in this genre. That also makes
  the character file self-describing for the sync-later constraint.
- **Character files store choices, not derived values.** A PCGen `.pcg` has no AC, no saves, no
  attack values — everything derived is recomputed from the loaded datasets.
- **CI that loads every pack and rebuilds golden characters.** corerules ships no 2e data, but it
  can ship a synthetic pack exercising every awkward shape and test the engine against it.

### Avoid

- **Name-as-identity.** PCGen identifies objects by display name in one global namespace and
  resolves collisions by a load-priority integer. Saved characters break whenever a data author
  renames anything — the whole `migration.lst` key-rewriting mechanism exists only because of
  this, and it covers just four object types. **This is not hypothetical in our corpus**: the
  Roll20 non-weapon proficiency table has 217 distinct names but 224 entries, because `Set
  Snares`, `Sign Language` and `Crowd Working` are each defined differently by different books.
  IDs must be pack-scoped, opaque and never reused; display names are pure presentation; and
  cross-pack collision must resolve by an explicit user-visible rule, never by load order.
- **Any second, fallback expression evaluator — ever.** PCGen's best-documented failure: three
  parsers still live, the fallback method is literally named `processBrokenParser`, and the same
  characters in the same file evaluate to 15 or 12 depending on which parser succeeded. One
  evaluator, specified before any content is authored, versioned explicitly, no fallback path. Fix
  rounding semantics in writing, with worked examples — PCGen rounds per tag, not globally, and 2e
  is full of `/2`.
- **Identifiers sharing a namespace with expression syntax.** A PCGen type named `Illumination`
  breaks because it contains the substring `MIN`. Never substitute text into a formula string,
  never case-fold, and make identifiers lexically distinguishable from operators.
- **An undeclared free-text taxonomy.** PCGen's `TYPE:` is its main grouping and prerequisite key
  and is declared nowhere. Packs will invent categories constantly; require them to be declared in
  the manifest.
- **One physical line per record**, and **`.MOD`-style patching**. The capability `.MOD` provides
  is exactly what kits need; the encoding is what to avoid — load-order dependent, unstructured,
  and it mutates the *shared* class record, so the kit would apply to every character.
- **A game-global XP table** and **a class-global level cap**. 2e needs an XP table per class
  (two live at once under dual-classing) and a race×class level-limit matrix.
- **Hardcoding the tables in the application.** Every dedicated FOSS 2e tool does this. That gap
  is precisely what corerules fills.

### Glossary hazard

PCGen's `KIT` / `STARTPACK` is a one-shot bundle of creation choices — **not** what AD&D 2e calls
a kit — and its `TEMPLATE` is a creature overlay. Both words are taken by other meanings in the
most obvious prior art. `CONTEXT.md` already avoids "template"; it should also record that "kit"
collides with PCGen's usage.

### What this research does *not* settle

- **How a kit patches its parent class.** No prior art exists anywhere. PCGen's four partial
  shapes each fail for a different reason, and the most complete open 2e implementation has no
  kits. corerules must invent this — split out as [ticket 10](./10-kit-modifies-parent-class.md).
- **Multi-class XP splitting and dual-class suppression.** No prior art found. PCGen's
  multiclassing is d20's; the Roll20 sheet handles multi-class only for saves, by taking the best
  value across class groups, and does nothing about XP. Input to
  [ticket 05](./05-generation-pipeline-depth.md).
- **Whether packs may define new *kinds* of object** (psionics as a parallel subsystem) or only
  new instances of engine-known kinds. Still fog.
- **Whether an authoring UI or a hand-editable text format should be primary.** Not investigated.
  Data point: PCGen's own docs say list editing needs "a text-editing program and patience", and
  it mitigates with an autoformatter, a converter and editor syntax definitions. Still fog.
- **Hero Lab, Fantasy Grounds and TSR's Core Rules 2.0** — all closed source, not investigated.
  Hero Lab is *reputed* to have the nearest commercial analogue to 2e kits; unverified.

### Licensing, worth stating in the README

An actively maintained, twenty-plus-year-old, explicitly data-driven engine with a volunteer data
team and a formal publisher-liaison process still has no AD&D 2e content, because it cannot get
permission from Wizards of the Coast. Shipping no content is not merely defensible for corerules;
it is the only configuration under which public 2e support exists at all.
