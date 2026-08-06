# Character file format and identity

Type: grilling
Status: resolved
Blocked by: —

## Question

What is a saved character?

- **Representation** — what does the persisted form contain? Only the user's choices, with
  everything derived recomputed on load; or the fully resolved sheet, snapshotted?
- **Identity** — the map's permanent constraint requires identifiers that survive travelling
  between machines, since clients will one day sync. That rules out local autoincrements. What
  identifier scheme, and what else does future reconciliation need — revision counters,
  timestamps, provenance?
- **References to content** — a character is built out of pack entries (a class, a kit, spells,
  equipment). How does it reference them, and what happens when the pack is **missing**, or
  present at a **different version** with the entry changed or gone? A character that becomes
  unopenable because a pack moved is a data-loss bug.
- **Portability** — is the file human-readable and hand-editable? Shareable with another
  player as a single file? **[Ticket 04](./04-validate-or-record.md) split this question in two:**
  Wagner's packs are authored from his own RTF and PDF copies of the books, so a pack is derived
  WotC content and sharing one is redistribution regardless of purchase, while a character is his
  own creation. Sharing is plausibly yes for a character and no for a pack — and the spec must say
  so explicitly rather than leave it implied, because the project's entire legal posture rests on
  packs not circulating.
- **Opening an invalid character** — 04 settled that loading never fails and that an invalid
  character is *quarantined*: fully readable and printable, with everything that extends it
  (levelling, XP, proficiencies, spells) locked until the violations clear. The file format has to
  make that state representable and distinguishable from "not repairable here" — the missing-pack
  case, which no edit to the character can fix.

**[Ticket 05](./05-generation-pipeline-depth.md) enlarged this ticket rather than merely unblocking
it.** Advancement is in v1, so a character is no longer a snapshot: it made choices *at each level*
— hit point rolls, proficiency picks, spells learned — and under ticket 04's hard validation a
choice's legality may depend on the level at which it was taken. **The character is a sequence of
level events.** How much of that sequence the file keeps, and whether suppression state and past
choices are stored or re-derived from it, is now this ticket's central question.

**[Ticket 14](./14-multi-class-and-dual-class-model.md) then fixed the shape of the sequence**, so
this ticket inherits rather than decides it: the class arrangement is a sum type
(`Single` | `Multi` | `Dual`), and every advance records **which class went up, the die rolled, and
what was chosen** — because hit points are *recorded randomness*, neither a choice nor a
derivation, and multi-class totals cannot be reconstructed from levels alone. Dual-class
suppression is derived from that record, never stored.

Three concrete demands fall out, and they are this ticket's real work:

- **Correcting a bad roll from 3rd level is an edit to history.** The sheet-side correction mode
  that ticket 04 made mandatory must expose the timeline, not just current fields.
- **A kit binds to a named target** — a specific class entry, or the race — once, at creation,
  never rebound. The file has to carry that binding, not just the kit's identity.
- **Proficiency debt is nominal.** An abandoned kit leaves the character owing *those specific
  proficiencies* against future slots, so the file stores a list, not a count.

**[Ticket 11](./11-engine-object-kinds.md) then named the character-side structures** this file has
to carry: Character, Class arrangement, Level event, **Attachable binding** (one shape covering
kit, deity and subrace), Proficiency debt, Weapon specialisation, Spellbook, Inventory. It also put
**magic items in v1** as a property of the Weapon / Armour / Gear kinds — so an inventory is not a
list of names but of things that modify computed values, and the file must reference them by
pack-scoped identity like anything else.

**[Ticket 10](./10-kit-modifies-parent-class.md) then made the character's own choices the topmost
layer of one uniform resolution model** — picking a proficiency is a `grant`, buying equipment is a
`grant`, spending a slot is an `adjust` — so a level event carrying an effect *is* a layer with a
date. The character file is therefore not a record of state plus a separate record of history: it
is a stack, and **provenance has to survive it**, because ticket 04's refusals must name their
cause.

10 also handed this ticket a question it had already been carrying: **what happens when a
referenced pack is absent, or present at a version where the target changed.** Naming a target is
settled — pack-scoped opaque IDs, display names as presentation — but the missing and moved cases
are this ticket's.

**[Ticket 06](./06-content-pack-format.md) then fixed what is being referenced**: a pack is a
directory of JSON declared by a manifest, versioned Foundry-style with a three-way `compatibility`
range. Two things bear directly here — **the character's active pack set is load-bearing, not
decorative** (06 uses it to disambiguate `phb:set-snares` from `cbarb:set-snares`, so the file must
record it precisely), and **the pack format carries its own version** separate from the engine's,
which is a third version the character may have to reconcile against.

Unblocked: 05 settled what a character contains, 14 settled how it holds classes, 11 named the
structures, 10 made choices a layer, 06 fixed what a reference points at.

## Answer

Five tickets emptied into this one, but nearly all of it arrived **already decided** —
representation by choices rather than derived values, the structures named by 11, the class
arrangement as a sum type, event-by-event recording, the kit binding, nominal proficiency debt, the
active pack set. What was genuinely open was identity, pack drift, and the file's own shape.

### Decision — the character and every level event carry a UUIDv7

Not individual choices within an event.

