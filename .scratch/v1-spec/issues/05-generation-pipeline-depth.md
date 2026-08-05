# Depth of the character generation pipeline

Type: grilling
Status: resolved
Blocked by: —

## Question

How much of 2e character generation does v1 cover?

The pipeline, roughly in book order:

- ability score generation — the PHB's several methods, including arranging rolls to taste
- race, with its ability adjustments, minimums/maximums and level limits
- class — and here the two hard cases: **multi-class** (simultaneous, XP split, demihuman-only)
  and **dual-class** (sequential, human-only, with the old class suppressed). They behave
  nothing alike and neither behaves like a single class.
- alignment
- weapon and non-weapon proficiencies, with slot budgets that differ by class group
- kits, which layer on top of all of the above
- spell selection for casters
- starting money, equipment, encumbrance
- derived values: THAC0, saving throws, armour class

Which of these does v1 own, and which can the spec defer?

And the adjacent question that needs its own answer: **does v1 stop at 1st-level creation, or
does it include advancement** — awarding XP, levelling, re-deriving everything that changes?
"Create and save characters" suggests creation only, but a character sheet nobody can level is
a character sheet nobody uses twice.

## Answer

**v1 owns the whole pipeline.** That is a large v1, and it was arrived at deliberately rather than
by drift — but it should be read as the headline, because every downstream ticket inherits it.

### Most of the pipeline was already forced

Not by this ticket: by [ticket 03](./03-which-complete-handbooks.md), which put kits in v1. A 2e
kit modifies allowed races, ability minimums, granted and forbidden proficiencies, starting
equipment and money, and sphere access. So **proficiencies with slot budgets, equipment, starting
money and spell selection were never open questions** — they are preconditions of having kits at
all. Race, alignment and derived values (THAC0, saves, AC, encumbrance) were in from the project's
premise. Only three things were genuinely undecided.

### Decision — advancement is in

v1 awards XP, levels characters, and re-derives everything that changes. Rejected: stopping at
1st-level creation.

The reasoning that made this cheaper than it looks: **the expensive machinery is mandatory at 1st
level anyway.** A level-1 character's THAC0, saving throws and proficiency budget already come out
of level-indexed tables, so the tables ship regardless; and ticket 01 already recommended storing
choices rather than derived values, recomputing on read. Given both, advancement is largely "change
the level and re-derive".

**The honest cost, and it lands on [ticket 07](./07-character-file-format-and-identity.md):
advancement introduces history.** A 5th-level character made choices *at each level* — hit point
rolls, proficiency picks, spells learned. It stops being a snapshot and becomes a sequence of
level events. That is new structure, and ticket 04's hard validation sharpens it further, since a
choice's legality may depend on the level at which it was taken.

### Decision — multi-class and dual-class are both in

Rejected: single-class only, and the tempting middle of multi-class now / dual-class later.

- **Multi-class is not an edge case in 2e** — it is how demihumans are played. A dwarf
  fighter/thief is the edition's bread and butter, and ticket 03 already committed to the racial
  handbooks, which are entirely about demihumans.
- **Splitting them would be a retrofit.** They live in the same "how many classes does this
  character hold" model, and bolting dual-class into a model designed without it is precisely the
  failure ticket 03's reasoning exists to avoid.

They behave nothing alike: multi-class is simultaneous, demihuman-only, XP split evenly, best value
from each table; dual-class is sequential, human-only, with the earlier class **suppressed** until
the new one exceeds it — a rule with state over the character's history, not over its sheet.

**This is the second place on the map with no prior art at all.** Ticket 01 searched and found
none: PCGen's multiclassing is d20's, and the Roll20 sheet resolves multi-class only for saving
throws and does nothing about XP. Split out as
[ticket 14](./14-multi-class-and-dual-class-model.md).

### Decision — the tool rolls dice (ii)

Entry stays a first-class path — the group rolls physical dice at the table — but the tool can also
roll: ability score methods, hit points on level-up, starting money.

**This is a real constraint on [ticket 06](./06-content-pack-format.md), and it was the argument
against.** A generation method is content: "4d6 drop lowest, arrange to taste" is in the PHB, so it
comes from the pack. If the tool rolls, **the pack's expression language needs dice semantics** —
not just arithmetic, but distribution, dropping, rerolling and arrangement. That lands on the
already most-loaded ticket on the map. Accepted anyway.

### Decision — no automatic character construction. Ever.

Not deferred to v2 — **ruled out permanently**, in the same category as multi-user rather than
alongside settings and psionics. corerules builds characters **step by step**, on the model of
TSR's AD&D Core Rules 2.0 and D&D Beyond.

Rolling dice and generating a character are different in kind, not in effort. Rolling is *rule* —
it is in the books, it comes from the pack, it is objective. Choosing a kit and proficiencies for
the user is *taste*: nothing in the PHB says a 3rd-level thief should take Open Locks before Climb
Walls. That is a preference model, and it exists in none of the source material.

There is also a cost interaction with [ticket 04](./04-validate-or-record.md) worth recording:
under hard validation an auto-generator cannot roll and hope. It would have to **navigate the space
of legal choices** — a constraint solver, not a sampler — because naive sampling produces dead
ends: ability scores no class in the pack accepts, a kit whose prerequisite an earlier choice has
already broken.

The use case argued against this and lost: a DM who needs six guards and a captain for tonight is
not helped much by step-by-step rolling. Recorded so the trade is visible, not hidden.

### Gap found later, and closed elsewhere: magic items

This ticket's pipeline listed starting money, equipment and encumbrance, and never mentioned magic
items. [Ticket 11's](./11-engine-object-kinds.md) enumeration surfaced the omission: with
advancement in v1, characters accumulate items that move AC, THAC0 and saving throws, so leaving
them out would make the computed sheet disagree with the table. Settled there — **in v1, as a
property of the Weapon / Armour / Gear kinds**, with acquisition unvalidated (DM fiat) and use
validated.

### Decision — wizard to build, sheet to correct

This resolves the map's "shape of the creation UI" fog, which was waiting on this ticket. Three
modes, and they are not the same problem:

- **Create** — a guided, step-by-step wizard.
- **Advance** — a mini-wizard carrying only that level's choices.
- **Correct, and edit later** — **direct editing on the sheet.**

Correction had to be sheet-first because ticket 04 made it mandatory: a quarantined character
"must be edited to be corrected", and re-walking a nine-step wizard to fix one mistyped Dexterity
is hostile — worse when the violation sits in a field the wizard only offers at step 3 of 9.

**The binding constraint is validation, not the path: the same rules must hold on both.** Otherwise
direct editing becomes the back door that undoes ticket 04 entirely.
