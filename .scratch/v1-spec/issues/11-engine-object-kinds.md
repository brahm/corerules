# The engine's closed set of object kinds

Type: grilling
Status: resolved
Blocked by: —

## Question

[Ticket 03](./03-which-complete-handbooks.md) settled that the engine is **native to AD&D 2e**:
it knows a closed set of object kinds, and packs supply instances, never new kinds. That decision
is only safe if the set can actually be closed. **What is it, for v1?**

The candidates, from the core books and the PHBR class and racial handbooks: character, race,
class, class group, kit, ability score, alignment, level, experience table, hit dice, THAC0
progression, saving-throw matrix, weapon proficiency, non-weapon proficiency, proficiency group,
proficiency slot budget, spell, spell school, spell sphere, spellbook, equipment, weapon, armour,
encumbrance, money, language, thieving skill, turning table, level limit. That list is a starting
point written from the outside, not an answer.

Two things have to come out of this ticket, and the second is the one with teeth:

- **Which kinds the engine knows**, and which apparent kinds are really instances of another kind
  (is a class group a kind, or a field on a class? is a thieving skill a proficiency?).
- **Where the closed/open line falls inside each kind.** Ticket 03's qualifier is that the *shape*
  is engine and the *enumeration* is data — the engine knows what a saving-throw matrix is, but
  which save categories exist is data, because Ravenloft adds three. Every kind needs that line
  drawn, because getting it wrong in the closed direction is what breaks the engine on contact
  with a book, and getting it wrong in the open direction is the generic path ticket 03 rejected.

Test the result against the shapes v1 does not implement but must not foreclose — psionics and
campaign settings. A kind that has to be *added* in v2 is a v1 mistake; a kind whose enumeration
merely grows in v2 is correct.

[Ticket 04](./04-validate-or-record.md) has already enlarged the set: hard validation means
**prerequisite predicates, restrictions and slot budgets are themselves object kinds**, not just
fields, and A3 adds one more — a pack's *declaration of which rule-sets it provides*, which the
engine must model in order to tell "no restriction" apart from "not transcribed yet".

[Ticket 05](./05-generation-pipeline-depth.md) settled the pipeline: v1 owns all of it, so no kind
is excluded on the grounds that nothing touches it. It also adds **level events** as a candidate
kind, since advancement makes a character a sequence rather than a snapshot, and **dice
expressions**, since the tool rolls and generation methods come from the pack.

[Ticket 14](./14-multi-class-and-dual-class-model.md) settled that class is a **sum type**
(`Single` | `Multi` | `Dual`) rather than one kind, and added three more candidates: **level
events**, **kit bindings** (a kit binds to a named target — a class entry or the race — once, and
never rebinds), and **proficiency debt**, which is nominal rather than numeric. It also rejected
group-indexed class slots specifically because they would hardcode the group enumeration, which is
the sharpest available example of this ticket's closed/open line.

**One sub-point 14 handed over:** whether "one kit per character" is structural or an engine-known
cardinality rule over a collection. Al-Qadim allows one kit per class and is a v2 setting, so a
structurally fixed single kit would foreclose it against the map's standing constraint. Wagner
ruled Al-Qadim out of consideration and did not settle the representation.

Unblocked. Blocks 06.

## Answer

### The criterion

**A kind is anything that must be referenced by stable identity across a pack boundary.** The test
is operational — "can I point at this from another pack, or from a character?" — and it sorts the
list without argument.

It was chosen over "does this have its own rules" and "can a pack supply it alone" because it
inherits ticket 01's central warning directly: name-as-identity was PCGen's worst mistake, and what
needs an ID is precisely what crosses a boundary.

It also gets the case that mattered most right. **Saving throw category becomes a kind**, which is
exactly what ticket 03's open-enumerations decision requires — Ravenloft adds `fear`, `horror` and
`madness`, and the matrices must reference them. Under a criterion that asked "does this look like
an object?", a save category would look like an enum and the engine would break on contact.

### The inventory splits three ways

