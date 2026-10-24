# Project plan and roadmap

Where the project stands, what the plan was and how much of it is done, and
what comes after 1.0. The decisions that shaped it are in the
[decision log](decisions.md).

## Where we are

**1.0.0, published on 2026-10-24, the first stable release.** Its users
ran 1.0.0-rc2 on Fedora 45 beta, Ubuntu 26.10, Bazzite, Arch Linux and
CachyOS, and Wagner Brahm confirmed it stable, so 1.0.0 is the code of
1.0.0-rc2 with the new About window (PR #89). It does everything
the original AD&D Core Rules 2.0 Expansion does, except what was left out on
purpose ([decision log](decisions.md#left-out-on-purpose)). Every feature was
checked against the original programs, and the 0.9.0 beta went through a
full round of manual tests whose 57 problems were fixed (PRs #76 to #79,
plus #80 and #81 for the items that needed a decision). It ships as a
`.deb` (Ubuntu 26.04), an `.rpm` (Fedora 44), a pacman package (Arch Linux,
CachyOS, Omarchy), a Flatpak and an AppImage (both on GNOME 51), and the
Community Add-ons Collection as its own AppImage, version 1.0.0. The
[website](https://brahm.github.io/corerules/) links every file.

The next step is Windows and macOS (step 2). Publishing on Flathub was
dropped from the plan (decision P15). No issue is open on GitHub.

## The original plan (2026-10-09)

The first plan had seven phases. All of them are closed, either done or
taken out of scope.

| Phase | Content | Status |
| --- | --- | --- |
| 0. Foundation and reverse engineering | Repository, CI, the original under Wine, a specification of every data file | Done. Every file the app reads or writes is in [`docs/formats`](formats/README.md) |
| 1. Reading | Read the base and user databases and `.xpt` packages | Done. The 1.0 and patch corpus was dropped (decision S7) |
| 2. Writing and rules engine | Write user data and packages, character creation, Player's Option | Done, byte for byte with the original |
| 3. Minimal GTK app | Library, sheet, creation wizard, rules browser, printing and PDF | Done. The dice roller was dropped (S6) |
| 4. Generators | Encounters, treasure, NPCs, monsters, custom tables | Done |
| 5. Maps and Expansion | Map Maker II and Campaign Mapper, Expansion classes, Flathub, pt-BR | Expansion classes and translations (six languages) done; maps dropped (S6); Flathub dropped (P15) |
| 6. macOS and Windows | Other platforms over the same backend | Moved after 1.0 (P15) |

## The revised plan (2026-10-17 to 2026-10-24)

| Phase | Content | Status |
| --- | --- | --- |
| Parity, first audit | Convert to NPC, sheet in Markdown, the full Update Character, help and About, summaries and printing | Done (PRs #22 to #31) |
| Parity, second audit | The 30 functions the second comparison of the about 200 screens of `TSRP3.EXE` found missing | Done, one PR per item (PRs #32 to #73) |
| Parity, last screens | Core Rules Vs Player's Option message and its Preferences switch; two screens confirmed unreachable in the original | Done (PR #74) |
| Release | 0.9.0 beta with the parity work | Done (2026-10-23) |
| Test round | Manual test notebook run on 0.9.0, 57 problems fixed by severity | Done (PRs #76 to #81) |
| Release candidate | 1.0.0-rc1, README rewritten in English with every difference from the original, pacman package | Done (PRs #82 and #83) |
| Real-user fixes | 1.0.0-rc2: saving with the mounted CD opened | Done (PR #85) |
| 1.0 | Release candidate confirmed stable by its users, 1.0.0 released with the new About window and the website | Done |
| Flathub | Publication on Flathub | Dropped (P15) |

### Releases

| Version | Date | Highlights |
| --- | --- | --- |
| 0.1.0, 0.1.1 | 2026-10-10 | First packages, app icon |
| 0.2.0 | 2026-10-10 | New Character (Core Rules and Player's Option), full editing, dual-classing |
| 0.3.0 | 2026-10-11 | Experience and levels, spell slots, character sheet with PDF |
| 0.4.0 | 2026-10-17 | Treasure, encounter and NPC generators, user database editors, books, Table Rolls, Ubuntu 26.04 |
| 0.5.0 | 2026-10-17 | `.xpx` and `.xpt` packages: export, import, new and edit |
| 0.6.0 | 2026-10-17 | Parties, 2.0 as import only, Windows sorting, GNOME 51 packages |
| 0.7.0 to 0.7.2 | 2026-10-17, 18 | Six interface languages, patched Expansion packages |
| 0.8.0, 0.8.1 | 2026-10-19 | Extract Data from CD…, community add-on pictures |
| 0.9.0 | 2026-10-23 | Parity with the original |
| 1.0.0-rc1 | 2026-10-24 | Fixes from the test round, pacman package |
| 1.0.0-rc2 | 2026-10-24 | Saving with the mounted CD opened goes into a local copy |
| 1.0.0 | 2026-10-24 | First stable release, the About window, the website |

## After 1.0

The steps run in order; each starts when the one before it is closed. Bug
reports are fixed at every step, and `main` stays green.

### Step 1: confirm that 1.0 is stable (done)

The release candidate is used by real players and Dungeon Masters. Problems
they report are fixed and shipped as further candidates (`1.0.0-rc2`, …).

Done when:

- a release candidate has been in use for a testing period Wagner Brahm sets, and
  Wagner Brahm confirms it is stable;
- no issue labeled critical or high is open, and none reports lost or
  damaged characters or user data;
- every report is answered: fixed, listed as a difference in the
  [README](../README.md#differences-from-the-original) and the
  [decision log](decisions.md), or closed as the same as the original.

Then the last candidate is tagged `v1.0.0` (same code, final version
number), the README drops the release candidate notice, and the metainfo
release type becomes `stable`.

Done on 2026-10-24: Wagner Brahm's friends ran 1.0.0-rc2 on Fedora 45
beta, Ubuntu 26.10, Bazzite, Arch Linux and CachyOS, found nothing wrong,
and Wagner Brahm gave the go-ahead (decision P13).

### Step 2: Windows and macOS

The original plan kept `cr-formats` and `cr-store` free of GTK for this, and
they still are.

1. **Backend on every platform.** Add Windows and macOS runners to CI for
   the backend crates and `cr` (`cargo test` and `cargo build`). This finds
   path, file system and line ending problems before any interface work.
2. **Choose the interface.** At the start of this step, as the first plan
   said: ship the existing GTK 4 and libadwaita app on both platforms, or
   write native interfaces over the backend through a C API (`cr-ffi`).
   GTK 4 reuses the whole app and its translations and is the faster path;
   native interfaces fit each platform better and cost a new front end per
   platform.
3. **Platform details.** The data folder (`%APPDATA%`, `~/Library/Application
   Support`), Extract Data from CD… with drive letters and `/Volumes`,
   opening an installation of the original (which on Windows may run on the
   same machine, with its files under `Program Files` and possibly in the
   UAC VirtualStore), printing and PDF, the help window, the interface
   language from the system settings.
4. **Packages.** An installer for Windows (MSI or MSIX) and a `.dmg` with an
   app bundle for macOS, built and started by the release workflow as the
   Linux packages are.
5. **Signing.** Without code signing, Windows SmartScreen and macOS
   Gatekeeper warn or block the app. Signing needs a certificate for
   Windows and an Apple Developer account for notarization, both paid.

Needs Wagner Brahm: the choice of interface, and whether to pay for signing.

### Step 3: new features, and independence from the original

From 1.0 on, the app improves on the original and moves away from it step
by step, until it no longer needs the original's files to work
(decision S12). What each release contains is planned when the step
starts, from Wagner Brahm's ideas and the users' requests; this is the direction:

1. **Improvements that keep compatibility.** Features the original never
   had that still save files it opens, so users can keep both programs.
2. **The app's own data.** Data the original's formats cannot hold gets its
   own files next to the original's, which the original ignores.
3. **Content without the discs.** Today the rules, books, pictures, help
   and some texts come from the user's disc or executable. Independence
   means the app works with rules data from another source: data the user
   or the community writes in the database editors and packages, or
   content published under an open license. The AD&D 2nd Edition content
   belongs to Wizards of the Coast and is not under an open license, so
   the app will still not ship it; this is the main open question of this
   step.
4. **Leaving the original's formats.** Only when the app no longer depends
   on them, and with an export to the original's formats kept for as long
   as users ask for it.

Needs Wagner Brahm: the list of features for each release, and how far
independence goes (step 3.3).

## Decisions pending

- GTK 4 or native interfaces on Windows and macOS, and code signing
  (step 2).
- The first features after 1.0, and the source of rules content without
  the discs (step 3).
