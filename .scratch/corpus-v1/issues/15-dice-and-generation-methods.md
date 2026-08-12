# Dice notation and generation methods

Type: grilling
Status: open
Blocked by: 01

## Question

Split out of [ticket 06](./06-expression-language.md), which suspected at charting that it was two
decisions and confirmed it on measurement. Dice are not predicates: `spec.md` §3.3 already lists
**dice expression** as a *value type*, and carrying both in one grammar is what made ticket 06 too
big.

`spec.md` §7.2 requires **dice semantics, not just arithmetic** — distribution, dropping, rerolling
and arrangement — because §9.1 has the tool roll dice and §5's generation methods come from the pack.

## What the corpus actually contains

Measured across the 13 v1 books while resolving ticket 06:

- **2,146 dice notations, 61 distinct forms.** The common ones are unsurprising — `1d4` (394),
  `1d6` (317), `2d4` (166), `1d10` (139), `2d6` (135), `1d8` (132), `1d3` (126), `1d20` (101),
  `1d2` (69), `3d6` (62) — but the tail is long, and 61 distinct forms is more than a hand-written
  enumeration wants to be.
- **383 notations carry a modifier** (`+N` / `-N`).
- **133 occurrences of `drop` / `reroll` / `arrange`** — the semantics §7.2 demands, and the reason
  this is not just arithmetic.

## What has to be decided

1. **Notation.** Whether a dice expression is a **string** (`"4d6 drop lowest"`) or **structure**
   (`{count, sides, modifier, drop, ...}`). Note ticket 06 chose structure for predicates precisely
   because it dissolved the second-parser trap — but dice notation is a genuine, universally
   understood mini-language with an existing convention, which predicates never were. **The argument
   that settled predicates does not automatically transfer.**
2. **Which semantics are in the vocabulary.** Drop lowest / drop highest, reroll on a condition,
   arrange to taste, take best of N. And which are named generation methods rather than notation.
3. **The line between notation and generation method.** §3.1 makes *generation method* a pack kind of
   its own. "4d6 drop lowest" is notation; "roll six times, arrange to taste, reroll if the total is
   below 60" is a method. Where exactly that line falls determines what the notation has to express.
4. **Whether rolled results are ever re-derived.** `spec.md` §6.3 says hit points are **recorded
   randomness** — neither choice nor derivation. So a dice expression is evaluated **once** and the
   result stored, which is a different contract from a predicate that is re-evaluated on every read.
   Say so explicitly; it decides whether the evaluator must be reproducible at all.

## Rounding — §7.2's other hard requirement, and it has no obvious home

§7.2 demands **rounding semantics written down with worked examples**, because PCGen rounded per tag
rather than globally and 2e is full of `/2`. Ticket 06 removed arithmetic from predicates entirely,
so rounding no longer arises there. It arises here, and in the Engine's own computation rules
(§6.2 — XP split evenly, hit points averaged across hit dice).

**Decide here whether rounding is this ticket's problem or the Engine's**, and if it is the Engine's,
record that §7.2's requirement has moved rather than letting it fall between the two.
