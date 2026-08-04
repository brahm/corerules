# Does the tool validate the rules, or merely record them?

Type: grilling
Status: open
Blocked by: —

## Question

When a user builds a character, does corerules **enforce** 2e's legality rules, or does it
**record** whatever the user enters and compute derived values without judgement?

The rules in question: class/race combinations, ability score minimums per class, race-based
level limits, kit prerequisites and restrictions, proficiency slot budgets, alignment
restrictions, spell access by level and school/sphere.

This is the cheapest ticket on the map to answer and the one that changes the most downstream.
If corerules validates, a content pack must encode *rules* — prerequisites, restrictions,
budgets — and the pack format becomes a small rules language. If corerules only records, a
pack is close to a catalogue with some lookup tables.

Sub-questions worth separating:

- Is the answer uniform, or per-rule? (Hard-block illegal class/race, but merely warn on
  ability minimums, say.)
- Does the answer differ between *creation* and *later editing*?
- House rules are near-universal in old-school play. Does validation have an escape hatch, and
  if so is it per-character or per-rule?
