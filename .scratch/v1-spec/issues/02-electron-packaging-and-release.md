# Electron packaging and cross-platform release

Type: research
Status: resolved
Blocked by: —
Findings: ../research/02-electron-packaging-and-release.md

## Question

What is the current state of the art for packaging and releasing an **unsigned** Electron
application to Linux, Windows and macOS from GitHub Actions?

- electron-builder versus Electron Forge — which is the live default, and why
- which artifact formats to ship per OS (AppImage / deb / rpm; NSIS / portable exe; dmg / zip)
- exactly what the end user sees when launching an unsigned build on each OS, and the
  documented steps to get past it
- what a GitHub Actions release workflow looks like for a build matrix across three runners
- which repository settings and token permissions a first-time Actions user must configure by
  hand, and which are defaults

Development happens on Fedora Linux, so macOS and Windows artifacts must come from CI runners.

## Answer

Full findings, with citations: [`../research/02-electron-packaging-and-release.md`](../research/02-electron-packaging-and-release.md)
— researched against the electron-builder tree (`53f649b5`), the Electron Forge tree (`36512372`),
Electron's in-repo docs, the GitHub REST and npm registry APIs for measured activity, the
`actions/runner-images` README, GitHub's own docs source, Apple's Platform Security guide and
developer news, Microsoft Learn, and the Fedora wiki.

**Headline: the build tool was the easy question. The real finding is that "unsigned" is not a
uniform posture across the three operating systems — it means "one config line" on Linux, "a
documented five-click detour" on Windows, and "the app does not start at all" on macOS.**

### Build tool: electron-builder

Measured 2026-08-04, not from reputation. electron-builder: 41 open issues, 116 closed against 72
opened in 90 days, 442 commits over 52 weeks, 13.3M npm downloads/month. Forge: 224 open issues,
35 closed against 13 opened, 151 commits (heavily dependabot), 4.0M downloads. Both alive;
electron-builder roughly 3× more active.

But activity is not what decides it. Two structural facts from Forge's own tree and docs are:

- **Forge has no AppImage maker and no NSIS maker.** Its Windows default is Squirrel.Windows,
  which electron-builder's own docs label "Legacy (not recommended)".
- **Forge's documentation has no CI or GitHub Actions guide at all** — `/guides/ci` 404s. For a
  maintainer who has never used Actions, that is disqualifying on its own.

electron-builder even ships Forge makers for NSIS and AppImage, with a note recommending a full
migration if you need publishing.

**Pin 26.x explicitly.** There is a live trap: npm's `latest` dist-tag is stale at 26.15.3 while
the `v26` tag is 26.15.7. v27 is alpha, ESM-only, requires Node ≥22.12, and renames
`mac.identity` → `mac.sign.identity`.

### The macOS problem — a v1-scope question, not a README question

Three facts compose into a silent failure:

1. Apple: *"A Mac with Apple silicon doesn't permit native arm64 code to execute unless a valid
   signature is attached"* (Platform Security guide, Rosetta 2 section).
2. electron-builder: if no valid certificate is found, signing is **skipped entirely** — it does
   *not* apply an ad-hoc signature automatically (`code-signing-mac.md:5`).
3. `macos-latest` is now **macOS 26 on arm64** (`actions/runner-images`).

The result is the *"damaged and can't be opened"* dialog, which — unlike *"Apple cannot check it
for malicious software"* — has **no Open Anyway path**. Electron's own code-signing doc leads with
a screenshot of exactly this.

The mitigation is `mac.identity: "-"` (ad-hoc) plus the `disable-library-validation` and
`allow-jit` entitlements, which converts an unrecoverable failure into a documented five-click
workaround. **This is a build-configuration decision, not a README decision** — which is why it
lands in the spec rather than in the release checklist.

Also current, and it changes the README's shape: **Control-click → Open was removed in macOS 15
Sequoia** (Apple developer news, 2024-08-06). The README needs two paths keyed by macOS version;
Apple's exact numbered steps for macOS 26 Tahoe are captured in the findings.

**The load-bearing uncertainty.** electron-builder's `mac.md:102` says an ad-hoc signed app "will
only run on the machine it was built on"; Apple developer forum guidance (secondary) says ad-hoc
signed code copies between machines fine and quarantine is the real obstacle. These contradict,
and the entire recommended macOS configuration rests on which is true. **If electron-builder is
right, there is no viable unsigned macOS story at all** and macOS is ZIP-only or out of v1. Split
out as [ticket 12](./12-verify-adhoc-signed-macos-build.md), which blocks
[ticket 09](./09-release-pipeline-and-unsigned-warning.md).

### Artifact formats, and a Linux constraint with a deadline

- **Linux** — AppImage, rpm, deb. But **Fedora 45 (an accepted change, roughly October/November
  2026) flips `%_pkgverify_level` to `all`, and `rpm` will refuse unsigned packages.** The unsigned
  rpm therefore has a shelf life measured in months, on the maintainer's own distribution. That
  argues for **AppImage as the documented primary Linux artifact**, with the rpm as a convenience
  that may have to be dropped or signed later.
- **Windows** — NSIS, optionally portable. SmartScreen is the familiar "More info → Run anyway"
  detour, but **Smart App Control is a hard block with no per-app override** for unsigned code:
  some Windows 11 users simply cannot run corerules at all. That is not a documentation problem
  and the README should say so plainly rather than imply a workaround exists.
- **macOS** — DMG and ZIP, both architectures, subject to ticket 12.

### This invalidates ticket 08's packaging premise

**`node:sqlite` is built into Electron.** Electron 43.3.0 bundles Node 24.18.1; `node:sqlite` is
Stability 1.2 and needs no flag since Node 23.4.0; Electron treats it as supported (it broke in
37.2.0 and was fixed within a day — issues #47671 / #47706). SQLite therefore costs **zero native
modules, zero per-platform rebuilds, zero ASAR unpacking**.

[Ticket 08](./08-persistence-files-or-embedded-db.md) listed packaging cost as one of its axes.
That axis is gone: 08 now decides on backup, portability, sync and querying alone. Caveat carried
forward: this was verified through the issue tracker and Node's docs, not a packaged-app spike —
08 should confirm with a five-minute test before relying on it.

### For ticket 09 specifically

- **A release created by `GITHUB_TOKEN` will not trigger an `on: release` workflow.** A two-stage
  pipeline silently never runs stage two.
- `permissions: contents: write` in the workflow file is the fix for the read-only default token;
  changing the repository setting is the fallback. Exact click paths for both are in the findings.
- Electron 44 drops Windows ia32 and Linux armv7l; the v43 line is supported to January 2027.
- The findings sketch a race: three matrix jobs each running `--publish always` against a
  non-existent draft release. An upload-artifact-then-release pattern avoids it.

### Recorded unknowns

Twelve items sit under "Things I could not establish" in the findings. Beyond the macOS one, the
ones that touch deliverables: the exact SmartScreen and Mark-of-the-Web dialog strings could only
be found in Microsoft community threads, not documentation, so **the README needs screenshots from
a real Windows 11 machine**; and the radio-button labels under Settings → Actions → General →
Workflow permissions are described functionally by GitHub but never reproduced, so ticket 09's
checklist must be verified against the live page.
