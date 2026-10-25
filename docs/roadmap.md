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
[website](https://brahm.github.io/corerules/) links every file and serves
the project's own Flatpak repository, which gets each final release
(decision P16).

The Windows 11 version (step 2) is under way: the backend and `cr` are
built and tested on Windows (phase 1), Wagner Brahm approved the
prototype of the Windows app on 2026-10-25 (phase 2, decision P21), and
the groundwork that keeps both apps the same is in place (phase 3): the
shared crate `cr-app`, one PDF of the sheet for both systems and a test
that fails when an action is in one app and not in the other. The screens
are being ported (phase 4): the library, the character sheet, the help,
Settings and About are done.
Publishing on Flathub was dropped from the plan (decision P15).

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
| 6. macOS and Windows | Other platforms over the same backend | Moved after 1.0 (P15); Windows 11 is step 2 after 1.0 |

## The revised plan (2026-10-17 to 2026-10-25)

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
| Flatpak repository | The project's own signed Flatpak repository on the website, updated after each release | Done (P16) |

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

### Step 2: Windows 11 (in progress)

Decided by Wagner Brahm on 2026-10-25 (decisions S13 and P17 to P20): the
Windows version lives in this repository, next to the Linux one, and from
then on every feature ships in every maintained version, Windows 11
included. Its interface is a native WinUI 3 interface that follows
Microsoft's design guidelines, written in Rust with windows-reactor, the
WinUI 3 library of the windows-rs project, over the same backend. It is
distributed as an MSIX package through the Microsoft Store, which signs it.
macOS is not part of this step.

How the two versions stay the same:

- Rules and data live only in `cr-store`; an interface shows them and calls
  it.
- Logic that is in `cr-gtk` today moves into a new shared crate, `cr-app`
  (screen state, checks, warnings, actions; no GTK and no Windows), before
  its screen is ported. The character sheet's PDF is made there in pure
  Rust, so both systems write the same file.
- Every action has one name, like the `sheet.pdf` that `cr-gtk` already
  uses; a test fails when an action exists in one interface and not in the
  other.
- Both interfaces take their texts from `cr-i18n`, whose test already fails
  on a missing translation.
- CI runs on Ubuntu and on Windows for every pull request, and one `v*` tag
  makes the Linux packages and the MSIX in the same release workflow.

The phases run in order:

1. **Backend on Windows (done).** A Windows job in CI lints and tests
   `cr-formats`, `cr-store`, `cr-i18n` and `cr`, and builds `cr.exe`. The
   tests link the original's data the same way on both systems, text files
   are checked out with LF on Windows too, and the data package lives in
   `Core Rules\data` in the user's Documents folder, which a packaged app
   writes to directly, so uninstalling the app keeps the characters.
2. **Prototype of `cr-win` (done, approved on 2026-10-25, decision
   P21).** In Portuguese and packaged as an MSIX: the character list
   with search, a page of the New Character assistant, an editing form, a
   confirmation dialog, opening an `.xpx` file and printing the sheet.
   Whatever blocks it is solved in `cr-win` or taken to windows-rs before
   the rest is ported.
   What it found: windows-reactor 0.100 (the release on crates.io, with
   Windows App SDK 2.4) has the controls the prototype needs, but no way
   to reach the window, the shell's Open dialog or printing, so `cr-win`
   calls Windows for those itself, on a thread of their own with the
   app's window as owner; printing draws the layout of `cr-store::sheet`
   with Direct2D through the Windows print dialog. Its ContentDialog has
   no DefaultButton, so Delete and Discard take the focus where Cancel
   should; that goes to windows-rs. WinUI's own texts (the Settings
   item, Back) follow the language chosen in Settings only in the
   package; a run without one shows them in the Windows display
   language. CI builds the MSIX
   (`crates/cr-win/package/make-msix.ps1`) unsigned, as the Store signs
   it; on a machine in Developer Mode it is installed for testing with
   `Add-AppxPackage -Register target\msix\AppxManifest.xml`.
