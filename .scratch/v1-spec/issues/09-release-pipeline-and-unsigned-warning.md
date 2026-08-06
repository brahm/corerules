# Release pipeline and the unsigned-app warning

Type: task
Status: open
Blocked by: —

## Question

Define the release pipeline, and produce the checklist Wagner executes by hand.

Decisions to make:

- which operating systems and artifact formats v1 actually ships
- what triggers a release — a pushed tag, a manual dispatch, a merge to main
- where artifacts land (GitHub Releases) and how versions are numbered
- how the README documents the unsigned-app warning, per OS, in terms a player at the table can
  follow
- **where the app puts its files, per OS.** [Ticket 08](./08-persistence-files-or-embedded-db.md)
  split them by ownership: content in a user-visible folder the user chooses, application state and
  the derived cache in the OS convention path. The first-run default must be *visible* on all three
  systems — `~/corerules` on Linux, Documents as the analogue on Windows and macOS — and the README
  has to say where things land, since backup is the user's job by design.

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

**Unblocked. [Ticket 12](./12-verify-adhoc-signed-macos-build.md) settled macOS by experiment, and
its findings are inputs here, not questions to reopen:**

- **macOS ships, and the only way in is Terminal.** An ad-hoc signature *does* survive leaving the
  build machine — electron-builder's documentation is wrong about that — but the app arrives
  quarantined, shows *"…is damaged and can't be opened"*, and **releasing it in System Settings →
  Privacy & Security does not work**. The README must give the command:
  `xattr -dr com.apple.quarantine /Applications/corerules.app`. Signing and notarising at
  US$99/year was put to Wagner and rejected.
- **The build recipe is proven**: electron-builder 26.15.7, Electron 43.3.0, `mac.identity: "-"`,
  `hardenedRuntime: true`, the two entitlements, on `macos-latest`. A working workflow exists on
  the throwaway branch `spike/macos-adhoc-signing` and can be cribbed from.
- **Four things the first run taught, all of which belong in the checklist**: a workflow on a side
  branch is invisible until its file reaches the default branch; the branch dropdown on manual
  dispatch defaults to the default branch and choosing wrong fails late; the push trigger never
  fired at all, cause unknown; and npm's `latest` tag for electron-builder is genuinely stale, so
  the version must be pinned explicitly.
- **Cleanup this ticket owns**: delete the spike branch and
  `.github/workflows/spike-macos-adhoc.yml` from `main` once a real release workflow exists.
