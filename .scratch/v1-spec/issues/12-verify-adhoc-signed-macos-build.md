# Verify an ad-hoc signed macOS build runs on a second Mac

Type: task
Status: open
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
