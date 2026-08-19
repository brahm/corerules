# Precedence when two Attachables contradict

Type: grilling
Status: resolved — precedence is declared or it does not exist

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

---

## The measurement

All three settling questions were run down. The first two came back in a way that dissolves the
ticket's framing, and the third turns out to have been answered by ticket 02 a day earlier.

### The extent, measured — and it is zero

The ticket's premise was that the contradiction is common: *"three of 24 CBD kits read a field the
Deity also writes … any dwarf priest with a kit is a candidate."* Counting the operations by kind
shows why no such pair exists:

| kind | `grant` | `forbid` | `except` | `require` | `adjust` | `set` |
|---|---:|---:|---:|---:|---:|---:|
| kits | 466 | **96** | **13** | **120** | 207 | 72 |
| deities | 690 | **0** | **0** | **0** | 93 | 1 |
| races | 2 | 0 | 0 | 0 | 26 | 68 |
| subraces | 11 | 0 | 0 | **1** | 19 | 14 |
| classes | 8 | 0 | 0 | 0 | 0 | 12 |

**Every restricting operation in the pack lives in a kit.** All 96 `forbid`s, all 13 `except`s, 120 of
the 121 `require`s. A Deity has 690 grants and cannot say *no* about anything — because the one thing
a deity restricts, its permit-list, was transcribed as marked grants and never as a prohibition
([ticket 02](./02-what-the-engine-does-with-an-unmodelled-effect.md)'s largest cluster, 60 of them).

Two kits never contradict either, and not by luck: kits are `one-per-target`, so two can only meet on
a multi-classed character. **2,969 kit pairs can sit on one character. None contradicts on a shared
ref.** Ignoring co-occurrence entirely — the loosest possible scan — 54 pairs grant what another
forbids, and **every one of the 54 is a pair that cannot meet.**

**The contradiction this ticket was opened for does not exist in the pack, and it is not an accident
of which books were transcribed. Only one arm out of five ever speaks in a restricting voice.**

### What does contradict, and it is not an Attachable

Two layers `set` the same field with different values in 404 record pairs. Filtered to pairs that can
actually sit on one sheet, **four** remain — and they are a different animal:

```
proficiency.navigation.modifier   phb:dwarf  0   /  phb:dwarf  -3      ← same record
infravision.range                 phb:elf   60   /  Aquatic Elf 360
infravision.range                 phb:elf   60   /  Drow         90
detect.directionUnderground       phb:halfling 3- on 1d6 / Stout 50
```

The first is **an artefact of the scan, and the most useful thing in it**: those two effects are
correction 41's group-conditioned Navigation scores, which never both fire. **Contradiction can only
be detected after predicates are evaluated, over the effects that actually fired** — a scan over the
text finds contradictions that do not exist.

The other three are a **race and its subrace**, and they are not defects. A subrace refining its race
is what a subrace is *for*. Which is the whole answer: the layer that wins is the one whose record
**declares** it, and the declaration is already there — `cbe:DD04777#2` (Drow) carries
`target: phb:elf`.

### What the other eleven books do

Every phrasing that could state a precedence, across all twelve books and the DMG — some 4,500 pages:

| | |
|---:|---|
| 17 | "overrides" / "takes precedence" / "supersedes" |
| 16 | "exception to this/the" |
| 4 | "regardless of the restrictions…" |
| 4 | "notwithstanding" |

Read one by one, **fourteen of the seventeen are DM discretion or narrative hierarchy** — the DM may
override a die roll, a deity's command outranks a government's edict — and none is one layer of a
character sheet outranking another. **Three statements in the whole corpus actually rank two
mechanical restrictions**, and they are decisive because *they do not agree on a direction*:

- **CBD, the Vindicator** — must be proficient in battle axe and warhammer *"regardless of the
  restrictions imposed by their religion."* Kit over mythos.
- **PHB, multi-classing** — *"Regardless of his other classes, a multi-classed priest must abide by
  the weapon restrictions of his mythos."* Mythos over class.
- **DMG, scrolls** — characters able to employ scroll spells may do so *"regardless of other
  restrictions."*

**A fixed arm order is refuted, not merely unattractive.** Any ordering that satisfies the Vindicator
gets the PHB's multi-class priest wrong. And "specificity wins" is unsupported: neither statement is
an appeal to specificity, both are explicit exceptions written at the site of the exception.

### The operation for this already exists, and it is in use

`except` takes a `ref` to a **named restriction**, and the pack has seven such `limitation` records —
every one of them pierced by at least one kit:

```
phb:thief-weapon-restriction              pierced by 4 kits (Assassin, Bounty Hunter, Spy, Thug)
phb:wizard-weapon-restriction             pierced by 2 (Amazon Sorceress, Militant Wizard)
phb:bard-armor-restriction                phb:thief-armor-restriction
phb:multiclass-specialization-restriction phb:two-weapon-penalty       phb:holy-symbol
```

The Assassin does not out-rank the thief rules by being a kit. **It names the rule it pierces**, which
is exactly how the book writes it. §4.3 already had the mechanism the ticket was proposing to invent.

## The decision

**Precedence is never inferred. It is declared by one record about another, or it does not exist —
and where it does not exist the Engine refuses the value rather than picking one.**

**1 — No arm precedence, and §4.3 gains no ordering.** Refuted by the corpus, not deferred for want of
evidence. Three precedence statements in twelve books point in two directions; any fixed order
contradicts the Player's Handbook or the Complete Book of Dwarves.

**2 — Two declarations already exist and no third is needed.**

| declaration | says | mechanism | uses |
|---|---|---|---|
| `target` | *this record refines that one* | subrace names its race, kit names its class | every attachable |
| `except` | *this record pierces that rule* | names a `limitation` record | 13 |

Where a layer collision is covered by one of these, the declaring layer wins **and the Engine can say
why, naming the record that claimed the right.**

**3 — Where neither covers it, the value is withheld and both books are named.** Not the character —
**the value.** §5.3 quarantines a character that is invalid; this quarantines one number that two
books disagree about, and the sheet around it is computed normally. This is the ticket's third
settling question answered: **yes, the Engine may refuse to compute — at the grain of a value.**

That is not a new mechanism. It is the **third user of the channel ticket 02 built** the day before:

| a value is withheld because | |
|---|---|
| a marked effect supplies it | ticket 02 |
| its `when` asks something the sheet cannot answer | first light's `when() → None` |
| two layers set it and nothing declares a winner | this ticket |

One posture, three causes, and in every one the sheet says which record and which book.

**4 — Contradiction is detected after predicates, never over the text.** The Navigation record shows
why: a scan over effects reports a record contradicting itself when the two effects can never both
fire.

## What the ticket was actually about

Strip the three CBD kits down and only one of them is about precedence at all:

| | what it needs |
|---|---|
| **Temple Guard** | `require weaponProficiency count 1` **with no `from`** — the list is the deity's. Not a contradiction: **an operand that lives in another layer.** |
| **Pariah** | `forbid weapon`, subject defined inline as *"weapons forbidden by the character's religion"* — the complement of another layer's list. Same thing. |
| **Vindicator** | `require [battle-axe, warhammer]` marked `UNMODELLED PRECEDENCE`. The only real one. |

And the Vindicator's marker says *"nothing records the ranking."* It is nearer the truth to say
**nothing records the thing being outranked.** There is no `phb:priest-weapon-restriction`; the
priestly weapon limit exists in the pack only as 60 marked grants. **The Vindicator has no rule to
name, because the losing side was never written down.**

The same hole shows up twice more, small: two `except`s point at the **thing permitted** instead of the
**rule pierced** — the Imagemaker's `phb:ventriloquism`, the Treetender's `phb:bow` — for the same
reason. The record they should have named does not exist.

**So the gap is not an operation and not a precedence rule. It is a missing `limitation` record, and
the machinery to use it has been in the schema since it was written.**

## Applied, and demonstrated

`tools/firstlight.py` now takes a `--subrace`, ranks colliding `set` layers by declared refinement,
and refuses the rest. The Drow, whose record declares `target: phb:elf`:

```
CHARACTER: Elf / Drow / Thief, level 1
    infravision.range                           90        ← the Drow's, not the Elf's 60
```

And with a synthetic kit made to set the same field — a kit targets a *class*, so neither record
declares anything about the other:

```
CONTESTED — two layers set this and nothing declares a winner (1)
    infravision.range
        60      Elf         Player's Handbook           phb:elf[9]
        15      Votary      Comp. Paladin's Handbook    synthetic:contradiction[0]
```

**That is `spec.md` §1's product promise reached at last**: not a number, but which rule and which book.

Swept over **14,910 sheets** — every (race × subrace × class × kit × deity) the pack can form:
**0 contested values, and 2,130 `set` collisions every one of which a declared refinement resolved.**
The rule costs nothing today and is the only thing standing between the Engine and a silent wrong
answer when the permit-lists are expressed.

## Findings, owed back to the corpus map

**Correction 46 — the priest weapon restriction has no record, and three kits and sixty priesthoods
need it.** `limitations` holds seven PHB restrictions and every one is pierced by a kit; the priestly
weapon limit is not among them, so the Vindicator's declared precedence has nothing to name, the
Temple Guard's requirement has no list, the Pariah's prohibition has no subject, and two `except`s
name the wrong thing. Writing it is not new machinery — it is one record per priesthood plus the PHB's
own, and it converts ticket 02's largest marker cluster from prose into an operation.

**Correction 47 — `set` cannot tell agreement from disagreement, because the operand carries the
scale.** The PHB gives the halfling's underground direction sense as `3- on 1d6`; the Complete Book of
Gnomes and Halflings gives the Stout's as `50`. **These are the same probability.** The pack records a
collision, the Engine resolves it by refinement and prints 50, and nothing anywhere notices that the
two books agree. A roll-under operand and a percentage operand are not comparable, and there are 17
roll-under operands in the pack against a great many percentages.

## What this closes and what it leaves

**§13's known unknown #1 is answered** — the one thing 71 sessions of transcription never closed.
Not by finding the precedence rule, but by measuring that there is not one to find: the books state
exceptions, the schema already expresses them, and precedence between *arms* is a category the corpus
does not contain.

Left open: **cross-layer parameterisation** — an effect whose operand is another layer's contents
(the Temple Guard's list, the Pariah's complement, the 60 permit-lists). It is a real gap in §4.3, it
is 62 effects wide against this ticket's one, and it is **not** what this ticket decided. It wants a
ticket of its own.
