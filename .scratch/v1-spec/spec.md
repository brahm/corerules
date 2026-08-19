# corerules v1 — specification

Consolidated from the [wayfinder map](map.md) and its fourteen resolved tickets. **This document
states what is decided; the tickets state why**, including what was rejected and on what grounds.
Every section links to its ticket — follow the link before reopening anything here.

Terminology is defined in [`CONTEXT.md`](../../CONTEXT.md) at the repository root. Words in
**bold-capitalised** form below (Engine, Content Pack, Character, Attachable, Layer, Level Event)
are used in the sense defined there.

---

## 1. What corerules is

A desktop tool for creating and managing AD&D 2nd Edition characters.

**It ships no licensed game content.** The Engine supplies the rules logic and the interface; the
user supplies the data from books they own, as Content Packs. AD&D 2e has no SRD and no open
licence — the tables, kit descriptions and spell text are Wizards of the Coast IP. This is not
merely defensible: [ticket 01](issues/01-prior-art-2e-content-modelling.md) established that it is
the only configuration under which public 2e support exists at all. PCGen, a twenty-year-old
data-driven engine with a volunteer data team and a formal publisher-liaison process, still has no
AD&D 2e dataset because it cannot get permission.

**v1's scope is creating and persisting Characters.** Not campaigns, not monsters, not multi-user.

### Roadmap

| | Contents |
|---|---|
| **v1** | Core books (PHB, DMG) and the PHBR "Complete" series — class kits and racial kits |
| **v2** | Campaign settings, psionics, further books to be decided |
| **v3** | Player's Option |

Set by [ticket 03](issues/03-which-complete-handbooks.md). The major version number tracks this
roadmap directly (§10).

### Standing premises

Settled during charting and not open here:

- **Single-user.** No accounts, no authentication, no permissions.
- **Desktop application.** Electron, TypeScript + React, cross-platform. Electron over Tauri
  because Tauri's system webviews mean three rendering engines, which is the cross-platform
  variance this project exists to avoid.
- **Unsigned builds.** Not a commercial product. What that costs per OS is §11.
- **The Engine is native to AD&D 2e**, not a generic RPG engine that happens to run 2e.

### Two permanent constraints

Both excluded from v1; **no v1 decision may foreclose either**.

1. **Sync between clients** (LAN or internet) is a certain future direction. In practice: stable
   global identifiers rather than local autoincrements, and a persistence format that diffs and
   reconciles.
2. **v2 and v3.** In practice: enumerations stay open, the patch mechanism takes scope as a
   parameter, and the Character records which packs it was built against.

---

## 2. The central architectural decision

**Closed kinds, open enumerations.** ([ticket 03](issues/03-which-complete-handbooks.md))

The Engine knows what a class, a class group, a kit, a proficiency slot, a sphere and a school
*are*. Packs supply instances and numbers, never new kinds.

This is affordable — and it is the reason the generic path was rejected — because **AD&D 2e is a
dead edition**. The concept set is closed by history; nothing new will ever be published. The fear
that justifies a generic engine ("what about rules I did not foresee?") does not apply. PCGen went
generic and still cannot express 2e: no per-class XP table, no race×class matrix, and it had to
retrofit data-declared fields when a closed token set failed to survive new books.

**The qualifier is load-bearing: the Engine owns the *shape*, the pack owns the *contents*.** The
Engine knows there is such a thing as a saving-throw matrix keyed by class group and category and
indexed by level — but *which categories exist* is data. Ravenloft adds `fear`, `horror` and
`madness`; a hardcoded enum would break on contact.

---

## 3. Object kinds

> **Non-normative since the corpus map closed.** Sections 3, 4 and 7 describe the pack format, and
> the format now has a second description — `pack-0.1.schema.json` — that is **22 commits ahead of
> this one and is machine-checked against 1,236 records on every run.** Where the two disagree, **the
> schema is right**, and it has been the deciding authority for every correction the corpus effort
> found. These sections are kept as the reading guide that explains *why* the schema is shaped as it
> is; they are no longer the definition of what a pack may contain.
> ([Engine ticket 01](../engine-v1/issues/01-which-spec-does-the-engine-implement.md))

A **kind** is anything that must be referenced by stable identity across a pack boundary. The test
is operational: *can I point at this from another pack, or from a Character?*
([ticket 11](issues/11-engine-object-kinds.md))

