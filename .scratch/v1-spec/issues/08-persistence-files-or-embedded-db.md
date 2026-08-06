# Persistence: files on disk or an embedded database

Type: grilling
Status: resolved
Blocked by: —

## Question

Does corerules store characters and content packs as plain files in a user-visible directory,
or in an embedded database such as SQLite?

Weigh:

- **Backup and portability** — a directory of files is copyable, syncable and version-
  controllable by the user with no help from the application.
- **Hand-editing and repair** — files can be fixed with a text editor when something goes
  wrong; a corrupt database usually cannot.
- **Future sync** — the map's permanent constraint. Files diff and merge; a database needs a
  replication story.
- **Querying** — searching across characters and packs (every kit available to a 3rd-level
  dwarf fighter, say) is trivial in SQL and manual over files.
- ~~**Packaging cost**~~ — **this axis is gone.** [Ticket 02's research](./02-electron-packaging-and-release.md)
  established that `node:sqlite` is built into Electron (43.3.0 bundles Node 24.18.1; Stability 1.2,
  no flag since Node 23.4.0), so SQLite costs zero native modules, zero per-platform rebuilds and
  zero ASAR unpacking. Decide on backup, portability, sync and querying alone. One caveat to
  discharge first: that was verified through Electron's issue tracker and Node's docs, not a
  packaged-app spike — confirm with a five-minute test before relying on it.

Also: where on disk? Each OS has a conventional application-data location, and Electron
exposes them — but a user-visible, user-chosen folder may serve the backup story better.

**Unblocked — and much of the headline question is already answered.**
[Ticket 06](./06-content-pack-format.md) made a pack a **directory of JSON**, and
[ticket 07](./07-character-file-format-and-identity.md) made a character a **single JSON file**.
Plain files are therefore the source of truth, decided on their own merits in both tickets rather
than here. Two questions remain, and they are the real ones:

- **Where on disk**, given that backup and portability were the arguments that produced the file
  formats in the first place.
- **Whether a derived index exists alongside them.** The querying axis is the one thing files did
  not answer — "every kit available to a 3rd-level dwarf fighter" is trivial in SQL and manual over
  files. An index that is *derived* and rebuildable is a different proposition from a database that
  is the source of truth, and ticket 02's finding that `node:sqlite` ships inside Electron means it
  costs no native module. Confirm that with a packaged-app spike before relying on it.

## Answer

The headline question — files or an embedded database — was already answered elsewhere: ticket 06
made a pack a directory of JSON and ticket 07 made a character a single JSON file, each on its own
merits. **Plain files are the source of truth.** What was left were the two real questions.

### Decision — split by who owns the data

Not by convenience. **What you created lives where you choose; what the tool derived lives where
the tool keeps its own things.**

- **Content** — packs and characters — goes in a **user-visible folder the user picks**. The backup
  and portability arguments that produced the file formats in 06 and 07 require the data to be
  somewhere the user sees and copies, and `~/.config` is hidden by convention and never backed up.
  Concretely: ticket 13 put the whole transcription under git, and a git repository inside
  `~/.config/corerules` is in the wrong place.
- **Application state** — window position, last character opened, preferences — and the derived
  cache go in **the OS convention path** (`app.getPath('userData')`). The cache belongs there
  *because* it is rebuildable and machine-local: in a visible folder it would be backed up and
  synced pointlessly, and worse, would travel to machines where it is stale. The OS path is the
  right home for what must **not** travel.

Two implementation notes, recorded rather than asked: **one content root, not multiple libraries** —
generality v1 does not need, and it would complicate ticket 03's active pack set. And **the
first-run default must be visible on all three systems** — `~/corerules` serves on Linux, and
Documents is the visible analogue on Windows and macOS. Ticket 09 needs this for the README.

### Decision — no persistent index; structures are built in memory

A measurement decided it. Ticket 01 recorded the Roll20 sheet's file sizes — 1.01 MB of wizard
spells, 857 KB of priest spells, 192 KB of weapons, 57 KB of non-weapon proficiencies — and that is
very nearly the whole 2e corpus, in JavaScript. As structured JSON the full corpus should land in
the tens of megabytes, which **fits in memory comfortably** in a desktop application.

Three reasons, the second being decisive:

1. **Ticket 13 already forces the whole pack to be read.** All-or-nothing loading means the engine
   parses the complete pack to validate it, so an on-disk index would duplicate what is already in
   memory.
2. **SQL does not help with the hard part.** The query that matters — "which kits can this 3rd-level
   dwarf fighter take" — requires **evaluating prerequisite predicates**, which under ticket 04 run
   in the pack's expression evaluator. SQL cannot evaluate a pack expression, so you would select
   candidates in the database and evaluate them in JavaScript anyway. The index would cover the
   cheap half and miss the expensive one.
3. **A persistent index is a second source of truth that can go quietly stale.** This map rejected
   YAML, required conflicts to be reported, produced A3, and refused silent references in ticket 07,
   always for the same reason. An index reintroduces that vector and adds invalidation as a new
   problem.

**Recorded for honesty: ticket 02's `node:sqlite` finding turns out not to matter.** It removed the
packaging cost that was the main argument *against* SQLite — but the answer is that there is no
database at all, not that a database became cheap. The obstacle that research demolished belonged
to a road not taken.

### Decision — a cache of the built form, keyed by content hash

Wagner's addition, and the reasoning holds: rebuilding tens of megabytes on every launch to produce
something that has not changed is waste with no return. It lives in the OS path, per the first
decision.

**The invalidation trigger had to change, though.** Invalidating on a pack being added or removed
leaves a hole that is large in this project specifically: for the next several years **the common
event is modification** — Wagner correcting transcription. Ticket 07 built its entire drift
detection on that, and ticket 13 put the corpus under git for the same reason. A cache invalidated
only on add and remove would go stale in exactly the most frequent case: fix the fighter's THAC0,
reopen the tool, and it runs against the old content while the file on disk says otherwise — the
very failure mode used as the third argument against a persistent index, reintroduced through the
invalidation rule rather than the architecture.

So the cache is **keyed on a content hash per pack**. Any change — added, removed, edited — misses
the key and rebuilds. Rejected: explicit events plus a "rescan" command, which is a button the user
must remember to press, whose failure symptom is the tool showing a wrong number with no signal at
all, indistinguishable from a calculation bug.

### It closes a hole ticket 07 had left open

The same hash hardens **ticket 07's drift detection**, which recorded that a character stores the
pack *version* it was last validated against. But version is a field declared in the manifest, and
**nobody bumps a version for a typo fix** — so under declared versions, drift would go undetected in
precisely the most frequent case. With a content hash the character records the hash, and drift is
caught even when the manifest was not touched.

One mechanism, two problems, and it repairs a gap ticket 07 had not noticed it was leaving.
