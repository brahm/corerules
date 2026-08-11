# Identity: who mints pack-scoped IDs, and do they survive re-extraction?

Type: grilling
Status: open
Blocked by: 05

## Question

`spec.md` requires three things that have never been considered together:

- **Pack-scoped, opaque, never-reused IDs.** Name-as-identity was PCGen's worst mistake and the sole
  reason its `migration.lst` exists. (§7.3)
- **References from a Character to pack entries are live**, not snapshotted — a Character carrying
  pack data would be a pack in disguise and collapse the licence posture. (§6.5)
- **A pack is produced by a pipeline** that will be re-run — many times, over years, every time a
  parser improves or a book is re-extracted. (this map)

Put together: **if re-extraction renumbers, every Character that referenced the old IDs breaks, and
breaks silently.** The Engine would report drift (§6.5's content-hash detection) but drift reporting
assumes the entries still exist under the same identity; here they would simply be gone and replaced
by strangers with the same display names.

Nothing in `spec.md` addresses this, because in the v1 map a pack was something that already existed.

## The anchor already exists — ticket 05 defined it

[Ticket 05](./05-pack-schema.md) put two provenance fields on every record: a `section` heading chain
for humans, and an **`anchor`** — rendition plus file or line offset — for machines. It defined the
anchor **deliberately as the artifact this ticket would otherwise invent**, so option 2 below is no
longer hypothetical: the source-position data exists, on every record, whether or not identity
derives from it.

So the question narrows. It is no longer *what could an ID derive from* but **should the ID be the
anchor, or something that outlives it** — given ticket 04's finding that the anchor moves whenever
the parser's segmentation changes, which early on is the common event.

## What has to be decided

**1. What an ID is derived from.** Three shapes, each with a real failure mode:

- **Content-derived** (hash of the record). Stable under re-extraction of unchanged text; **changes
  whenever the record is corrected**, which is the common case for years. Corrections are precisely
  what §6.5's drift machinery exists to propagate, and this would turn every correction into a
  deletion plus an insertion.
- **Source-position-derived** (book, page, ordinal). Stable under content correction; **changes when
  the parser's segmentation changes**, which is the common case early on.
- **Minted once and recorded** in a persistent map from source anchor to ID, carried alongside the
  corpus. Stable under both; needs the map itself to be maintained, versioned and never lost — a new
  artifact with its own failure modes.

**2. Who owns the mint.** The pipeline, a checked-in registry, or the pack itself. Whatever it is, it
has to survive a machine change and a repository move.

**3. What "never reused" means operationally** when a record is deleted because it was extraction
rubbish rather than a real entry. §7.3 says never reused; a pipeline that renumbers on every run
technically honours that while destroying every reference.

**4. Whether display-name collisions across books need anything here.** §7.4 says `phb:set-snares`
and `cbarb:set-snares` are two proficiencies, disambiguated by the Character's active pack set. That
settles the *semantics*; this ticket must confirm the ID scheme actually delivers it.

## What ticket 04 already settled

[Ticket 04](./04-llm-assisted-extraction.md) resolved, and it removes an option this ticket was
implicitly holding open: **reproducible model output is not available.** No `seed` parameter exists;
`temperature` is rejected outright on current models; and the cause is *batch invariance* — server
output varies with other users' load, diverging around token 100. Locally there is a version *pin*,
not determinism.

**Therefore an ID must never be a function of model output.** That is now a constraint, not a
preference. The consequence splits by bucket: the deterministic half can safely carry
source-position-derived IDs, while the judged half needs either a deterministic anchor or option 3
below.

## Why this is blocked by ticket 05 and not by 09

Identity is a schema property — it constrains what a record *is*, not how it is produced. But the
answer strongly shapes [ticket 09](./09-extraction-pipeline.md)'s reproducibility requirements, and
[ticket 04](./04-llm-assisted-extraction.md) is researching exactly how much determinism is
available. If ticket 04 reports that reproducible model output is not achievable, options 1 and 2
above become much more attractive than option 3.

## Why this is on the map at all

Raised during charting as the risk least visible from the original request. It is the kind of
decision that costs nothing now and cannot be repaired later: once Characters exist that reference
these IDs, the scheme is frozen by every file on disk.
