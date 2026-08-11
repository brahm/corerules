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

1. **Which kinds the slice needs.** Falls out of [ticket 08](./08-which-slice-proves-the-format.md),
   which is why this is blocked by it. Expect the kit-or-Deity record plus whatever `spec.md` §3.1
   kinds its effects reference — proficiencies, weapons, armour, spheres — since an Attachable that
   cannot resolve its references proves nothing.
2. **The record shape for each**, against [ticket 01](./01-what-the-source-yields.md)'s measurement
   of what the source yields, per rendition.
3. **How §3's three-way split is enforced in the schema.** Pack kinds require identity; value types
   (dice expression, prerequisite predicate, effect, slot budget, money, physical properties) must be
   structurally unable to acquire one. If a value type can carry an `id`, §3's criterion has quietly
   stopped being true.
4. **How the six operations of §4.3 are expressed.** This is where the vocabulary meets real records
   for the first time, and [ticket 05](./05-pack-schema.md) established that **Deity will exercise
   them harder than any kit** — sixty records of ten fields each.

## What this ticket must not do

**Do not add a seventh operation.** If the slice's records cannot be expressed in `adjust`, `grant`,
`forbid`, `except`, `require`, `set`, that is **v1 spec known unknown #4 firing** — record it as a
finding against the v1 spec and surface it, rather than quietly widening the vocabulary here.

Likewise **do not re-open §3's kind list**. [Ticket 05](./05-pack-schema.md) already made one
correction to it (the Complete Priest's records are Deity, and Deity is far larger than v1 ticket 11
expected); further corrections are findings, not edits.
