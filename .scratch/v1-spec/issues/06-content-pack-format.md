# Content pack format

Type: grilling
Status: open
Blocked by: 01, 03, 04

## Question

What is a content pack, concretely?

This is the load-bearing decision of the whole project: corerules ships no licensed data, so
the engine is only as capable as the pack format is expressive.

- **Shape** — a single file, an archive, a directory? Text or binary? Hand-editable?
- **Schema** — how is it declared and validated, and is the schema published so third parties
  can author packs?
- **Expressing 2e** — per-class XP tables, level progressions, THAC0 and saving-throw
  matrices, race level limits, kits that modify their parent class, proficiency groups, spell
  schools and spheres. If ticket 04 lands on "validate", also: prerequisites, restrictions and
  slot budgets, in a form the engine can evaluate.
- **Identity and versioning** — how a pack is named and versioned, whether packs can depend on
  or extend one another (a Complete handbook pack extending the core pack), and what happens
  on conflict.
- **Trust** — packs come from outside. Does the format permit anything executable?

Answers to 01 (prior art), 03 (handbook scope) and 04 (validate or record) all constrain this;
resolve them first.
