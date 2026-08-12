# Record shapes for the proving slice's kinds

Type: grilling
Status: resolved
Blocked by: 05, 08

## Question

[Ticket 05](./05-pack-schema.md) wrote the schema's **spine** and deliberately stopped there. This
ticket writes the **record shapes** — but only for the kinds the proving slice actually needs, not
for all twenty-seven.

That scoping is the whole point, and ticket 05's own warning is the reason: *designing for the books
already read*. Twenty-seven shapes designed before a single record is transcribed is that error at
maximum exposure. Shapes are written for what is about to be transcribed, tested by
[ticket 13](./13-transcribe-the-proving-slice.md), and the rest follow once the spine has survived
contact.

## What the spine already fixed, and this ticket must not re-open

- **JSON Schema is canonical**, TypeScript generated from it.
- **Every record carries `section` and `anchor`** for provenance — never a page number.
- **One file per kind.**
- **The manifest names the source files by hash.**
- Enforcement is **two-tiered**: what JSON Schema can declare, and what the §7.6 validator checks.
  Recursive structures, numeric ranges and string lengths are the validator's, not the schema's.

## What has to be decided

1. **~~Which kinds the slice needs~~ — settled.** [Ticket 08](./08-which-slice-proves-the-format.md)
   chose the slice, and it needs **ten kinds, not twenty-seven**:

   > Kit · Deity · Subrace · Thieving skill · Lookup table · Weapon proficiency ·
   > Non-weapon proficiency · Class · Race · Ability

   That bound is the deferral in [ticket 05](./05-pack-schema.md) paying off. Note that **all three
   Attachable arms are present**, which is deliberate: §4.1's claim that they are one shape used
   three times is what this slice exists to test, and the schema is where that claim either holds or
   visibly does not.
2. **The record shape for each**, against [ticket 01](./01-what-the-source-yields.md)'s measurement
   of what the source yields, per rendition.
3. **How §3's three-way split is enforced in the schema.** Pack kinds require identity; value types
   (dice expression, prerequisite predicate, effect, slot budget, money, physical properties) must be
   structurally unable to acquire one. If a value type can carry an `id`, §3's criterion has quietly
   stopped being true.
4. **How the six operations of §4.3 are expressed.** This is where the vocabulary meets real records
   for the first time, and [ticket 05](./05-pack-schema.md) established that **Deity will exercise
   them harder than any kit** — sixty records of ten fields each.

## Known unknown #4 already fired — this ticket must express the answer

[Ticket 15](./15-dice-and-generation-methods.md) found what the v1 spec predicted might be there:
**31 occurrences, inside kit effect fields, of halving, division or explicit rounding** — *"at twice
the normal cost"*, *"experience level divided by three (rounded down), plus one"*, *"creatures whose
Hit Dice total no greater than half her level"*. None fits §4.3's six operations, and the last is a
predicate carrying arithmetic against ticket 06's decision.

The resolution was **a small closed set of computed operands** — `half(<scalar>)`,
`<scalar>/N rounded down` — not a seventh operation: `adjust` still sums, and only its *operand*
widens, so the count of six and the order-independence both survive. **Rounding is a property of the
operand**, which is where §7.2's requirement finally lands.

This ticket has to give that a shape in the schema, on both sides — an effect's operand **and** a
predicate's value, since the arithmetic appears in both.

## Two requirements handed to this ticket by later resolutions

**A record needs somewhere to say "the book is ambiguous and this is my reading."**
[Ticket 11](./11-human-review-protocol.md) found the reviewer has no place to record an
interpretation as distinct from an extraction. That is a property of the record, not an attestation,
so it is a schema field and it lands here.

**A pack declares its provenance mode.** [Ticket 12](./12-how-much-tool.md) found that
[ticket 05](./05-pack-schema.md)'s mandatory `section` and `anchor` forbid a hand-authored house-rule
record, which §5.1 explicitly supports — *"the escape hatch is the pack"*. So the manifest declares
**extracted** or **hand-authored**, and the per-record provenance requirement follows from the
declaration rather than being unconditional. Express both halves: what an extracted pack owes, and
what a hand-authored one does not.

## Answer

**Artifact: [`schema/pack-0.1.schema.json`](../schema/pack-0.1.schema.json)** — eleven `$defs`, ten
kinds, valid against draft 2020-12. This ticket produced a file, not only decisions.

### Decision 1 — §4.1's claim is expressed structurally, not by resemblance

`$defs/attachable` is a single shape; **Kit, Deity and Subrace each `allOf` it and add only their
`target` and `cardinality`** — the two things §4.1 says they cannot share. §4.1's third column is
encoded too: `referenceable: true` on Deity and Subrace, absent on Kit.

