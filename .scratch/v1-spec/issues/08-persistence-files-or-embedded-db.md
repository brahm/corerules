# Persistence: files on disk or an embedded database

Type: grilling
Status: open
Blocked by: 07

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

Depends on ticket 07: the storage medium follows from what a character file is.
