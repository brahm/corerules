# How multi-class and dual-class are modelled

Type: grilling
Status: resolved
Blocked by: —

## Question

Split out of [ticket 05](./05-generation-pipeline-depth.md), which put both in v1. This is the
**second** place on the map with no prior art whatsoever — the first being
[ticket 10](./10-kit-modifies-parent-class.md). [Ticket 01's research](../research/01-prior-art-2e-content-modelling.md)
searched and found nothing: PCGen's multiclassing is d20's (levels in several classes, an XP
penalty tag), its `EXCHANGELEVEL` is mechanically adjacent to dual-classing but was built for
monster classes with no dataset using it that way, and the Roll20 sheet — the largest open 2e
implementation in existence — resolves multi-class only for saving throws and does nothing about
XP at all.

The two mechanics behave nothing alike:

| | Multi-class | Dual-class |
|---|---|---|
| Timing | simultaneous | sequential |
| Who | demihumans only | humans only |
| XP | split evenly across classes | all to the new class; the old is frozen |
| Old class | fully active | **suppressed** until the new class exceeds its level |
| Hit points | averaged across classes | from the new class only |
| Derived values | best value from each class's table | the new class's, until suppression lifts |

Questions to settle:

- **Is "class" a field on the character, or a collection with rules of its own?** The answer
  decides whether ticket 11 lists one kind or two, and whether the character file holds one class
  reference or an ordered set.
- **Where suppression lives.** Dual-class suppression is state over the character's *history*, not
  over its sheet: it lifts when the new class's level exceeds the old one's. Is that stored, or
  derived from the level record every time? [Ticket 05](./05-generation-pipeline-depth.md) already
  made the character a sequence of level events; this may be the thing that most needs it.
- **How XP division is expressed.** Even splitting across classes is a rule, and under
  [ticket 04](./04-validate-or-record.md) rules come from the pack, not the engine. Does the pack
  express "split XP evenly", or does the engine know 2e's rule natively? This is a live test of
  ticket 03's closed-kinds/open-enumerations line.
- **Best-of-table resolution.** Saving throws take the best value across the character's classes;
  THAC0 and proficiency budgets resolve differently again. Is "how a derived value resolves across
  multiple classes" a property of the value, of the class, or a single engine rule?
- **Making illegal states unrepresentable.** Ticket 04 demands hard blocking. The Roll20 sheet's
  approach is crude but correct for 2e: five fixed class slots whose *index* fixes the group
  (1 warrior, 2 wizard, 3 priest, 4 rogue, 5 psionicist), so two classes from the same group cannot
  be expressed. Worth weighing against a general collection with a validation rule — noting that
  ticket 03 put psionics out of v1, so a fifth slot would sit empty.
- **The seam with kits.** A kit attaches to a class. Which class, when the character holds two?
  Ticket 10 must not assume a single attachment point.

Blocks 07 (a character file cannot be shaped until it is known how a character holds classes) and
11 (whether class is one kind or a collection).

## Answer

### Decision — the class arrangement is a sum type

`Single(class)` | `Multi([class])` with arity ≥2 | `Dual(original, new)`. Each arm carries exactly
the fields it needs and no others.

Rejected: one collection with a discriminator flag, and the Roll20 sheet's five fixed slots indexed
by class group.

The principle that decided it, and it recurs throughout this ticket: **structural for model
incoherence, pack-declared for game rules.** Under [ticket 04](./04-validate-or-record.md)'s A3, a
restriction that lives in the pack may simply not be there — the engine then does not enforce it
and says so. That is right for rules the user has not yet transcribed. But "dual-class with three
classes" and "multi-class with one" are not AD&D rules the pack transcribes; they are incoherences
of the model itself, and must be impossible to *represent*, because structure cannot fail to be
declared.

The group-indexed slots died on ticket 03's own terms: indexing slots by class group **hardcodes
the group enumeration**, and ticket 03 decided enumerations stay open. Psionicist is precisely a
group v2 adds. Roll20 can do it because Roll20 hardcodes everything.

Accepted cost: the arms do not share read code, so "what is this character's THAC0" becomes two
implementations rather than one with a branch.

### Decision — the engine owns the combination rules

XP split evenly, hit points averaged across hit dice, best saving throw across classes, best
THAC0, best slot progression, and the dual-class suppression threshold (new level > original
level) are all **engine knowledge**, not pack data.

The reasoning rests on a distinction this ticket surfaced and which is worth carrying forward:
**A3 governs validation, not computation.** If a pack does not declare a restriction, nothing is
enforced and the user is told — no harm. But if a pack did not declare "XP divides evenly", the
engine would not fail to validate; it would give full XP to every class. That is not an unenforced
rule, it is **a wrong number**. A rule the engine needs in order to *compute* cannot be optional.

It also sits on the right side of ticket 03's line. The saving-throw matrix is content — those are
the PHB's numbers. "With two classes, take the best" is in no table anywhere; it is part of what
*saving throw* means in this engine. That is shape.

Recorded because it touches the licence posture: under this decision the engine encodes PHB
mechanics. Assessment offered and accepted — mechanics are not the protected part, expression is,
and the engine has computed THAC0 from the start. Computing the better of two is not a different
category.

### Decision — the arrangement records every advance, event by event

Each advance records **which class went up, the die rolled, and what was chosen**. Rejected:
storing only current level per class plus total hit points.

The argument is that the aggregate form is not implementable, not merely worse. **Hit points are
neither a choice nor a derivation — they are recorded randomness**, a third category ticket 01 did
not name but which PCGen stores per level. Multi-class makes this decisive: hit points accrue by
rolling the advancing class's die and dividing by the number of classes, so the total cannot be
reconstructed from "fighter 5 / mage 4" — it depends on which rolls, in what order, with how many
classes active. Dual-class is the same shape: hit points freeze at the switch and resume only when
the new class passes the original.

So under the aggregate form, ticket 04's mandatory sheet-side correction would break the character:
editing a class's level gives the engine no way to recompute hit points, because the information it
would need was never kept.

**Corollary, so it is not asked again: dual-class suppression is derived, never stored.** It is a
pure function of the frozen original level and the current new level, both of which the event
record already holds.

Accepted cost: the file grows, and correcting a bad roll from 3rd level becomes an edit to
*history*. The sheet-side correction mode must therefore expose the timeline, not only current
fields — direct load on [ticket 07](./07-character-file-format-and-identity.md).

### Decision — exactly one kit, bound at creation, never rebound

Wagner's rule, which is stricter than the collection-with-pack-restriction that was recommended:

- **One kit per character**, chosen **at creation**, compatible with the race-and-class combination.
- The kit **binds to a named target** — a specific class entry, or the race for a racial kit. The
  binding is made once and never remade.
- On dual-classing the kit **stays with the original class**. It does **not** need to be compatible
  with the new class, and nothing is checked at the switch.
- The kit may be **abandoned**, but abandoning does not free a slot: a new kit can never be adopted,
  because kits are chosen at creation.

This *simplifies* [ticket 10](./10-kit-modifies-parent-class.md) rather than loading it — two kits
with contradictory effects become unrepresentable, so 10 needs no conflict-resolution rule.

**One sub-point Wagner did not rule on, left to [ticket 11](./11-engine-object-kinds.md):** whether
"one" is structural or an engine-known cardinality rule over a collection. It was raised because
Al-Qadim allows one kit per class, and Al-Qadim is a setting — v2 — so a structurally fixed single
kit would foreclose it, against the map's standing constraint. The proposal was that the
representation be a collection with an engine-known cardinality of one in v1, which changes no
observable behaviour: the user still gets one kit, chosen at creation, not swappable. Wagner ruled
Al-Qadim out of consideration for now and did not address the representation either way.

### Decision — abandoning a kit creates a debt

Abandonment removes **all the kit's special benefits and penalties**. The **free proficiencies it
granted are not lost** — but they must be **paid for**. As advancement opens new proficiency slots,
the character must spend them on the proficiencies the kit had granted, until the debt clears.

This is a third kind of effect, and neither this ticket nor ticket 10 anticipated it. The
vocabulary of kit effects therefore has three natures, not two:

1. **standing modifier** — bonuses, penalties, restrictions, sphere access. Removed on abandonment.
2. **one-time grant** — proficiencies, equipment, starting money. Survives abandonment.
3. **obligation against future budget** — created *by* abandonment, consuming later slots.

**The debt is nominal, not numeric.** The character owes *those specific proficiencies*, not "three
slots", which changes what the model stores.

### Decision — an outstanding debt does not invalidate the character

A character carrying debt is **valid**. What changes is the level-up mini-wizard: when a new slot
opens it does not offer free choice, it presents the debt and allocates there.

Quarantine was rejected because it deadlocks. Ticket 04's quarantine locks exactly what *extends*
the character — levelling — but the debt is only payable *by* levelling, since that is when slots
open. Quarantining would make the debt permanently unpayable and the state unescapable.

**Implementation requirement, recorded rather than ticketed: the debt may be unpayable.** If the
kit granted a proficiency the character's class cannot normally take, or the character never earns
enough slots, free choice never returns. That is a consequence of the rule, not a defect — but the
sheet must display the outstanding debt, or it becomes a phantom bug.
