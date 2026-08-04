# Release pipeline and the unsigned-app warning

Type: task
Status: open
Blocked by: 02, 12

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

Depends on ticket 02, which establishes what the tooling actually supports, and on
[ticket 12](./12-verify-adhoc-signed-macos-build.md), which decides whether macOS is shippable at
all. Constraints 02 already fixed, so this ticket does not re-derive them:

- **electron-builder, pinned to 26.x explicitly** — npm's `latest` tag is stale, and v27 is alpha
  and renames `mac.identity`.
- **Fedora 45 (~October/November 2026) makes `rpm` refuse unsigned packages.** The unsigned rpm has
  a shelf life on Wagner's own distribution — AppImage should be the documented primary Linux
  artifact.
- **Windows Smart App Control is a hard block with no per-app override.** Some Windows 11 users
  cannot run an unsigned build at all. The README must say so rather than imply a workaround.
- **Control-click → Open was removed in macOS 15 Sequoia**, so the README needs two macOS paths
  keyed by version.
- **A release created by `GITHUB_TOKEN` does not trigger an `on: release` workflow** — a two-stage
  pipeline silently never runs stage two.
- Two things in the checklist must be verified against the live pages rather than copied from the
  research: the radio-button labels under Settings → Actions → General → Workflow permissions
  (GitHub documents them functionally but never reproduces the strings), and the SmartScreen and
  Mark-of-the-Web dialog wording, which exists only in Microsoft community threads. **The README
  needs screenshots from a real Windows 11 machine.**
