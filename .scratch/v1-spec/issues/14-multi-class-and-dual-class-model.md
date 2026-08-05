# How multi-class and dual-class are modelled

Type: grilling
Status: open
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