Rejected: **three independent shapes** — which makes the claim decorative, since nothing breaks when
they drift apart. Rejected: **one kind with a discriminator**, which §4.1 explicitly refuses when it
says they remain distinct kinds because cardinality is the one thing they cannot share.

Why this matters more than it looks: [v1 ticket 11](../../v1-spec/issues/11-engine-object-kinds.md)
records that this claim **shrank ticket 10 from designing a kit mechanism to designing one
applicable-modifier mechanism**. Half the Attachable architecture rests on it, and it had never been
tested. Now it meets resistance twice — **in the schema, here, and in reality, in
[ticket 13](./13-transcribe-the-proving-slice.md)**.

**First evidence, and it is weak on purpose:** all three arms fit the shared base with nothing forced.
That is schema-level agreement only. Ticket 13 is the real test.

### Decision 2 — the six operations are a discriminated union

Each of `adjust`, `grant`, `forbid`, `except`, `require`, `set` carries exactly the fields its verb
needs, with `additionalProperties: false` on each arm. **An `adjust` without an operand is
unrepresentable**, not merely invalid.

The governing principle is the map's own: §5 opens with *illegal states are unrepresentable at the
point of choice*, and §6.1 applied it literally in making the class arrangement a sum type —
**structural for model incoherence, pack-declared for game rules**. An `adjust` with no operand is
not a rule a pack might have failed to transcribe; it is incoherence. So it is structural.

It is also a layer argument: [ticket 05](./05-pack-schema.md) fixed that the schema declares what it
can and the rest falls to the validator. A discriminated union puts combination-legality in **tier
one**; a single flat shape would push it to tier two for no reason.

**Writing it surfaced a fit that was invisible in prose:** §4.3 says `except` names the *subject*
rather than the prohibition, and §3.3 forbids Effects from carrying identity. Those are the same
requirement — naming the prohibition would have forced prohibitions to have ids. The schema shows
them as one constraint.

### Decision 3 — the computed operand is a value type used on both sides

`$defs/operand` is `oneOf: [integer, computedOperand]`, and `$defs/computedOperand` carries
`of` (a scalar), `divideBy` or `multiplyBy`, and a **required** `round`.

Both `adjust.by` and `set.to` reference it — and so does a **condition's `value`**, which is what
[ticket 15](./15-dice-and-generation-methods.md) meant by *the arithmetic appears on both sides*.
`§7.2`'s rounding requirement is satisfied by `round` being required rather than defaulted: the
schema makes it impossible to write a division without saying which way it rounds.

**The operation count stays at six.** `adjust` still sums; only its operand widened.

### Decision 4 — §3's three-way split is enforced by what value types do *not* reference

`$defs/record` carries `id`, `name`, `provenance`, `interpretation`, and **every pack kind
`allOf`s it**. Value types — operand, condition, predicate, effect, scalar — **simply do not
reference it**, and each closes with `additionalProperties: false`.

So a value type cannot acquire an `id` by accident. §3's criterion is enforced by the shape of the
document rather than by a rule someone must remember.

### Decision 5 — provenance is conditional on the manifest's declaration

The manifest carries `provenanceMode: extracted | hand-authored`, and an `if/then` makes `sources`
required only when extracted. That is [ticket 12](./12-how-much-tool.md)'s finding expressed:
§5.1 says *the escape hatch is the pack*, so a hand-authored house-rule pack must be writable, and
the way to allow it without weakening anything is **declaration rather than inference** — A3 applied
to provenance.

### Decision 6 — `interpretation` is a record field, not an attestation

[Ticket 11](./11-human-review-protocol.md)'s requirement: `{note, confidence}` with `confidence` one
of `reading` or `guess`. **Absence means the source was unambiguous — never that nobody looked**,
which is why review attestation stays in the ledger and out of the record.

### Where it lives

`schema/pack-0.1.schema.json` in this public repository, per ticket 05. Format version `0.1`, born
under §7.3's own version and converter machinery, which is the shock absorber for everything ticket
13 is about to break.

## What this ticket must not do

**Do not add a seventh operation** — beyond the computed operands above, which are already decided. If the slice's records cannot be expressed in `adjust`, `grant`,
`forbid`, `except`, `require`, `set`, that is **v1 spec known unknown #4 firing** — record it as a
finding against the v1 spec and surface it, rather than quietly widening the vocabulary here.

Likewise **do not re-open §3's kind list**. [Ticket 05](./05-pack-schema.md) already made one
correction to it (the Complete Priest's records are Deity, and Deity is far larger than v1 ticket 11
expected); further corrections are findings, not edits.
