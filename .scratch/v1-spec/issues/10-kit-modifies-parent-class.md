# How a kit modifies its parent class

Type: grilling
Status: resolved
Blocked by: —

> **Scope changed by [ticket 11](./11-engine-object-kinds.md).** This ticket no longer designs a
> kit mechanism. It designs **one applicable-modifier mechanism serving three kinds** — Kit, Deity
> and Subrace — which carry the same closed shape (a binding to a target, a prerequisite predicate,
> an ordered list of effects in three natures) and differ only in what they may target, whether
> other records reference them, their cardinality rule and their abandonment rule. The
> scope-as-a-parameter decision already below covers all three radii. This is less invention than
> the ticket was created for, not more.

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
  **[Ticket 14](./14-multi-class-and-dual-class-model.md) already established that it has three
  natures, not two** — and the third was not anticipated by either ticket:
  1. **standing modifier** — bonuses, penalties, restrictions, sphere access; removed on abandonment;
  2. **one-time grant** — proficiencies, equipment, starting money; survives abandonment;
  3. **obligation against future budget** — created *by* abandonment, consuming later proficiency
     slots until the granted proficiencies are paid for. Nominal, not numeric: the character owes
     *those* proficiencies, not a count.
- **Composition.** Can a character carry more than one kit? Can a kit modify another kit? Racial
  kits from the racial handbooks layer differently from class kits — does one mechanism cover
  both? ([Ticket 03](./03-which-complete-handbooks.md) settled that both shapes are in v1, so
  the attachment point must be nameable, not assumed to be a class.)
- **Scope as a parameter.** Ticket 03's non-foreclosure constraint lands here: v2 brings campaign
  settings, and a setting is a patch to the character's *base tables* the way a kit is a patch to
  the character's *class*. Same mechanism, wider radius. The v1 design must leave that radius
  expressible — without deciding now whether a setting is just another pack or a first-class
  concept, which ticket 03 deferred to v2.
- ~~**Ordering and conflict** between two kits~~ — **dropped.** Ticket 14 settled that a character
  holds exactly one kit, bound at creation to a named target (a class entry, or the race) and never
  rebound, so two kits with contradictory effects are unrepresentable. Conflict between a kit's
  effects and the *base* record it patches still needs an answer; conflict between kits does not.
- **Attachment survives what the character does later.** Ticket 14: on dual-classing the kit stays
  with the **original** class, is not required to be compatible with the new one, and nothing is
  checked at the switch. The binding is made once and never remade.
- **The seam with prerequisites.** A kit is a prerequisite predicate plus a bundle of effects, and
  [ticket 04](./04-validate-or-record.md) settled that the predicate half is fully load-bearing:
  validation is a hard block at the point of choice, so a kit whose prerequisites the character
  fails is simply not offered. The predicate must also declare itself under 04's A3 rule — a kit
  pack that does not announce it provides prerequisites gets no enforcement, and the user is told.

Unblocked: 03 settled that both kit-on-class and kit-on-race are in v1, and 04 settled that
prerequisites are enforced. Blocks 06.

## Answer

The mechanism that had no prior art anywhere turns out to be four decisions, and none of them
needed new machinery — three earlier tickets had already forced most of it.

### Forced, not decided — an Attachable never touches the shared record

It acts on **that character's view** of the target. Patching the shared record would apply the kit
to every character using the class. This is one of the two axes on which PCGen's `.MOD` fails, and
it has no reading in which it works.

### Decision — layered resolution, not overwrite (B)

Nothing is rewritten. The character's view of any value is computed by walking base record then
layers. Rejected: effects overwriting fields on a copy of the base record.

Three decisions already taken made this close to obligatory:

- **Kit abandonment (ticket 14).** Abandoning removes standing modifiers but keeps one-time grants.
  Under overwrite you would have to recompute from base and reapply everything except that kit's
  standing modifiers — which requires knowing where each value came from, precisely the information
  overwrite destroyed. Under layers, abandoning is **dropping a layer**.
- **Magic items (ticket 11).** AC is armour plus magic plus Dexterity; that is already a stack.
  Under overwrite, removing a ring means recomputing the sheet.
- **Explained refusals (ticket 04).** Hard validation refuses choices, and the refusal was required
  to name its cause. "Your kit forbids this" is only sayable if provenance survives the
  computation, and overwrite erases provenance by construction.

Layers also dissolve a question this ticket was created with: **conflict between an Attachable's
effect and the base record** needs no rule of its own — it is resolved by the stack.