The inventory splits three ways. Only the first needs identity.

### 3.1 Pack kinds

Race · Class · Class group · Kit · Deity ·
Alignment · Ability · Language ·
Weapon proficiency · Non-weapon proficiency · Proficiency group · Proficiency slot type ·
Thieving skill · Class ability ·
Spell · Spell school · Sphere ·
Weapon · Armour · Gear ·
Saving throw category · Encumbrance category · Coin ·
Generation method · Lookup table · Rule-set · Content Pack

Notes on the non-obvious ones:

- **Class group** (warrior / wizard / priest / rogue) is a real modelling entity, not a UI grouping.
  Saving throws, THAC0 and proficiency budgets all key on it.
- **Weapon and non-weapon proficiency are two kinds**, and their **slots are convertible
  currencies** — hence *proficiency slot type* being a kind of its own. The exchange rate is a book
  rule and comes from the pack; an undeclared rate means no exchange is permitted. This is why
  Player's Option can add character points as a third currency in v3 without a format break.
- **Weapon, Armour and Gear are three kinds** sharing one value type for physical properties. They
  are referenced by different things — weapon proficiency points at a Weapon, class and kit
  restrictions point at Armour, Gear is referenced by no rule at all. One kind would make "chain
  mail with a damage die" representable.
- **Lookup table** is ticket 01's named, typed table. XP progressions, THAC0 progressions,
  saving-throw matrices and race×class level-limit grids are **instances** of it, owned by their
  class, group or race.
- **Subrace is a Race with a parent reference**, not a kind of its own.

### 3.2 Character structures

Live in the Character file, not in packs:

Character · Class arrangement · Level Event · Attachable binding · Proficiency debt ·
Weapon specialisation · Spellbook · Inventory

### 3.3 Value types

Structure without identity; never referenced:

Dice expression · Prerequisite predicate · Effect · Slot budget · Hit dice · Money ·
Physical properties (weight, cost, size)

### 3.4 Enumerations

**Every enumeration is open except one.** Saving throw category, encumbrance category, coin,
proficiency slot type and ability all grow from packs.

**Rule-set names are the exception and must be closed.** A3 (§5) has a pack declare which rule-sets
it provides, and the Engine cannot act on a name it does not understand — a pack declaring
`lunar-phase-restriction` would have no effect. That catalogue is the Engine's.

---

## 4. The Layer model

> **Non-normative since the corpus map closed.** Sections 3, 4 and 7 describe the pack format, and
> the format now has a second description — `pack-0.1.schema.json` — that is **22 commits ahead of
> this one and is machine-checked against 1,236 records on every run.** Where the two disagree, **the
> schema is right**, and it has been the deciding authority for every correction the corpus effort
> found. These sections are kept as the reading guide that explains *why* the schema is shaped as it
> is; they are no longer the definition of what a pack may contain.
> ([Engine ticket 01](../engine-v1/issues/01-which-spec-does-the-engine-implement.md))

**Nothing is ever overwritten.** A Character's view of any value is computed by walking a stack:
the base record, then each Attachable, with **the Character's own choices as the topmost layer**.
([ticket 10](issues/10-kit-modifies-parent-class.md))

An Attachable never touches the shared record. Patching the shared class record would apply a kit
to every Character using that class — one of the two axes on which PCGen's `.MOD` fails.

Three earlier decisions made layering close to obligatory: kit abandonment must *drop* a layer;
magic items are already a stack (armour + magic + Dexterity); and hard validation's refusals must
name their cause, which requires provenance to survive computation. Overwriting erases provenance
by construction.

### 4.1 Attachables

**Kit, Deity and Subrace are one closed shape used three times** — a binding to a target, a
prerequisite predicate, and an ordered list of effects. They remain **distinct kinds**, because
cardinality is the one thing they cannot share.

| | Target | Referenced by other records | Cardinality |
|---|---|---|---|
| **Kit** | a class entry, or the race | no | one per target |
| **Deity** | a priest class entry | yes | one per target |
| **Subrace** | the race | yes | one per target |

Radius is implied by the target, never a separate field. A v2 campaign setting is the same
mechanism pointing at something wider.

### 4.2 Effects — three natures

1. **Standing modifier** — bonuses, penalties, restrictions, sphere access. Removed when the
   Attachable is removed.
