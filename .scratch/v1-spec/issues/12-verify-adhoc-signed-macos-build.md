# Verify an ad-hoc signed macOS build runs on a second Mac

Type: task
Status: resolved
Blocked by: 02

## Question

Nothing to decide here — a fact has to be produced before
[ticket 09](./09-release-pipeline-and-unsigned-warning.md) can decide whether v1 ships macOS at
all.

[Ticket 02's research](../research/02-electron-packaging-and-release.md) found two primary sources
in direct contradiction:

- **electron-builder** (`mac.md:102`): an ad-hoc signature means the app "will only run on the
  machine it was built on".
- **Apple developer forum guidance** (secondary, not documentation): ad-hoc signed code copies
  between machines fine, and quarantine is the real obstacle.

The whole recommended macOS configuration — `mac.identity: "-"` plus the `disable-library-validation`
and `allow-jit` entitlements — rests on the second being true. It matters because a genuinely
unsigned arm64 build does not execute at all on Apple Silicon: Apple does not permit it, and the
resulting "damaged and can't be opened" dialog has no Open Anyway path.

**If electron-builder is right, there is no viable unsigned macOS story.** macOS becomes ZIP-only,
or leaves v1.

## The test

Two Macs are required, and the build must come from CI — building locally on the target machine is
exactly the case that cannot distinguish the two claims.

1. Configure a minimal Electron app with `mac.identity: "-"` and the two entitlements.
2. Build it on `macos-latest` in GitHub Actions (arm64, macOS 26). Produce both DMG and ZIP.
3. Download the artifact **on a different Mac** — not the runner, and not a machine that has ever
   built it.
4. Record, for each of DMG and ZIP: does it launch? Which dialog appears, verbatim? Is there an
   Open Anyway path, and where — Finder, or System Settings → Privacy & Security?
5. Repeat after `xattr -dr com.apple.quarantine`, and record whether that alone is sufficient.
6. If an Intel Mac is available, repeat there — Apple's restriction is arm64-specific, so the two
   architectures may differ and the README would then need separate instructions.

## What blocks this

Development happens on Fedora. **Does Wagner have access to any Mac, let alone two?** If not, this
cannot be tested and ticket 09 must decide macOS's fate without the fact — in which case the
honest default is to ship macOS **ZIP-only with the quarantine command documented**, and say in
the README that macOS is untested.

Answer for this ticket should record the observed dialogs verbatim, since the README has to
reproduce them.

## Answer

Run: https://github.com/brahm/corerules/actions/runs/31127678832 — `macos-latest` (macOS 26,
arm64), Electron 43.3.0, electron-builder 26.15.7, built twice: `adhoc` (221.8 MB) and `unsigned`
(224.0 MB). Scaffold on the throwaway branch `spike/macos-adhoc-signing`. Tested on Wagner's
MacBook Pro M1 Max — a machine that did not build it.

**A second physical Mac was never needed.** The runner is the building machine; any other Mac is
the second. This ticket's own text said "not the runner", and the repeated claim that two Macs were
required was simply wrong.

### The contradiction is settled: electron-builder's documentation is wrong

**An ad-hoc signature survives leaving the build machine.** `mac.md:102` says such an app "will
only run on the machine it was built on"; it ran on a machine that did not build it. Apple's
guidance was right — **quarantine is the obstacle, not the signature**.

### But the escape hatch does not work, which is worse than the research inferred

This establishes what [ticket 02's research](../research/02-electron-packaging-and-release.md) had
listed as item 2 under "Things I could not establish" — whether a fully unsigned app shows an Open
Anyway button. The answer is worse than the inference:

1. macOS showed the **second** dialog — *"…is damaged and can't be opened"*, offering to move it to
   the Trash — not the recoverable *"Apple cannot check…"*.
2. **System Settings → Privacy & Security does offer a release path, and releasing the app there
   did not make it run.** The research inferred there would be no button; in fact there is one, and
   it is ineffective. An inference that predicted the outcome for the wrong reason.
3. **Only `xattr -dr com.apple.quarantine` worked.** Terminal is the sole route.

### Both builds behaved identically

The `unsigned` build behaved exactly like the `adhoc` one — blocked, then working after `xattr`.
**Left unestablished:** whether it was genuinely unsigned. `-c.mac.identity=null` may have passed
the *string* `"null"` rather than a null value, and the `codesign` output logged by the run was
never read, since GitHub's logs endpoint requires authentication even on a public repository (403).
This does not affect the decision, because v1 ships the ad-hoc configuration either way.

**Architecture was not read off the window, but it is determinate**: the spike's build config
targets `arch: ["arm64"]` only, so the app carries no x86_64 slice, and Rosetta translates x86_64
*to* arm64 rather than the reverse. An arm64-only app running on an M1 Max ran natively.

### Decision — macOS ships in v1, and the README documents the Terminal command

```
xattr -dr com.apple.quarantine /Applications/corerules.app
```

Rejected: **signing and notarising** with a paid Apple Developer account (US$99/year, recurring,
which would make installation entirely normal), and **dropping macOS from v1**.

Wagner's call. The reasoning weighed against it and recorded here: the map fixed "unsigned builds,
not a commercial product" during charting, **without knowing this cost**, and ticket 02 already
amended that note once. macOS is the only one of the three platforms where **no graphical path
exists at all** — Windows has "More info → Run anyway" and Linux has `chmod +x`, while Windows Smart
App Control (which has no override) hits only some users, whereas this hits **every** Mac user. That
was put, and A was chosen anyway: $99/year recurring is disproportionate for a hobby tool, and the
command is one line run once.

### Process findings, for [ticket 09](./09-release-pipeline-and-unsigned-warning.md)

These came out of getting the run to happen at all, and belong in its checklist:

- **The build recipe works.** electron-builder 26.15.7 with `mac.identity: "-"`,
  `hardenedRuntime: true` and the two entitlements produced DMG and ZIP on `macos-latest` with no
  errors. It had never been run anywhere before.
- **A workflow living only on a side branch is invisible.** GitHub builds the Actions list from the
  **default branch**, so the repository showed the onboarding page and the API reported zero
  workflows registered — with the file demonstrably present on the branch. It only appeared after
  the file was committed to `main`.
- **The branch dropdown on manual dispatch defaults to the default branch**, and choosing wrong
  fails late and silently: the first dispatch ran against `main`, which had the workflow but not the
  scaffold, and died at `npm ci`. The spike's workflow now hardcodes its checkout ref.
- **The push trigger never fired**, before or after Actions was confirmed enabled; every run came
  from manual dispatch. Cause not established.
- **`latest` on npm is stale for electron-builder** — 26.15.3 against 26.15.7 on the `v26` tag,
  exactly as ticket 02 reported. Verified again here.

### Cleanup outstanding

The branch `spike/macos-adhoc-signing` and the workflow file `.github/workflows/spike-macos-adhoc.yml`
on `main` are both throwaway. They are left in place for ticket 09 to crib from, and should be
deleted once it has a real release workflow.
