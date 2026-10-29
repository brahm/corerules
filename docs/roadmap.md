# Project plan and roadmap

Where the project stands, what the plan was and how much of it is done, and
what comes after 1.0. The decisions that shaped it are in the
[decision log](decisions.md).

## Where we are

**1.0.1, published on 2026-10-27, ready on Linux and Windows 11.** The
Windows 11 version went through a beta on the Microsoft Store, where
Wagner Brahm found no problem, and ships with Linux as 1.0.1 (decision
P26). 1.0.0, published on 2026-10-24, was the first stable release. Its users
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

The Windows 11 version (step 2) is done: the backend and `cr` are built
and tested on Windows (phase 1), Wagner Brahm approved the prototype on
2026-10-25 (phase 2, decision P21), the groundwork that keeps both apps
the same is in place (phase 3), every screen is ported (phase 4), the
release workflow builds and tests the MSIX for the Microsoft Store
(phase 5), and 1.0.1 is the first release on both systems (phase 6).
Publishing on Flathub was dropped from the plan (decision P15).

The macOS version (step 3) started on 2026-10-28: the backend and `cr`
are built and tested on Apple Silicon (phase 1), the bridge between
Swift and the Rust crates is laid (phase 2), and the prototype is built
as a `.dmg` by CI (phase 3).

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
| 6. macOS and Windows | Other platforms over the same backend | Moved after 1.0 (P15); Windows 11 is step 2 after 1.0, macOS step 3 |

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
| 1.0.1 | 2026-10-27 | Windows 11 on the Microsoft Store, the Flatpak repository, small fixes |

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