2. **One-time grant** — proficiencies, equipment, starting money. Survives.
3. **Obligation against future budget** — created by abandonment, consuming later slots.

### 4.3 Effects — six operations, closed

`adjust` (sums) · `grant` (unions) · `forbid` (subtracts, beats `grant`) · `except` (pierces a
prohibition) · `require` (obliges a future choice) · `set` (fixes a value)

Each may be **conditioned** by level or by a predicate. That is a qualifier, not a seventh
operation.

**Operations are order-independent by design.** "Which weapons may I use" has one answer regardless
of how packs were loaded. This is the direct remedy for the load-order dependence ticket 01 found
in PCGen's `.MOD` and its `RANK:` patch, and it honours that research's own prescription that
cross-pack collisions resolve by an explicit, visible rule.

- **Two `set`s on one field are a reported conflict** — "kit X and deity Y disagree about hit die"
  — never a silent last-loaded-wins.
- **`except` names the subject, not the prohibition.** `except: long sword` means "long sword is
  permitted notwithstanding any prohibition". This avoids giving Effects identity, which §3.3
  forbids.

**Anything outside the vocabulary is carried as text and not computed.** "Must tithe 10%" has a
mechanical part and becomes `adjust`; "never refuses a challenge" is text the sheet displays. The
same line applies to magic items whose mechanics do not fit — charges, curses, artifacts.

---

## 5. Validation

**corerules validates, and it validates hard.** Illegal states are unrepresentable **at the point
of choice**. ([ticket 04](issues/04-validate-or-record.md))

Because corerules ships no content, the rules being enforced are the user's own transcription.
**This makes the pack format a small rules language** — a constraint, not an option.

### 5.1 A3 — packs declare their rule coverage

While a corpus is being transcribed, absence of data is otherwise indistinguishable from absence of
a rule.

- A pack that **declares** it provides a rule-set makes absence from its permit-list a hard block.
- A pack that declares nothing about a rule means the Engine **does not validate that rule and says
  so visibly**.

**Declarations scope to the subjects a pack introduces**, and the Engine unions them.
([ticket 13](issues/13-how-packs-get-authored.md)) A global claim of authority breaks at the second
book transcribed: the PHB states race/class restrictions as a permit-list, and the Complete
Barbarian's Handbook then adds a class that list predates, which would make Barbarian unplayable by
any race. Union also makes rule *extension* work for free — the Complete Book of Elves permitting
elf paladins simply adds to the list.

A **declared but empty** rule-set is reported as suspicious: strictly it means "nothing is
permitted", which is almost always half-finished extraction rather than intent.

The declaration **cannot be derived from the contents**, which would otherwise prevent
desynchronisation — deriving it would lose the distinction between "does not restrict" and "not
transcribed yet", which is the entire purpose of A3.

**A3 subsumes the house-rule escape hatch.** No override mechanism is needed: an optional DMG rule
or a table's house rule is expressed by what the pack declares and contains. The escape hatch is
the pack.

**And A3 governs provenance, for the same reason.** (correction 6) §7.1 made book-and-page citation
mandatory on every record, which **forbade the pack this paragraph promises** — a hand-authored rule
has no rendition and no source file to point into, and the two rules could not both hold. Resolved
the way A3 resolves everything else: **the pack declares which kind it is and the requirement
follows.** A manifest declaring `extracted` means every record carries an anchor and the manifest
names its sources by hash; `hand-authored` means no record carries an anchor and the pack names no
sources. Deriving the mode from the contents was rejected for A3's own reason — it would lose the
distinction between *a house rule that cites nothing* and *an extraction that has not been finished*.

The condition spans the manifest and the records, which is **two files**, so it is enforced by the
checker rather than the schema — the same place, and for the same reason, that duplicate ids are.

### 5.2 A3 governs validation, not computation

([ticket 14](issues/14-multi-class-and-dual-class-model.md)) A missing *validation* rule is an
unenforced restriction and the user is told — no harm. A missing *computation* rule is a **wrong
number**. Rules the Engine needs in order to compute are therefore Engine knowledge, never optional
pack data (§6.2).

### 5.3 Quarantine

**Loading a Character never fails.** An invalid Character opens **fully readable and printable**;
what is locked is everything that *extends* it — levelling, spending XP, adding proficiencies,
buying spells. The lock lifts when the violations clear.

