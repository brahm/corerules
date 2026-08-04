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
- **Packaging cost** — SQLite in Electron means a native module, rebuilt per platform, which
  complicates the CI matrix in ticket 09.

Also: where on disk? Each OS has a conventional application-data location, and Electron
exposes them — but a user-visible, user-chosen folder may serve the backup story better.

Depends on ticket 07: the storage medium follows from what a character file is.
