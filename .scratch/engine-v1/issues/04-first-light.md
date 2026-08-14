# First light: the smallest program that loads the pack

Type: task
Status: resolved

## Question

Nothing to decide. **This is the experiment the corpus map closed by naming and could not perform**:
its last sentence is *"nothing has ever loaded this pack"*, and everything both prior maps concluded
about the format rests on artifacts that no program has ever consumed.

`validate.py` proves the pack conforms to a schema. `verdict.py` proves things about its contents.
**Neither is a program building a character**, and the distinction is the same one A3 exists to keep:
a pack that validates is not a pack that works.

## What it must produce

1. **A program that loads the pack** — the manifest, the 20 kinds, the references between them — and
   fails loudly on anything it cannot make sense of. Not the Engine. A hundred lines that read JSON
   and build an object graph.
2. **One character, computed.** A dwarf fighter with a kit is enough: it exercises a Race with
   effects, a Class, an Attachable, the layer model, a `tableValue` read and at least one marked
   effect. **The sheet does not have to be right. It has to exist**, and the ways it is wrong are the
   finding.
3. **A list of everything the pack turned out not to say.** The prediction, stated in advance so it
   can be scored: the gaps will not be in the operations — those were measured over 1,910 effects —
   but in **the joins between kinds**, which nothing has ever traversed.

## Why now, before the Electron app

Because it is cheap and because the alternative is worse. The specified application is Electron +
TypeScript + React with packaging, persistence and a product surface; a wrong assumption about the
pack discovered *there* is discovered under three layers of scaffolding. **Discovering it in a
throwaway script costs an afternoon.**

There is a second reason, and it is the stronger one. Ticket 01 has to decide what an implementation
session reads on day one, and ticket 02 has to decide what the Engine does with 380 marked effects.
**Both of those decisions get better with one running example in hand** — the first character will
show whether the corrections list is a day-one document or a reference, and how many markers a real
character actually touches.

## What it must not do

**Do not let it become the Engine.** The temptation, once JSON is loading and a sheet is printing, is
to keep going — add a UI, add persistence, and quietly skip the map. This produces a character in a
terminal and then stops.

**Do not fix the pack while writing it.** Everything the loader stumbles on is evidence. Record it;
the corpus map's own method was that a finding recorded is worth more than a defect quietly repaired,
and this is the first time anything has looked at the pack from the consuming side.

## Its result feeds

Tickets [01](./01-which-spec-does-the-engine-implement.md), [02](./02-what-the-engine-does-with-an-unmodelled-effect.md)
and [03](./03-precedence-when-two-attachables-contradict.md) — all three ask questions that a single
worked character makes concrete. **Ticket 03 especially**: a dwarf priest with a kit is the exact case
where two Attachables contradict, and it can be built on purpose.


---

## Result

**It works.** [`tools/firstlight.py`](../tools/firstlight.py) loads all 1,233 records, applies a race,
a class group, a class, a deity and a kit as layers, evaluates predicates, reads lookup tables and
prints a sheet. A dwarf fighter with the Clansdwarf kit comes out with 44 fields, two granted
proficiencies, one forbidden weapon list and one choice owed.

**The prediction scored.** Not one gap was in the six operations. **Every single one was a join**, or
something no measurement of the pack's contents could have shown.

### 1. Eight ids are defined twice, and the checker could not see it

```
phb:armorer          nonweaponProficiencies + secondarySkills
phb:bowyer-fletcher  nonweaponProficiencies + secondarySkills
phb:divination       spheres + spellSchools
phb:healing          nonweaponProficiencies + spheres
phb:warrior          classes + proficiencyGroups     (also priest, rogue, wizard)
```

**The namespace is `book:slug` and carries no kind**, so two different things with the same name in
the same book collide. `phb:warrior` is both the warrior class group and the warrior proficiency
group; `phb:divination` is both a school of magic and a sphere.

`validate.py` never saw it because it builds `defined` as a **set** — the collision is invisible to
the one tool whose job is identity. A consumer that indexes by id, which is the only sane way to
consume this pack, **silently loses eight records**.

The irony is exact: the same flat namespace made the concept→array join *free* — an effect says
`kind: "ability"` and the record lives in `grantedAbilities`, and `by_id[ref]` finds it without any
mapping. **The property that removed one join is the property that broke identity.**

### 2. Two `set` layers can contradict inside a single record — and the book is at fault

`phb:dwarf` sets `proficiency.navigation.modifier` to **0** at effect 4 and to **−3** at effect 29.
Both cite the same page. Reading it:

```
['Navigation', '1', 'Intelligence', '0']
['Navigation', '1', 'Intelligence', '-3']
```

**The Complete Book of Dwarves prints Navigation twice, with two different modifiers.** The
transcription is faithful; the source contradicts itself.

§4.3's commutation was tested across records and **never within one**. Two `set`s on one field are
order-dependent, so this record's meaning depends on array order — which nothing guarantees, and which
[ticket 03](./03-precedence-when-two-attachables-contradict.md) was about to assume could only happen
between two Attachables.

### 3. The sheet has no combat numbers at all

Forty proficiency modifiers, five detection rolls, two saving-throw bonuses — and **no THAC0, no hit
points, no armour class, no experience.** Because 15 of the 19 class records say
`effectsModelled: false`, and *the class is what supplies those*.