Reading and printing an invalid Character harms nothing, because the error is already there. What
harms is building on top of it, since the error then propagates into every value derived from that
point on.

Forced correction was rejected because **some invalid states no edit can fix**: if a referenced pack
is missing, no choice in the UI resolves it, because the thing to choose is not loaded. Quarantine
distinguishes "invalid" from "not repairable here".

**Validation must understand the pipeline, not just the sheet.** A dual-class Character legally
occupies states that look illegal in isolation — old class suppressed, XP frozen. A naive per-field
validator would reject a perfectly legal dual-class on load.

### 5.4 A third kind of answer

([Engine ticket 02](../engine-v1/issues/02-what-the-engine-does-with-an-unmodelled-effect.md))
A transcriber who models an effect as far as the format allows and writes down what is left over
produces an effect that is **neither a value nor a refusal**. 381 of the pack's 1,919 effects are
of this kind. §5.1's A3 is a *pack-level* declaration about a rule-set and §5.3 quarantines a whole
*character*; neither reaches one clause of one record.

**A marked effect never reaches the total**, and what happens to it is decided by its **operation**,
never by any category its prose declares:

- **structural** — `grant`, `forbid`, `require`, `except`: the thing is right and its edges are
  under-described. **Applied**, with the transcriber's note carried on that entry.
- **numeric** — `adjust`, `set`: the number is right and the circumstance is missing. **Withheld from
  the sum** and shown beside it as a named situational line the player applies when the circumstance
  holds.

This is §5.2's line drawn at the grain of an effect rather than a rule-set: those four operations are
validation-shaped by construction and those two are computation-shaped by construction.

**A value may therefore be withheld for three distinct reasons** — a marked effect supplies it, its
predicate asks something the sheet cannot answer, or two layers set it and nothing declares a winner
([Engine ticket 03](../engine-v1/issues/03-precedence-when-two-attachables-contradict.md)). In every
case the sheet says which record and which book, which is §1's promise reached at the level of a
single number.

### 5.5 The one exception

**Magic items are recorded, not validated.** ([ticket 11](issues/11-engine-object-kinds.md))
Acquiring one is DM fiat and no rule governs it. **Use** stays validated: a wizard may not use a
sword. Acquisition free, use by the book.

---

## 6. Characters

### 6.1 The class arrangement is a sum type

`Single(class)` | `Multi([class])` with arity ≥2 | `Dual(original, new)`.
([ticket 14](issues/14-multi-class-and-dual-class-model.md))

The governing principle, which recurs throughout this spec: **structural for model incoherence,
pack-declared for game rules.** "Dual-class with three classes" is not a rule a pack transcribes;
it is an incoherence of the model, and must be impossible to *represent* — because under A3 a
pack-declared restriction may simply not be there, whereas structure cannot fail to be declared.

Group-indexed class slots (the Roll20 approach) were rejected because indexing by class group
**hardcodes the group enumeration**, which §3.4 keeps open.

Accepted cost: the arms do not share read code.

### 6.2 The Engine owns the combination rules

XP split evenly · hit points averaged across hit dice · best saving throw across classes · best
THAC0 · best slot progression · the dual-class suppression threshold (new level > original level).

These are shape, not content. The saving-throw matrix is the PHB's numbers; "with two classes, take
the best" is in no table anywhere.

### 6.3 A Character is a sequence of Level Events

Every advance records **which class went up, the die rolled, and what was chosen**.

**Hit points are recorded randomness** — neither a choice nor a derivation, a third category. Under
multi-class, hit points accrue by rolling the advancing class's die and dividing by the number of
classes, so a total cannot be reconstructed from "fighter 5 / mage 4". Storing only current levels
plus a hit point total is therefore **not implementable**, not merely worse: correcting a level on
the sheet would leave the Engine unable to recompute.

**Dual-class suppression is derived, never stored** — a pure function of the frozen original level
and the current new level.

### 6.4 Kits on a Character

One kit, **chosen at creation**, compatible with the race-and-class combination, bound to a named
target, and **never rebound**. On dual-classing it stays with the **original** class, is not
required to be compatible with the new one, and nothing is checked at the switch. It may be
**abandoned**, but a new one can never be adopted.

**Abandonment** removes all standing modifiers — benefits *and* penalties — and leaves the granted
proficiencies as a **nominal debt**: the Character owes *those specific proficiencies*, not a count,
against future slots. The level-up flow allocates new slots to the debt rather than offering free
choice.

