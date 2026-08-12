# Dice notation and generation methods

Type: grilling
Status: resolved
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

## Answer

### The measurement reframed this ticket exactly as it reframed ticket 06

The dice semantics §7.2 demands turn out to be almost absent, and the 133 `drop`/`reroll`/`arrange`
occurrences counted while resolving ticket 06 were **mostly ordinary English** — "drop of", "arrange
for". Re-measured properly:

- **8 occurrences** in the whole v1 tier of `NdM` followed by *drop / keep / best / lowest* within 40
  characters.
- **1,474 of 2,146 dice notations sit in tab-delimited lines** — dice are overwhelmingly **table cell
  values**, not expressions in prose.
- The PHB defines **exactly six generation methods**, `Method I` through `Method VI`, each a prose
  procedure.

### Decision 1 — dice notation is a string with a validated pattern

Form: `"3d6"`, `"1d4+1"` — the grammar is `NdM±k`.

[Ticket 06](./06-expression-language.md) chose *structure* for predicates, and this ticket warned when
it was written that **that argument does not automatically transfer**. It does not, and the reason is
factual: what killed strings for predicates was **opacity to the schema** — a general predicate is
too complex for a regex, so a string would sit entirely in the second enforcement tier. **Dice are
not like that.** `NdM±k` is regular, expressible as a JSON Schema `pattern`, and therefore validated
in full at tier one. The objection does not apply.

Everything else points the same way: a forty-year-old convention every reader knows, 2,146
occurrences where verbosity costs, and a divergence risk between two implementations of a regex that
is effectively nil — nothing like the parser that killed PCGen.

### Decision 2 — the notation/method line is drawn by measurement, not judgement

The ticket's item 3 asked where notation ends and generation method begins. It does not need a
judgement call: **notation is `NdM±k`, with eight exceptions in the entire corpus**, and
**generation methods are the PHB's six**, which §3.1 already makes a pack kind of its own, each
carrying its own procedure.

This also empties item 2. The vocabulary needs count, sides and modifier — nothing else.
`drop lowest` is not notation; it is Method V.

**Accepted cost:** those eight exceptions. Under this decision a cell reading something like
"1d6, rerolling 1s" does not fit the notation and becomes either displayed text — the project's
standing posture — or a named generation method. Eight times, and they are identifiable.

### Decision 3 — rounding lives on a closed set of computed operands, and this fires known unknown #4

**The easy answer died on measurement.** Rounding was expected to belong to the Engine's §6.2
computation rules, with this ticket merely recording that §7.2's requirement had moved. But division
appears **in pack data**: measured inside kit effect fields across the eight RTF kit books,
**31 occurrences** of halving, division or explicit rounding — *"at twice the normal cost"*,
*"experience level divided by three (rounded down), plus one"*, *"at half the price"*, *"creatures
whose Hit Dice total no greater than half her level"*.

**None of these fits §4.3's six operations.** `adjust` sums; there is no scale or divide. And the
last example is worse: a **predicate carrying arithmetic**, against
[ticket 06](./06-expression-language.md)'s decision that predicates have none.

**This is v1 spec known unknown #4 firing** — the spec said the six-operation vocabulary might prove
insufficient for some PHBR kit and that this was checkable before code was written, because the books
are in hand. It is now checked. It is also the first thing this map has found that contradicts a
**closed** decision rather than a premise.

**The decision: a small closed set of computed operands** — `half(<scalar>)`,
`<scalar>/N rounded down` — enumerated, not general arithmetic. This is the same discipline as ticket
06's closed predicate vocabulary, applied one level down.

Rejected: **carrying them as text**, the standing posture. 31 occurrences means roughly **one kit in
five carries a rule that would not be enforced**, and under §5's hard validation that is silent
under-enforcement — the very thing A3 exists to make visible. Not comparable to the four disjunctions
ticket 06 discarded.

Rejected: **a seventh operation.** A closed operand is not one. `adjust` still sums; what changes is
that its *operand* may be `half(level)` rather than only a literal, so §4.3's count of six and its
order-independence both survive.

**And rounding finally has a home:** it is a property of the computed operand —
`half(level, round: down)` — which satisfies §7.2 in the place the division that causes it actually
lives.

**Recorded as a correction owed to the v1 spec** (the map's fifth), because §4.3 was a closed
decision and this widens what its operands may be. [Ticket 14](./14-record-shapes-for-the-slice.md)
must express it and [ticket 13](./13-transcribe-the-proving-slice.md) must report how many of the
slice's effects need it.

## Rounding — §7.2's other hard requirement, and it has no obvious home

§7.2 demands **rounding semantics written down with worked examples**, because PCGen rounded per tag
rather than globally and 2e is full of `/2`. Ticket 06 removed arithmetic from predicates entirely,
so rounding no longer arises there. It arises here, and in the Engine's own computation rules
(§6.2 — XP split evenly, hit points averaged across hit dice).

**Decide here whether rounding is this ticket's problem or the Engine's**, and if it is the Engine's,
record that §7.2's requirement has moved rather than letting it fall between the two.
