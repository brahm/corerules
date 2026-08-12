# Identity: who mints pack-scoped IDs, and do they survive re-extraction?

Type: grilling
Status: resolved
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

## Answer

### The measurement that removed the central objection

The ticket's case against source-position IDs was that they *change when the parser's segmentation
changes*. Measured on the HTML — which [ticket 09](./09-extraction-pipeline.md) made the only parse
target — **there is barely any segmentation to change**:

| kind | records | files | records per file |
|---|---:|---:|---|
| **Kit** | 133 | 133 | **1:1, no exceptions** |
| **Deity** | 57 | 57 | **1:1, no exceptions** |
| Subrace | 5 | 1 | five in one |

**For 190 of 195 measured records the HTML file *is* the record boundary** — and TSR drew it in 1996,
not our parser. The file sits on an ISO that cannot change and whose hash
[ticket 02](./02-where-the-corpus-lives.md) recorded.

### Decision 1 — the ID derives from the source anchor, and git is the registry

Form: **`<pack>:<file-stem>`** — `cth:DD05808` is the Assassin kit.

Rejected: **content-derived hashing**, decisively. Under it, fixing a comma deletes one record and
creates another — and over a transcription corrected for years that destroys every live reference
§6.5 depends on, at every correction. That is the exact failure this ticket was opened to prevent.

**The third option costs nothing because it already exists.** The ticket's option 3 was "minted once
and recorded in a persistent map, which needs the map maintained, versioned and never lost". Ticket
09 made **git the overlay and the pack the registry**: the ID is derived from the anchor on first
extraction and then **written into the record and committed**. Re-extraction does not recompute an
identity — it produces a diff, so a parser that later derives something else surfaces as a change to
adjudicate rather than a silent renumbering. Nothing new has to be built or kept.

**"Never reused" stops being a rule to enforce and becomes a property.** If the ID is a function of
source position and the source is immutable and hashed, the same file cannot become a different
record. §7.3's requirement is satisfied structurally rather than by discipline — except in the case
below.

**§7.4 falls out unchanged**: IDs are pack-scoped and file-derived, so `phb:…` and `cbarb:…` are two
proficiencies without any further mechanism, exactly as that section requires.

### Decision 2 — multi-record files use a positional ordinal, persisted the same way

Form: **`<pack>:<file-stem>#<n>`**.

Rejected: a **name-derived discriminator**, which is name-as-identity through the back door — §7.3
calls that PCGen's worst mistake and the sole reason its `migration.lst` exists. Scoping the name to
a file weakens the objection without removing it, and the first errata that corrects a name would
move the ID with it.

Rejected: **manual assignment**, defensible for the PHB's 67 numbered tables — assign once, buy
permanent stability — but it does not scale to the other twelve books.

The ordinal uses **the same mechanism as decision 1 one level down**: the parser proposes, git
persists, a reordering becomes a diff. Consistency matters here on its own terms — an identity scheme
with two different rules depending on the file is one somebody will eventually apply wrongly.

**The residual risk, and it is larger than the five subraces suggest:** the ordinal is the
parser-dependent part of the scheme, and the PHB's tables live entirely in it — 692 HTML tables
across 963 files. The mitigation is that tables are the **mechanical** bucket, where segmentation is
the most stable kind there is, because `<TABLE>` is markup rather than judgement.

### The cost this scheme accepts

**Identity is tied to one rendition of one product.** `DD05808` means nothing in the errata'd edition
[ticket 03](./03-prior-art-core-rules-extraction.md) found this CD predates, or in the PDF-only books
of a later tier. The answer is that it does not need to mean anything — once minted and committed the
ID is opaque, which §7.3 requires anyway. But **the first time a new source enters, somebody maps by
hand.** Recorded rather than discovered.

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