3. **Parity groundwork (done, PR #99).** A new crate, `cr-app`, with no GTK and
   no Windows in it, holds what the two apps share above the backend:
   - the character sheet laid out on printed pages, which both apps now
     print from, measuring the text with Pango or DirectWrite;
   - the sheet as a PDF, written in pure Rust with the PDF's built-in
     Helvetica, so Save as PDF writes the same file on Linux and on
     Windows, on A4 or Letter paper as the user's region uses (decision
     P22). The GNOME app saves sheets, treasures and encounters with it,
     and the Windows app gained Save as PDF for the character sheet;
   - the catalog of the app's 51 actions, each with one name
     (`sheet.pdf`). Both apps refer to actions only through it, and
     `crates/cr-app/tests/parity.rs` fails when the GNOME app lacks an
     action, when the Windows app lacks one that `NOT_YET_ON_WINDOWS`
     does not list, or when it has one that the list still holds. The
     list has the 44 actions not yet on Windows, and the release with
     Windows waits until it is empty (decision P23).
4. **Porting the screens (in progress).** One pull request per stage,
   each merged once CI passes. Each stage first moves what the screens
   decide into `cr-app`, where the GNOME app uses it too, and then writes
   the Windows screens over it.
   - **The library and the sheet (done, PR #100).** `cr-app` gained the library's
     search and filters, the commands an entry offers, the pictures of
     items and monsters, the sheet's options and exports, and the rules of
     the user database's records. The Windows app now shows every section
     of the catalog in its navigation pane, with the search, the origin,
     party, type and magic filters, the details of any entry with its
     pictures (a monster's frames and Full Screen), the character sheet
     as the GNOME app lays it out, Print and Save as PDF of any sheet,
     export as HTML, text or Markdown, Character Sheet Options, Convert to
     Character or NPC, and Delete. Sharing the filters fixed a GNOME bug:
     the part type filter hid every entry of the other sections. 41
     actions are left on `NOT_YET_ON_WINDOWS`.
   - **Help, Settings and About (done, PR #101).** `cr-app` gained the help (the
     topic a target names, the list beside it, where a link leads, what a
     right-click on a control or a row shows, a monster's View and Print)
     and About's texts. The Windows app has a Help page in its
     navigation: the contents as a tree, the search over the topics, and
     the topic as WinHelp draws it, with its fonts, colours, tables,
     pictures, links and popups, shown in a WebView2 (decision P24), with
     Back and Print. F1 opens the help of the page shown, each entry's
     Help shows what a right-click on its row shows in the original, and
     a monster's View and Print open its topic. Settings gained the Core
     Rules Vs Player's Option message, and About the description, What's
     New, the credits, the legal notice, the original program's About box
     and Credits, and the details a bug report needs. 38 actions are left
     on `NOT_YET_ON_WINDOWS`. The same pull request fixed what the
     Windows test of the first stage found: Other Parts no longer closes
     the app, a character's details show at the default window size, and
     reals read as the records keep them (0.1) in both apps; About's
     sections open, and a record the data repeats (Garnet) is selected
     by its row.
   - **The books (done, PR #102).** `cr-app` gained the reader's zoom
     steps, the reading positions and zoom kept between sessions, Find
     over the book's folded text, and the heading a paragraph is under.
     The Windows app reads a book on a page of the window: the contents
     beside it (over it in a narrow window, scrolled to the heading the
     text is under), the book's fonts, colours, indents, tab stops and
     pictures as one web page in a WebView2 (decision P25), tables whose
     rows keep to one line, Find with every match highlighted, Zoom In,
     Out and Reset, and the place it was left. F9, Ctrl+F, Ctrl+plus,
     Ctrl+minus, Ctrl+0 and Ctrl+W work as in the GNOME reader. 31
     actions are left on `NOT_YET_ON_WINDOWS`. The Windows test found the
     help and the books blank: WinUI's WebView2 needs
     Microsoft.Web.WebView2.Core.dll, which neither Windows nor the
     Windows App Runtime has, so the build now puts it next to the app
     and the MSIX declares its classes, and a WebView2 that does not
     start says so. It also found the place kept moving back a paragraph
     on every open, click in the contents and zoom, which it no longer
     does, and tables whose last column broke a word over lines in a
     narrow window, which now keeps to one line.
   - **Character creation and Enter Character (PR #103).** `cr-app` gained
     the creation wizard, free of any toolkit: its pages from the campaign
     setting to the name, kit and appearance, then the proficiencies
     (nonweapon, languages, weapons, groups, focus, specialization and
     unarmed combat, with the kit's picks, warnings and slots lent) and
     Skills & Powers' Combat page (weapon of choice, expertise,
     specialization, shields, armor and fighting styles), what each choice
     allows, the original's questions and messages, and the help topic of
     each page; the GNOME wizard takes from it the texts and checks they
     share, and now refuses a weapon or a barred proficiency past the
     slots as it refuses the others. The Windows app's New Character goes
     through those pages, with Back, the questions as dialogs, the Racial
     Abilities and class abilities of Skills & Powers, and Enter
     Character's All Classes. Back and Next again show a page as it was
     left when nothing before it changed, and keep the details page's
     rolls. `cr-app` then gained the wizard's last pages (thieving skills
     with the kit's bonus and chosen skills, spells known and eliminated
     schools, the Expansion's wild talent test, psionics, and the
     equipment with the money, the standard pack and the kit's notices),
     Create, and two forms that editing will use too: the Skills page
     (rogue skills, the kit's bonuses, psionics and wild talents) and the
     Levels dialog with its Add Experience. The GNOME app takes from them
     the text of a power and of a class's level. The Windows app creates a
     character from start to end, and Enter Character sets each class's
     level, the hit points and the PSPs; the details page chooses the
     portrait among the installation's, the race's first, or makes one
     from a picture of the user's. 30 actions are left on
     `NOT_YET_ON_WINDOWS`. The Windows test found Enter Character missing
     from the Characters bar, which now has it, and Choose Portrait
     closing the app (WinUI refuses a tooltip inside a dialog), whose
     tiles now name their files under them. In both apps it found the
     Psionics step of a thief and psionicist showing the rogue skills
     again, in English, and saving those over the Rogue Skills step's,
     which it no longer does; the details rolled again past the class
     after the gender or the kit changed, which they no longer are; and
     climbing walls scored without Dexterity's adjustment, which the
     sheet gives it. Zero now takes the plural in Brazilian Portuguese,
     thousands are grouped as the language does it, a psionicist's
     allowance counts each kind of power, a subrace named as its race
     is said once, and the Windows labels keep their words whole in a
     narrow window.
   - Still to port: editing; the generators (NPCs, treasure,
     encounters, tables, parties); the user database editors; packages
     and Import Core Rules 2.0; Extract Data from CD….
5. **Package and Store.** The release workflow builds and tests the MSIX;
   a beta goes to a closed group through the Microsoft Store.
6. **Release with Windows**, once the beta's users confirm it, with the
   same version and tag as Linux. The Community Add-ons Collection follows
   as a package the Windows app imports.

The interface follows Windows 11's patterns rather than GNOME's: settings
cards with combo boxes, number boxes and toggle switches for the forms,
ContentDialog for dialogs, a NavigationView for the sections, the title bar
with a CommandBar, InfoBar for notices, Settings (with About inside) at the
foot of the navigation, the system's light, dark and high-contrast themes
and the Mica backdrop.

Platform details:

- **Data folder:** `Documents\Core Rules\data` (decision P19), where
  Linux has `~/.local/share/corerules/data`.
- **The original on the same machine:** its installation under
  `Program Files (x86)` or `Program Files`, and what it saved into the UAC
  VirtualStore (`%LOCALAPPDATA%\VirtualStore`) when Windows kept it from
  writing there.
- **Extract Data from CD…:** the CD by its drive letter, an ISO image read
  directly or mounted by File Explorer, or an installation.
- **Printing:** the Windows print dialog, drawing the pages of
  `cr-app` with Direct2D; Save as PDF writes the PDF of `cr-app`.
- **Help:** the app's own Help page, since Windows 11 has no WinHelp
  reader either: the topics drawn as web pages in a WebView2 (decision
  P24), and F1 for the page shown.
- **Language:** the Windows display language or Preferences, with the same
  catalogs.
- **`cr`:** `cr.exe` inside the same package, reached from a terminal
  through an MSIX alias.

Distribution: the Microsoft Store's individual developer account is free
since September 2025 and the Store signs the MSIX after certification, so
Windows SmartScreen and Smart App Control do not warn. A self-signed or
unsigned package is not used: Windows blocks it for anyone who has not
trusted its certificate by hand. The product Core Rules was reserved on
2026-10-25; the Store drops a reservation not used within three months, so
it may need to be reserved again before submission.

Needs Wagner Brahm: confirming the beta (phase 5).

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

- Confirming the Windows beta (step 2).
- The first features after 1.0, and the source of rules content without
  the discs (step 3).
