# The pack schema, version 0.1

Type: grilling
Status: open
Blocked by: 01, 04

## Question

`spec.md` §7 describes the Content Pack format **in prose** — directory of JSON, manifest declares
contents, book and page citation on every record, pack-scoped opaque IDs, expressions as strings,
Foundry-style three-way compatibility range. It is precise and it is not machine-readable. **Nothing
anywhere says what a valid kit is.**

This ticket writes it. The map's charting settled that the schema is authored **here** and is
**canonical** — the Engine consumes it — because whoever transcribes first learns most about the
format, and the corpus is the long pole. It is born `0.x`.

## What has to be decided

**1. What the schema is written in.**

JSON Schema, or TypeScript types as the source of truth with a validator derived from them (zod,
valibot, typebox), or a schema language with codegen in both directions. The Engine is TypeScript +
React ([the v1 map](../../v1-spec/map.md)), which pulls one way; the pipeline may not be TypeScript
at all, which pulls the other. Whatever is chosen has to serve **two consumers in different
languages** without a second source of truth — `spec.md` §7.2's warning about PCGen's three live
parsers is about evaluators, but the same disease applies to schemas.

**2. The record shape of every kind in §3.1.** The bulk of the work, and
[ticket 01](./01-what-the-source-yields.md) is what makes it possible: it says what each kind's
source actually yields, which is the difference between designing a schema and guessing one.

**3. How the three-way split of §3 lands in files.** Pack kinds get identity; value types
(dice expression, prerequisite predicate, effect, slot budget, money, physical properties) do not and
are structure inside their owner. The schema has to enforce that, or a value type acquires an `id`
by accident and §3's criterion quietly stops being true.

**4. The manifest.** `id`, `version`, `compatibility: {minimum, verified, maximum}`, dependencies with
their own ranges, **plus the A3 rule-set declarations** — which §5.1 says scope to the subjects the
pack introduces and cannot be derived from contents. And the file list: §7.1 requires declaration
over discovery, with a file present but unlisted **reported**.

**5. Book and page citation as a validated field.** §7.1 makes this the answer to JSON's lack of
comments. Decide its shape now — free text, or structured (book id, page, table number) — because
it appears on every record in the corpus and changing it later is the most expensive migration
available.

**6. Where the schema physically lives**, given the pipeline is published here and the Engine does
not exist yet. It must be findable by a repository that has not been created.

## What this ticket must not do

**Do not re-open §3's kind list or §4's six operations.** Those are settled. If ticket 01 finds
something the six operations cannot express, that is **v1 spec known unknown #4 firing** — record it
as a finding against the v1 spec, do not silently add a seventh operation here.

## The thing most likely to go wrong

**Designing for the books already read.** Ticket 01 covers 13 books; the schema will outlive them and
must survive v2's campaign settings and psionics, which the v1 map deliberately excluded but forbade
foreclosing. The relevant discipline is already written down — `spec.md` §2: **closed kinds, open
enumerations.** Every enumeration in this schema stays open except rule-set names.
