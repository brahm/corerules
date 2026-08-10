# Where the corpus lives and how it is version-controlled

Type: grilling
Status: resolved
Blocked by: —

## Question

The transcribed corpus **cannot go where this map lives.** `.scratch/` is committed to
`git@github.com:brahm/corerules.git`, which is public, and `spec.md` §1 fixes the posture: a Content
Pack is derived Wizards of the Coast content and **does not circulate**.

So before anything is transcribed, the corpus needs a home. This is small, unblocked, and everything
downstream assumes an answer.

## What has to be decided

**1. Where the packs live.**

- A **private** GitHub repository. Gets history, diff, revert, branch and off-machine backup.
- A **local-only** git repository — same benefits minus the remote, and minus the question below.
- Inside the Engine's **content folder** from [`spec.md` §8](../../v1-spec/spec.md) — the
  user-visible directory the Engine reads packs from — with git initialised there or not at all.

[v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md) leaned on version control as a
reason not to build an editor: *"A pack is a directory of JSON, which puts the entire transcription
under git for free — history, diff, revert, branch."* That argument assumed git, not which git.

**2. Whether a private remote counts as circulation.**

`spec.md` says packs do not circulate. A private repository is not distribution in any ordinary
sense, but the posture was stated absolutely and this is the first thing that tests it. Decide it
explicitly rather than by omission — and note that the answer also governs whether the **source
RTF** may be pushed anywhere, which is a stronger claim than pushing derived JSON.

**3. Whether the source RTF is version-controlled at all.**

32 MB of RTF that never changes. It is the input, not the output. Cheap to track and it makes the
pipeline reproducible from a single clone; but it is also the most obviously copyrighted artifact in
the whole effort. The two halves — sources and derived packs — do not have to get the same answer.

**4. What relationship the corpus repository has to this one.**

