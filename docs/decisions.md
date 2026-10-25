# Decision log

Every decision that set the project's scope, changed how the app behaves
compared with the original AD&D Core Rules 2.0 Expansion, or left a feature
out on purpose. The [README](../README.md#differences-from-the-original)
gives the user's view of the same differences; the
[roadmap](roadmap.md) says what comes next.

Each entry has the date it was decided, what was decided, why, and where it
landed (pull request or release). Entries marked **(code)** were not asked
for separately: they are documented in the code as places where the app
deliberately does not copy the original, and are listed here so that the
log is complete.

## Scope and direction

| # | Date | Decision | Why | Where |
| --- | --- | --- | --- | --- |
| S1 | 2026-10-09 | Build a Linux version first, in Rust with GTK 4 and libadwaita, following the current GNOME guidelines. macOS and Windows come only after the backend and the databases are complete. | One platform done well first; the backend crates have no GTK dependency so other front ends can reuse them. | Original plan; `cr-formats`, `cr-store` |
| S2 | 2026-10-09 | The app reads and writes the original program's own files, byte for byte where possible, and stays backward compatible with data made by the original. The original's files are the only store; no separate database was added. | Characters, monsters and packages must open in either program. | `docs/formats` |
| S3 | 2026-10-09 | Public repository `brahm/corerules` under the MIT license, with no TSR or Wizards of the Coast content in it. The app reads the rules data from a copy the user owns. | Copyright of the rules content. | First commit |
| S4 | 2026-10-10 | An edit never changes a character's race or creation classes. Taking levels in another class (dual-classing) is allowed only for the races the rulebook allows, which in the standard data is the human. | Player's Handbook rules, as the original's Dual Class… button applies them. | 0.2.0 |
| S5 | 2026-10-17 | Everything new follows the Expansion. A Core Rules 2.0 installation opens read-only, and Import Core Rules 2.0… converts its characters and records into an Expansion installation at once, the way the Expansion converts a 2.0 package. The 2.0 NPC, treasure and encounter generators were removed. | Scope is creating, editing and exporting in the Expansion's format; 2.0 is only a source to import. | 0.6.0 |
| S6 | 2026-10-17 | Map Maker II, the Campaign Mapper and the dice roller are out of scope. Campaign Mapper drawings were removed from the app and `cr`: an imported package's drawings are ignored, and an edited package keeps them untouched. | Decided by Wagner Brahm; the maps were the least known format and the dice roller adds nothing to the toolkit. | 0.5.0, 0.6.0 |
| S7 | 2026-10-17 | No integration with Core Rules 1.0 or with the original's patch files; the test corpus uses the 2.0 and Expansion discs only. | Decided by Wagner Brahm. | Plan revision 0.6.0 |
| S8 | 2026-10-17 | No `.mon` file: monsters travel inside `.xpx`/`.xpt` packages, as in both originals. | The format never existed; the original's monster Export/Import buttons point to the DM Toolkit. | 0.5.0 |
| S9 | 2026-10-17 | Rules texts the original keeps in its executable (intelligent weapon abilities, the turn undead table, the About box, control help) are read from the user's executable at run time instead of being written into the code. | Keeps TSR text out of the repository. | `cr-store`, PR #29 |
| S10 | 2026-10-19 | Where the original finds a help topic by a fixed name (psionic disciplines, fighting styles, weapon groups, three monsters), the app does the same, with those names in its code. | Names used as fair use, decided by Wagner Brahm. | PR #30 |
| S11 | 2026-10-20 | Parity rule: only what was explicitly taken out of scope stays out; everything else the original does must be replicated, or given an equivalent. | Decided by Wagner Brahm while the kit rules were being done. | Phase 1 (PRs #32 to #74) |
| S12 | 2026-10-24 | After 1.0 the app improves on the original and moves away from it step by step, until it no longer needs the original's files to work. | Decided by Wagner Brahm with the release candidate. | [Roadmap](roadmap.md) |

## Left out on purpose

| # | Date | Feature | Why |
| --- | --- | --- | --- |
| X1 | 2026-10-17 | Map Maker II and the Campaign Mapper (drawing, viewing, import of `.fcw` drawings) | Out of scope (S6). |
| X2 | 2026-10-17 | The dice roller | Out of scope (S6). |
| X3 | 2026-10-17 | Core Rules 1.0 files and conversion | Out of scope (S7). |
| X4 | 2026-10-17 | Creating NPCs, treasure or encounters in a 2.0 installation | Everything new follows the Expansion (S5). |
| X5 | 2026-10-19 | The original's animations and sounds, the animated assistant of the Help menu, and the links to TSR's update and web pages | Left out of the parity audit; the pages no longer exist. |
| X6 | 2026-10-23 | The User Options settings other than turning the "Core Rules Vs Player's Option" message back on (which went to Preferences) and the Options › Interface and Updates pages | They configure Windows-only behavior. |
| X7 | 2026-10-19 | The `.tar.gz` output of the old `extract-data.sh` script | Extraction moved into the app (F6). |
| X8 | 2026-10-22 | Sell Character Points and Buy Character Points on the Equipment page | The original always hides both buttons (0x5d856a). |
| X9 | 2026-10-23 | Analyze and Repair Database and Delete User Data | The original deletes its own Database menu when it starts (`DeleteMenu` at 0x4b6f6a), so no user reaches them. Checked under Wine. |
| X10 | 2026-10-19 | Evermore's monthly goodies (and their copies in the AD&D Core Rules User Database) in the Community Add-ons Collection | They were published by the original's maker, not by the community. |
| X11 | 2026-10-17 | Map add-ons in the Community Add-ons Collection | Maps are out of scope (S6). |

## Features the original does not have

| # | Date | Feature | Where |
| --- | --- | --- | --- |
| F1 | 2026-10-10 | Campaign setting as the first step of New Character. The original has no settings; each setting only narrows the races and classes the installed databases offer, following its campaign book (`settings.rs`); Generic offers everything. | PR #4 |
| F2 | 2026-10-10 | The `cr` command-line tool, which does most of what the app does from a terminal. | 0.1.0 onward |
| F3 | 2026-10-11 | Character sheet saved as PDF, besides printing and the original's TXT and HTML exports. | 0.3.0 |
| F4 | 2026-10-17 | Interface in English (US), Português (Brasil), Español, Italiano, Français and Deutsch. The app follows the system language and Preferences can choose another; rules texts from the original files, `cr`, exports and sheets stay in English. The pt-BR terms ("perícias comuns", "Combatente", "companhia", PdM, PP, CdT) were approved by Wagner Brahm. | PR #7, 0.7.0 |
| F5 | 2026-10-17 | Packages written by the later patched Expansion import (the CD's `TSRP3.EXE` rejects them). | 0.7.2 |
| F6 | 2026-10-19 | Extract Data from CD… inside the app (and `cr extract-data`): reads the CD's ISO image directly, the mounted CD, an installation or a Wine prefix. Extracting the Expansion over 2.0 data moves the 2.0 data aside to `corerules-data-core20`; files copied from a read-only CD come out writable. The separate `extract-data.sh` script was removed. | PR #10, 0.8.0 |
| F7 | 2026-10-19 | Character sheet exported as Markdown in place of the Expansion's RTF. | PR #23, decided by Wagner Brahm |
| F8 | 2026-10-19 | Portraits from the user's own BMP, JPEG, GIF or PNG pictures, converted to an 8-bit bitmap in the original's palette so the original shows them too. Picture pickers for monster types, races and item icons. | PRs #18 to #21 |
| F9 | 2026-10-19 | The app's own help window, which reads the original's WinHelp files, since Linux has no WinHelp; it can print. | PRs #29 to #31 |
| F10 | 2026-10-21 | The Monster Attack editor. The original has it but hides its New and Edit buttons, so attacks only arrive by import. First dismissed, then included at Wagner Brahm's request. | PR #48 |
| F11 | 2026-10-24 | Large packages import in the background, and the Import dialog folds a category of more than 200 items into one row. | PR #76 (B01) |
| F12 | 2026-10-17 | The Core Rules Community Add-ons Collection: 672 community add-ons converted to the Expansion's format and consolidated into 36,525 items, with their 668 pictures and their authors' texts, shipped as its own self-installing AppImage on GitHub Packages (not in the releases and not inside the repository). | `crates/cr-addons`, PRs #8 to #13 |

## Behavior that differs from the original

| # | Date | Decision | Why | Where |
| --- | --- | --- | --- | --- |
| B1 | 2026-10-17 | Table Rolls lines its columns up; the text is the same. | Copying the misalignment would only make it harder to read. Confirmed by Wagner Brahm on 2026-10-23. | 0.6.0 |
| B2 | 2026-10-17 | The first Table Roll after the app starts begins from 44, the value the original finds under Wine. | The original reads an uninitialized value that comes from Windows memory, so there is no single value to copy. Confirmed 2026-10-23. | 0.6.0 |
| B3 | 2026-10-17 | The NPC generator finishes an NPC whose kit names an item missing from the database, or who runs out of priest spells, and says what was left out. | The original stops with an error; there is no result to copy. Confirmed 2026-10-23. | 0.6.0 |
| B4 | 2026-10-23 | Rebuilding the character list leaves out a character file that cannot be read and says which one. | The original lists it as an empty character, which is useless and easy to delete by mistake. Confirmed by Wagner Brahm. | PR #73 |
| B5 | 2026-10-23 | Schools… on a standard spell writes `Spells.dat` when the spell is saved, keeping the previous file as `Spells.dat.bak`. | The original writes it only from its internal Save Options screen, which users never see, so the change would be lost. Confirmed by Wagner Brahm. | PR #71 |
| B6 | 2026-10-24 | In an encounter, Re-roll rolls only the monsters whose hit points were not set by hand. The encounter file has no place for the mark, so it holds while the encounter is open. | The unpatched original rolls them all, which was a bug its later patch fixed (Wagner Brahm). Test round item B12. | PR #80 |
| B7 | 2026-10-19 | **(code)** Lowering hit points with the arrows skips a roll where the original would loop for ever (a multi-class level 1 roll that cannot give less than one point per class). | Avoids a hang. | PR #24, `advance.rs` |
| B8 | 2026-10-17 | **(code)** A table roll that must be rolled again while it is within 1 of the previous result gives up after 1,000 tries. | The original hangs on a 1d2 table after a 1 or 2. | `tables.rs` |
| B9 | 2026-10-24 | **(code)** Update Character refuses a purchase that would take character points or slots below 0, where the original lets them go negative after asking. | Test round item B07: negative counters confused testers and broke later pages. | PR #77 |
| B10 | 2026-10-17 | **(code)** The user database editors write the one changed file on each save, where the originals write every user file when their menu closes. The bytes are the same the original would write. | Smaller writes, each with a `.bak` copy. | `userdb.rs` |
| B11 | 2026-10-22 | **(code)** A user record that arrived by import with a standard record's name can still be edited as long as it keeps that name; the original refuses it. | Otherwise such records could never be fixed. | `userdb.rs` |
| B12 | 2026-10-24 | Opening an Expansion folder the app cannot write to (the mounted CD or ISO image, or a folder shared read-only with the Flatpak) copies it into the app's data folder, as Extract Data from CD… does, and opens the copy, where everything made is saved; opening the CD again opens the same copy. `cr` does the same for commands that save. A Core Rules 2.0 CD still opens read-only from the disc. | The original is installed from the CD onto the hard disk and saves there; saving onto the CD always failed with a permission error. Reported by Wagner Brahm on Ubuntu with v1.0.0-rc1. | PR #85, 1.0.0-rc2 |

## Kept the same as the original

Reported during testing and kept, because the original does the same:

| # | Date | Behavior | Test item |
| --- | --- | --- | --- |
| K1 | 2026-10-24 | Roll Again in Table Rolls replaces the previous result instead of adding to it; only the follow-up rolls of one roll are listed together. Decided by Wagner Brahm. | B36 |
| K2 | 2026-10-24 | A new Proficiency starts with MAC Bonus turned on. | B37 |
| K3 | 2026-10-24 | Move Into in a treasure bundle offers any item. | B27 |
| K4 | 2026-10-23 | View NPC and Print NPC in an encounter always print the standard sections, ignoring the character's own sheet options. | Item 24 |

## Platforms, packages and releases

| # | Date | Decision | Where |
| --- | --- | --- | --- |
| P1 | 2026-10-10 | Packages: `.deb` for Ubuntu, `.rpm` for Fedora 44 or newer, a Flatpak and an AppImage, each installed and started on clean containers by the release workflow; releases come from `v*` tags. | PR #1, 0.1.0 |
| P2 | 2026-10-10 | The repository's About text and topics are set by hand by Wagner Brahm: the cloud environment cannot change repository settings. | — |
| P3 | 2026-10-17 | The `.deb` targets Ubuntu 26.04 instead of 24.04. | 0.4.0 |
| P4 | 2026-10-17 | The Flatpak and the AppImage are built on GNOME 51 and its libraries. | 0.6.0 |
| P5 | 2026-10-19 | Every package is started once in each of the six languages, checking that every text on screen is in that language. | PR #14 |
| P6 | 2026-10-23 | Targets stay Ubuntu 26.04, Fedora 44 and GNOME 51 for the AppImage and Flatpak. | 0.9.0 |
| P7 | 2026-10-24 | Ubuntu 24.04 is no longer supported: the `.deb` requires Ubuntu 26.04's `libc6` and refuses to install on 24.04; the Flatpak runs there. Test round item B35. | PR #81 |
| P8 | 2026-10-24 | A pacman package (and its `PKGBUILD`) for Arch Linux, CachyOS and Omarchy, tested on clean Arch and CachyOS containers, added to 1.0.0-rc1. | PR #83 |
| P9 | 2026-10-24 | The release before 1.0 is 1.0.0-rc1, a pre-release. Version suffixes are written `1.0.0~rc1` in the `.deb` and `.rpm` and `1.0.0rc1` in the pacman package, so 1.0.0 sorts after them. | PR #82 |
| P10 | 2026-10-24 | Flathub publication waits until 1.0 is confirmed stable. Replaced by P15. | [Roadmap](roadmap.md) |
| P11 | 2026-10-24 | Windows and macOS builds come after the Flathub publication; until then the backend stays free of GTK. Replaced by P15. | [Roadmap](roadmap.md) |
| P12 | 2026-10-24 | Fixes found by real users ship as further release candidates (1.0.0-rc2 onward) rather than by republishing 1.0.0-rc1. | Decided by Wagner Brahm, 1.0.0-rc2 |
| P13 | 2026-10-24 | 1.0.0 is released as the final version, the repository's latest release: the code of 1.0.0-rc2 with the new About window. The release candidate was confirmed stable by its users on Fedora 45 beta, Ubuntu 26.10, Bazzite, Arch Linux and CachyOS; the release workflow adds Ubuntu 26.10 and Fedora 45 to its test systems. | Decided by Wagner Brahm, 1.0.0 |
| P14 | 2026-10-24 | The add-ons AppImage is also published as the file itself on GitHub Packages, in its Maven registry (`io.github.brahm:corerules-community-addons`), since GitHub Packages has no registry for plain files; the OCI artifact on ghcr.io and the addons release stay, the release being the direct download link. | Decided by Wagner Brahm, 1.0.0 |
| P15 | 2026-10-24 | Publishing on Flathub is dropped from the plan, after a review of Flathub's current requirements. The Flatpak is still built, tested and attached to every release. Windows and macOS come right after 1.0, and the backend stays free of GTK until then. | Decided by Wagner Brahm, [Roadmap](roadmap.md) |
| P16 | 2026-10-25 | The Flatpak is also distributed from the project's own signed Flatpak repository, published with the landing page on GitHub Pages (`https://brahm.github.io/corerules/flatpak/`), instead of Flathub. It holds the latest final release and is rebuilt from that release's `.flatpak` after every final release; the `.flatpak` files name the repository, so they update from it too. The signing key is the `FLATPAK_GPG_PRIVATE_KEY` secret. | Decided by Wagner Brahm, [README](../README.md#the-flatpak-repository) |
