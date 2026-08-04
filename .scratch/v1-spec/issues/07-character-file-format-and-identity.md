# Character file format and identity

Type: grilling
Status: open
Blocked by: 05

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
  player as a single file?

Depends on ticket 05: the file can't be shaped until it's known what a character contains.