**A Character carrying debt is valid, not quarantined.** Quarantine would deadlock: the debt is only
payable by levelling, which quarantine locks. The debt may be unpayable — if the kit granted a
proficiency the class cannot take — and the sheet must display it, or it becomes a phantom bug.

### 6.5 The Character file

A **single JSON file**. ([ticket 07](issues/07-character-file-format-and-identity.md))

- **The Character and every Level Event carry a UUIDv7.** Not individual choices within an event.
  The reasoning is retrofit: Characters created without event identity could never be reconciled
  per-event afterwards. Time-ordered, so file order is chronological order with no extra field.
- **References to pack entries are live**, and the file records a **content hash** of each pack it
  was validated against ([ticket 08](issues/08-persistence-files-or-embedded-db.md) — a declared
  version would miss every typo fix, since nobody bumps a version for one). On open, if a pack has
  moved, the Engine re-validates and **reports what changed**. Loading still never fails.
- **Snapshotting pack data into the Character is forbidden.** A Character carrying pack data *is a
  pack in disguise*, and sharing it would distribute WotC-derived content, collapsing §1's posture.
- **The active pack set is load-bearing, not decorative.** It disambiguates `phb:set-snares` from
  `cbarb:set-snares` (§7.4).
- **Corrections rewrite history in place.** The old value stops existing; auditability is
  deliberately traded for a file that stays legible in a text editor.
- **Quarantine and provenance are derived, never stored.**

**Cross-user sharing is not a v1 goal.** A Character is legally the user's own to share, but
functionally useless without the packs it references — and the recipient would need the same books
transcribed *under the same pack identifiers*, which nothing can guarantee. Portability means across
the user's own machines. **The README must state this**, because it is easy to assume otherwise.

---

## 7. Content Packs

> **Non-normative since the corpus map closed.** Sections 3, 4 and 7 describe the pack format, and
> the format now has a second description — `pack-0.1.schema.json` — that is **22 commits ahead of
> this one and is machine-checked against 1,236 records on every run.** Where the two disagree, **the
> schema is right**, and it has been the deciding authority for every correction the corpus effort
> found. These sections are kept as the reading guide that explains *why* the schema is shaped as it
> is; they are no longer the definition of what a pack may contain.
> ([Engine ticket 01](../engine-v1/issues/01-which-spec-does-the-engine-implement.md))

### 7.1 Shape

A **directory** of **JSON** files whose contents a **manifest declares**.
([ticket 06](issues/06-content-pack-format.md))

- **Directory**, because diffability is a standing constraint, a half-authored pack is the normal
  state, and a 5 MB document cannot be repaired in an editor. The usual objection — that a directory
  is not one thing you can hand someone — does not apply, since packs do not circulate.
- **JSON**, because **YAML is silently wrong by specification** (`NO` → false, `1.20` → float) and
  never being silently wrong is this project's posture. JSON's lack of comments is already answered:
  provenance is a **validatable field rather than a comment**. *(Corrections 6 and, earlier, ticket
  05: this said "book and page citation is required on every record". Neither half survived. The
  corpus has no page numbers, so provenance is a heading chain plus a machine anchor; and the
  requirement is **conditional on the manifest's declared `provenanceMode`**, because an
  unconditional one forbids the house-rule pack §5.1 promises.)*
- **Manifest**, because it is A3 at the file level — declaration over discovery. Scanning would let
  a leftover file from an earlier extraction join the pack in silence. A file present in the
  directory but absent from the manifest is **reported**.

Tables stay tables: `{"warrior": {"paralyze": [16,10,...]}}`. Expressions stay strings the single
evaluator interprets.

### 7.2 The rules language

- **One evaluator, versioned explicitly, with no fallback path ever.** PCGen's best-documented
  failure is three live parsers, a fallback method literally named `processBrokenParser`, and the
  same characters evaluating to 15 or 12 depending on which succeeded.
- **Rounding semantics must be written down with worked examples.** PCGen rounds per tag rather than
  globally — a rule no reimplementer would guess, and 2e is full of `/2`.
- **Identifiers must be lexically distinguishable from operators.** Never substitute text into a
  formula string; never case-fold. A PCGen type named `Illumination` breaks because it contains
  `MIN`.
