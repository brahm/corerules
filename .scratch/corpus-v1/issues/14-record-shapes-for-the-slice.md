# Record shapes for the proving slice's kinds

Type: grilling
Status: open
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

## What this ticket must not do

**Do not add a seventh operation** — beyond the computed operands above, which are already decided. If the slice's records cannot be expressed in `adjust`, `grant`,
`forbid`, `except`, `require`, `set`, that is **v1 spec known unknown #4 firing** — record it as a
finding against the v1 spec and surface it, rather than quietly widening the vocabulary here.

Likewise **do not re-open §3's kind list**. [Ticket 05](./05-pack-schema.md) already made one
correction to it (the Complete Priest's records are Deity, and Deity is far larger than v1 ticket 11
expected); further corrections are findings, not edits.
