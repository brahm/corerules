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
_Avoid_: data pack, module, plugin, ruleset, library

### The game

**Character**:
A fully statted individual built by AD&D 2e's character generation rules. The central entity
of the tool.
_Avoid_: PC, hero, sheet

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