- **Dice semantics are required**, not just arithmetic: distribution, dropping, rerolling and
  arrangement. Generation methods ("4d6 drop lowest, arrange to taste") are PHB content and come
  from the pack ([ticket 05](issues/05-generation-pipeline-depth.md)).

### 7.3 Identity, versioning, trust

- **Pack-scoped, opaque, never-reused IDs.** Display names are presentation only. Name-as-identity
  was PCGen's worst mistake and the sole reason its `migration.lst` exists.
- **Foundry-style manifest**: `id`, `version`, and a three-way `compatibility` of `minimum` /
  `verified` / `maximum`, with dependencies carrying their own ranges. PCGen has no pack version
  field at all.
- **The pack format carries its own version**, separate from the Engine's. If the expression
  language must change: a converter and a format version bump, never a second evaluator.
- **A pack never carries code.** JSON executes nothing and the expression grammar has no assignment,
  loop or I/O. There is no sandbox because there is nothing to isolate.
- **The schema is the Engine's, and published.** Under closed kinds a pack conforms to a schema
  rather than declaring one.

### 7.4 The same name in two books

`phb:set-snares` and `cbarb:set-snares` are **two proficiencies**, not a collision. `Set Snares` is
Dexterity −1 in the PHB and Intelligence −1 in the Complete Barbarian's Handbook; the Roll20 corpus
has 217 distinct names across 224 entries for exactly this reason. The books treat them as different
things.

Disambiguation is the Character's **active pack set**: a PHB-only Character never sees the other.
**The sheet must always show the source book**, not only when ambiguous.

### 7.5 Loading

**All or nothing.** A pack with one malformed record does not load at all.
([ticket 13](issues/13-how-packs-get-authored.md))

Corpus integrity is binary: "the PHB loads" means the PHB is entirely there. Partial loading would
reintroduce quiet incompleteness — warnings ignored for months leaving Characters built against a
corpus full of holes.

**Incomplete is not invalid.** A pack that declares fewer rule-sets loads normally; that is A3
working. A pack that is *malformed* does not load. Missing transcription and extraction rubbish are
different things, and only the second blocks.

### 7.6 Authoring

**corerules does not author packs.** Extraction from the user's own RTF and PDF copies is a script's
job, done outside the tool. The Engine loads and validates.

**Its whole contribution to authoring is a validator that names the file, the record and the
field.** Under all-or-nothing loading this stops being a convenience and becomes the only route to
a bad record — otherwise the user faces "the PHB does not load" and hundreds of files.

A pack being a directory of JSON puts the entire transcription under git for free, which an embedded
editor would fight. **Extraction must not deduplicate by name** (§7.4) — the natural instinct when
parsing 24 books is exactly wrong.

---

## 8. Persistence

([ticket 08](issues/08-persistence-files-or-embedded-db.md)) **Plain files are the source of
truth.** Storage splits **by who owns the data**:

| | Location |
|---|---|
| **Content** — packs and Characters | a **user-visible folder the user picks**; backup is the user's job by design, and `~/.config` is never backed up |
| **Application state and derived cache** | the OS convention path (`app.getPath('userData')`), precisely because these must **not** travel |

One content root, not multiple libraries. The first-run default must be *visible* on all three
systems — `~/corerules` on Linux, Documents as the analogue on Windows and macOS.

**No persistent index.** The corpus fits in memory; all-or-nothing loading already parses the whole
pack; and **SQL cannot evaluate prerequisite predicates**, which run in the pack's expression
evaluator — an index would cover the cheap half and miss the expensive one. A persistent index would
also be a second source of truth able to go quietly stale.

**A cache of the built form does exist**, keyed by a **per-pack content hash** so that added,
removed and edited are all covered without a button anyone must remember to press. The same hash
serves the Character's drift detection (§6.5).

---

## 9. Product surface

### 9.1 v1 owns the whole generation pipeline

([ticket 05](issues/05-generation-pipeline-depth.md)) Ability scores, race, class, alignment,
weapon and non-weapon proficiencies with slot budgets, kits, spell selection, starting money,
equipment, encumbrance, and derived values. Most of it was already forced by kits, which modify
proficiencies, equipment, money and sphere access.

- **Advancement is in.** XP, levelling, re-deriving. The expensive machinery — level-indexed tables
  — is mandatory at 1st level anyway.
- **Multi-class and dual-class are both in.**
- **The tool rolls dice**, and entry stays a first-class path.

