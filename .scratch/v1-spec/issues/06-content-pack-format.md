# Content pack format

Type: grilling
Status: open
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
