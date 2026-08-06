# Does the tool validate the rules, or merely record them?

Type: grilling
Status: resolved
Blocked by: —

## Question

When a user builds a character, does corerules **enforce** 2e's legality rules, or does it
**record** whatever the user enters and compute derived values without judgement?

The rules in question: class/race combinations, ability score minimums per class, race-based
level limits, kit prerequisites and restrictions, proficiency slot budgets, alignment
restrictions, spell access by level and school/sphere.

This is the cheapest ticket on the map to answer and the one that changes the most downstream.
If corerules validates, a content pack must encode *rules* — prerequisites, restrictions,
budgets — and the pack format becomes a small rules language. If corerules only records, a
pack is close to a catalogue with some lookup tables.

Sub-questions worth separating:

- Is the answer uniform, or per-rule? (Hard-block illegal class/race, but merely warn on
  ability minimums, say.)
- Does the answer differ between *creation* and *later editing*?
- House rules are near-universal in old-school play. Does validation have an escape hatch, and
  if so is it per-character or per-rule?

## Answer

**corerules validates, and it validates hard.** Wagner's table follows the book: his earlier "all
combinations can appear" meant all *legal* combinations, respecting the limits the rules impose.
Illegal states are unrepresentable at the point of choice — a dwarf paladin cannot be built.

Rejected: advisory validation (record anything, flag violations) and pure recording. The concerns
raised against hard blocking — that the rules are the user's own transcription, so a typo becomes
indistinguishable from a rule, and that in old-school play the DM outranks the tool — were put and
overruled.

**Note that validation is a separate question from computation**, which was never in doubt: the
engine computes THAC0, saving throws, armour class and encumbrance either way. This ticket only
decided whether the engine *blocks*.

### The consequence that is not a decision

corerules ships no content, so "dwarves cannot be paladins" is not in the engine — it is Wagner's
transcription of PHB Table 6. **A validating engine therefore makes the pack format a small rules
language.** Prerequisites, restrictions and slot budgets must be expressible in a form the engine
can evaluate. This is a hard constraint on [ticket 06](./06-content-pack-format.md) and
[ticket 11](./11-engine-object-kinds.md), not an option they get to weigh. **Corrected by ticket
11:** the sentence below calling predicates, restrictions and budgets "object kinds" is imprecise —
under 11's criterion they are **value types**, since nothing references a predicate by identity;
it lives inside the kit that carries it. None of this ticket's decisions change; what changes is
what ticket 06 must give identity to. Ticket 01's warning
applies with full force: one evaluator, versioned, no fallback path, rounding semantics written
down.

### Decision — packs declare their rule coverage (A3)

The problem hard validation creates: while the corpus is being authored, **absence of data is
indistinguishable from absence of a rule**. If the pack has no Table 6 yet, is a dwarf paladin
legal or merely unjudged?

Rejected: permissive-by-omission (blocks only what is explicitly forbidden — which means the tool
silently appears to approve when it does not know) and restrictive-by-omission (permits only what
is explicitly allowed — unusable until the corpus is complete).

**Chosen: a pack declares which rule-sets it provides.** A pack that announces "I provide
race/class restrictions" makes absence from its permit-list a hard block. A pack that announces
nothing about a rule means the engine does not validate that rule **and says so visibly to the
user**. Hard blocking where there is a rule; explicit honesty where there is none.

This also carries a real property of the source material: **2e states its restrictions in two
different forms.** PHB Table 6 is a *permit-list* — it says what each race may be. Kit
prerequisites are *restrictions* — they say what disqualifies you. A3 is the only option of the
three that carries both, because the pack declares the form rather than the engine assuming one.

**A3 subsumes the house-rule escape hatch.** No separate override mechanism is needed: an optional
DMG rule or a table's house rule is expressed by what the pack declares and what it contains. The
escape hatch is the pack.

### Decision — validate at the point of choice; never refuse to open (A3-i)

Validation runs when the user *chooses*, and again on load — but loading never fails. Rejected:
revalidating on load with refusal or restricted opening.

Two reasons: A3 guarantees the pack's rule coverage grows over time, so revalidation-with-refusal
would mean **every improvement to the transcription retroactively breaks old characters** — the
incentive exactly inverted. And a character that will not open is a data-loss bug, which
[ticket 07](./07-character-file-format-and-identity.md) already flags from the missing-pack side.

**Spec requirement that falls out of this: dual-classing must be validated as a sequence, not as a
snapshot.** During the transition a 2e dual-class character legally occupies states that look
illegal in isolation — old class suppressed, XP frozen. A naive per-field validator on load would
reject a perfectly legal dual-class. Whatever validates must understand the pipeline, not just the
sheet.

### Decision — invalid characters are quarantined, not blocked (ii)

An invalid character opens **fully readable and printable**. What is locked is everything that
*extends* it: levelling, spending XP, adding proficiencies, buying spells. The lock lifts when the
violations clear.

Rejected: a passive warning (too weak — Wagner wants correction to be mandatory) and a forced
correction flow on open.

Forced correction was rejected for a concrete reason: **some invalid states no edit can fix.** If
the referenced pack is missing from the machine, the character no longer knows what its class *is*
— no choice available in the UI resolves that, because the thing to choose is not loaded. A forced
flow strands such a character with no exit. Quarantine treats "invalid" and "not repairable here"
as different states; forced correction cannot.

The principle: reading and printing an invalid character harms nothing, because the error is
already there. What harms is **building on top of it**, since the error then propagates into every
value derived from that point on. Quarantine locks exactly that and nothing else.

### New fact, from the grilling — the books are already digital

Wagner will not be typing the books in. He holds them as **RTF and PDF** and will use those to
author the packs. This reframes pack authoring from transcription to **extraction**, which makes
taking the corpus in bulk plausible rather than a years-long incremental slog. It graduates the
map's "how book content actually gets into the tool" fog into
[ticket 13](./13-how-packs-get-authored.md).

**Noted once, because it is spec load rather than a caveat:** the project's legal posture never
depended on how a pack was produced — it depends on packs not circulating. Personal use from one's
own copies is one thing; a pack derived from the books leaving the machine is redistribution of
WotC IP regardless of purchase. Ticket 07 currently asks whether a file is "shareable with another
player as a single file"; the answer is plausibly yes for a **character** and no for a **pack**,
and the spec must say so explicitly rather than leave it implied.

**Sharpened by [ticket 07](./07-character-file-format-and-identity.md):** "plausibly yes for a
character" is true legally and false functionally. A character is useless without the packs it
references, and under pack-scoped IDs the recipient would need to have transcribed the same books
*under the same pack identifiers* — which nothing can guarantee for hand-authored packs.
Cross-user sharing is therefore **not a v1 goal**; portability means across the user's own
machines. The spec must state that limitation, because this ticket's wording implies otherwise.
