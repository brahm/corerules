# Map: the v1 Content Pack corpus

## Destination

The **v1 corpus as loadable Content Packs** — the PHB, the DMG and the eleven Complete handbooks
that exist as AD&D Core Rules 2.0 RTF — together with **the concrete pack schema** they conform to
and **the published pipeline** that produces them.

The map is done when a **proving slice** has been transcribed end to end, the schema and the
expression language are fixed at `0.x`, and the remaining books are mechanical work with no
decisions left inside them.

This effort is the sequel to [the v1 spec map](../v1-spec/map.md), which specified an Engine that
ships no content and then stopped at the point where content has to exist.

## Notes

**Domain.** AD&D 2nd Edition transcription: turning the books into the Content Pack format that
[`spec.md`](../v1-spec/spec.md) §7 describes in prose. Read `spec.md` §3 (object kinds), §4 (the
Layer model), §5 (validation and A3) and §7 (Content Packs) before touching any ticket here — this
map does not restate them.

**Skills every session should consult.** `/grilling` and `/domain-modeling`. Keep the glossary in
`CONTEXT.md` current as terms resolve — do not batch glossary updates.

**This map carries execution.** Wayfinder is planning by default; this effort overrides that.
Ticket 13 transcribes a real slice, and tickets 01, 09 and 10 produce running code. The reason is
the scoping decision below: the destination is the corpus, and a plan written without touching real
RTF is exactly the guess this map exists to replace.

### Hard constraint — no book text in this repository

`.scratch/` is committed to a **public** repository. The books are Wizards of the Coast IP and
`spec.md` §1 fixes the posture: **derived content does not circulate.** Therefore no RTF, no
converted `.txt`, no extracted JSON, and **no pasted passage inside a ticket or a research file**
may live here. Quote a heading or a field label when a ticket needs one; never a paragraph of body
text, never a table's contents.

Instruments are fine: [`tools/dertf.py`](tools/dertf.py) is in this repo because it contains no
content of its own.

Where the corpus itself lives is [ticket 02](issues/02-where-the-corpus-lives.md), and it is on the
frontier for this reason.

### Measured during charting — do not re-measure

The source material was found and characterised while this map was being drawn. These are
measurements, not estimates.

**Location.** `/run/media/brahm/PocketNAS/Projects/corerules/books/` — 20 RTF files, 32 MB,
~2.2M words. The filenames (`phbbk`, `dmgbk`, `poctbk`…) identify them as **AD&D Core Rules 2.0**
exports: TSR's own digital editions. This is extraction, not OCR archaeology.