Accepted cost: every derived read is a traversal, and the layer order becomes part of the spec
rather than an implementation detail — which the next decision then removes.

### Decision — operations are order-independent (ii)

Rejected: an ordered sequence in which later layers see earlier results.

**This is a direct application of ticket 01's findings.** Load-order dependence was the first of
three verified problems with PCGen's `.MOD`, patched over with `RANK:` load priorities and
`FORWARDREF:` declarations — and the same research was explicit about the remedy: cross-pack
collisions must resolve "by an explicit, user-visible rule, not by load order". An ordered stack
reimports `RANK:` wholesale. Order-independence means "which weapons may I use" has **one** answer
regardless of how the packs were loaded.

Two consequences:

- **`set` cannot be silently resolved by order.** Two layers setting the same field become a
  **reported conflict** — "kit X and deity Y disagree about hit die" — rather than a silent
  last-loaded-wins. Same philosophy as ticket 04's A3: honest about what it cannot resolve.
- **`forbid` beating `grant` requires an escape valve**, because some kits explicitly pierce a class
  restriction. Hence `except` as a first-class operation, which also produces the user-facing
  message for free.

### Decision — the operation vocabulary closes at six

`adjust` (sums into a number) · `grant` (unions into a set) · `forbid` (subtracts, beats `grant`) ·
`except` (pierces a prohibition) · `require` (obliges a future choice by the character) · `set`
(fixes a value, conflict-detecting).

Each may be **conditioned** — by level, or by a predicate. That is a qualifier on an effect, not a
seventh operation.

`require` is not new: it is the third nature ticket 14 discovered, the same shape as proficiency
debt.

**`set` earns its place on an authoring argument, not a modelling one.** Without it, "grey elves get
+1 Int, −1 Str" would have to be transcribed as a *delta* against the generic elf. Wagner is
extracting from RTF and PDF, so the natural act is to copy the book's number, not to compute a
difference — and a vocabulary that demands mental arithmetic during transcription produces typos,
which under hard validation become false rules.

**The line for what does not fit already exists on the map.** Ticket 11 settled that a magic item
whose mechanics fall outside the vocabulary — charges, curses, artifacts — is carried as text with
nothing computed. Same line here: "must tithe 10%" has a mechanical part and becomes `adjust`;
"never refuses a challenge" is text the sheet displays and the engine does not compute.

Risk recorded: if some PHBR kit needs a seventh operation, that is discovered during transcription
— late. The defence is that the edition is dead and the books are in hand today, so it is checkable
before any code is written.

### Two things that fell out while checking the vocabulary

- **`except` names the *subject*, not the effect.** Piercing "that particular prohibition" would
  require effects to have identity, contradicting ticket 11, which classed Effect as a value type.
  Unnecessary: `except: long sword` means "long sword is permitted notwithstanding any prohibition"
  — no effect ID, order-independent, and the semantics the books actually use.
- **Radius is implied by the target**, not a field of its own. A binding pointing at a class entry
  has that class's radius; at the race, the race's. v2's setting layer simply points at something
  wider. A separate radius field could disagree with the target it accompanies.

### Decision — the character's own choices are the topmost layer

One uniform resolution model, not two. Rejected: layers from packs, choices from the character,
combined at read time.

- **Choices already fit the vocabulary.** Picking a proficiency is `grant`; buying equipment is
  `grant`; spending a slot is `adjust`. Nothing needs inventing.
- **It completes the provenance story** that decided the layering question. Every value on the sheet
  traces to a layer, including "because you chose it". Under two mechanisms, half the sheet can
  explain itself and half cannot.
- **Ticket 05 already put choices in level events.** A level event carrying a `grant` *is* a layer
  with a date; the two models converge with no effort, where two mechanisms would have to be kept
  consistent by hand.

Discomfort recorded and judged acceptable: it becomes structurally possible for a character to
grant itself something its class forbids, since a `grant` is a `grant` wherever it comes from. The
defence is that `forbid` beats `grant` by design and the wizard never offers the illegal choice —
a resolution rule rather than a structural impossibility. Ticket 14 established a preference for
structural where the incoherence is the model's; here it is not. "The character chose something" is
always coherent. Whether it is *legal* is ticket 04's question.

### Handed on

**How an Attachable names its target across a pack boundary** is settled in principle by ticket 01
— pack-scoped opaque IDs, never reused, display names as presentation only — and the closed kind
system guarantees the target's shape, so no declared expectation is needed. But **what happens when
the target's pack is absent, or present at a version where the target changed**, belongs to
[ticket 07](./07-character-file-format-and-identity.md), which already carries it.
