# Where the corpus lives and how it is version-controlled

Type: grilling
Status: open
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