The ticket anticipated one list. There are three, and the distinction is load-bearing for
[ticket 06](./06-content-pack-format.md), which only has to give identity to the first.

**A — Pack kinds.** Identity, cross-pack referenceable.

| | |
|---|---|
| Race · Class · Class group · Kit · Deity | the spine |
| Alignment · Ability · Language | referenced by restrictions and racial adjustments |
| Weapon proficiency · Non-weapon proficiency · Proficiency group · Proficiency slot type | |
| Thieving skill · Class ability | kits name these across pack boundaries |
| Spell · Spell school · Sphere | school and sphere as many-to-many over IDs, never free text |
| Weapon · Armour · Gear | |
| Saving throw category · Encumbrance category · Coin | |
| Generation method | ticket 05: the tool rolls, and the method is PHB content |
| Lookup table | ticket 01's named, typed table. XP progressions, THAC0 progressions, saving-throw matrices and race×class level-limit grids are **instances** of it, owned by their class, group or race |
| Rule-set · Content Pack | |

**B — Character structures.** Identity within a character; not pack content.

Character · Class arrangement (the sum type from ticket 14) · Level event · Attachable binding ·
Proficiency debt · Weapon specialisation · Spellbook · Inventory

**C — Value types.** Structure without identity; never referenced.

Dice expression · Prerequisite predicate · Effect · Slot budget · Hit dice · Money · Physical
properties (weight, cost, size)

### Correction to ticket 04

