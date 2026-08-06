# Spike: does an ad-hoc signed Electron app run on a Mac that did not build it?

**This branch is throwaway.** It is not the beginning of the implementation. It exists to
produce one fact that wayfinder [ticket 12](.scratch/v1-spec/issues/12-verify-adhoc-signed-macos-build.md)
needs, after which it gets deleted.

## Why

Two primary sources contradict each other:

- **electron-builder** (`mac.md:102`): an ad-hoc signature means the app "will only run on the
  machine it was built on".
- **Apple's developer guidance**: ad-hoc signed code copies between machines fine, and
  quarantine — not the signature — is the obstacle.

The entire recommended macOS configuration rests on the second being true. It matters because a
genuinely unsigned arm64 build **does not execute at all** on Apple Silicon: Apple does not permit
it, and the resulting *"damaged and can't be opened"* dialog appears to have no Open Anyway path.

**If electron-builder is right, there is no viable unsigned macOS story** and macOS becomes
ZIP-only, or leaves v1.

## What it builds

`macos-latest` is macOS 26 on **arm64** — the architecture the question is about. The workflow
builds twice, so one run answers two open questions:

| Artifact | Config | Question it answers |
|---|---|---|
| `adhoc` | `mac.identity: "-"` plus the two entitlements | does an ad-hoc signature survive leaving the build machine? |
| `unsigned` | no signature at all | is the "damaged" dialog real, and does it truly offer no way out? |

The runner is the machine that built them. Your MacBook is a machine that did not. That is the
whole experiment — a second physical Mac is not needed.

## Running it

1. Go to the repository on github.com → **Actions** tab.
2. If this is the first workflow in the repository, GitHub shows a banner asking to enable
   Actions. Enable it.
3. In the left sidebar pick **spike / macOS ad-hoc signing**.
4. Press **Run workflow**, choose this branch (`spike/macos-adhoc-signing`), press the green
   **Run workflow** button.
5. When it finishes, open the run and download both artifacts from the **Artifacts** section at
   the bottom of the summary page. They arrive as `.zip` files containing the `.dmg` and `.zip`
   builds.

No repository setting has to change first. The workflow only uploads artifacts — it publishes no
release — so the default read-only `GITHUB_TOKEN` is enough.

Also read the **Show what was signed** step in the log. It runs `codesign -dv` on both builds, so
the run records what actually landed rather than what the config asked for. An ad-hoc signature
reports `Signature=adhoc`.

## What to report back

Do this **on the MacBook**, for each of the two builds, and from the `.dmg` and the `.zip`
separately — quarantine behaves differently depending on how the app arrives.

For each combination:

1. Double-click to launch. **Does it open?**
2. If a dialog appears, record its **exact wording** — the title and the body — and which buttons
   it offers. The difference between *"Apple cannot check…"* and *"…is damaged and can't be
   opened"* is the whole question.
3. If it refused: is there an **Open Anyway** path? Check both Finder (Control-click → Open) and
   **System Settings → Privacy & Security**, scrolling to the bottom. Note that Control-click →
   Open was **removed in macOS 15 Sequoia**, so on macOS 26 the System Settings route may be the
   only one.
4. Then run `xattr -dr com.apple.quarantine /path/to/corerules-spike.app` and try again. **Does
   that alone fix it?**
5. If it launched, the window reports the architecture. **It must say `arm64`.** If it says `x64`,
   Rosetta translated the app and the run proved nothing about the case we care about.

Verbatim wording matters more than a summary here: the README has to reproduce these dialogs for
players who are not developers.
