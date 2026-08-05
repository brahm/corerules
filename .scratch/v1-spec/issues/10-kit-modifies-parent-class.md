# How a kit modifies its parent class

Type: grilling
Status: open
Blocked by: —

## Question

A kit is authored in one book and modifies a class authored in another. In corerules that is
cross-*pack*. What is the mechanism?

Split out of [ticket 06](./06-content-pack-format.md) because
[ticket 01's research](../research/01-prior-art-2e-content-modelling.md) found **no prior art
anywhere**. PCGen offers four partial shapes and every one of them fails:

| PCGen shape | Why it does not fit |
|---|---|
| `SUBCLASS` / `SUBCLASSLEVEL` | must live in the parent class's own file; not a cross-pack patch |
| `SUBSTITUTIONCLASS` | same file-locality problem; replaces levels rather than layering restrictions |
| `TEMPLATE` | creature overlay; no notion of "only legal on class X" |
| `KIT` / `STARTPACK` | a one-shot bundle of creation choices — a different thing that shares the name |
| `.MOD` | a genuine cross-pack patch, but load-order dependent, unstructured, and it mutates the **shared** class record, so the kit would apply to every character |

The Roll20 `ADnD_2E_Revised` sheet — the largest open 2e implementation in existence, with
schools, spheres, proficiencies, psionics and Player's Option crit tables — does not model kits
at all. This has to be invented.

What a 2e kit actually does to its parent class: narrows allowed races and ability minimums,
grants and forbids weapon and non-weapon proficiencies, changes starting equipment and money,
adds benefits and hindrances, and sometimes alters advancement or grants abilities at level.

Questions to settle:

- **Attachment.** How does a kit name the classes it may attach to, across a pack boundary? What
  happens when the class's pack is absent, or present at a version where the class changed?
- **Where the modification lands.** Does a kit patch the shared class record, or the character's
  own computed view of its class? (The research argues strongly for the latter — patching the
  shared record applies the kit to every character — but it flags this as its own analysis, not
  a sourced finding.)
- **The vocabulary of effects.** Grant, forbid, replace, add-at-level, adjust-a-number. Is there
  a closed set of effect kinds, or does this become open-ended?
- **Composition.** Can a character carry more than one kit? Can a kit modify another kit? Racial
  kits from the racial handbooks layer differently from class kits — does one mechanism cover
  both? ([Ticket 03](./03-which-complete-handbooks.md) settled that both shapes are in v1, so
  the attachment point must be nameable, not assumed to be a class.)
- **Scope as a parameter.** Ticket 03's non-foreclosure constraint lands here: v2 brings campaign
  settings, and a setting is a patch to the character's *base tables* the way a kit is a patch to
  the character's *class*. Same mechanism, wider radius. The v1 design must leave that radius
  expressible — without deciding now whether a setting is just another pack or a first-class
  concept, which ticket 03 deferred to v2.
- **Ordering and conflict.** If two effects touch the same field, what resolves it — and is the
  resolution visible to the user, or silent?
- **The seam with prerequisites.** A kit is a prerequisite predicate plus a bundle of effects, and
  [ticket 04](./04-validate-or-record.md) settled that the predicate half is fully load-bearing:
  validation is a hard block at the point of choice, so a kit whose prerequisites the character
  fails is simply not offered. The predicate must also declare itself under 04's A3 rule — a kit
  pack that does not announce it provides prerequisites gets no enforcement, and the user is told.

Unblocked: 03 settled that both kit-on-class and kit-on-race are in v1, and 04 settled that
prerequisites are enforced. Blocks 06.
