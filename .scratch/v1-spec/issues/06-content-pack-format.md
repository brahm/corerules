# Content pack format

Type: grilling
Status: resolved
Blocked by: —

## Question

What is a content pack, concretely?

This is the load-bearing decision of the whole project: corerules ships no licensed data, so
the engine is only as capable as the pack format is expressive.

- **Shape** — a single file, an archive, a directory? Text or binary? Hand-editable?
- **Schema** — how is it declared and validated, and is the schema published so third parties
  can author packs?
- **Expressing 2e** — per-class XP tables, level progressions, THAC0 and saving-throw
  matrices, race level limits, proficiency groups, spell schools and spheres. **And, settled by
  [ticket 04](./04-validate-or-record.md): prerequisites, restrictions and slot budgets, in a form
  the engine can evaluate.** 04 landed on hard validation, so this format *is* a small rules
  language — that is a constraint, not an option. It must also carry a pack's **declaration of
  which rule-sets it provides** (04's A3), since the engine has to tell "this rule does not
  restrict that" from "this rule was never transcribed". (Kits patching their parent class is its
  own decision — ticket 10.)
- **Expression language** — wherever a number is expected, is there a formula? Ticket 01's
  research is unambiguous that there must be exactly one evaluator, versioned, with no fallback
  path, and that rounding semantics must be written down with worked examples. **And
  [ticket 05](./05-generation-pipeline-depth.md) added dice**: the tool rolls, and a generation
  method ("4d6 drop lowest, arrange to taste") is PHB content, so it comes from the pack. The
  language therefore needs distribution, dropping, rerolling and arrangement — not just arithmetic.
- **Identity and versioning** — how a pack is named and versioned, whether packs can depend on
  or extend one another (a Complete handbook pack extending the core pack), and what happens
  on conflict. **[Ticket 11](./11-engine-object-kinds.md) fixed what needs identity at all**: its
  list A, and only that. Character structures live in the character file, and value types —
  prerequisite predicates, effects, slot budgets, dice expressions — carry no identity and are never
  referenced, which corrects a loose sentence in ticket 04.
- **The one closed enumeration** — ticket 11: every enumeration stays open except **rule-set
  names**, which must be the engine's, because A3 has packs declare which rule-sets they provide and
  the engine cannot act on a name it does not understand.
- **Trust** — packs come from outside. Does the format permit anything executable?

**Unblocked — all five blockers are resolved, and between them they have already fixed most of this
ticket's answer.** What remains is genuinely about the *format*: shape, syntax, schema declaration,
versioning and trust. The modelling is done.

Inherited, and not to be reopened:

- **What needs identity** (11): pack kinds only. Character structures live in the character file;
  value types — predicates, effects, slot budgets, dice expressions — are never referenced.
- **The effect vocabulary** (10): six operations — `adjust`, `grant`, `forbid`, `except`, `require`,
  `set` — closed, each optionally conditioned by level or predicate. Anything outside it is carried
  as text with nothing computed.
- **Order-independence** (10): the format must not permit a pack's meaning to depend on load order.
  Two `set`s on one field are a reported conflict, not a last-wins. This is the direct remedy for
  PCGen's `RANK:`, which ticket 01 identified as a verified failure.
- **The rules language is mandatory** (04): prerequisites, restrictions and budgets must be
  evaluable, and packs must declare which rule-sets they provide.
- **Dice semantics** (05): generation methods are pack content, so the expression language needs
  distribution, dropping, rerolling and arrangement.
- **One evaluator, versioned, no fallback path** (01), with rounding semantics written down and
  worked examples.

## Answer

The map called this the load-bearing decision of the project, and it arrived disarmed: five
resolved tickets had already fixed the modelling. What was left was the format itself.

### Decision — a pack is a directory

Rejected: a single file, and an archive as the native form. An archive remains available for free
as a transport wrapper if one is ever wanted; it is not the format.

- **Diffability is the map's permanent sync constraint** — "a persistence format that can be diffed
  and reconciled". Under a single file every typo correction rewrites megabytes; under a directory
  it touches one small file. It is the same problem ticket 01 documented in PCGen's
  thousand-column lines, on a different axis.
- **A half-authored pack is the normal state.** A3 depends on the engine knowing what a pack
  provides; a directory shows it in the structure — spells done, equipment empty — before any
  parser runs.
- **Hand repair.** Ticket 01 is explicit that text is fixed with an editor; a 5 MB JSON document is
  not, in practice, and extraction from PDF will produce localised rubbish.

The usual cost of a directory — "not one thing you can hand someone" — **does not apply here**,
because [ticket 04](./04-validate-or-record.md) already settled that **packs do not circulate**.
The licence posture removes the main reason to want a single file.

