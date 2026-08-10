# corerules

A tool for creating and managing AD&D 2nd Edition characters. The application ships no
licensed game content: it supplies the rules logic, and the user supplies the data from books
they own.

## Language

### The tool

**Engine**:
The part of corerules that is distributed — the rules logic and the interface. Carries no
licensed content of its own. Native to AD&D 2e, not a generic RPG engine: it knows a closed set
of 2e concepts and Content Packs supply instances, never new kinds. **Closed kinds, open
enumerations** — the Engine owns the shape of a thing, the pack owns its contents.
_Avoid_: core, app, platform

**Content Pack**:
A bundle of AD&D 2e rules content — classes, kits, proficiencies, spells, equipment — supplied
by the user and loaded by the Engine. Carries not only the catalogue but the **rules**:
prerequisites, restrictions and slot budgets, in a form the Engine evaluates. A pack also
declares which rule-sets it provides, so the Engine can tell "unrestricted" from "not yet
transcribed". Derived from the user's own books, and therefore does not circulate.
Concretely: a **directory** of **JSON** files whose contents a **manifest** declares — never
discovered by scanning — versioned with a three-way compatibility range, citing book and page on
every record, and carrying no executable code.
_Avoid_: data pack, module, plugin, ruleset, library

**Active pack set**:
Which Content Packs a Character was built against, recorded on the Character itself rather than
globally. Load-bearing, not decorative: it is what makes the PHB's *Set Snares* and the Complete
Barbarian's *Set Snares* two different things a player never has to choose between, because only
the books at their table are offered.
_Avoid_: loaded packs, enabled books, library

### The game

**Character**:
A fully statted individual built by AD&D 2e's character generation rules. The central entity
of the tool. Persisted as a single JSON file: not a snapshot but a **sequence of Level Events**,
each carrying its own UUIDv7, from which everything derived is recomputed. Portable across the
user's own machines; not shareable with another user, because it is meaningless without Content
Packs that user cannot be guaranteed to have transcribed under the same identifiers.
_Avoid_: PC, hero, sheet

**Level Event**:
One advance in a Character's history — which class went up, the die rolled, what was chosen.
Carries its own identity, because hit points are **recorded randomness**: neither a choice nor a
derivation, and not reconstructible from levels alone once a Character holds more than one class.
Corrections rewrite the event in place; the old value does not survive.
_Avoid_: level-up, history entry, audit record

**NPC**:
A role a Character carries, distinguishing who controls it. Not a separate kind of thing — an
NPC is built by the same rules as any other Character.
_Avoid_: non-player, monster

**Stat Block**:
The abbreviated form used for monsters: Hit Dice, no class, no proficiencies. Not a Character,
and not an NPC.
_Avoid_: monster sheet, creature, NPC

**Kit**:
A package from a Complete Handbook that modifies a parent class — restrictions, benefits,
proficiencies. Meaningless without the class it attaches to.
_Avoid_: subclass, archetype, specialisation, template
_Collision_: PCGen, the nearest prior art, uses "kit" for a one-shot bundle of creation choices
and "template" for a creature overlay. Neither is our meaning; say so when comparing.

**Attachable**:
The shape shared by Kit, Deity and Subrace — a binding to a target, a prerequisite predicate, and
an ordered list of effects. They remain three distinct kinds: each declares what it may target,
and each carries its own cardinality and abandonment rules. The shape is closed, not a general
patching mechanism.
_Avoid_: modifier, overlay, mixin, patch

**Effect**:
What an Attachable does, in one of exactly three natures. A **standing modifier** applies while
the Attachable holds and stops when it does not. A **one-time grant** happens once and survives.
An **obligation** consumes future budget — created when a Kit is abandoned, leaving its granted
proficiencies owed against later slots.
Expressed by exactly six operations, each optionally conditioned by level or predicate:
`adjust` · `grant` · `forbid` (beats `grant`) · `except` (pierces a prohibition, naming the
subject rather than the prohibition) · `require` · `set` (two on one field is a reported conflict).
_Avoid_: bonus, buff, modifier (unqualified)

**Layer**:
How the Engine computes a Character's view of anything. Nothing is ever overwritten: the base
record and each Attachable contribute a layer, and the Character's own choices are the topmost
one. Order never changes the answer — the operations commute by design — so provenance survives
computation, which is what lets a refusal name its cause and an abandonment drop a layer.
_Avoid_: override, patch, merge, cascade

**Campaign**:
A table's ongoing game: which rule options are in force, which Content Packs are loaded, and
which Characters belong to it.
_Avoid_: game, session, party

### The source material

**Core Books**:
The AD&D 2nd Edition Player's Handbook and Dungeon Master's Guide.

**Complete Handbooks**:
The PHBR "Complete Book of…" / "Complete …'s Handbook" series. Layers kits, proficiencies and
some classes onto the Core Books.
_Avoid_: red books, brown books, the completes

**Corpus**:
The body of Content Packs that has actually been transcribed — not one pack, and not the books.
The distinction that earns it a place here: **Engine scope is which mechanical shapes are
understood; Corpus scope is which books exist as packs.** They are different axes and they move at
different speeds — the Engine's is fixed by a spec, the Corpus grows book by book over years. A
partial Corpus is the normal state rather than a broken one, which is precisely what A3 exists to
make honest.
_Avoid_: library, collection, database, the books
