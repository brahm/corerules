# Persistence: files on disk or an embedded database

Type: grilling
Status: open
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
