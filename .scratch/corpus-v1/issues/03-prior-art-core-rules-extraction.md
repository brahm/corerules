# Prior art: has anyone already extracted the Core Rules 2.0 corpus?

Type: research
Status: resolved
Blocked by: —

## Question

The books are not a private archive. **AD&D Core Rules 2.0 is a widely known CD-ROM product**, and
its RTF exports have been in circulation since 1996. If a community project has already parsed these
exact files into structured data, this map inherits work rather than repeating it — and if several
have tried and abandoned it, the reasons are worth more than the code.

This is the same move that [v1 ticket 01](../../v1-spec/issues/01-prior-art-2e-content-modelling.md)
made, and that ticket paid for itself: it established that nobody models 2e's rules as loadable data,
which shaped every decision after it. The question here is narrower and more concrete.

## What to find out

1. **Extractions of these specific files.** Anything that parses `phbbk.rtf`, `dmgbk.rtf` and the
   `*bk.rtf` Complete handbooks — scripts, dumps, databases, wikis whose provenance is clearly this
   CD. Filenames are the strongest search key; so is "Core Rules 2.0" with "RTF".
2. **What they produced and whether it survives.** A parser is worth more than a dump, because
   [ticket 09](./09-extraction-pipeline.md) needs a pipeline and a dump is a one-off. A dump still
   has value as a **cross-check** for [ticket 10](./10-mechanical-verification.md) — two independent
   extractions disagreeing is a free error detector.
3. **Where they broke.** Which parts of the corpus defeated them. The map's charting found tables
   flattened to tabs and a per-book-family label vocabulary; whoever went further hit worse, and
   knowing what it was is exactly what [ticket 01](./01-what-the-source-yields.md) would otherwise
   discover the slow way.
4. **Adjacent structured 2e data of any provenance.** The Roll20 `ADnD_2E_Revised` sheet's data
   files, 2e spell databases, thieving-skill tables, kit lists. v1 ticket 01 already surveyed these
   as *modelling* prior art; this asks a different question — **is any of it usable as a corpus or
   as a cross-check**, independent of how it models things.
5. **Licence and provenance reality.** Anything found is derived WotC content under the same
   constraint as our own packs. Record what a thing *is* and where it lives; do not vendor it, and
   do not assume a permissive licence on a repository means the game content inside it is licensed.

## What the answer must not do

**Do not paste book content into the findings file.** The map's hard constraint applies to research
output exactly as it applies to tickets: this repository is public. Describe structure, cite URLs,
quote field labels — never a table's contents or a paragraph of body text.

## Expected outcome, stated honestly

**Most likely: nothing directly reusable.** v1 ticket 01 found that the 2e data ecosystem is thin and
that the good prior art is architectural rather than content. If that repeats, the finding is still
worth having — it converts an assumption into a checked fact, and it is what justifies
[ticket 09](./09-extraction-pipeline.md) building a pipeline instead of adopting one.

Findings go in `research/03-prior-art-core-rules-extraction.md`.

## Answer

Findings: [`research/03-prior-art-core-rules-extraction.md`](../research/03-prior-art-core-rules-extraction.md)
(502 lines).

### The literal question: the expected negative

**Nobody has parsed the RTF.** No script, dump, database or wiki anywhere is traceable to
`phbbk.rtf`, `dmgbk.rtf` or the `*bk.rtf` handbooks. That confirms the ticket's stated expectation
and justifies [ticket 09](./09-extraction-pipeline.md) building a pipeline rather than adopting one.

### The finding that outranks it: there is a second rendition, and it is on this disk

The same product ships the books **twice**. Beside `BOOKS/*.RTF` there is `WEBHELP/` — a RoboHELP
HTML rendition — and every extraction that has ever been done from this product came from *that*,
never from the RTF.

**Verified directly, not taken from the research.** Both CD images are at
`/run/media/brahm/PocketNAS/Backup/ISO/` (`add_core_rules_2.iso`, `add_core_rules_2_expansion.iso`).
The expansion image holds **13,099 HTML files**, and `/WEBHELP/` carries a subdirectory per book —
**the entire v1 tier is present**: PHB (963 files), DMG (965), and all eleven Completes (CFH, CTH,
CPRH, CWH, CBD, CBE, CBGH, CBH, CRH, CPAH, CDH).

Two of the research's structural claims check out exactly:

