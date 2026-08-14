# What the Engine does with an UNMODELLED effect

Type: grilling
Status: open

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