### Decision — JSON, with a documented profile and JSON Schema

The decisive argument is against YAML, which would otherwise win on readability: **YAML produces
silently wrong values.** Type coercion turns `NO` into `false`, `1.20` into a float, `Yes` into a
boolean. In a corpus of spell components, item names and version numbers typed out of PDF
extraction, that is the exact class of bug this map has been designing against since ticket 01 —
A3's honesty, ticket 10's reported conflicts, PCGen's `processBrokenParser`. **The project's whole
posture is never to be silently wrong, and YAML is silently wrong by specification.**

TOML sits badly on the data: almost everything here is a list of records, and arrays of tables get
noisy fast. A bespoke format is ruled out by ticket 01, which is a twenty-year report on what
becomes of bespoke formats.

**JSON's real weakness — no comments — was already solved by an earlier decision.** Ticket 01
requires book and page citation on every record. The main reason to want comments in a
transcription is to note provenance and doubt; that becomes a **field**, which is validatable and
searchable in a way a comment is not.

Two consequences:

- **Tables stay tables.** Ticket 01's named, typed lookup table is an object of arrays in JSON —
  `{"warrior": {"paralyze": [16,10,...]}}` — compact, diffable and readable. No second syntax is
  needed.
- **Expressions stay strings.** `"floor(level/2)+1"` and `"4d6kh3"` are text the JSON carries and
  the single evaluator interprets. The format does not interpret them.

Accepted cost: JSON is tedious to edit by hand. How much that matters depends on
[ticket 13](./13-how-packs-get-authored.md) — heavily if authoring is external in a text editor,
barely if it goes through the tool.

### Decision — the manifest declares the contents

Rejected: a fixed layout the engine knows by filename, and self-describing files the engine
discovers by scanning.

Fixed layout collapses the diffability that chose a directory — the PHB's hundreds of spells would
land in one `spells.json` when you would want them split by level or school. Scanning makes the
engine's view depend on whatever happens to be on disk, so a leftover file from an earlier
extraction joins the pack in silence — under hard validation, a false rule entering with nothing
declared.

**A manifest is A3 at the file level.** Ticket 04 settled that a pack declares what it provides and
the engine distinguishes "does not restrict" from "not transcribed". Listing files is the same
principle: declaration over discovery. It also makes the orphan file detectable by construction —
present in the directory, absent from the manifest, reported.

**Worth recording: here the prior art is good.** PCGen's `.pcc` does exactly this, listing its data
files by type. Ticket 01 criticised much of PCGen; not this.

### Decision — same name in two packs means two objects

Ticket 01 left this to be answered here, with data: the Roll20 proficiency table has **217 distinct
names and 224 entries**, because books define the same thing differently. `Set Snares` is
Dexterity −1 in the PHB and Intelligence −1 in the Complete Barbarian's Handbook.

Under pack-scoped IDs — ticket 01's central prescription — `phb:set-snares` and `cbarb:set-snares`
**are already different things**. There is no collision to resolve: two proficiencies, different
rules, different books. The books treat them that way; calling it a collision was our error.

The alternative needs an identity that spans packs — something asserting these two *are* the same
Set Snares. That is name-as-identity, which ticket 01 documented as PCGen's worst mistake and the
sole reason its `migration.lst` exists.

**And the disambiguation was already decided elsewhere.** Ticket 03 established, from the Roll20
sheet's 21 per-character book toggles, that **a character records which packs it was built
against**. That active set filters the list: a PHB-only character never sees the Complete
Barbarian's version — not because the engine chose, but because that book is not on their table.

Real cost: a character with both books active sees "Set Snares (PHB)" and "Set Snares (CBH)" in one
list and must know which their table uses. That is honest — a table decision, not a tool decision —
but it is interface friction, and the sheet must always show the source book, not only when
ambiguous.

### Settled by inheritance, recorded so they are not reopened

- **Trust: a pack never carries code.** JSON executes nothing, and the expression grammar has no
  assignment, loop or I/O, so it cannot express a side effect. Ticket 01's verdict stands — there is
  no sandbox because there is nothing to isolate.
- **Versioning follows Foundry's manifest**, which ticket 01 recommended with evidence: `id`,
  `version`, and a three-way `compatibility` of `minimum` / `verified` / `maximum`, with
  dependencies carrying their own ranges. PCGen has no pack version field at all, and ticket 01 was
  explicit about not copying that.
- **The schema is the engine's, and published.** Under ticket 11's closed kinds a pack cannot
  introduce kinds, so it does not declare a schema — it conforms to one. The pack *format* carries
  its own version, separate from the engine's: if the expression language must change, ticket 01's
  prescription is a converter and a format version bump, never a second evaluator.