### 9.2 Three modes

| Mode | Surface |
|---|---|
| **Create** | a guided, step-by-step wizard |
| **Advance** | a mini-wizard carrying only that level's choices |
| **Correct / edit later** | direct editing on the sheet, exposing the timeline |

**The same validation rules must hold on both paths**, or sheet editing becomes the back door that
undoes §5.

### 9.3 Never

**Automatic character construction is ruled out permanently** — not deferred to a later version.
corerules builds step by step, on the model of TSR's AD&D Core Rules 2.0 and D&D Beyond.

Rolling dice is *rule*: it is in the books, it comes from the pack, it is objective. Choosing a kit
and proficiencies for the user is *taste*, and no source book contains it. Under hard validation an
auto-generator would also need a constraint solver rather than a sampler, since naive sampling
dead-ends on ability scores no class accepts or a kit whose prerequisite an earlier choice broke.

---

## 10. Packaging and release

([ticket 02](issues/02-electron-packaging-and-release.md),
[ticket 09](issues/09-release-pipeline-and-unsigned-warning.md))

- **electron-builder**, pinned to **26.15.7**. Forge has no AppImage or NSIS maker and its docs have
  no CI guide at all. npm's `latest` tag for electron-builder is genuinely stale at 26.15.3 —
  verified twice. v27 is alpha, ESM-only, and renames `mac.identity`.
- **Electron 43.3.0.** The v43 line is supported to January 2027. Electron 44 drops Windows ia32 and
  Linux armv7l.

### 10.1 Artifacts

| OS | Ships | Not shipped |
|---|---|---|
| **Linux** | AppImage | rpm, deb |
| **Windows** | NSIS installer | portable exe |
| **macOS** | DMG | ZIP |

**rpm is out because of the maintainer's own machine**: Fedora 45 (~October/November 2026) flips
`%_pkgverify_level` to `all` and `rpm` will refuse unsigned packages. Publishing a package that
stops installing within months of v1 is worse than not publishing it.

### 10.2 macOS signing — proven configuration

```yaml
mac:
  identity: "-"                      # ad-hoc
  hardenedRuntime: true
  entitlements: build/entitlements.mac.plist
  entitlementsInherit: build/entitlements.mac.plist
```

with `com.apple.security.cs.disable-library-validation` (without it the app dies at launch on a Team
ID mismatch, because Electron's prebuilt frameworks carry Apple's Team ID and an ad-hoc signature
does not) and `com.apple.security.cs.allow-jit`.

**Ad-hoc signing is not optional.** Apple Silicon does not execute unsigned arm64 code at all, and
electron-builder does *not* ad-hoc sign automatically when it finds no certificate.

[Ticket 12](issues/12-verify-adhoc-signed-macos-build.md) verified this by building on CI and
running on a different Mac: **an ad-hoc signature does survive leaving the build machine**, contrary
to electron-builder's own documentation.

### 10.3 Release process

- **A pushed tag triggers the build; the release is born a draft.** The only click in the process is
  publishing. Manual dispatch was rejected because its branch dropdown defaults wrong and fails
  late — demonstrated during ticket 12.
- **SemVer with the major tied to the roadmap**: major 1 is the v1 milestone. Affordable because the
  pack format carries its own version, so the break that matters already has a number. Releases
  before v1 are `0.x`.
- **Three versions exist and do not move together**: the application version (the tag), the pack
  format version, and each pack's own version.
- `permissions: contents: write` at workflow level; naming any permission sets every unnamed one to
  `none`.
- **Pin runner images** rather than using `-latest`, which migrates over 1–2 months.
- **No two-stage pipeline**: a release created by `GITHUB_TOKEN` does not fire an `on: release`
  workflow.
- **The workflow file must live on the default branch** or GitHub will not register it.

The executable checklists — one-time setup, cutting a release, and what the workflow must do — are
in [ticket 09](issues/09-release-pipeline-and-unsigned-warning.md).

---

## 11. What the README must say

**"Unsigned" is three different stories, not one warning.**

| OS | The user must |
|---|---|
| **Linux** | `chmod +x` the AppImage. No gatekeeper to get past. |
| **Windows** | Click **More info** → **Run anyway** past SmartScreen. |
| **macOS** | Open Terminal and run `xattr -dr com.apple.quarantine /Applications/corerules.app`. |

Three things the README must state plainly rather than imply:

1. **Some Windows 11 users cannot run corerules at all.** Smart App Control blocks unsigned software
   with **no per-app override**, and it cannot be re-enabled without reinstalling Windows.
2. **On macOS the app reports itself "damaged", and System Settings → Privacy & Security does not
   help.** Ticket 12 confirmed by experiment that allowing the app there does not make it run.
   Terminal is the only route. Say why in one line: no Apple Developer certificate, which costs
   US$99 a year.
3. **Where files are stored**, per OS, since backup is the user's job by design.

The README must also record the licence position: **packs do not circulate.** A Character is the
user's own work; a Content Pack is derived WotC content.

---

## 12. Out of scope

| | Why |
|---|---|
| **Campaign settings** | v2. A setting is not more content — it is a set of overrides to the central model. |
| **Psionics** | v2. The one PHBR volume that is not additive: a second character subsystem. |
| **Player's Option** | v3. Replaces the core character model rather than extending it. |
| **Automatic character construction** | **Never.** See §9.3. |
| **Campaigns** | Post-v1. A genuinely second aggregate. |
| **Monster stat blocks** | Not Characters at all. |
| **Multi-user / player logins** | Including live sync and GM-hidden data. |
| **Self-hosted or browser delivery** | A different product with a different persistence and security model. |
| **Unearthed Arcana** | 1st Edition. |

---

## 13. Known unknowns

**Rewritten after the corpus map (71 sessions) and the Engine map's first four tickets.** The nine
entries this section carried have been judged one by one; what follows is what is actually still
unknown, plus a note on why three of the nine could never have closed in the form they were written.

**Three closed.** #1 *the kit mechanism has no prior art* and #4 *the six-operation vocabulary may
prove insufficient* and #2 *"the Engine computes, the user supplies the tables" has no shipping
precedent*. §4.1's one-shape claim held across **238 records and three arms** without a single
per-arm exception; six operations expressed **1,910 effects**, and the largest gap found since was
closed with a record field rather than a seventh operation; a table now declares `supplies`.

**Five were never visited**, because nothing has been packaged or shipped: the SmartScreen wording,
the Workflow permissions labels, Intel Macs, `node:sqlite` in a packaged app, and whether the spike's
unsigned build was unsigned. They stand exactly as written and belong with §10, where the
work that would answer them lives — none is a question about the format or the Engine.

**And the lesson worth carrying**: #1 and #2 were both phrased as anxieties about *precedent* — "no
prior art anywhere", "no shipping precedent" — and **neither was answered by finding a precedent.**
Both were answered by building the thing and measuring it. An entry here should therefore name a
**question with a test**, not a worry. The four below do.

---

1. **Can the format express psionics?** Unchanged, and now the only substantive entry inherited from
   the original nine. Deferring it entirely — rather than carrying a worked example as proof of load
   — risks discovering in v2 that the format cannot express it, which is how PCGen failed. Raised and
   overruled; the overruling has not been revisited.
   **The test**: transcribe one psionic discipline against the current schema and count the
   `UNMODELLED` markers it needs.

2. **Can a bound be stated over item properties?** Seventy-two effects restrict by a property no
   record carries — *metallic*, *more than a tenth metal by weight*, *larger than a knife*,
   *concealable*. A weapon record holds cost, weight, size, damage type and speed factor and **no
   material**; armour is seven category-shaped records. Enumeration reaches the other 125 bounds and
   cannot reach these.
   **The test**: whether the item properties the twelve books actually discriminate on are a closed
   set. If they are, it is a field; if they are not, this is where the format stops.

3. **What does the Engine do with the corpus the pack cannot contain?** Seventeen effects ask a
   character to choose a totem animal, a hated foe, a guarded site, a terrain, an undead type. The v1
   tier holds no animals, no terrains and no monsters, so the choice is unbounded by construction —
   and the same boundary produced fourteen creature names living inside field-path strings.
   **The test**: whether the Monstrous Manual tier, when it exists, dissolves this or merely moves it.

4. **Does a marker expire, and can anything tell?** A marker records what the format could not do
   **at the moment it was written**, and the format kept moving: 37 of the 381 markers name an
   obstacle a later schema commit removed, and nothing in the pack ties a marker to the schema version
   that provoked it.
   **The test**: whether a marker can carry the schema version it was written against, cheaply enough
   that transcribers actually do it.