- **Real `<TABLE>` / `<TR>` / `<TD>` markup exists.** The map's hardest measured obstacle — *"No
  table is a table"* — is a property of the **RTF rendition**, not of the corpus.
- **`<TITLE>` names the record and the book**: `Credits (Comp. Fighter's Handbook)`,
  `Ability Scores (Comp. Fighter's Handbook)`.

### Correction — the research overstates it, and the overstatement lands on the kits

The research had no access to the disc and inferred page structure from an importer's regexes, six
sample pages and a fan mirror. Measured against the actual image, the WebHelp is **not** one titled
record per page:

- **124 of the Complete Fighter's Handbook's 264 files carry no `<TITLE>` at all**, and those
  untitled files hold **59,746 of the book's ~86,000 words — roughly 70% of the text.**
- **Only one kit — Myrmidon, the book's own worked example — has a titled page of its own.** Every
  other CFH kit is named in the text (Amazon 47×, Berserker 43×, Noble Warrior 38×, Samurai,
  Wilderness Warrior, Peasant Hero) but has no record page. The page titled `The Warrior Kits` is
  2,260 bytes — an index, not the kits.
- The WebHelp is **not abridged**: 85,897 words against the RTF's 84,360. The content is all there;
  it is the *segmentation* that does not respect record boundaries.

**So the two renditions are good at opposite things, and the kits fall on the wrong side.** The
WebHelp is a genuine and large win for **tables**, which is exactly what the RTF destroyed. It is
**not** a solution for **kits** — the map's highest-value and least-precedented record shape, whose
regularity in the RTF (measured during charting: ~100 records across five reliable field labels) is
the better handle.

### Two facts that collide with the v1 spec

1. **Page numbers are not recoverable from the RTF.** Verified: **zero** `\page`, zero bookmarks,
   zero footnotes, zero hyperlinks across the corpus. [`spec.md` §7.1](../../v1-spec/spec.md)
   requires **book and page citation on every record** and calls it the answer to JSON's lack of
   comments. If the WebHelp does not carry page numbers either, that requirement cannot be met from
   this corpus, and [ticket 05](./05-pack-schema.md) must decide what replaces it.
2. **The two renditions can only be aligned by heading text** — there is no shared identifier. Any
   pipeline that reads tables from one and prose from the other owns that alignment problem.

### Corpus hygiene, from the research

- The corpus is **pre-errata**: TSR issued official PHB/DMG corrections the CD does not contain.
- **At least two byte-different DMG RTF variants exist** in circulation. Ours should be hashed and
  the hash recorded, or "the DMG" is ambiguous.
- **2,020 non-ASCII escapes, 54 distinct codes** across the 13 v1 books — verified independently,
  matching the research exactly. Includes **124 vulgar fractions and 19 footnote daggers**, and the
  known prior art corrupts precisely these (mapping ¼ → `1/2`, then silently dropping high bytes).
  [Ticket 09](./09-extraction-pipeline.md) must handle them explicitly; a silent drop here is a
  wrong number, which is the error class [ticket 10](./10-mechanical-verification.md) exists for.

### Cross-checks found, for [ticket 10](./10-mechanical-verification.md)

Four independent datasets usable as external references — the only kind of check that works, per
[ticket 04](./04-llm-assisted-extraction.md): EZDM's JSON THAC0/XP/saving-throw/ability tables (the
best table-level diff), two independent spell indexes that agree closely with each other, Dungeon
Craft's per-class THAC0/XP/ability-minimum databases, and a live 2026 JSON Schema for 2e spells
whose `raw_legacy_value` pattern is worth copying into [ticket 05](./05-pack-schema.md).

**Licence reality:** everything found is derived WotC content under the same constraint as our own
packs — including a GPL-3.0 repository shipping book pages verbatim, which is exactly the trap this
ticket warned about. Use them as references to diff against; do not vendor them.

### Two corrections to [v1 ticket 01](../../v1-spec/issues/01-prior-art-2e-content-modelling.md)

Dungeon Craft **does** model per-class XP and per-level THAC0 as loadable data, and a live project
**does** model 2e spells mechanically. Its headline survives — nobody models **kits**, and nobody
models the ruleset as a whole — but it is narrower than it was stated.

### What is still open

The research could not run an exhaustive code search (GitHub code search needs auth, grep.app
rate-limits, searchcode's API is dead). Its own flagged gap — how the WebHelp handles kit records —
**is now answered above**, by reading the disc rather than the mirror, and the answer is
unfavourable.
