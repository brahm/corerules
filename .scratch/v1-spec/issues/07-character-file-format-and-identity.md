# Character file format and identity

Type: grilling
Status: open
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

Unblocked: 05 settled what a character contains, 14 settled how it holds classes, 11 named the
structures, 10 made choices a layer.
