# The expression language: grammar, dice, rounding

Type: grilling
Status: open
Blocked by: 01

## Question

`spec.md` §7.2 mandates a rules language and states four hard requirements for it, all of them
lessons from PCGen's failures:

- **One evaluator, versioned explicitly, with no fallback path ever.**
- **Rounding semantics written down with worked examples** — PCGen rounds per tag rather than
  globally, and 2e is full of `/2`.
- **Identifiers lexically distinguishable from operators.** Never substitute text into a formula
  string; never case-fold. A PCGen type named `Illumination` breaks because it contains `MIN`.
- **Dice semantics, not just arithmetic** — distribution, dropping, rerolling, arrangement, because
  §9.1 has the tool roll dice and generation methods come from the pack.

What it does not do is define the language. **And transcribing a kit means writing in it.** Every
prerequisite predicate, every conditioned effect, every generation method in the corpus is an
expression, so this cannot wait for the Engine.

## What has to be decided

1. **The grammar.** Operators, precedence, literals, function calls if any. Small enough to
   implement twice without divergence — the pipeline validates expressions, the Engine evaluates
   them, and §7.2 forbids the two drifting.
2. **How identifiers are written**, satisfying the lexical-distinguishability requirement. This is
   where PCGen actually died; treat it as the primary constraint rather than a detail.
3. **Dice notation and its semantics.** `4d6 drop lowest arrange to taste` is PHB content that must
   round-trip. Decide what is grammar and what is a named generation method.
4. **Rounding, with the worked examples §7.2 demands.** Not "round down" — *where* in the evaluation
   order, and what happens to a chain of divisions.
5. **What predicates can refer to.** Ability scores, class, level, race, alignment, another kit's
   presence, the active pack set. This is the surface between the language and §3's kinds, and it
   determines whether a prerequisite can be checked at all.
6. **The version marker.** §7.2 says versioned explicitly. Decide whether it lives on the pack, on
   the expression, or both.

## Why this is blocked by ticket 01 and not by 05

The grammar has to cover **what the books actually say**, and ticket 01 is what collects that. A
language designed from the spec's four bullets would be an invention; a language designed from ~100
kits' worth of real prerequisites and effects is a reading.

It is deliberately **not** blocked by [ticket 05](./05-pack-schema.md): the schema carries
expressions as opaque strings (§7.1 — *"expressions stay strings the single evaluator interprets"*),
so the two can be designed in parallel. If that turns out to be false, say so — it would mean the
schema and the language are one ticket, not two.

## A constraint from ticket 04, found there rather than here

**Recursive schemas are unsupported by structured outputs.** If this language is recursive — and
parenthesised sub-expressions make it so — a constrained decoder **cannot enforce it at all**.
Numeric ranges and string lengths are likewise unenforceable and fall to the validator.

That does not forbid a recursive grammar; it decides *who checks it*. If any part of the pipeline
generates expressions with a model, the grammar's correctness is the validator's job end to end,
never the decoder's. Weigh that when choosing between a recursive grammar and a flat one.

## This ticket may be too big

Flagged at charting. It plausibly splits into **grammar and evaluation semantics** on one side and
**dice and generation methods** on the other. Split it if the session finds that shape; do not force
one answer to cover both if they are not one decision.

## The trap

**A second evaluator is how PCGen failed** — three live parsers and a fallback method literally named
`processBrokenParser`, with the same character evaluating to 15 or 12 depending on which succeeded.
This map creates the first *other* implementation of the language, in the pipeline's validator. That
is precisely the moment the disease starts. Whatever is decided here must make one implementation
authoritative and the other derived from it, not written twice from a description.
