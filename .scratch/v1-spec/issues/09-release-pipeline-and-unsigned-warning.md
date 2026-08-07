# Release pipeline and the unsigned-app warning

Type: task
Status: resolved
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

## Answer

### Decision — a pushed tag triggers the release, and the release is born a draft

Rejected: manual dispatch, and merging to `main`.

**The argument against manual dispatch came from this map's own experiment.** Ticket 12 spent
several rounds on it: the branch dropdown defaults to the default branch, and choosing wrong fails
*late* — the run starts, passes several steps, and dies further down. On a release that means
artifacts built from the wrong tree with nothing in the UI warning beforehand. A tag involves no web
UI at all, and it makes the version number single-sourced: the tag is the truth and the workflow
reads it, where a dispatch field can silently disagree with `package.json`.

Merging to `main` is not a release event — plenty of merges are not versions, and a README typo
would publish.

**The tag's own risk is real** — a wrong tag is awkward to retract once pushed — so the workflow
creates the GitHub Release as a **draft**. Everything is built and attached, and nothing is public
until one button is pressed. That button is the only click in the process.

### Decision — SemVer, with the major tied to the roadmap

Major 1 is the v1 milestone, major 2 is settings and psionics, major 3 is Player's Option, per
[ticket 03](./03-which-complete-handbooks.md)'s roadmap. Releases before v1 ships are `0.x`.

Rejected: SemVer tracking technical compatibility independently, which could ship "roadmap v1" as
`3.2.0`.

For a single-maintainer project, a published version that disagrees with the vocabulary the whole
map uses is gratuitous confusion. **The usual objection — that tying major to a milestone spends the
major number you need for breaking changes — does not apply here**, because
[ticket 06](./06-content-pack-format.md) gave the **pack format its own version**. The break that
matters in this project is the one that invalidates a transcription, and it already has a number of
its own.

**Three versions therefore exist and do not move together**, and the README must distinguish them:
the application version (the tag), the pack format version (ticket 06), and each Content Pack's own
version (its manifest).

### Decision — the artifact matrix

| OS | Ships | Does not ship |
|---|---|---|
| **Linux** | AppImage | rpm, deb |
| **Windows** | NSIS installer | portable exe |
| **macOS** | DMG | ZIP |

**rpm is out because of Wagner's own machine.** Ticket 02 found that Fedora 45 — an accepted change,
due around October/November 2026 — flips `%_pkgverify_level` to `all`, after which `rpm` refuses
unsigned packages. Publishing an rpm that stops installing on the maintainer's own distribution
within months of v1 is worse than not publishing one: it is a promise with an expiry date. AppImage
depends on no package manager and has no such date.

**deb is out for lack of demand, not for any technical reason.** v1's user base is one table; adding
deb later is one line in the matrix.

**NSIS over portable** because a desktop app in the Start Menu is what people expect, and the
SmartScreen detour happens once either way. Portable would dodge the elevation prompt and trade it
for "where did I put that file".

**DMG without ZIP** because after ticket 12 both need `xattr` equally, so ZIP buys no easier path.
The research raised that command-line `unzip` might not propagate quarantine, but that stayed
unverified and is still Terminal. ZIP returns trivially if auto-update ever needs it.

---

## The checklist

### Part 1 — one-time repository setup

Done once, before the first release ever runs.

1. Put this at the top of the release workflow file, above `jobs:`:
   ```yaml
   permissions:
     contents: write
   ```
   On a repository created in a personal account, `GITHUB_TOKEN` is **read-only by default**, and
   creating a Release with it fails. Granting it in the workflow is the preferred fix and needs no
   settings change. **Note the trap ticket 02 found:** naming any permission sets every unnamed one
   to `none`, so if the workflow later needs another scope, it must be named too.

2. **Only if the first release fails with a 403 on creating the release**, change the repository
   default instead:
   1. Go to `https://github.com/brahm/corerules`.
   2. Click **Settings** in the tab row across the top (Code, Issues, Pull requests, …, Settings).
      If it is not visible, use the **⋯** overflow at the end of that row.
   3. In the left sidebar, under **Code and automation**, click **Actions**, then **General**.
   4. Scroll to **Workflow permissions**, near the bottom.
   5. Choose the option granting read *and write* access for all scopes — the first of the two radio
      buttons.
   6. Click **Save** *inside that box*. There are several Save buttons on this page.
   - Leave **Allow GitHub Actions to create and approve pull requests** unchecked; it is unrelated.
   - **Verify the radio-button labels against the live page** when you get there. GitHub's
     documentation describes them functionally and never reproduces the strings, so ticket 02 could
     not confirm them.

3. **The release workflow file must be on `main`.** Ticket 12 learned this the hard way: GitHub
   builds its Actions list from the default branch, so a workflow living only on a side branch is
   invisible — the repository shows the onboarding page and the API reports zero workflows
   registered, with the file demonstrably present.

### Part 2 — cutting a release

