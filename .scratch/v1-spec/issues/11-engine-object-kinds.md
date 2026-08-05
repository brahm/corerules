# The engine's closed set of object kinds

Type: grilling
Status: open
Blocked by: 14

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

Depends on [14](./14-multi-class-and-dual-class-model.md), which decides whether class is one kind
or a collection with rules of its own. Blocks 06.