The map's constraint is precise and was followed to the letter: sync is out of v1, but **no v1
decision may foreclose it** — "stable global identifiers rather than local autoincrements". That
does not say build sync. It says do not shut the door.

**The argument is retrofit, not elegance.** With identity only on the character, reconciling two
machines is a whole-file operation — last-writer-wins or a manual merge. With identity on each
event, it is per-event: two machines that advanced different levels both land. And that difference
**cannot be added later**, because characters created in v1 without event IDs will never carry the
information. It is exactly the shape of decision the permanent constraint exists to catch. The cost
is sixteen bytes per event.

Identity on each *choice within* an event was rejected as building sync rather than leaving the
door open: two machines editing the same event is a real conflict that no identifier scheme
resolves, and it needs a human either way.

**UUIDv7 over v4**: the same uniqueness guarantee, but time-ordered, so in an append-oriented log
file order is chronological order with no extra field.

### Decision — live references, with drift detection

The character records which pack version it was last validated against. On open, if the pack has
moved, the engine re-validates and **reports what changed**. Loading still never fails (ticket 04).

**Corrected by [ticket 08](./08-persistence-files-or-embedded-db.md): the recorded value is a
content hash, not the declared version.** A version is a manifest field, and nobody bumps one for a
typo fix — so under declared versions drift would go undetected in precisely the most frequent
case, which is Wagner correcting transcription. 08 needed a per-pack content hash anyway to key its
cache; the same hash serves here.

**One option eliminated itself, and the reason is worth keeping.** The obvious answer to pack drift
is for the character to snapshot the pack data it used. That is impossible here — not for size, but
because ticket 04 settled that **characters may circulate and packs may not**. A character carrying
pack data *is a pack in disguise*, and sharing it would distribute WotC-derived content. The
project's entire legal posture would fall to a file-format decision.

Rejected: silent live references, because ticket 04's whole posture is never to be wrong in
silence, and a character whose THAC0 changed between two openings with no notice is exactly that —
and indistinguishable from a bug.

Rejected: quarantine on drift, on a practical argument. **Pack drift will be constant for years.**
Wagner is transcribing 24 books; every extraction session moves some pack. Under quarantine every
character would be locked on every pack edit, and quarantine blocks advancement — the tool would be
unusable precisely during the period it is most in use. Same reasoning that rejected
restrictive-by-omission in ticket 04.

Live references also have a property that fits the way this corpus will be built: **a transcription
fix propagates.** Correct the fighter's THAC0 once and every affected character corrects itself,
with a report. Under pinned versions you would repair character by character — the worst outcome
given that the transcription is hand-made and will contain errors.

The **removed entry** case falls out: a dangling reference is not repairable by editing the
character if the entry is genuinely gone, but is repairable by choosing another — and the engine can
tell which case it is, because an opaque never-reused ID means a disappearance is a disappearance
rather than a rename. This is precisely what PCGen's `migration.lst` tries and fails to remedy,
because there identity was the display name.

### Decision — cross-user sharing is not a v1 goal

Portability means **across Wagner's own machines**. Ticket 04 established that a character may
legally circulate, being his own work rather than derived content — but that is a legal fact, not a
functional one.

**A shared character is useless without the packs it references, and packs do not circulate.** The
recipient would have to have transcribed the same books — and, under pack-scoped IDs, transcribed
them **with the same pack identifiers**. If one names theirs `phb` and the other
`players-handbook`, nothing resolves. For hand-authored packs there is no registry or convention
that could guarantee it, and building one would be coordination machinery this project has no
reason to own.

The map's permanent constraint says "sync between clients — LAN or internet", and multi-user has
been in *Out of scope* since charting. **This must be stated in the spec rather than left implied**:
the wording carried over from ticket 04 — "shareable with another player as a single file" —
suggests it works, and it does not.

### Decision — corrections rewrite in place

Ticket 14 established that fixing a bad roll from 3rd level is an edit to history. It rewrites the
event; the wrong value is gone. Rejected: a strictly append-only log where a correction is a new
event superseding the old.

Consistency with the identity decision above is the reason. Append-only correction is what makes
automatic reconciliation work — two machines correcting the same event would not conflict, both
corrections landing with the later winning — and that is *sync*, which the map said not to build.
Under rewrite, that case becomes a conflict needing a human, which is the same standard already
accepted for event-level identity.

Rewriting also keeps the file legible: under append-only the character has a raw state and an
effective state, so reading it means applying corrections over events, and the correction UI must
show the second while editing the first. For a format that gets opened in a text editor when
something goes wrong, that is one layer of indirection between the user and the problem.

**Recorded as the cost: auditability is gone.** Under rewrite, "why did my hit points change?" has
no answer in the file, because the old value no longer exists.

### Settled by precedent, not by decision

- **A character is a single JSON file** — unlike a pack, which is a directory. Packs are directories
  because they are enormous and half-authored; a character is neither, and it is the thing one
  actually hands to someone. Same JSON profile, same validator, same tooling.
- **Quarantine is derived, never stored.** Validation runs on open and the result *is* the state.
  Same precedent ticket 14 set for dual-class suppression.
- **Provenance is likewise derived**, not stored: under ticket 10's layers, walking the stack
  produces it.
