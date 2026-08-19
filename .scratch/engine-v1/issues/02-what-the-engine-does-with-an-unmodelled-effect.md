# What the Engine does with an UNMODELLED effect

Type: grilling
Status: resolved — the operation decides, not the category

## Question

**380 of the pack's 1,910 effects carry an `UNMODELLED` marker**, and `spec.md` does not contain the
word. The convention was invented mid-transcription by
[corpus ticket 13](../../corpus-v1/issues/13-transcribe-the-proving-slice.md) and never fed back into
the specification, so the Engine has 380 objects it must do *something* with and no instruction.

This is the largest gap between what was specified and what was built.

## What a marker actually is

Not an error, and not missing data. A marked effect is one the transcriber **modelled as far as the
format allowed and then wrote down what was left over**:

```
adjust reactionCheck by -2
  when member {field: "reactor.race"} anyOfIds [phb:halfling]
  text: "UNMODELLED CONDITION: … what remains is 'only those who KNOW what the
         character does for a living' — the other party's knowledge."
```

The number is right. A condition on it is missing. **Applying the effect is wrong and ignoring it is
wrong**, and which is *less* wrong depends on the sign, which no tool can read.

232 of the markers declare their own category — `CONDITION`, `VALUE`, `SUBJECT`, `SHAPE` — a
convention that
[finding 115](../../corpus-v1/issues/13-transcribe-the-proving-slice.md#finding-115--the-markers-were-classifying-themselves-all-along)
found had been carrying more information than anyone realised.

## Why the spec's existing machinery does not cover it

**A3 is the right idea at the wrong grain.** §5.1 says a pack that declares nothing about a rule means
the Engine does not validate that rule *and says so visibly*. That is a **pack-level** statement about
a **rule-set**. A marker is an **effect-level** statement about **one clause of one record**, and
§5.1's mechanism — a declaration in the manifest — cannot express it.

**§5.2 draws the line this ticket lands on.** *A missing validation rule is an unenforced restriction
and the user is told — no harm. A missing computation rule is a wrong number.* Sorted by that line,
the 380 split badly: the largest category is **conditions** (122), which are validation-shaped and
therefore survivable, but **operands and values** (81) are computation-shaped and each one is §5.2's
wrong number.

**And §5.3's quarantine is about the character, not the pack.** It locks a character that is invalid.
A marked effect makes no character invalid; it makes a *number quietly approximate*.

## The options, and why none is obviously right

| | |
|---|---|
| **Apply and annotate** | The number appears, flagged. Honest about provenance, dishonest about arithmetic — an unconditioned bonus applies when it should not. |
| **Withhold and annotate** | The number is absent, flagged. Safe for computation, and it silently under-reports a character who legitimately has the bonus most of the time. |
| **Ask the player** | Turns the marker into a prompt at the point of choice, which is §5's whole posture. Also turns a 45-effect race into an interrogation. |
| **Quarantine by category** | Apply the validation-shaped, withhold the computation-shaped, using the marker's own declared category. Uses information already in the pack — and stakes the Engine's arithmetic on a convention invented in passing. |

## What would settle it

- **A count of what actually reaches a character.** The 380 are spread across records, and a first
  character touches a handful. The decision is easier if the real exposure is six effects rather than
  sixty.
- **A reading of the sign.** For a sample of markers, does applying the effect unconditionally
  overstate or understate the character? If it is overwhelmingly one direction, the choice is not
  symmetric and the table above collapses.
- **Whether the marker's declared category can be trusted as data.** It is prose today. If the Engine
  is going to branch on it, it stops being prose and becomes a field — which is a schema change and a
  correction owed back to the corpus map.

## Why it is worth doing early

The Engine's product promise, in `spec.md` §1, is that **it tells you which rule refused and which
book that rule came from**. The markers *are* that promise's raw material — they are the only place in
the corpus where a refusal is written down with its reason attached. **An Engine that discards them
throws away the feature it was built for.**

---

## The measurement

Three things were named above as what would settle it. All three were measured, and all three
came back differently from the way the ticket framed them.

### The pack now carries 381 markers, not 380

The Navigation repair added one. They are not spread evenly: **369 of 381 sit on an Attachable** —
196 in kits, 173 in deities — against 7 in races, 4 in classes, 1 in a subrace. Every one of the 59
priesthoods carries at least one. 112 of 164 kits do.

### 1. What actually reaches a character

Over all **36,126** (race × class × attachable) combinations the pack can form:

| | markers on the sheet | of which numeric |
|---|---:|---:|
| minimum | 0 | 0 |
| median | 3 | 2 |
| mean | 3.0 | 1.7 |
| maximum | 14 | 12 |

**93.4% of sheets carry at least one.** The ticket asked whether the real exposure was six or sixty;
it is **three**, and the tail reaches fourteen. That number decides more than it looks like it does.
Three is few enough to *print* — a sheet can carry three annotated lines and stay readable — and too
many to *ask about*: three modal dialogs before a character exists is the interrogation the ticket
feared. **The measurement admits annotation and rules out prompting.**

### 2. The sign does not collapse

The ticket hoped the direction would be one-way. It is not:

| adjust/set operand under a marker | |
|---|---:|
| positive | 87 |
| negative | 40 |
| zero | 28 |
| computed / rolled / tabular | 4 |

Applying unconditionally **overstates 87 characters and understates 40**. There is no safe direction
and therefore no honest way to describe the error to a user in one sentence — which is the argument
against "apply and annotate" that the ticket could not make without the count.

And the arithmetic is worse than a bounded error. **43 marked numeric effects have an unmarked sibling
writing the same field**, 36 of them the same shape: a priesthood whose experience bonus is written
*twice*, once as a modelled conjunction and once as a marked unconditional twin.

```
cprh:DD05530  Agriculture
  adjust experienceAward.percent by 5        ← marked: "Wisdom OR Constitution 16"
  adjust experienceAward.percent by 5
    when Wisdom >= 16, Constitution >= 16    ← unmarked, the AND case
```

Under "apply and annotate" a priest with Wisdom 16 and Constitution 16 receives **+10%** where the
book says +5%. Not an approximation — a wrong number, in 36 priesthoods, **manufactured by the marker
convention itself.**

There is a further fact that makes this urgent rather than theoretical: **133 of the 140 marked
`adjust` effects carry no `when` at all.** The pack as it stands already *is* "apply
unconditionally". Leaving this ticket open is not holding the question open; it is answering it.

### 3. The declared category cannot be trusted as data

This is the measurement that reshaped the answer. The ticket, following corpus finding 115, described
a four-value vocabulary — `CONDITION`, `VALUE`, `SUBJECT`, `SHAPE` — over 232 markers. The pack
actually declares **32 distinct labels over 351 markers**:

```
CONDITION 116   SHAPE 86   QUANTITY 59   SUBJECT 16   SCOPE 11   VALUE 8   CAP 6
OPERAND 5   CHOICE 4   REDUCTION 4   PARAMETER 3   EARMARK 3   CONDITIONS 3   FREQUENCY 3
OPTION 2   LINK 2   TRADE 2   PROCEDURE 2   SUBSTITUTION 2   SPLIT 2
CLAMP · PRECEDENCE · ELECTION · ENTIRELY · PROPORTION · SHIFT · TRIGGER · FORFEIT
  · ANTI · TEMPORARY · FRACTION · RELATION            (one occurrence each)
```

Twelve labels are used once. `CONDITION` and `CONDITIONS` are both present, which is the whole
argument in two words: **this is prose that looks like an enum.** Branching the Engine's arithmetic on
it would stake correctness on a spelling.

**And it is unnecessary, because the distinction it was wanted for is already a schema field.** The
category is very nearly a function of the operation:

| | |
|---|---|
| `CONDITION` | 106 of 116 sit on `adjust` |
| `SHAPE` | 81 of 86 sit on `grant` |
| `QUANTITY` | 59 of 59 sit on `grant` |

Sorted by operation instead, the 381 split cleanly and without reading a word of prose:

| | | |
|---|---:|---|
| **structural** | 222 | `grant` 204, `require` 10, `forbid` 5, `except` 3 |
| **numeric** | 159 | `adjust` 140, `set` 19 |

## The decision

**The operation decides, not the category. A marked effect never reaches the total.**

**1 — A marker on a structural operation: apply it, and carry the marker on that entry.**
The *thing* is right and its *edges* are under-described. The Clansdwarf's Dwarf Runes, a priesthood's
sphere access, a warrior's permissions: withholding these removes a capability the book plainly grants
in order to avoid overstating its boundary, which trades a large error for a small one. It is applied,
and the marker text is printed against it as a rider. 120 of the 204 marked grants point at a real
record and 84 define a placeholder inline; both are nameable on a sheet.

**2 — A marker on a numeric operation: withhold it from the total, and show it as a named situational
line.** The *number* is right and the *circumstance* is missing. It is not folded into the sum and it
is not discarded; it is printed beside the total with its condition in the transcriber's words, for
the player to apply when the circumstance holds.

This is the option the ticket's table did not contain, and the measurement is what produced it. "Apply"
corrupts the total in both directions and double-counts 43 times. "Withhold" throws away a bonus the
character legitimately has. **Printing it as a rider does neither** — and a reaction check that reads

```
reactionCheck    +3    with others of his own clan.          (CBD, Clansdwarf)
                 +2    with dwarves of other clans in the same craft.
```

is *more* faithful than either `+5` or `0`, because it is what the book says.

**3 — The declared category is never read by the Engine.** It stays as prose for a human and for the
maps. If it is ever to be branched on it must first become a field, which is a schema change and a
correction owed back to the corpus map — and nothing here needs it.

**4 — A marker naming an obstacle the format has since removed is a defect in the pack, not a case
for the Engine.** See below; this is the largest single result of the ticket.

### Why this is §5.2 confirmed rather than §5.2 overruled

§5.2 draws the line at *a missing validation rule is an unenforced restriction and the user is told —
a missing computation rule is a wrong number*. It named the right line and had no way to find it in a
record. **The operation is where that line lives.** `grant`/`forbid`/`require`/`except` are validation-
shaped by construction; `adjust`/`set` are computation-shaped by construction. The rule above is
§5.2 applied at the grain of an effect, using a discriminator the schema already enforces.

What §5 must gain is the statement that a marked effect is a **third kind of answer** alongside a value
and a refusal: *withheld from the total, shown in full, attributed.* §5.1's A3 is a pack-level
declaration and §5.3's quarantine locks a whole character; neither reaches one clause of one record.

## Applied, and demonstrated

`tools/firstlight.py` now implements the decision. The dwarf Clansdwarf fighter of ticket 04:

```
FIELDS — the total, and nothing in it is approximate
    ...
    savingThrow.vsPoison                        +4
    startingWealth                              5d4x10

GRANTED (4)
    «Warrior permissions»                   phb:warrior[4]
        ⤷ Any weapon and any type of armour ... UNMODELLED SHAPE: 'any weapon' and 'any
          armour' are permit lists, and the restriction on magical items and spells is
          their complement — finding 43's shape.
    Dwarf Runes                             cbd:DD04638[0]

SITUATIONAL — withheld from the total, applied when the circumstance holds (3)
    magicalItem.malfunctionChance +20   phb:dwarf[4]
        ... UNMODELLED SCOPE: the exclusions ... are a list of item CATEGORIES, and the
        pack has no kind that holds one.
    reactionCheck                 +3    cbd:DD04638[4]
        UNMODELLED CONDITION: with others of his own clan.
    reactionCheck                 +2    cbd:DD04638[5]
        UNMODELLED CONDITION: with dwarves of other clans in the same craft.
```

Before the change this sheet reported `reactionCheck 5`, a number no book contains, assembled from two
conditions that hold against different people and can never both hold at once. **The fabricated field
is gone and nothing was lost.**

## Finding — 37 markers name an obstacle that no longer exists

Grouping the 381 by the obstacle their text names:

| | |
|---:|---|
| 60 | a **permit-list** — "Weapons Permitted: …" means *only these*, and no operation says "only" |
| 37 | **cannot express disjunction** |
| 30 | a **second subject** — the other party's race, culture, or knowledge |
| 7 | per-encounter or duration |
| 6 | DM adjudication |
| 5 | negation |

**The 37 are stale.** They say, in the transcriber's own words, *"the predicate is a flat conjunction
and cannot express disjunction, so the condition is carried as text"*. That was true when it was
written and is false now: corpus ticket 13 finding 10 added the `anyOf` clause arm in schema commit
`b12e851`, and **nobody went back**. Every one of the 37 is expressible today as
`{anyOf: [{Wisdom ≥ 16}, {Constitution ≥ 16}]}`, and 36 of them are half of a double-count.

This is the pattern to watch rather than the instance. **A marker is a note about the format written
at a moment, and the format kept moving.** Nothing in the pack ties a marker to the schema version
that provoked it, so a marker cannot expire and no tool can tell a live one from a dead one. The
`anyOf` arm landed and 37 notes quietly became wrong.

That the Engine's own decision above **contains** this — a stale marker withholds a number that could
now be computed exactly — is the reason it is a defect and not a case. Rule 2 makes the sheet honest
about the 37; it does not make it right. Repairing the pack does.

Owed back to the closed corpus map as **correction 45**: re-express the 37 disjunction markers with
the `anyOf` clause arm and delete the doubled siblings. Closing that map did not make its pack
immutable, and this is the first correction the Engine has sent back.

## What this ticket does not decide

- **Whether the 60 permit-list markers deserve a seventh operation.** They are the largest cluster and
  they fail in one direction — the character may take weapons the book forbids. Rule 1 applies them
  with the rider, which is honest and is not enforcement. That is a v1 scope question, not a marker
  question.
- **Ticket 03 is untouched.** Contradiction between two Attachables is a different problem: two
  effects that are each fully modelled and disagree. No marker is involved and the rule above does not
  reach it.

---

## Postscript — correction 45 applied, and the convention became a field

This ticket branched on `"UNMODELLED" in text`. **That was never a classifier**, and applying
correction 45 proved it by breaking on the note that explained the repair: prose *about* a marker
read as a marker, and the Engine withheld a number it had just been taught to compute. The same
false positive had been introduced once before, by the corpus effort, also while repairing markers.

**`unmodelled` is now a field on the effect** — 286 of them — and the checker reports a text and a
field that have parted company. Nothing else in this ticket changes: the declared **category** stays
prose and the Engine still branches on the **operation**, exactly as decided. The field says only
*whether* an effect is marked, which is the one thing the Engine has to know to keep it out of a
total.

The convention had already drifted without anyone noticing. Six markers are punctuated differently
from the other 280 — `UNMODELLED CAP only:`, `UNMODELLED as a cap:`, `UNMODELLED CONDITIONS, as for
the elf:` — so even the tightened regex that made the migration possible was a one-time instrument,
not a rule.

**Markers: 381 when this ticket measured them, 323 after correction 46 turned the permit-lists into
bounds, 286 after correction 45 repaired the doubled experience effects.**

### And a fourth reason a value is withheld

Correction 57 added one. A value now stays off the sheet when

| | |
|---|---|
| a marked effect supplies it | this ticket |
| its predicate asks something the sheet cannot answer | first light's `when() → None` |
| two layers set it and nothing declares a winner | [ticket 03](./03-precedence-when-two-attachables-contradict.md) |
| **the book marks the rule optional and no table has decided** | correction 57 |

The fourth is different from the other three in a way worth noticing: **nothing is missing.** The
effect is fully modelled, the predicate is decidable, no layer disagrees. What is absent is a
decision that belongs to neither the pack nor the Engine — *"the DM may decide"* — and the sheet says
so rather than choosing on the table's behalf.