| Present as RTF | Scope |
|---|---|
| `phbbk`, `dmgbk` | **v1** |
| `bardbk` `druidbk` `dwarfbk` `elfbk` `fightrbk` `gnmhlfbk` `paladnbk` `priestbk` `rangerbk` `thiefbk` `wizardbk` — eleven Complete handbooks | **v1** |
| `armsbk` (Arms & Equipment), `tomebk` (Tome of Magic), `dohlcbk` (DM Option: High-Level Campaigns) | out |
| `poctbk`, `posmbk`, `pospbk` (Player's Option) | v3 |
| `monbk` (Monstrous Manual) | out |

**The v1 tier is 13 books, ~1.27M words.**

**There is a second rendition of the same books, and it is also on this machine.**
Established by [ticket 03](issues/03-prior-art-core-rules-extraction.md). Both CD images sit at
`/run/media/brahm/PocketNAS/Backup/ISO/` — `add_core_rules_2.iso` and
`add_core_rules_2_expansion.iso`. The expansion image contains **13,099 HTML files** under
`/WEBHELP/`, one subdirectory per book, covering **the whole v1 tier**: PHB (963 files), DMG (965),
and all eleven Completes. It is a RoboHELP export with **real `<TABLE>`/`<TR>`/`<TD>` markup** and
`<TITLE>`s that name the record and the book. It is not abridged (85,897 words for the Complete
Fighter's Handbook against the RTF's 84,360).

**The HTML rendition is better, and not narrowly.** All **3,603** v1-tier WebHelp files carry a
non-empty `<TITLE>`, it gives **one titled page per kit in every book**, and it carries at least the
RTF's kit count everywhere — including the three books where the RTF exposes no kit structure at all.
It also wins decisively on tables.

> An earlier version of this note said the two renditions were "good at opposite things", with the
> RTF winning on kits. **That was wrong**, and it was a tooling failure: see the ugrep caution below.
> The RTF's remaining value is narrow and is [ticket 09](issues/09-extraction-pipeline.md)'s question.

1. **No table is a table.** Zero `\trowd` markup in all 20 files; every table was flattened into
   tab-delimited paragraphs. It survives anyway — the PHB has 161 numbered tables and they come out
   as clean tab-separated rows with a header line and a consistent column count.
2. **Kits are regular, but the label vocabulary is per book family.** Roughly **100 kit records**
   carry `Weapon Proficiencies` (106), `Role` (99), `Special Benefits` (99), `Special Hindrances`
   (98), `Secondary Skills` (98). Others vary: `Equipment` 81, `Wealth Options` 51, `Description`
   44. `Nonweapon Proficiencies` splits across two labels — 52 plus 54 `Bonus Nonweapon
   Proficiencies` — summing to 106 and matching `Weapon Proficiencies` exactly.
3. **The Complete Priest's Handbook has a different record shape entirely.** ~59 entries with
   `Spheres of Influence`, `Races Allowed`, `Duties of the Priest`, `Followers and Strongholds`,
   `Possible Symbols`, `Powers`, `Weapon and Armor Restrictions`, `Minimum Ability Scores`,
   `Alignment`, `Other Limitations`. Not a warrior kit with different values — a second kind of
   thing, and it lands near `spec.md` §4.1's **Deity**.
4. **Tooling.** No `pandoc`, no `unrtf`; LibreOffice 26.2 **refuses to load these files**
   (`source file could not be loaded`, from the mount and from local disk, with a clean profile). A
   70-line Python stripper works and is kept at [`tools/dertf.py`](tools/dertf.py). Available:
   Python 3.14.6, Node 24.18.1, jq 1.8.1.
5. **`grep` on this machine is `ugrep`, and it silently skips files it classifies as binary.** The
   WebHelp's cp1252 HTML qualifies, so **every grep-based HTML measurement in this map was an
   undercount** until it was redone in Python — on the Complete Fighter's Handbook it skipped 47% of
   the files, and it produced two published findings that were simply false (see the corrections on
   [tickets 01](issues/01-what-the-source-yields.md) and
   [03](issues/03-prior-art-core-rules-extraction.md)). **Measure with Python, or pass `grep -a`.**
   [`tools/census.py`](tools/census.py) exists for this reason.
6. **Hardware, measured while resolving [ticket 04](issues/04-llm-assisted-extraction.md).** The
   workstation is **14 GiB RAM (~9.3 GiB available), Radeon 860M integrated, no discrete GPU, and no
   inference runtime installed** — a practical local-model ceiling around 7–9B at Q4. The **M1 Max
   with 32 GB is the only viable local inference host**, at roughly one overnight run for a full
   corpus pass.

One correction worth carrying, because it would otherwise be re-discovered: the corpus **does not**
break words mid-line. An early reading said it did; that was the stripper preserving raw `\n`, which
RTF ignores by specification. The corpus is clean.

### Settled during charting — do not re-litigate

- **The destination is the corpus; the tool is subordinate.** Rejected: making the authoring tool
  the destination. `spec.md`'s own ticket 13 already found that "authoring" is two activities with
  different owners — bulk extraction (once, a script) and spot correction (forever, discovered while
  using the Engine). Treating them as one product risks building an editor for twenty-odd kinds and
  discovering that a script did 90% of the work. **How much tool is justified is a measurement**
  ([ticket 12](issues/12-how-much-tool.md)), taken last, not a premise taken first.
- **The schema is authored here, and it is canonical.** `spec.md` §7.3 says "the schema is the
  Engine's, and published" — but the Engine does not exist, and nothing machine-readable says what a
  valid kit is. Whoever transcribes first learns most about the format. The corpus is the long pole
  (26 books); the Engine is months. Churn is absorbed by machinery the spec already mandates: the
  pack format carries its own version, and §7.3 requires *"a converter and a format version bump,
  never a second evaluator"*. The schema is born `0.x`.
- **The corpus target is the RTF tier only.** Rejected: covering the PDF-only handbooks too.
  **A3 already makes a partial corpus a first-class state** — `spec.md` §5.1 exists precisely to
  distinguish "does not restrict" from "not transcribed yet". PDF is a *different* pipeline (column
  layout, reading order, tables as coordinates), not more volume of the same, and pairing them lets
  the hard half block the easy half. Note the distinction this forces, because it reads as a
  contradiction otherwise: **ticket 03 of the v1 map scoped the *Engine* to all mechanical shapes.
  Corpus scope is a different axis and grows book by book.**
- **The pipeline is published, not generalised.** Same licence and repository posture as the Engine;
  it ships no content. What is *not* built: generic input formats, packaging, installers, an
  interface for someone who does not know what a kit is. **Publishing is free; generalising is the
  cost**, and it preserves the third-party authoring product without building it.
  Accepted and declared: **corerules v1 ships with a usability hole.** The Engine is FOSS, arrives
  empty, and packs cannot circulate — so without an accessible authoring path corerules is unusable
  by anyone but its author. The README must say so plainly rather than let it be discovered.
- **Verification is hybrid, and the asymmetry is deliberate.** Mechanical checking where it is cheap
  and objective (referential integrity, plausible ranges, row and column sums, record counts against
  the book's own index); human review only on the judgement half — kits, predicates, effects.
  **Valid against the schema is not faithful to the book**: a malformed pack does not load at all
  (§7.5) and fails loudly, but a pack that is merely *wrong* loads perfectly, and under §4 nothing
  is stored — every value is recomputed through the layers, so one wrong number in a lookup table
  poisons every Character that passes through it, silently. Tables therefore end up with a stronger
  guarantee than kits. **That is a choice, not an oversight**, and it is recorded here so nobody
  assumes otherwise a year from now.

**Language.** Artifacts in English. Conversation with Wagner in Portuguese.

## Decisions so far

<!-- one line per closed ticket: enough to judge relevance, then follow the link for detail -->

- [How much tool, and where it lives](issues/12-how-much-tool.md) — **the question this map was
  opened with, answered last and by measurement.** Eleven resolutions left exactly one piece needing
  something that does not exist, and for a mechanical reason: **nobody types a SHA-256 by hand.** So
  the tool is four commands and one template — `extract`, `check`, `review`, `attest` — where review
  **generates a static page** rather than being an application. That is nearly free by an accident of
  ticket 09: making the WebHelp the only parse target means every record's source **is already an
  HTML file**, and ticket 05's anchor points at it, so side-by-side is a template with no framework,
  no state and no server. **The original request's premise did not survive**: it asked for a tool
  reading "pdf, rtf, html, txt or any other format" and the answer is **one format**, not because
  generality is hard but because the corpus does not need it. And the map closed a door the spec had
  deliberately left open without noticing — mandatory `anchor` forbids a house rule, which §5.1
  supports outright — so **a pack declares its provenance mode**, extracted or hand-authored, with
  the per-record requirement following the declaration. A3 applied to provenance: declare rather than
  infer. **The prediction recorded in this ticket before any measurement held, and undershot** — the
  surviving interface is for reviewing, and is not even a review application.
- [The human review protocol](issues/11-human-review-protocol.md) — the reviewer compares a record
  against **its own source passage**, not the printed book and not the Engine, whose behaviour the
  ticket called the strongest test but which **does not exist yet**. Side-by-side is already paid for:
  ticket 05's `anchor` delivers the source next to the record with nothing to search. That forces a
  scoping worth stating — **the review verifies extraction fidelity, not corpus fidelity**; a TSR
  digitisation error or the pre-errata problem is a corpus-level fact pinned by ticket 02's hashes,
  and loading per-record review with it would add a problem review cannot solve. **A review that
  finds nothing leaves no trace in git** — a correction is a commit, a confirmation is silence — so
  attestation goes in a **ledger keyed by record id and content hash**, the hash making a stale
  review mechanically detectable rather than confidently wrong. Reviewing is **risk-ranked with a
  mandatory floor**, and the ranking signal is **never the model**: ticket 04 measured
  self-consistency at ρ 0.10–0.30, so model confidence is *worse than random*. The floor is
  structural rather than cautious — ticket 10 is nearly blind on the judgement half, so risk-ranking
  without a floor degenerates into reviewing nothing. And the number ticket 04 said nobody had:
  **5–15 minutes per judgement record — about 3–9 hours for the proving slice and 17–52 for the v1
  tier's Attachables**, a prediction ticket 13 must check, with the trigger written down: if it
  holds, reviewing everything is affordable and the risk-ranking is unnecessary.
- [Mechanical verification](issues/10-mechanical-verification.md) — the checks divide **twice, and
  neither division was a preference**. By what a failure *means*: **invariants** admit no judgement
  and **fail the pipeline run**, while **divergences** are statistical and expected — the two
  renditions disagree a few percent per field — so they enter a **committed baseline**, known ones
  passing and a *new* one failing. That snapshot is the load-bearing part: without it the divergence
  report becomes exactly what v1 ticket 13 refused, *a list nobody reads because it is never empty*.
  And by **what each side can see**: the pipeline has the sources and the Engine never will, since a
  pack carrying its sources would be the error §6.5 forbade; the Engine has the active pack set and
  the pipeline never will, since cross-pack references and A3's union do not exist until a second
  pack does. **Even referential integrity splits** — within-pack to the pipeline, between-pack to the
  Engine — so §7.2's ban on implementing one thing twice does not bite, because neither side can do
  the other's job. The boundary is written as a test rather than a negotiated split: *does that side
  have the data?* What none of it claims is faithfulness — the cross-rendition check establishes only
  that two digitisations agree, not that either matches the page, and the map's accepted asymmetry
  is now visible in the table: **tables carry exact redundancy to exploit and kits carry almost
  none.**
- [Dice notation and generation methods](issues/15-dice-and-generation-methods.md) — measurement
  reframed it as it reframed ticket 06. The dice semantics §7.2 demands are **almost absent**: the
  133 `drop`/`reroll`/`arrange` occurrences were mostly ordinary English, and the whole tier holds
  **8** cases of `NdM` near *drop/keep/best/lowest*, while **1,474 of 2,146 notations sit in
  tab-delimited lines** — dice are table cell values, not prose expressions. So notation is a
  **string with a schema `pattern`**, `NdM±k`, and here ticket 06's structure argument deliberately
  **does not transfer**: what killed strings for predicates was opacity to the schema, and a regular
  grammar is fully validatable at tier one. The notation/method line needs no judgement either —
  **generation methods are the PHB's six**, `drop lowest` being Method V rather than notation. But
  the ticket's orphan turned into its largest finding: rounding was expected to belong to the Engine,
  and **division appears in pack data** — 31 occurrences inside kit effect fields, which §4.3's six
  operations cannot express. **Known unknown #4 has fired**, resolved by a closed set of computed
  operands that keeps the operation count at six and gives rounding a home where the division lives.
- [Identity: who mints pack-scoped IDs](issues/07-identity-and-id-stability.md) — **the last
  structural question on the map**, and measurement removed its central objection. The case against
  source-position IDs was that they move when the parser's segmentation moves; on the HTML there is
  barely any segmentation to move — **133 kits in 133 files and 57 Deity in 57 files, one to one with
  no exceptions**, a boundary TSR drew in 1996 on an ISO that cannot change and whose hash ticket 02
  recorded. So the ID is **`<pack>:<file-stem>`**, and the ticket's third option — a persistent
  anchor-to-ID registry — **costs nothing because it already exists**: ticket 09 made git the overlay
  and the pack the registry, so the ID is written into the record and committed, and a later parser
  deriving something else surfaces as a diff rather than a silent renumbering. **"Never reused" stops
  being a rule and becomes a property**, since a function of an immutable source cannot collide.
  Content-derived hashing was rejected decisively: under it **fixing a comma deletes one record and
  creates another**, destroying every live reference §6.5 depends on at every correction. Multi-record
  files take a positional ordinal persisted the same way — a name-derived discriminator being
  name-as-identity through the back door. Accepted and recorded: identity is tied to **one rendition
  of one product**, so the first time a new source enters, somebody maps by hand.
- [The extraction pipeline](issues/09-extraction-pipeline.md) — the ticket that **inverted and then
  shrank**. Its question stopped being *how do we join two renditions* and became *is the RTF needed
  at all*, and the measurement closed it: the HTML does not merely win on record boundaries, **it
  marks up the field labels themselves** (`<I>Weapon Proficiencies:</I>`), which beats
  labels-by-typographic-convention because the parser never has to infer what is a heading. So **the
  HTML is the only parse target and the alignment stage disappears**. The RTF is kept for the one
  role that survives: the two renditions' field counts **disagree by a few percent in both
  directions**, and that is precisely the check ticket 04 established as the only kind that works —
  a reference that is not the model, already on disk, free. **Local models only**: ticket 02 had
  already kept verbatim book text off third-party services, and **inference is a stronger form of
  egress than storage**; ticket 04 had measured that reproducibility exists only under a local pin;
  and decision 1 removed the cost objection by shrinking the model's share to the judgement half —
  hundreds of field values, not 1.27M words. **Python**, because ticket 05 made JSON Schema canonical
  precisely so the pipeline need not be TypeScript and ticket 06 left no evaluator to share. And
  **git is the overlay**: the corpus repository *is* the Engine's content folder, so a correction is
  a commit, re-extraction produces a diff, and `git add -p` is the adjudication tool — rejecting a
  separate correction layer as the same *second source of truth able to go quietly stale* that §8
  refused.
- [Which slice proves the format](issues/08-which-slice-proves-the-format.md) — **the Complete
  Thief's 24 kits, the five PHB thieving-skill tables they adjust, six to eight Deity records, and
  all five Elves subraces**: about 40 records, a session's work. Three arms rather than one, because
  **§4.1 is the largest single piece of invention in the spec** — v1 ticket 11 records that "Kit,
  Deity and Subrace are one closed shape used three times" shrank ticket 10 from designing a kit
  mechanism to designing one applicable-modifier mechanism, so half the Attachable architecture rests
  on it — and **a kit-only slice cannot test it at all**, while Subrace costs five records.
  The ticket had called format-stress versus pipeline-stress "a real trade"; **for kits it is not
  one**, since they exist in 8 books of differing rendition difficulty, so *what* to transcribe and
  *which book* are independent axes. The Complete Thief's was chosen less for its 24-against-7
  rendition gap than for what thief kits **drag in**: adjusting thieving-skill percentages pulls in
  the PHB's five tables, which makes the slice test the cross-pack reference where §5.1's A3 union
  lives, `adjust` on a lookup table, and tables at all — answering the objection that sank a
  Deity-only slice. It favours **representativeness over extremity**, because ticket 04 keeps it
  forever as the regression test and a set chosen for hard cases cannot later say whether the
  pipeline regressed or the case was always pathological. Psionics is explicitly **untouched** — v1
  known unknown #3 stands unchanged.
- [The expression language](issues/06-expression-language.md) — **the ticket split**, dice and
  generation methods moving to [ticket 15](issues/15-dice-and-generation-methods.md). What reframed
  it was a measurement: the ticket assumed the grammar had to cover what the books say, and what the
  books say is **English** — 1,478 "may not"/"cannot", 968 "must be", 595 "or more" in 1.23M words.
  **The corpus contains no expressions**; every predicate is written by hand, so the design target is
  hand-writability at volume and checkability, not source coverage. Hence a **closed, flat predicate
  vocabulary** — the same discipline §4.3 already applies one level up, decoder-enforceable where a
  recursive grammar is not, and it **dissolves §7.2's identifier requirement instead of satisfying
  it**, since with typed subjects there is no formula string to substitute into and PCGen's
  `Illumination`-contains-`MIN` becomes a category of bug that cannot occur. **Conjunction only** —
  measured, not argued: genuine disjunction between subjects occurs **4 times** in the whole corpus,
  while the 90 race lists are set membership and the 803 `unless`/`except` are §4.3's `except`.
  **Subjects derive from §3.1's kinds** via `has(kind, id)` so a v2 kind gains predicate reach for
  free, with ability and level the only scalars — and **`level` is always qualified by class**, the
  unqualified form not existing, because §6.1's sum type means `Fighter 5 / Mage 4` has no "level 5".
  Finally **predicates are structure, not strings**, which contradicts §7.1 deliberately and
  **dissolves this ticket's own trap**: PCGen's disease was three live parsers, and under structure
  there is no parser at all.
- [The pack schema, version 0.1](issues/05-pack-schema.md) — **the spine only**, and the per-kind
  record shapes deferred to [ticket 14](issues/14-record-shapes-for-the-slice.md), on the ticket's own
  warning about *designing for the books already read*: twenty-seven shapes written before a single
  record is transcribed is that error at maximum exposure, and the two tickets blocked here need only
  the spine. **JSON Schema is canonical, TypeScript generated** — the discriminator being *when* the
  schema is written, since TypeScript-first would put its home in the one language this effort has no
  reason to be in, and ticket 04 named a third consumer nobody had: a constrained decoder, which
  reads JSON Schema natively. **Provenance becomes two fields, not one**, because the page number
  §7.1 demanded was quietly doing two jobs: a `section` heading chain a human can find in a printed
  book, and an `anchor` a machine can relocate — the anchor deliberately being the same artifact
  ticket 07 will weigh as identity. A hand-filled page field was rejected as the *silently incomplete*
  state A3 exists to prevent. **The Complete Priest's records are Deity** — Kit self-destructs on
  §6.4, since "abandon being a priest of war and remain a priest" is not an operation the game has,
  and Class fails because ticket 01 found no experience progression — which forces a **correction to
  [v1 ticket 11](../v1-spec/issues/11-engine-object-kinds.md): it was right that Deity exists and
  wrong that it enters thin**, and Deity will now exercise §4.3's six operations harder than any kit.
  **The manifest names its source files by hash**, because an anchor points into a file and ticket 02
  established that "the DMG" is ambiguous without one — which also lets a pack notice its own source
  has been replaced. **One file per kind**, since per-record filenames would collide on the
  case-insensitive exFAT mirror and per-section would bind the pack to the parser's segmentation.
- [What each pack kind actually yields at source](issues/01-what-the-source-yields.md) — the
  bottleneck ticket. **Corrected after the fact**: its HTML numbers came from ugrep and were
  undercounts, so its headline claim that *neither rendition wins and the split is per book* was an
  artifact. **The HTML is uniformly better for record boundaries** — all 3,603 files titled, one
  titled page per kit in every book, at least the RTF's kit count everywhere. What survives is the
  **RTF's poverty**, measured in Python: the Complete Paladin's, Ranger's and Book of Elves expose no
  kit structure in the RTF at all — `paladnbk` has 70 labelled lines across 58 distinct labels, none
  of them fields. What the RTF is still *for* became [ticket 09](issues/09-extraction-pipeline.md)'s
  question. On tables the HTML wins in all thirteen books, and the argument is **ambiguity, not
  volume** — a first count suggesting the RTF held more rows was tab-indented prose being counted as
  data — though its markup is unevenly applied, from 692 tables in the PHB to **3** in the Complete
  Priest's. **Four record shapes** appeared where the
  ticket expected one — Kit (~146 records), Priest specialty (60, ten fields), Subrace (5, mapping
  onto §4.1), Spell — and the **PHB's 67 numbered tables map almost one-to-one onto §3.1's kinds**,
  confirming both that experience tables are keyed by class group and that Coin arrives as a table of
  exchange rates. **Page numbers exist in neither rendition**, so `spec.md` §7.1's requirement of book
  *and page* on every record is **unmeetable from this corpus** and ticket 05 must replace it. One
  measurement caution recorded for every later session: **`grep` here is `ugrep`** and its flag
  combinations silently produced wrong counts, so every number came from a Python census instead.
- [Where the corpus lives and how it is version-controlled](issues/02-where-the-corpus-lives.md) —
  **a private remote is backup, not circulation**: `spec.md` §1's posture exists so a pack never
  reaches *another user*, and a private repository has an audience of one. What settled it is that
  §8 already made backup the user's job, and a corpus of years of transcription living on one machine
  plus a removable card on the same desk is the project's largest single point of failure. **The
  corpus repository *is* the Engine's content folder** — one directory, not two, because the
  correction loop closes tightest, ticket 08's content-hash cache was built for exactly this, and two
  copies reintroduce the stale second source of truth §8 rejected. The `.git` inside it is free
  because §7.1 requires declaration over discovery. **Sources are not version-controlled — they are
  hashed**: git solves change and the sources never change, while ticket 03's two byte-different DMG
  variants make *identity* the real requirement, and the hash manifest can live in the **public**
  repository because a fingerprint is not the content. Third-party reference artifacts get the same
  treatment. **Sibling checkouts, never a submodule** — a submodule would publish the private
  repository's URL in the public one's `.gitmodules` while breaking every third-party clone. Measured
  along the way and load-bearing: **the corpus sits on a removable exFAT card**, which is the worst
  case for thousands of small JSON files under constant write, so the working tree moves to `/home`
  and the card becomes the third copy.
- [Prior art: has anyone already extracted the Core Rules 2.0 corpus?](issues/03-prior-art-core-rules-extraction.md)
  — the literal answer is the expected negative: **nobody has parsed the RTF**, which is what
  justifies [ticket 09](issues/09-extraction-pipeline.md) building a pipeline rather than adopting
  one. But the question had a bigger answer than it asked for: **the same product ships the books
  twice**, and every extraction anyone has ever done from it used the *other* rendition. Both CD
  images are on this machine; the expansion holds **13,099 HTML files with real table markup**,
  covering the whole v1 tier — so the map's hardest measured obstacle, *"no table is a table"*, is a
  property of **the RTF rendition, not the corpus**. A "correction" claiming the HTML fails on kits
  was then published against the research and **was itself wrong** — ugrep again — so the research
  agent, working from an importer's regexes and six sample pages, read this corpus more accurately
  than a session holding the disc. **Every kit has its own titled page.** Two collisions with the v1
  spec surfaced: **page numbers are not recoverable from the
  RTF** (zero `\page`, verified) against §7.1's requirement of book *and page* on every record, and
  the two renditions **share no identifier**, so anything reading from both owns an alignment problem
  by heading text. Four external cross-check datasets were found for
  [ticket 10](issues/10-mechanical-verification.md) — the only kind of check ticket 04 says works —
  and the corpus is **pre-errata**, with two byte-different DMG variants in circulation.
- [Reproducible LLM-assisted extraction of structured records](issues/04-llm-assisted-extraction.md)
  — [v1 ticket 13](../v1-spec/issues/13-how-packs-get-authored.md)'s unchecked claim is **half true,
  and the false half is the half that did the work**. The capability is real and cheap, but "from RTF
  to structured JSON" names the wrong problem: the corpus's dominant volume is *already*
  tab-delimited, so the bulk is less manual because the **tables are structured**, not because a
  model reads them — which withdraws one input to [ticket 12](issues/12-how-much-tool.md), since what
  LLM assistance actually accelerates is the judgement half, the half needing correction forever.
  **Reproducibility does not exist on a hosted API by vendor admission** — no `seed` parameter,
  `temperature` rejected outright on current models, and the cause is *batch invariance*: server-side
  batch size varies with other users' load, so output is a function of other people's traffic and
  divergence begins around token 100, inside a single kit record. Locally there is a *pin*, not
  determinism. **Every model-internal verification check fails on precisely the error this map
  fears**: self-consistency correlates with accuracy at ρ 0.10–0.30, intrinsic self-correction
  *degrades* performance, and semantic round-tripping cannot see `18` where the book says `19`. The
  rule that falls out — **a check only works if its reference is not the model** — promotes
  [ticket 10](issues/10-mechanical-verification.md) from the cheap half of verification to the only
  thing that reliably *localises* the error class. Ticket 01's three buckets are confirmed with one
  correction: **the ambiguous middle belongs to the parser, not the model.** Cost cannot carry this
  decision (~$10–$100 a pass); **human review hours dominate and no ticket has estimated them.**

## Corrections owed to the v1 spec

<!-- accumulating; whoever updates spec.md needs exactly this list -->

This map keeps finding that the v1 spec decided correctly on premises that measurement has since
changed. None of these is an oversight in `spec.md`; each is a conclusion that did not survive
contact with the corpus. Collected here so the eventual spec update has one place to read.

1. **§7.1 requires book *and page* citation on every record — the corpus has no page numbers.**
   Verified in both renditions ([ticket 01](issues/01-what-the-source-yields.md)). Replaced by a
   `section` heading chain plus a machine `anchor`
   ([ticket 05](issues/05-pack-schema.md)).
2. **§7.1 says expressions stay strings — they are structure instead.**
   ([Ticket 06](issues/06-expression-language.md)) The decision was right for the general nested
   language §7.1 assumed; the language changed, and with a closed flat vocabulary structure removes
   the parser entirely, which is what §7.2's one-evaluator rule was defending against.
3. **[v1 ticket 11](../v1-spec/issues/11-engine-object-kinds.md) decided "Deity enters thin" — it
   enters fat.** ([Ticket 05](issues/05-pack-schema.md)) Sixty records of ten fields. Right that
   Deity exists, wrong about its size, and it will exercise §4.3's six operations harder than any
   kit.
6. **Mandatory provenance forbids a house-rule pack, which §5.1 explicitly supports.**
   ([Ticket 12](issues/12-how-much-tool.md)) This map closed a door the spec left open and did not
   notice: [ticket 05](issues/05-pack-schema.md) made `section` and `anchor` required on every record
   and had the manifest name its sources by hash, and a hand-authored record has none of the three —
   while §5.1 says *"A3 subsumes the house-rule escape hatch. The escape hatch is the pack."*
   Resolved by having **the pack declare its provenance mode**, extracted or hand-authored, with the
   per-record requirement following from the declaration. A3 applied to provenance.
5. **§4.3's six operations cannot express the corpus — known unknown #4 has fired.**
   ([Ticket 15](issues/15-dice-and-generation-methods.md)) Measured inside kit effect fields:
   **31 occurrences** of halving, division or explicit rounding — *"at twice the normal cost"*,
   *"experience level divided by three (rounded down), plus one"*, *"creatures whose Hit Dice total
   no greater than half her level"*. `adjust` sums; there is no scale or divide, and the last example
   is a predicate carrying arithmetic against ticket 06's decision that predicates have none.
   Resolved by a **small closed set of computed operands** — `half(<scalar>)`,
   `<scalar>/N rounded down` — which keeps the operation count at six and preserves
   order-independence, since `adjust` still sums and only its *operand* widens. **This is the first
   correction against a closed decision rather than a premise**, and it is where §7.2's rounding
   requirement finally lands.
4. **[v1 ticket 13](../v1-spec/issues/13-how-packs-get-authored.md)'s LLM-extraction claim is half
   refuted.** ([Ticket 04](issues/04-llm-assisted-extraction.md)) The bulk is less manual because the
   tables were already delimited, not because a model reads them — so the inference that a pack
   editor's value drops proportionally does not follow.

## Not yet specified

<!-- see "Fog of war": in-scope fog you can't ticket yet; graduates as the frontier advances -->

- **What happens to an already-transcribed corpus when the schema breaks.** The format version and
  a converter are mandated by `spec.md` §7.3, but the *migration* of tens of thousands of records
  authored against `0.1` is not the same problem as versioning the format. Can't be phrased sharply
  until [ticket 05](issues/05-pack-schema.md) exists and [ticket 09](issues/09-extraction-pipeline.md)
  has said whether re-extraction is cheap. If re-running the pipeline is cheap and deterministic,
  migration may not be a problem at all — which is why this is fog and not a ticket.
- ~~Whether the Complete Priest's records are Deity, Class, or something the spec lacks~~ —
  **graduated by [ticket 01](issues/01-what-the-source-yields.md)**, which characterised them:
  **60 records with a ten-field shape** — `Duties of the Priest`, `Followers and Strongholds`,
  `Possible Symbols`, `Powers`, `Weapon and Armor Restrictions`, `Other Limitations`,
  `Spheres of Influence`, `Minimum Ability Scores`, `Races Allowed`, `Alignment` — sharing nothing
  with a kit. The source question is answered; **which kind they become is now a modelling decision
  inside [ticket 05](issues/05-pack-schema.md)**, not fog.
- **The ergonomics of correction over a multi-year haul.** Ticket 13 of the v1 map settled that spot
  correction happens in a text editor and accepted the cost. Whether that survives contact with
  years of it is a real question, but it cannot be asked until something has actually been
  transcribed and used.

## Out of scope

- **The PDF-only handbooks** — Complete Book of Humanoids, Barbarian's, Ninja's, Necromancers,
  Villains and the rest. Ruled out with the corpus-tier decision above. A different pipeline, and
  A3 makes their absence an honest state rather than a broken one. Ironically this puts the
  **Complete Barbarian's Handbook** outside the corpus — the very book `spec.md` §5.1 uses as its
  worked example for A3's union rule.
- **A generalised authoring product for third parties** — generic input formats, packaging,
  installers, a UI for someone else's books. Preserved, not built.
- **The v2 and v3 RTF that are sitting right there** — Player's Option (Combat & Tactics, Spells &
  Magic, Skills & Powers), Tome of Magic, Arms & Equipment Guide, DM Option: High-Level Campaigns,
  Monstrous Manual. Present in the same directory and out of the v1 corpus by
  [the v1 map's roadmap](../v1-spec/map.md). Proximity is not scope.
- **Building the Engine.** This map produces the schema the Engine will consume and the corpus it
  will load. It does not implement corerules.
- **Monsters and campaigns** — already out of scope for v1 and unchanged here.