1. Set `version` in `package.json` to the version you are about to tag, without the `v`
   (`0.1.0` for tag `v0.1.0`). Commit it.
2. `git tag v0.1.0`
3. `git push origin main --follow-tags`
4. Watch it: `https://github.com/brahm/corerules/actions`. Three jobs run in parallel, one per OS.
   Expect several minutes — each downloads Electron and builds.
5. When all three are green, go to `https://github.com/brahm/corerules/releases`. The release is
   there as a **Draft**.
6. Check the attached files before publishing. There must be three:
   - `corerules-0.1.0.AppImage`
   - `corerules-Setup-0.1.0.exe`
   - `corerules-0.1.0-arm64.dmg`
7. Click **Edit** on the draft, then **Publish release**.

**If a tag was wrong:** delete the draft release in the web UI, then
`git push --delete origin v0.1.0` and `git tag -d v0.1.0`. Nothing was public, because of the draft.

### Part 3 — what the release workflow must do

Not written yet, and deliberately: there is no application to build. The specification, with what
ticket 12 proved:

- Trigger on `push:` with `tags: ['v*']`.
- `permissions: contents: write` at the top level.
- A matrix over three runners. **Pin the runner images rather than using `-latest`** —
  `actions/runner-images` warns that `-latest` migrates over 1–2 months, and a pipeline touched
  rarely should not move under you.
- Action versions, current as of ticket 02's research and worth re-checking at implementation
  time: `actions/checkout@v7.0.1`, `actions/setup-node@v7.0.0`, `actions/upload-artifact@v7.0.1`.
- **Pin electron-builder explicitly to 26.15.7.** npm's `latest` tag is genuinely stale at 26.15.3
  — verified twice, once by the research and once by the spike. v27 is alpha, ESM-only, and renames
  `mac.identity` to `mac.sign.identity`.
- The macOS configuration is **proven working** by ticket 12 and should be copied verbatim:
  ```yaml
  mac:
    identity: "-"
    hardenedRuntime: true
    entitlements: build/entitlements.mac.plist
    entitlementsInherit: build/entitlements.mac.plist
  ```
  with `com.apple.security.cs.disable-library-validation` and `com.apple.security.cs.allow-jit`
  in the plist. Without the first, the app dies at launch on Team ID mismatch.
- Create the release as a **draft** and attach all three artifacts.
- **Do not build a two-stage pipeline.** Ticket 02: a release created by `GITHUB_TOKEN` does not
  fire an `on: release` workflow, so stage two silently never runs.

A working macOS build workflow exists on the throwaway branch `spike/macos-adhoc-signing` and can
be cribbed from.

### Part 4 — what the README must tell a player

Three separate stories. They are not variations of one warning.

**Linux — AppImage.** No gatekeeper to get past.
1. Download `corerules-x.y.z.AppImage`.
2. Make it executable: `chmod +x corerules-*.AppImage`, or Files → right-click → Properties →
   Permissions → *Allow executing file as program*.
3. Double-click it.

**Windows — installer.**
1. Download `corerules-Setup-x.y.z.exe` and run it.
2. Windows shows a blue **"Windows protected your PC"** box. This is SmartScreen reacting to an
   unsigned installer, not a virus warning.
3. Click **More info**, then **Run anyway**.
4. **State plainly that some users cannot run it at all.** If **Smart App Control** is enabled,
   it blocks unsigned software with **no per-app override**. Turning it off is possible but it
   cannot be turned back on without reinstalling Windows, so the honest advice is: if Smart App
   Control is on, corerules will not run, and that is not something the README can work around.
5. **The exact wording of both dialogs needs a screenshot from a real Windows 11 machine.** Ticket
   02 could only find those strings in Microsoft community threads, never in documentation.

**macOS — DMG.** The ugliest of the three, and ticket 12 measured every step of it.
1. Download `corerules-x.y.z-arm64.dmg`, open it, drag corerules to Applications.
2. Double-click it. macOS says **"corerules is damaged and can't be opened. You should move it to
   the Trash."**
3. **It is not damaged.** That is what macOS says about an unsigned app carrying a quarantine flag
   from being downloaded.
4. **Do not bother with System Settings → Privacy & Security.** It offers a way to allow the app,
   and ticket 12 confirmed by experiment that **allowing it there does not work**.
5. Open Terminal and run, exactly:
   ```
   xattr -dr com.apple.quarantine /Applications/corerules.app
   ```
6. Open corerules again. It works, and it keeps working.
7. Say why in one line: corerules is not signed with an Apple Developer certificate, which costs
   US$99 a year, and this project does not charge anyone.

### Cleanup

`.github/workflows/spike-macos-adhoc.yml` is removed from `main` as part of resolving this ticket —
it was only ever there so GitHub would register the workflow for manual dispatch, and that job is
done. **The branch `spike/macos-adhoc-signing` is kept**, because it holds the only proven macOS
build recipe and the real release workflow does not exist yet. Delete it once implementation has
one.