The pack is dense exactly where the books are dense — kits, priesthoods, racial oddities — and empty
where a character sheet begins. **No measurement of the corpus could have shown this**, because the
pack is complete by its own accounting: those 15 records honestly declare themselves untranscribed,
and the verdict counts them correctly. It takes trying to print a sheet to notice that what is missing
is the middle of the page.

### 4. The pack supplies deltas and nothing supplies the base

`ability.constitution` comes out as **1**, not 16. The dwarf's `+1 Constitution` is a layer over a
value the pack never holds and the player rolls. The loader has the scores; **the field path and the
score are not connected by anything**, and I connected them by hand in `scalar()`.

This is the join the spec calls the character's *view* of a value, and it is the one join the pack
cannot make on its own — correctly. But nothing anywhere says which field paths are rooted in a
player's input and which are rooted in nothing.

### 5. A table read returns what the book printed

`savingThrow.vsPoison` resolves through Table 9 at Constitution 15 and yields **`"+4"`** — a string
with a sign, because that is what the cell says. The `operand` contract says integer. Every consumer
of a `tableValue` will need to parse the corpus's own typography, and
[finding 138](../../corpus-v1/issues/13-transcribe-the-proving-slice.md#finding-138--most-of-the-corpuss-tables-cannot-be-consumed-and-that-is-the-useful-part)
already said the tables are printed rather than typed. Nine tables declare a `supplies`; **none
declares a cell type.**

### 6. Two mutually exclusive bonuses summed, exactly as ticket 02 predicted

The Clansdwarf's `reactionCheck` comes out **5**: +3 *"with others of his own clan"* and +2 *"with
dwarves of other clans in the same craft"*. Both are marked, both fired, and **a dwarf is never both
at once.** [Ticket 02](./02-what-the-engine-does-with-an-unmodelled-effect.md)'s *apply and annotate*
row is not a hypothetical cost — it is a wrong number on the first character anyone builds.

### 7. Ticket 03's contradiction did not materialise, and that is the finding

Built on purpose: a dwarf fighter/priest with the **Vindicator** kit and a deity. The two Attachables
did **not** collide, because the deity-dependent weapon restriction the corpus map found —
*"any weapon not forbidden by his religion"* — was never modelled as an effect. **It is a marker.**

So the hole [ticket 03](./03-precedence-when-two-attachables-contradict.md) was opened for is real in
the *books* and currently absent from the *pack*: the transcription sidestepped it by declaring it
unmodellable. The Engine meets it the day someone models a religion's permit-list — which is also the
day the pack stops being able to sidestep it. **Ticket 03 should be re-read with that in mind**: it is
not blocking first light, and it is not gone.

## Fixed, same session

The eight collisions are gone and the pack is **1,229 records**.

**Four were the same thing twice.** Table 37's Warrior, Wizard, Priest and Rogue proficiency groups
**are** the four class groups — the PHB names them identically because a class buys from its own
group — and the modelling had already half-admitted it: each proficiency-group record carried
`openTo: [its own id]`, a self-reference nothing had ever read. They are now one record each, and
only **General** remains in `proficiencyGroups`, because no class group is called General.

**One was the book's own name, lost in transcription.** DD01473 lists the nine schools and calls that
one **Lesser Divination**; the pack had shortened it to `Divination` and collided with the priest
sphere. Restoring the book's name dissolves the collision instead of papering over it.

**Three took a suffix** — `phb:armorer-skill`, `phb:bowyer-fletcher-skill`,
`phb:healing-proficiency` — and the rule behind that is the interesting part:

> **Always qualify when the set is open; minimally disambiguate when it is closed.**

Spell ids are caster-qualified *always*
([corpus finding 123](../../corpus-v1/issues/13-transcribe-the-proving-slice.md#finding-123--30-spell-names-are-two-different-spells)),
because more books will add spells and a scheme that renames `phb:bless` the day a wizard Bless
arrives is the instability ticket 07 exists to prevent. Secondary skills are **Table 36 entire, 23
rows, closed forever** — so the two colliders can move and nothing later can undo it.

**And the kind was deliberately NOT put into the id.** It is the obvious fix and it is wrong: kinds
were still being created in the corpus effort's last ten sessions, and
[finding 105](../../corpus-v1/issues/13-transcribe-the-proving-slice.md#finding-105--a-packs-ids-encode-the-order-its-books-were-transcribed-in)
established that the prefix names *the book that introduces a thing*. **A kind is not stable enough to
be part of identity.**

`validate.py` now reports duplicates, **counted apart from schema errors** — the schema cannot express
this at all, because uniqueness spans arrays and a JSON Schema sees one array at a time. Verified by
introducing a collision on purpose.

### What this says about the other tickets

- **[02](./02-what-the-engine-does-with-an-unmodelled-effect.md)** — a real character touched **four**
  marked effects out of 380, and one of them produced a demonstrably wrong number. The exposure is
  small and the damage is immediate, which argues against *apply and annotate* far more strongly than
  the ticket could argue on paper.
- **[03](./03-precedence-when-two-attachables-contradict.md)** — reframed by result 7, and result 2
  adds a case it did not contemplate: contradiction **within** one record.
- **[01](./01-which-spec-does-the-engine-implement.md)** — three corrections owed to the corpus map
  come out of this session alone (results 1, 2 and 5). **The corrections list is not closed**, which
  is itself an answer to what an implementation session reads on day one: not a frozen document.