[Ticket 04's resolution](./04-validate-or-record.md) states that "prerequisite predicates,
restrictions and slot budgets are themselves object kinds". **Under the criterion they are not** —
nobody points at a predicate by ID; it lives inside the kit that carries it. They are value types.

This changes none of 04's decisions. It changes what ticket 06 must give identity to, which is why
it is recorded here rather than left as a discrepancy.

### The one closed enumeration

Ticket 03 decided enumerations stay open. **Rule-set is the exception, and must be closed.** A3
has a pack declare which rule-sets it provides — but for the engine to *act* on a declaration it
must know what the name means. A pack declaring `lunar-phase-restriction` would have no effect.
The catalogue of rule-set names is the engine's, and it is the only enumeration in the model that
cannot grow from a pack.

Open enumerations, confirmed: saving throw category (Ravenloft), encumbrance category, coin,
proficiency slot type, ability (Player's Option splits Strength in v3).

### Decision — weapon and non-weapon proficiency are two kinds, and slots are currencies

Two kinds: separate budgets advancing at different rates, specialisation on one side and ability
checks on the other, one referencing a weapon and the other an ability.

**Wagner's framing, which is stronger than a compatibility note: slots are currencies of different
countries, exchangeable at each one's value.** That promotes **Proficiency slot type** from a value
type to a **kind** — a kit says "grants one non-weapon slot", an exchange rule references both
currencies, and both cross pack boundaries.

It pays for itself immediately: with slot type an open enumeration, **Player's Option adds a third
currency in v3** — character points, which replace slots in Skills & Powers — rather than breaking
the model. The map's non-foreclosure constraint comes out satisfied for free.

The exchange rate itself is a book rule, so the pack declares it. An undeclared rate means the
engine permits no exchange, which is missing validation rather than a wrong number — correct under
A3.

### Decision — weapon, armour and gear are three kinds

Sharing one value type for physical properties (weight, cost, size). Rejected: one kind with a
discriminator.

They are referenced by different things — weapon proficiency points at a **weapon**; class and kit
restrictions point at **armour**; gear is referenced by no rule at all, only carried — and the
fields do not overlap beyond weight and cost. Under one kind, "a suit of chain mail with a damage
die" would be representable, and ticket 14 settled that model incoherence is impossible by
structure.

### Decision — magic items are in v1, as a property of those three kinds

Not excluded, and not a fourth kind. A long sword +1 is a **Weapon** carrying standing modifiers.

Excluding them was rejected because it corrodes the project's premise: the engine exists to compute
THAC0, AC and saving throws correctly, and a sheet reading AC 5 when the table reads AC 2 is worse
than no sheet — it lies with authority. A fourth kind was rejected because it would duplicate the
whole catalogue, when the effect vocabulary from ticket 14 already covers what an item does.

Two riders, recorded so they are not discovered later:

- **This is the one place corerules records rather than validates.** Acquiring a magic item is DM
  fiat — no rule governs *receiving* a sword +2, so hard validation has nothing to enforce. **Use**
  stays validated: a wizard may not use a sword. Acquisition free, use by the book. An explicit
  exception to ticket 04.
- **The strange tail does not fit the vocabulary.** Charges, cursed items, artifacts, wishes are new
  machinery, not standing modifiers. v1 models items whose effect fits the existing vocabulary; the
  rest is carried as text with nothing computed. The sheet shows "Ring of Wishes: 3 charges" and
  attempts no arithmetic on it.

### Decision — Deity is a kind, thin

Included, carrying little more than identity, name, alignment and granted spheres.

The argument that decided it is **this ticket's own test**: *a kind that must be added in v2 is a
v1 mistake; a kind whose enumeration merely grows in v2 is correct.* A specialty priest of Thor can
be modelled as a **Class**, which is how the books present it, making deity a text field — but
Forgotten Realms' *Faiths & Avatars* (v2, and a setting Wagner plays) attaches rules to the deity
itself rather than to the class. Under the field model that becomes a new kind in v2.

Recorded as the honest counter: this is the first place on the map where something enters v1
*because of* v2 rather than because v1 needs it. If that door opens too far, the inventory inflates
by anticipation — the opposite of the discipline the map has been keeping.

### Decision — Kit, Deity and Subrace are one shape, three kinds

Wagner's proposal, extended and then corrected in one respect.

**Subrace is a Race with a parent reference**, not a kind of its own — everything referencing a race
would otherwise need duplicating to also accept subraces, and the Complete Book of Elves' grey elf
extending the core pack's elf is exactly the cross-pack patch ticket 10 is already inventing.

**All three carry the same shape**: a binding to a target, a prerequisite predicate, and an ordered
list of effects in the three natures ticket 14 established. Nothing a deity does falls outside it —
advantages and disadvantages are standing modifiers, granted proficiencies are one-time grants,
equipment and sphere restrictions are negative standing modifiers.

| | target | referenced by other records? | cardinality |
|---|---|---|---|
| **Kit** | a class entry, or the race | no | one per target |
| **Deity** | a priest class entry | **yes** — which is why it is a kind | one per target |
| **Subrace** | the race | yes | one per target |

**The correction: same *shape*, distinct *kinds* — not one kind.** Modelling deity *as* a kit would
put it under ticket 14's "exactly one kit per character", so a priest with a deity would be
forbidden a kit — and the Complete Priest's Handbook is entirely priest kits. Both are needed at
once, and it is the cardinality rule that guarantees it. Cardinality is the one thing that cannot
be shared. Abandonment likewise: abandoning a kit creates proficiency debt, apostasy is a different
rule, and a subrace is not abandoned at all. Per-kind rules over a common shape.

**This shrinks [ticket 10](./10-kit-modifies-parent-class.md), the map's heaviest invention.** It
was designing "how a kit patches its parent class"; it now designs **one applicable-modifier
mechanism serving three kinds**, and the scope-as-a-parameter decision already in it covers all
three radii.

Risk recorded: over-generalising here reinvents PCGen's `.MOD`, which ticket 01 ruled out. The
defence is that the shape is **closed** — binding, predicate, effects of three natures — and each
kind declares what it may target. That is a specific shape used three times, not a general
mechanism.

### The kit cardinality question ticket 14 handed over — settled by consequence

Ticket 14 left open whether "one kit" is structural or an engine-known cardinality rule over a
collection. The shape decision above settles it: **a character holds a collection of attachable
bindings, and each kind carries its own cardinality rule.** In v1 all three rules read "one per
target".

That satisfies the non-foreclosure constraint that raised the question: Al-Qadim's one-kit-per-class
becomes a change to the kit kind's cardinality rule, not a break in the format.
