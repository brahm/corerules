# An operand that lives in another layer

Type: grilling
Status: resolved — a bound is a record with members, and no seventh operation

## Question

**§4.3's effects name fields and refs. They cannot name another layer's contents, and 62 effects need
to.**

[Ticket 03](./03-precedence-when-two-attachables-contradict.md) went looking for precedence between
Attachables and found none — one arm out of five ever restricts anything, and 14,910 sheets produced
zero contested values. What it found instead, underneath all three of the cases it was opened for, is
a different shape:

| | |
|---|---|
| **Temple Guard** (CBD) | `require weaponProficiency count 1` with **no `from`**. The list is *the character's deity's*. |
| **Pariah** (CBD) | `forbid weapon`, subject written inline as *"weapons forbidden by the character's religion"* — the **complement** of another layer's list. |
| **60 priesthoods** (CPRH) | *"Weapons Permitted: …"* means **only** those. Transcribed as marked grants, because no operation says "only". |

None of these is a contradiction. Each is an effect whose **operand is another record's effect set**,
and the layer model has no expression for that. An effect can say *grant `phb:sickle`*; it cannot say
*grant the intersection of what my Deity permits with what my Kit lists*, nor *forbid everything my
Deity does not permit*.

## Why it is the same problem three times

The permit-list is not a separate issue from the two kits. A permit-list **is** a set-valued fact
about one layer, and the two kits are effects that want to **read** it. Give the priesthoods a
`limitation` record naming their permitted set — [correction 46](../corpus-v1/map.md#corrections-owed-to-the-v1-spec)
— and the Pariah's forbid has a subject and the Temple Guard's require has a list. **Sixty-two effects
collapse to one missing capability**, which is a good sign that the capability is real rather than a
pile of special cases.

## Why it cannot be deferred to the evaluator

Because the character sheet is wrong in a direction that matters. Today a priest of a permit-listed
faith is granted **one representative weapon** and prohibited from nothing, so the Engine will let a
player take a weapon the book plainly forbids **and will not know it is doing so.** That is the exact
failure §5.2 calls survivable — an unenforced restriction, with the user told — except that nothing
tells the user, because the prohibition was never written.

## The options

| | |
|---|---|
| **A seventh operation** | `permit`, `restrictTo`, whatever it is called: the complement of `grant`. §4.3's six operations have survived 1,910 effects and correction 13 measured that they suffice; adding one is a real cost and this is the first thing that has ever plainly wanted it. |
| **A set-valued `limitation` record, read by `ref`** | The permitted list becomes a record; `forbid`/`require` point at it. **No new operation** and it reuses the machinery `except` already uses. The cost is that "the set of things I may use" becomes a first-class thing the Engine must combine across layers. |
| **Resolve at transcription** | Expand the permit-list into explicit `forbid`s at pack-build time. Honest arithmetic, no schema change — and it writes out every weapon in the game per priesthood, and the two kits still cannot name the deity's list. |
| **Leave it marked** | Ticket 02's rule already makes it *visible*: applied, with the rider printed. It stays wrong and says so. Defensible for v1 and it forfeits the promise for 60 of the 59 priesthoods plus every CBD priest kit. |

## What would settle it

- **How many permit-lists there are outside the CPRH.** 60 in one book is a strong case; if the CFH,
  CRH and CTH weapon lists are the same shape the number is much larger and option 3 dies.
- **Whether "the set of things I may use" is genuinely one concept** across weapons, armour, spheres
  and schools, or four that happen to rhyme. The pack already forbids by `spellSchool`, `armor`,
  `sphere` and `weapon`, which suggests one.
- **What it costs the evaluator.** A layer that publishes a *set* other layers read is a different
  computation from a layer that writes a field, and §4.3's commutation guarantee has to be re-checked
  against it. That is the question, and it is not obviously answerable without trying it.

## Note on scope

This is the gap the corpus map's known unknown #1 was pointing at without naming — ticket 03 answered
the question as asked (there is no precedence to find) and this is what was underneath. It is
**62 effects wide against that ticket's one**, and it is the largest remaining hole in §4.3.

---

## The measurement

The ticket estimated 62 effects. **It is 142**, in four encodings rather than three, and the count was
low because the ticket was written from ticket 03's three examples rather than from a sweep.

| | encoding | in records |
|---:|---|---:|
| 61 | **grant a representative** — one member listed, the prohibition on everything unnamed lost | 61 |
| 44 | **`require` with no `from`** — choose one, from a set the effect cannot name | 37 |
| 24 | **`forbid` a placeholder complement** — *"weapons outside the Explorer's list"* | 20 |
| 13 | **`except` a contentless limitation** — lifts a restriction whose contents the pack does not hold | 13 |
| **142** | | |

**Four encodings of one fact, and which one a record uses depends on which side the book's sentence
happened to be shorter on.** The CPRH says *"Weapons Permitted: …"* and became grants; the CRH says
*"confined to the following"* and became a placeholder forbid. Same rule, opposite encodings, and only
one of them even mentions a prohibition.

### It is not a CPRH quirk

69 effects name a permit-list, across **six books at the effect level and eight of the twelve counting
interpretation notes** — CPRH 58, CRH 4, CBGH 3, PHB 2, CBE 1, CPAH 1, plus notes in CBD and CTH.
**Option 3 of the table above — expand them at transcription — dies here.** It was affordable against
one book.

### It is two concepts, and only one of them is this ticket

Sorting the 142 by whether the pack contains records of the kind being bounded:

| | | |
|---:|---|---|
| **125** | **a restriction set** over things the pack holds | weaponProficiency 86, weaponSpecialization 12, weapon 11, armor 5, nonweaponProficiency 4, sphere 2, ability, combatStyle, requirement |
| **17** | **an open choice** from a category the pack does not contain at all | chosenAnimal, chosenTerrain, chosenUndeadType, hatedFoe, guardedSite, totemAnimal, bondedMount, familialSpecies, sacredAnimal, hatedFaith, language, follower, mount, mysticAbility, savageAbility |

The second group is not a §4.3 gap. *"Choose a totem animal"* cannot be bounded because **the v1 tier has
no animals**; that is correction 23's boundary — the creature vocabulary — arriving from a new
direction. Ticket 02's rule already prints these as choices owed, with the category named, and that is
the right answer for v1.

**So the ticket is 125 effects, not 142 and not 62.**

### The intersection is exercised, not hypothetical

Of the 5,414 (class, kit, deity) combinations the pack can form, **1,765 — 32.6% — carry two or more
weapon restriction sets that would have to intersect.** The worst carries four. Whatever the mechanism
is, it must compose across layers on a third of the space.

### The machinery already exists and is populated

This is the result that decided the ticket. Twenty records in the pack already carry an explicit
member list:

```
cfh:group-polearms       tight   21 members
cfh:group-blades         broad   16 members
cfh:group-pole-weapons   broad   26 members
cfh:group-flails         tight    2 members     … 20 in all, with groupKind tight | broad | none
```

The Complete Fighter's Handbook needed *a named set of weapon ids* for its group-proficiency rules, and
it got one — `members`, on a `weaponProficiency` record flagged `isGroup`. **A set-valued record is not
a thing this ticket has to invent. It is a thing the pack has been carrying since the CFH was
transcribed**, and nothing has ever pointed at one in order to *bound* rather than to *price*.

### What blocks it is a decision, and the schema says so out loud

```
"A limitation has no effects: it is a thing to be pointed AT."
        — pack-0.1.schema.json, on `limitations`
```

Ticket 16 decision 4 made `limitation` deliberately contentless, and it was right about the problem it
was solving: most complements were dangling ids, and *"a complement is not a record: it is the
record's own sentence."* Seven survived because `except` must name what it lifts. **Those seven are
names with a provenance and nothing else** — `phb:thief-weapon-restriction` says the thief has a
limited selection of weapons and does not say which. The prose is in an `interpretation` note.

**The pack already has the handle for a bounded set and has never had the set.**

## The decision

**A bound is a record with `members`. No seventh operation.**

**1 — `limitation` gains `members`**, the same field, the same shape the CFH weapon groups already use.
It stops being a name and becomes the set it is named after.

**2 — A record applies a bound by `forbid`-ing the limitation**, meaning *this bound applies to you*;
`except` lifts it, which it already does. Both operations exist, both already take a `ref` to a record,
and **13 `except`s become computable the moment the records have contents.**

**3 — The evaluator computes `permitted(kind) = (∩ bounds still standing) \ (∪ explicit forbids)`.**
This is the ticket's third settling question, and it comes back clean: **intersection and union
commute and associate, so §4.3's central guarantee is inherited rather than re-argued.** That is the
decisive argument over a seventh operation, whose interaction with `grant` would have to be defined
from nothing and defended against the same question.

**4 — Ticket 03's decision composes with this one rather than competing with it.** The Vindicator does
not out-rank the deity: it `except`s the deity's limitation **by name**, and the term drops out of the
intersection. Two tickets, one mechanism, and neither needed an ordering.

**5 — The 17 open choices stay as they are**, marked and printed as choices owed. They bound categories
the v1 tier does not contain, and forcing them into this shape would put empty records in the pack to
make a mechanism look complete.

## The prototype

`tools/firstlight.py` implements it: a `forbid` pointing at a limitation that carries `members` becomes
a bound, bounds intersect, `except` drops one by name. Run against two limitation records injected in
memory — what correction 46 would add, not a change to the pack:

```
priest of Agriculture         : 5 of 117 weapons
      Bill-guisarme, Footman's flail, Hand or throwing axe, Horseman's flail, Sickle
+ a second bound intersecting : 1        Hand or throwing axe
Vindicator, `except` the bound: 117 of 117  — the bound is gone, not out-ranked

commutation: 1 distinct result over both layer orders
```

The 5-of-6 is the mechanism being honest: the book's list names a **scythe**, and `phb:scythe` is not
in the pack. A missing member shrinks the permitted set visibly instead of passing silently.

## What correction 46 actually costs, measured

The CPRH has **59 pages carrying a "Weapons Permitted" field, naming 61 distinct member phrases.**
Resolved against the pack's weapon names by a naive normalisation, about half match. The rest are not
missing data — they are three known shapes:

- **notation** — the CPRH writes `sword/long`, `dagger/dirk`, `hand/throwing axe` where the PHB table
  writes *Long sword*, *Dagger or dirk*, *Hand or throwing axe*. §7.3's rule that a name is never
  identity, arriving as a bill.
- **group references** — `bows (all)`, `flails (both)`, `picks (all)`, `swords (all)`. These point at
  group records that exist, and **the six PHB group records carry no members**, so a permit-list member
  can be a set that is itself empty.
- **another book's ids** — *belaying pin*, *stiletto*, *main-gauche*, *cutlass*, *net*, *lasso* exist
  under `cfh:` ids, inside the very group records that prove the machinery works.

**So the correction is a resolution pass with a checker, not 59 re-readings** — the shape of work this
corpus effort did well 149 times. And it has a prerequisite nobody had noticed: **the six PHB weapon
groups need their members before any permit-list that names one can be written.**

## The half this does not solve, and must not pretend to

**72 of the bounds are stated as a predicate over item properties, not as an enumeration**: *metallic
weapons*, *weapons more than a tenth metal by weight*, *metal weapons larger than a knife*, *armour
other than leather*, *weapons other than the concealable ones*, *all non-metal armour*. The CPRH's own
marker said it first — *"Armour is worse: a predicate over items, not an enumeration."*

A member list cannot express these, and the pack cannot compute them either: a `weaponProficiency`
record carries `cost`, `weight`, `size`, `damageType` and `speedFactor` and **no material**, and `armor`
is seven records of which two have an armour class. *Larger than a knife* is nearly computable from
`size`; *metallic* is not computable at all.

**This is a separate gap and it belongs to the item vocabulary, not to §4.3.** Recorded as a correction
rather than folded in here, because a decision that quietly covered a third of its cases by leaving
them marked would be the same mistake ticket 02 found in the markers themselves.

## Correction 48, applied

Made and proved in the same session the ticket was decided, which is the argument for landing a
correction in the schema rather than in prose: **it is testable the moment it exists.**

- **Schema** — `limitations.items` gained `members`, optional, an array of ids, with the field name
  taken deliberately from the `weaponProficiency` groups so a bounded set is one concept.
- **Pack** — two of the seven limitations took members: the thief's **twelve** weapons and the
  wizard's **five**. `1,236 records, 0 schema errors`, and all 17 member ids resolve.
- **No pack effect was needed, and that was a surprise.** This ticket predicted *"a record applies a
  bound by `forbid`-ing the limitation"*. The pack has 13 `except`s and **zero matching `forbid`s** —
  because a limitation already names the class that imposes it. **`imposedBy` was the imposition all
  along**; the missing half was only ever the contents.

On real pack data, no synthetic records:

```
thief, no kit                 weaponProficiency: 12 of 117
                                  bound by Thief Weapon Restriction (12)
                                  Broad sword, Club, Dagger or dirk, Dart, Hand crossbow, Knife,
                                  Lasso, Long sword, Quarterstaff, Short bow, Short sword, Sling

thief / Assassin (CTH)        weaponProficiency: 117 of 117
                                  LIFTED  Thief Weapon Restriction   cth:DD05808[0]

mage, no kit                  weaponProficiency: 5 of 117
mage / Militant Wizard (CWH)  weaponProficiency: 117 of 117   LIFTED
```

### The five abstentions are the more useful half

`phb:thief-armor-restriction` and `phb:bard-armor-restriction` were left **empty on purpose**. Table
46's rows are coarser than the sentences that restrict them: `phb:padded-hide-studded` carries **hide**,
which the thief's list does not name, and `phb:leather` conflates leather with padded. Members there
would permit armour the book forbids — **the exact failure the field exists to stop**. Correction 50's
boundary, met on the second record tried. The other three bound nothing at all.

An optional field that two records decline is a better test of the design than seven that accept it.

### Three findings from the application

1. **§7.3 inside a single book.** Of the thief's twelve, the PHB names a *lasso* that Table 44 does
   not carry — the only record is the Complete Fighter's — and says *staff* where the table says
   *quarterstaff*. Two of twelve, in the core book, between two pages of the same book.
2. **The wizard's list is split across two sentences** — *"a dagger or a staff … Other weapons allowed
   are darts, knives, and slings."* An enumeration a reader assembles without noticing is one a
   transcriber has to be told to look for.
3. **Multi-class composition is not an intersection, and nothing implements it.** The PHB states the
   rule **per class**: a multi-classed warrior is unrestricted, a multi-classed *priest* keeps his
   mythos weapons (*"a fighter/cleric can use only bludgeoning weapons"*), and the thief's restriction
   is about armour and thieving skills. **Intersecting bounds across `combines` would bind a
   fighter/thief to twelve weapons the book allows him.** §6.2 already assigns the combination rules
   to the Engine, so this is not a spec correction — but the Engine does not own them yet, and the
   prototype declines and says so rather than guessing.

And one question deliberately not answered: `phb:thief-weapon-restriction` is excepted as
`weaponProficiency` three times and as `weapon` once. Sent back as **correction 51**.

## Owed back to the corpus map

- **Correction 48 — RESOLVED in this session, see above.** The
  schema says a limitation is a thing to be pointed at; 125 effects need it to be a set. The field
  exists on `weaponProficiency` groups with 2 to 26 ids each. Adding it to `limitation` makes 13
  `except`s computable and gives correction 46 somewhere to put its contents.
- **Correction 49 — the six PHB weapon groups carry no members.** `phb:bow`, `phb:crossbow`,
  `phb:lance`, `phb:polearm`, `phb:sword`, `phb:bastard-sword` are `isGroup` with nothing in them,
  while the CFH's twenty are full. Permit-lists name these groups by preference — *bows (all)*,
  *swords (all)* — so an empty group is a bound that silently permits nothing.
- **Correction 50 — a bound over item properties has no vocabulary, and 72 effects want one.** Weapon
  records carry no material; armour is seven category-shaped records. Enumeration cannot express
  *metallic*, and this is the half of the restriction problem that a member list leaves untouched.
