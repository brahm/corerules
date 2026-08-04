# Release pipeline and the unsigned-app warning

Type: task
Status: open
Blocked by: 02

## Question

Define the release pipeline, and produce the checklist Wagner executes by hand.

Decisions to make:

- which operating systems and artifact formats v1 actually ships
- what triggers a release — a pushed tag, a manual dispatch, a merge to main
- where artifacts land (GitHub Releases) and how versions are numbered
- how the README documents the unsigned-app warning, per OS, in terms a player at the table can
  follow

**This ticket is HITL and its output is a literal checklist.** Wagner has never used GitHub
Actions. Everything requiring the GitHub web UI — repository settings, workflow permissions,
tag creation, release publishing — must be written out as numbered steps naming the exact
pages and buttons, not as a reference to GitHub's documentation.

Depends on ticket 02, which establishes what the tooling actually supports.