The pipeline is published here ([the map's Notes](../map.md)); the corpus is not. So a script in a
public repo reads books in a private one and writes packs into it. Whether that is a submodule, a
sibling checkout, or a configured path is a small decision that becomes annoying to change later.

## Why this is on the frontier

It gates [ticket 13](./13-transcribe-the-proving-slice.md) absolutely — nothing can be transcribed
into a place that has not been chosen — and it is cheap. It also protects against the failure mode
where a proving slice gets transcribed into `.scratch/` out of convenience and has to be scrubbed
from a public history afterwards.

## Not in scope here

Where the *Engine* stores things at runtime. `spec.md` §8 settled that: content in a user-visible
folder the user picks, application state and cache in the OS path. This ticket is about the
authoring side, which is a different machine role even when it is the same machine.

## Answer

Three facts measured while resolving this, none of which the ticket knew when it was written:

- **The corpus currently lives on a removable exFAT card** (`/dev/mmcblk0p1`, 929 GB, 1% used).
  exFAT has no permission bits, no symlinks, and is case-insensitive — git runs there badly — and the
  card can be pulled mid-write. `/home` is ext4 under LUKS with 315 GB free.
- **The sources are 535 MB, not 32 MB**: 32 MB of RTF plus **503 MB of ISOs**, which
  [ticket 03](./03-prior-art-core-rules-extraction.md) promoted to first-class material because the
  WebHelp rendition lives inside them.
- **`gh` is not authenticated.** Not a blocker; a step.

### Decision 1 — a private remote is backup, not circulation

The posture in [`spec.md` §1](../../v1-spec/spec.md) exists so that a pack never reaches **another
user**, because that is what would make corerules a distribution channel for WotC IP. A private
repository has an audience of one; it is the same act as an external drive, minus the house.

Weighed against it and overruled: the posture was stated absolutely, and this is the first case that
tests it. What decided it is that **§8 already made backup the user's responsibility by design**, and
a corpus representing years of transcription living on one machine plus a removable card on the same
desk is the project's largest single point of failure. Losing it is not losing a file, it is losing
the years.

**This applies to derived packs only.** Pushing verbatim book text is a stronger claim and gets its
own answer in decision 3.

### Decision 2 — one directory: the corpus repository *is* the Engine's content folder

Rejected: a separate authoring repository that publishes into the content folder.

Three settled decisions make one directory the cheaper answer:

- **The correction loop closes tightest.** [v1 ticket 13](../../v1-spec/issues/13-how-packs-get-authored.md)
  designed exactly this loop — notice the error in the app, fix it in an editor, reopen the
  character, the tool reports what moved. A copy step breaks it and creates "fixed it and forgot to
  publish".
- **[Ticket 08](../../v1-spec/issues/08-persistence-files-or-embedded-db.md) already made the Engine
  notice edits by itself**, with a cache keyed on content hash precisely because for years the
  common event is modification. Editing the working tree in place is the behaviour that cache was
  designed for.
- **Two directories create the question "which copy is true"** — the same defect §8 rejected when it
  refused a persistent index: *a second source of truth able to go quietly stale*. Not worth
  reintroducing through the back door.

**The `.git` directory inside the content folder is free**, and this is not luck: §7.1 requires
**declaration over discovery**, so the Engine reads the manifest and never scans. Under a scanning
design this decision would have been unpleasant.

**The working tree lives on `/home`, not on the card** — thousands of small JSON files under constant
write is the worst case for exFAT. **The card becomes the third copy**, alongside the machine and the
private remote.

Accepted cost: while a branch holds half-finished transcription, the Engine reads *that* state. The
escape is a second clone the Engine points at — the rare case, which the rejected option would have
made the normal one.

### Decision 3 — sources are not version-controlled; they are hashed

**Git solves change, and the sources do not change.** The 1996 RTF is immutable; so are the ISOs.
History, diff, revert and branch — everything v1 ticket 13 invoked as the reason not to build an
editor — are worth exactly nothing over an immutable input. What the sources actually need is not
history but **identity**: proof that the pipeline ran over *these* bytes.

**Ticket 03 made that load-bearing rather than decorative.** Two byte-different DMG variants are in
circulation and the corpus is pre-errata, so without a recorded hash **"the DMG" is an ambiguous
phrase**, and in two years nobody can say which file a table was transcribed from. A `sha256` per
file settles it, and settles it better than git would.

Three smaller reasons that all point the same way: 503 MB of immutable binary in history is a
permanent tax that never pays for itself; git-lfs adds a quota and a configuration step; and this
keeps **verbatim book text off every third-party service**, which is the stronger claim decision 1
deliberately did not make.

Two riders adopted with it:

- **The hash manifest lives in the *public* repository.** A SHA-256 is a fingerprint, not the
  content. Publishing it makes the pipeline's inputs identifiable without distributing anything, and
  lets anyone holding the same CD verify they hold the same material. This is the map's
  "published, not generalised" posture doing real work.
- **The sources need a second copy now.** They exist in exactly one place — a removable exFAT card.
  535 MB against 315 GB free. Unlike the packs this is not years of work, but it is what makes the
  years of work redoable.

**Third-party reference artifacts are treated as sources, not packs** — the Perl importer and the
cross-check JSON datasets ticket 03 found. They are somebody else's derived WotC content: on disk,
hashed, out of git, with the public repository recording only where they came from.

### Decision 4 — sibling checkouts and configured paths; no submodule

Rejected: a git submodule, and the objection is concrete rather than aesthetic. **A submodule writes
the private repository's URL into the public repository's `.gitmodules`** — publishing the corpus's
address to everyone, while simultaneously breaking `git submodule update` for every third party who
cannot fetch it. It leaks what should stay private and breaks what should work in public.

Sibling checkouts are also **the shape the spec already uses**: §8 has the Engine read a
user-visible folder the user picks. A configured path is not a workaround, it is that same model
applied to the authoring side — and it keeps the pipeline/corpus boundary in the design rather than
resting on repository permissions.

**The pipeline therefore takes three configured paths**, not one: sources, corpus, and its own
output location. That belongs in [ticket 09](./09-extraction-pipeline.md).

### Setup checklist

Not yet performed — `gh auth login` is interactive and creating a repository is an outward-facing
act.

1. `gh auth login`
2. Create the **private** repository — the corpus, not the pipeline.
3. Clone it to the §8 content path (`~/corerules` on Linux) — this is the Engine's content folder.
4. Copy the sources to a second location on `/home`; leave the originals on the card.
5. Generate `sha256` for every source file and every reference artifact; commit that manifest to the
   **public** repository.
6. Add the corpus path and sources path to the pipeline's configuration (ticket 09 decides its
   shape).
7. Set the card up as a mirror clone — the third copy.
