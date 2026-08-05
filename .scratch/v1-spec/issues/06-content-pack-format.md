# Content pack format

Type: grilling
Status: open
Blocked by: 01, 03, 04, 10, 11

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
  on conflict.
- **Trust** — packs come from outside. Does the format permit anything executable?

Answers to 01 (prior art), 03 (handbook scope), 04 (validate or record), 10 (kit mechanism) and
11 (object kinds) all constrain this; resolve them first.
