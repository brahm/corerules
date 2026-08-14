# Precedence when two Attachables contradict

Type: grilling
Status: open

## Question

**The layer model's central guarantee does not cover the case the books actually produce.**

§4.3's six operations commute — that is what makes a character's view of a value a stack of layers
applied in any order. Commutation says nothing about two layers that are *inconsistent*, and the
Complete Book of Dwarves produces exactly that:

- Three CBD kits take their permitted weapons **from the character's Deity** — the Temple Guard's must
  be *"representative of his religion"*, the Pariah may use *"any weapon not forbidden by his
  religion"*. The **Kit** arm and the **Deity** arm speak about the same field.
- The **Vindicator** must be proficient in battle axe and warhammer *"regardless of the restrictions
  imposed by their religion."*

A `forbid` from the Deity and a `require` from the Kit are not order-dependent. They are simply
contradictory, **and the book states which one wins.**

This is [corpus finding 90](../../corpus-v1/issues/13-transcribe-the-proving-slice.md#finding-90--a-kit-that-overrules-a-deity-with-the-book-stating-the-precedence),
named at session 33 of that effort and untouched at session 71. Transcribing a thousand more records
never brought it closer, **because it is a hole in the arms' composition rather than in their shape**
— which is also why it lands here rather than there. The Engine is the first thing that has to
actually resolve it.

## Why it cannot be deferred to "the evaluator will figure it out"

The evaluator is where the contradiction *materialises*. Loading the Vindicator produces a set of
weapon proficiencies that both must and must not contain a battle axe, and the program has to emit
something. Silence is a decision made by accident.

And it is not a rare shape. Three of 24 CBD kits read a field the Deity also writes; the Complete
Priest's 59 priesthoods all write weapon permissions; **any dwarf priest with a kit is a candidate.**

## The options

| | |
|---|---|
| **Arm precedence** | A fixed order — Kit beats Deity beats Subrace, say. Simple, total, and **invents a rule no book states**; it happens to match the Vindicator and would silently invert a book that said the opposite. |
| **Declared precedence in the record** | The Vindicator's *"regardless of the restrictions imposed by their religion"* becomes a field on the effect. Faithful to the source, and it is one more thing every transcriber must notice — and the corpus map's evidence is that transcribers miss what nothing checks. |
| **Refuse and tell** | The Engine detects the contradiction, applies neither, and reports it with both books named. Consistent with §5's posture and with the product promise; leaves the player with a character that cannot take a weapon the book plainly grants. |
| **Specificity wins** | The more specific layer overrides the more general — a rule about *battle axes* beats a rule about *weapons of my religion*. Attractive, and it requires the Engine to compare the generality of two field paths, which is not something field paths support. |

## What would settle it

- **The full extent, measured.** How many (record, record) pairs in the current pack write the same
  field with contradictory operations? The corpus map found one by reading; a program can find all of
  them, and the answer changes which option is affordable.
- **What the other eleven books do.** The CBD states its precedence in prose. If no other book ever
  states one, "declared precedence" is a mechanism with a single user. If several do and phrase it
  differently, that is the strongest possible argument for the field.
- **A decision on whether the Engine may ever refuse to compute.** §5.3 quarantines a *character*;
  nothing yet says a single *value* may come back as "two books disagree". That may be the honest
  answer, and it is a product decision as much as a technical one.

## Note on scope

This ticket decides **how contradiction resolves**, not how the evaluator is structured. If the answer
is a schema change, it is a correction owed back to the corpus map — which is now closed, and closing
it did not make its schema immutable.