### Step 2: Windows 11 (done)

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
   - **Character creation and Enter Character (done, PR #103).** `cr-app` gained
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
     narrow window, where a settings card now puts its control under its
     header. Enter Character no longer refuses a thief and psionicist
     with more rogue points than New Character gives.
   - **Editing (done, PR #104).** `cr-app` gained the character editor, free of
     any toolkit, with the original's Update Character in its tabs:
     Details (who the character is, the portrait, the background and
     history, the classes with Add Class for dual-classing, the Levels
     dialog and Add Experience, the hit points and PSPs, the appearance
     and the abilities); Abilities (the subrace and its package, racial
     and class abilities, schools and spheres with their rules of one);
     Proficiencies (a rank more for each taken again, the slots or points
     left, the kit's warning, the native languages); Combat (weapon
     proficiencies and groups, weapon of choice, expertise,
     specializations and mastery, unarmed combat, shields, armor,
     fighting styles, traits and disadvantages); Skills (the Skills form
     New Character shares, with the wild talent test and roll); Spells
     (the kit's barred schools, schools to eliminate and spheres to keep,
     the spells known with the Learn check, the spellbooks and the spells
     memorized within the slots); and Equipment (the spending money, Buy
     and Sell at the kit's prices, the item check and each bundle's
     items). Save checks the character points the pages share and asks
     the kit's questions first. The GNOME editor takes from it the
     classes offered, Add Experience and the slot texts, and both apps
     now show the social class in the language chosen. The Windows app's
     Edit Character shows every tab. 30 actions are left on
     `NOT_YET_ON_WINDOWS`. The Windows test found two tabs named
     "Habilidades" in Brazilian Portuguese, where Skills is now
     "Talentos" and a test keeps the tabs apart in every language; Save
     offered with nothing changed; the last tabs cut off in a narrow
     window, where the bar now scrolls; and the Combat tab's free
     purchase blind to what the Abilities tab spent: the Abilities,
     Combat and Proficiencies tabs now share the character points as they
     are edited. A quest spell is named so among the memorized spells,
     and a list's Add box keeps Add inside a narrow window.
   - **The generators (done, PR #105).** `cr-app` gained the DM Tool Kit's generators,
     free of any toolkit, and the GNOME app uses them: New NPC and New
     NPC Group (the race, class, level and alignment, how the abilities
     are rolled, and the party to put them in); Roll Treasure (the
     treasure types and their counts, the bundles of items, saved
     treasures to edit or copy, print and PDF); Roll on a Table (the
     books' tables with their options and Roll Again, where a roll's
     leftovers carry to the next); Manage Parties (the parties and their
     members, New Party, Delete Members and Group Encounter); and the
     encounters (New Encounter from a template or empty, the Encounter
     page with its monsters, NPCs, traps, hit points, items and treasure,
     the 2-20 encounter table editor with Roll and Create Encounter, and
     Group Encounter's encounter or sheet). The encounter rules can now
     be read away from the window, which the Windows app does. The
     Windows app has every generator in the bar's More menu and Edit,
     Copy and Create Encounter on saved encounters, tables and
     templates. 16 actions are left on `NOT_YET_ON_WINDOWS`: the user
     database editors with the traps, packages, Import Core Rules 2.0
     and Extract Data from CD…. The Windows test found the Encounter
     page's cards that never opened, which the window now keeps open;
     an encounter just made, Group Encounter's among them, lost without
     a word on leaving, where leaving an unsaved encounter or table now
     asks first; thousands grouped the English way in the other
     languages; a box title left alone at the foot of a printed page;
     and sheet tables cut off in a narrow window, where a sheet's
     columns now regroup and a wide table wraps its words and is scaled
     down only by what still does not fit. Saving a treasure, encounter
     or table now shows its section, as in the GNOME app.
   - **The user database editors.** `cr-app` gained the
     record editor free of any toolkit: its pages and groups, every kind
     of field, the questions the original asks before a change or a save,
     the small dialogs of a group to choose from, a class level's free
     spells, NWPs and ability, and a monster's attack, and what New, Copy,
     Edit, Delete, New Subrace and Schools… open; with it the treasure
     table editor and its entry details, and a race's Allowed Classes.
     The GNOME app lays its editor out and sums up a table's entries with
     them, and a change it asks about now detaches the record as it was
     stored rather than as edited, asking once. The Windows app edits the
     records on a page of the window, with the commands of the GNOME
     app's details, Choose Picture for a monster type's or race's picture
     and an item's icon (with a search over the items that have it), and
     New Trap, Edit, Copy and Delete in the encounter's Add Trap and New
     Monster in its Add Monster, which put what they make in the
     encounter. 5 actions are left on `NOT_YET_ON_WINDOWS`. The Windows
     test found a record just saved left unselected, the class's level
     tables taking the wheel from the page (their columns now go to
     tables below rather than scroll sideways), a new monster entering an
     encounter as none, an empty group kept, and texts left untranslated,
     all fixed before the merge (PR #106). In a second
     pull request `cr-app` gained the encounter templates' editor: its
     pages, the Properties of a monster, NPC or item line, the pickers of
     monsters, monster types, items and traps, and the monsters that come
     with an item. The GNOME app reads its lists and pickers' data with
     it; the Windows app edits a template on a page of the window, with
     NPC Creation in template mode for its NPCs. The library's list now
     scrolls to the entry selected when it is made anew. Its Windows test
     found dice rows cut in a narrow window, the placement's long choices
     cut in a combo (radios now, as in the original), padding spaces in
     the lines and texts left in English, the template's description
     among them, all fixed before the merge (PR #107).
   - **Packages.** `cr-app` gained what Export…, Import…
     and the package editor do free of any toolkit: the items of a package
     taken or left one by one, what a clash with a record already there
     does, Name Conflict's questions for a part a standard part's name
     hides, the report of an import, and the editor's items, Add Items and
     Save. The GNOME app asks its names and reports its imports with them.
     The Windows app offers Import Core Rules 2.0…, Export…, New Package…
     and Edit Package… in the library's tools, and opens a package
     chosen in Import… or in File Explorer on a page with every item to
     take or leave; a category of hundreds lists them when opened. Only
     Extract Data from CD… is left on `NOT_YET_ON_WINDOWS`. The Windows
     test found a part of the other types shown on the wrong section once
     saved, a large category's partial choice not shown and two texts
     left in English, all fixed before the merge (PR #108).
   - **Extract Data from CD (done).** `cr-app` gained what the app
     says around the copy and when a folder opened is copied first; the
     GNOME app says it with them. The Windows app asks for the CD's ISO
     image or a folder, copies its data into `Documents\Core Rules\data`
     (or beside it when a package is already there, with Open to open
     it), and offers it in the library's tools and on the page shown
     when no installation is found. An Expansion CD opened with Open
     Folder… opens its copy instead, as in the GNOME app. With it
     `NOT_YET_ON_WINDOWS` is empty: every screen of the GNOME app is on
     Windows (PR #109).
   - **Help and details (done, PR #110).** The Windows test of the Help
     page with the CD's help files found its side panel too wide for a
     narrow window, long lines of some topics not wrapped, the book
     list left blank after a search or a link to another file, and F1
     opening the help behind a dialog. The Help page now shows its
     contents through a Contents button in a narrow window, wraps the
     topics' lines and names the book shown; F1 waits for the dialog to
     close. A row just saved is no longer cut at the foot of the
     library's list, the New Encounter templates list makes only the
     rows it shows, and "not added" messages no longer repeat "Error."
     in the translations.
5. **Package and Store (done).** The release workflow builds and tests
   the MSIX; a beta went to a closed group through the Microsoft Store.
   - **The MSIX in the release workflow (done, PRs #111 and #112).** The workflow
     builds the MSIX with the version of the tag, installs it on a clean
     Windows with the Windows App Runtime (signed there with a
     certificate of its publisher that only that machine trusts), starts
     the app from Start and runs the Windows App Certification Kit. The
     MSIX stays among the run's artifacts for the Store, not in the
     GitHub release, where Windows would not install it unsigned.
6. **Release with Windows (done).** Wagner Brahm found no problem in the
   beta and released 1.0.1 on 2026-10-27 on Linux and Windows 11, with the
   same version and tag (decision P26). The Community Add-ons Collection
   follows as a package the Windows app imports.

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

Done on 2026-10-27 with 1.0.1.

### Step 3: macOS (in progress)

Decided by Wagner Brahm on 2026-10-28 (decisions S14 and P27 to P29): the
macOS version lives in this repository too, and once it is released every
feature ships on GNOME, Windows 11 and macOS. Its interface is native
SwiftUI, with AppKit where SwiftUI falls short, following Apple's Human
Interface Guidelines and keeping the organization of the other two apps:
the sections, the entries and the details as the three columns of a
`NavigationSplitView`, and every action of `cr_app::action` in the menu
bar. The Swift code only draws what `cr-app` decides; a new crate,
`cr-mac-ffi`, exposes `cr-app` to Swift through UniFFI. It runs on Apple
Silicon only, macOS 14 or newer, and is distributed as a `.dmg` attached
to the same GitHub release as the other packages, without an Apple
developer account: it is signed without a certificate from Apple and not
notarized, so macOS blocks its first launch until the user allows it in
System Settings › Privacy & Security › Open Anyway, again after each
update, and a Mac managed by a company or school may not allow it at all.

The phases run in order:

1. **Backend on macOS (done, PR #117).** A macOS job in CI lints and
   tests `cr-formats`, `cr-store`, `cr-i18n`, `cr-app` and `cr` on Apple
   Silicon, and builds `cr`. The data package lives in
   `~/Library/Application Support/Core Rules/data`, where macOS asks no
   permission and which stays when the app is moved to the Trash. A name
   is looked up on the Mac's disk as on Windows, since both ignore case.
2. **The bridge and the shared groundwork (done, PR #118).** The help
   topics and the books as web pages moved from `cr-win` into
   `cr_app::web`, and their scripts post to the app through a WebView2 or
   a WKWebView, so the Mac shows the same pages. The new crate
   `cr-mac-ffi` exports to Swift with UniFFI what the library needs
   (the sections, the search, an entry's details), the languages and the
   actions; `macos/scripts/make-bindings.sh` writes its Swift side, and
   CI builds and runs a Swift program that calls it. `NOT_YET_ON_MACOS`
   lists every action, and the parity test checks the Mac app's Swift
   sources, which refer to an action as `Action.NAME`.
3. **Prototype (done, PRs #119 and #120)**, with the scope of the Windows one,
   packaged as a `.dmg`, approved by Wagner Brahm after a run on a Mac.
   The SwiftUI app in `macos/CoreRules` shows the library in three
   columns with its search, the first page of New Character, the
   character's description in Edit Character, Delete with its
   confirmation, Import… of a `.xpx` or `.xpt` package (also opened from
   the Finder), the sheet printed, saved as PDF and exported, and
   Settings with the language. `cr-mac-ffi` holds the wizard, the editor
   and the import as sessions over `cr-app`'s state machines, so the
   Swift side keeps no rule. The Mac app's texts are wrapped in
   `tr("…")` and extracted into the catalogs with the Rust ones.
   `macos/scripts/make-app.sh` builds `Core Rules.app` with `swiftc`
   (no Xcode project), signs it ad hoc and packs the `.dmg`, which CI
   uploads on every pull request.
4. **Porting the screens (done)**, in the stages of the Windows
   port, one pull request each, until `NOT_YET_ON_MACOS` was empty.
   Wagner Brahm approved the prototype on a MacBook Pro M4 Pro on
   2026-10-29; its test found Open Anyway not offered, so the `.dmg`'s
   notes lead with removing the quarantine in Terminal, and the weapons
   drawn with a hammer, now the sword of the other apps.
   - **The library and the sheet (done).** `cr-mac-ffi` gained the
     library's filters (origin, party, part type, Magic Only), the
     commands an entry offers, the items' icons and the pictures (a
     monster type's frames with Next Image, and Full Screen), and the
     sheet as a session: a character's sheet or the page of an
     encounter, table, template or treasure, its PDF, its exports and
     Character Sheet Options. The Mac app filters the list, shows those
     pictures, Read-only for Core Rules 2.0, and the sheet in a window of
     its own as the PDF it prints, in PDFKit as Preview shows a
     document, with Print, Save as PDF, Export and Character Sheet
     Options; Print and Save in the menu bar act on the sheet's window in
     front. Convert to Character and Convert to NPC are in the
     character's More menu.
   - **Help and About (done).** `cr-mac-ffi` holds the help as a
     session (the book listed, its contents or what a search finds, the
     topic as the web page `cr-app` makes, its links and popups, Back),
     the help of an entry and of a monster, and About's texts. The Mac
     app shows the help in a window of its own, with the book selector,
     the search in the sidebar and the topic in a WKWebView, Back and
     Print; the Help menu opens the help of the screen in front (⌘?),
     the help of the entry in view and the books, and the entry's
     toolbar has Help, and View and Print for a monster. About is a
     window with What's New, Credits, Legal and Troubleshooting, as the
     other apps' About.
   - **The books (done).** `cr-mac-ffi` reads a book as a session:
     its page (`cr_app::web::book`) written to a file, its contents,
     Find with its matches, zoom, and the reading position kept in the
     reader's state file in Application Support. The Mac app's Read Book
     opens each book in a window of its own, with the contents in the
     sidebar, Find (⌘F, ⌘G, ⇧⌘G) in a bar over the text, and Zoom In,
     Zoom Out and Reset Zoom in the View menu.
   - **New Character and Enter Character (done, PR #124).** `cr-mac-ffi`
     describes each page of `cr_app::create` as a form (`form.rs`):
     groups of rows (choices, radio groups, numbers, text, switches,
     check boxes, lists with Add and Remove, the portrait), each control
     doing an input of the wizard by its number, and Enter Character's
     Levels as a dialog over the page. The Mac app draws any such form
     with one SwiftUI view in a grouped Form, so the Swift side keeps no
     rule and the editors of the next stages draw with it too. Every page
     of the wizard, its questions (Add One Point, All Classes…), Choose
     Portrait with the race's portraits first or a picture of the user's,
     and Create, which writes the character and shows it in the library,
     are on the Mac.
   - **Edit Character (done, PR #125).** Every page of `cr_app::editor`
     (Details, Abilities, Combat, Proficiencies, Skills, Spells and
     Equipment) is described as a form like the wizard's, chosen with a
     segmented control above it; Levels… and Add… open as dialogs over
     the page, the bundles show their items' icons, and Save's questions
     come in the page's place. The Levels dialog, the experience fields,
     the Skills form and the portrait row are those of the wizard.
   - **The generators (done)**, in four pull requests. First New
     NPC and New NPC Group, as forms in a sheet whose Create makes and
     saves the NPCs off the main thread, and Roll on a Table in a window
     of its own, with Roll, Roll Again, Copy Results and Read in Book;
     they are in a Tools menu, as in the GNOME app's main menu (PR
     #126). Then Roll Treasure in a window of its own, its treasure
     drawn as a tree with each item's edits and the bundle editor's
     dialogs, Save, Print, Save as PDF and Delete Saved Treasure, and Edit
     and Copy on a saved treasure in the library (PR #127). Then the
     encounters, in a window of their own: New Encounter's templates
     with a search, then the encounter as a form, each monster or NPC
     package with its number, hit points, Re-roll and stat line, Add
     Monster, Add NPC and Add Trap as searchable dialogs, View NPC and
     Print NPC, the items with the bundle editor, Roll Treasure opening
     the treasure window with Use Treasure, Save, Print, Save as PDF and
     Delete Encounter; the 2-20 table editor in another window, with its
     filter, Fill, Roll (which opens the encounter generated) and the
     dialog that picks each roll's template; and Edit, Copy, Create
     Encounter and Roll and Create Encounter on the saved encounters,
     tables and templates in the library. A list of more than 200 names
     shows the first 200 and asks for a search (PR #128). Last, Manage
     Parties in a sheet: the party chosen, New Party, the members and
     the characters in no party each with the button that moves it,
     Delete Members, and Group Encounter's dialog, which opens the
     encounter of the party's NPCs or prints or shows their page. New
     Trap, New Monster and the traps' Edit, Copy and Delete in Add Trap
     come with the database editors. (PR #129)
   - **The user databases (done)**, in three pull requests. First
     the record editor of `cr_app::records::form` in a sheet: each page
     described as a form, every kind of field drawn (text, numbers with
     their suggestion, choices, lists with their numbers, checks over a
     number or a list, dice, tables of numbers by level, items, attacks,
     groups and levels with their dialogs, Auto Fill, pictures and icons
     with Choose Picture and the user's own), the questions it asks, and
     Save; New in the list's toolbar and the File menu, Edit, Copy, New
     Subrace, Schools… and Delete on a record, New Part asking for the
     type, and New Trap, New Monster and the traps' Edit, Copy and
     Delete in the encounter's Add Trap and Add Monster (#130). Then
     Allowed Classes on a race, Restore on a shipped treasure table, and
     the treasure table editor in the same sheet, with an entry's details
     and "How many to remove?" in a dialog over it (#131). Then the
     encounter template editor in the same sheet: its seven pages, and in
     a dialog over them a line's Properties, the monster, item and trap
     pickers, an escort's numbers and NPC Creation's Add to Template
     (#132).
   - **Packages (done)**: Export… with the user's records and characters
     checked by category, New Package… and Edit Package… in a sheet with
     the items by category, Remove, Add Items and Save, and Import Core
     Rules 2.0… from the folder of a Core Rules 2.0 installation, through
     the Import… sheet; all in the File menu (#133).
   - **Extract Data from CD (done).** A Mac has had no CD drive for
     years, so the Mac app takes only the CD's ISO image or a folder its
     files were copied into, with its own wording of the question, and
     extracts off the main thread under a progress sheet; the copy opens
     at once when it is the data folder's, or is offered to open when
     that folder already holds the other CD's. It is in the File menu
     and on the No Installation screen. With it `NOT_YET_ON_MACOS` is
     empty.
5. **Beta and release** on the three systems with the same tag, with an
   install page on the website and in the README that shows the Open
   Anyway steps.

### Step 4: new features, and independence from the original

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
independence goes (step 4.3).

## Decisions pending

- Confirming the Windows beta (step 2).
- Who tests the macOS prototype on a Mac, how its `.dmg` is signed
  (a free self-signed certificate or ad-hoc), and its oldest macOS (step 3).
- The first features after 1.0, and the source of rules content without
  the discs (step 4).
