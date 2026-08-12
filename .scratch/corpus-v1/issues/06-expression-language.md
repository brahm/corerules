# The expression language: grammar, dice, rounding

Type: grilling
Status: resolved
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

## Answer

**The ticket split, as it suspected it would.** Dice notation and generation methods moved to
[ticket 15](./15-dice-and-generation-methods.md); this ticket kept the predicate language.

### The measurement that reframed it

The ticket assumed the grammar had to *cover what the books say*. Measured over the 13 v1 books, what
the books say is **English**:

| | |
|---|---|
| 2,146 dice notations, **61 distinct forms** | + 383 with modifiers, 133 `drop`/`reroll`/`arrange` |
| 554 ability-with-number references | 386 "at Nth level", 61 "every N levels" |
| 595 "or more/better/higher" | 336 "no more than/less than" |
| 968 "must be" | **1,478 "may not"/"cannot"** |

**The corpus contains no expressions.** Every predicate is written by hand by the transcriber. So the
design target is not source coverage — it is **hand-writability at volume, and checkability**.

### Decision 1 — a closed, flat predicate vocabulary

Rejected: a general language with arithmetic, functions and parenthesised nesting.

Four independent arguments converge:

- **§4.3 already set this discipline one level up** — six closed operations, with *anything outside
  the vocabulary carried as text and not computed*. A closed predicate vocabulary is the same rule
  one level down, not a new invention.
- **A flat vocabulary is decoder-enforceable and a recursive one is not**
  ([ticket 04](./04-llm-assisted-extraction.md)). Parentheses are what make it recursive.
- **It dissolves §7.2's identifier requirement rather than satisfying it.** There is no formula
  string to substitute text into. A predicate's subject is a typed field, so PCGen's
  `Illumination`-contains-`MIN` failure is not a bug to avoid — it is a category of bug that cannot
  occur.
- **Dice is a different thing**: 61 distinct notations with drop and arrange semantics, and §3.3
  already lists *dice expression* as a **value type**, not a predicate. Carrying both in one grammar
  is what made this ticket too big.

Accepted cost, and it is the project's standing posture: some rule will not fit and becomes displayed
text rather than computed rule — as with effects outside the six operations and magic items whose
mechanics do not fit.

### Decision 2 — conjunction only; no boolean combination

A predicate is a **flat list of conditions, all of which must hold**, plus set membership. No `or`,
no nesting.

**Measured rather than argued: genuine disjunction between different subjects occurs 4 times in
1.23M words.** What looked like it needed `or` is covered by something else — the 90 race lists
("X, Y, or Z") are **set membership**, and the 803 `unless`/`except` occurrences are §4.3's `except`
operation, which already exists and already pierces a prohibition by naming its subject.

An `or` connective would cost grammar, cost decoder-enforceability, and force a precedence decision —
for four cases. Those four are handled the standing way: carried as text, or split into two records.

### Decision 3 — subjects derive from §3.1's kinds; scalars are the closed part

Rejected: a separate closed enumeration of subjects, which would duplicate the kind list and be able
to drift from it.

- **`has(<kind>, <id>)`** for anything carrying identity. §2's *closed kinds, open enumerations*
  already governs this, so a kind added in v2 gains predicate reach for free and the language is
  never touched.
- **A small closed list of scalars** — ability and level. The measurement says almost nothing else
  appears in requirement position.

**`level` is always qualified by class, and the unqualified form does not exist in the language.**
§6.1 made the class arrangement a sum type: `Fighter 5 / Mage 4` has no "level 5". A kit requiring
5th level requires it *of which class* — and dual-class freezes the original. Left implicit this
produces predicates transcribed with one meaning and evaluated with another, thousands of times. The
cost is verbosity on every level predicate; what it buys is that the question is never open.

### Decision 4 — predicates are structure, not strings

**This contradicts `spec.md` §7.1**, which says *"expressions stay strings the single evaluator
interprets"*. Recorded as a deliberate departure, not an oversight.

**It dissolves this ticket's own trap instead of avoiding it.** The trap was that the pipeline's
validator becomes a second implementation — and PCGen's disease was three live parsers plus
`processBrokenParser`. **Under structure there is no parser at all**, so there cannot be a second
one.

Two further gains: predicate correctness moves from the validator up into the schema, since a string
is opaque to JSON Schema while a flat object is fully validatable (this is
[ticket 05](./05-pack-schema.md)'s two-tier split, with predicates moving to tier one); and **git can
diff a predicate**, which was ticket 02's argument for the whole corpus and does not work on an
opaque string.

Accepted cost: verbosity, thousands of times, in a text editor.

**The defence of departing from §7.1:** it decided correctly for the language it assumed. Strings
make sense when the language is general and nested. Decision 1 changed the language, and §7.1's
conclusion did not survive the change of premise.

**No separate version marker.** §7.3 already versions the pack format and the language is part of it;
a second number is one more thing to desynchronise.

## The trap

**A second evaluator is how PCGen failed** — three live parsers and a fallback method literally named
`processBrokenParser`, with the same character evaluating to 15 or 12 depending on which succeeded.
This map creates the first *other* implementation of the language, in the pipeline's validator. That
is precisely the moment the disease starts. Whatever is decided here must make one implementation
authoritative and the other derived from it, not written twice from a description.
