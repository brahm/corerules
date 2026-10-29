**Core Rules 1.1.0 beta: macOS.** This beta adds a native Mac version,
with every screen of the Linux and Windows 11 versions, over the same
backend and the same data files. It has the look of a Mac app: the
sections, the entries and the details in three columns, every action in
the menu bar, and the sheets, the help and the generators in windows of
their own. It is a beta for testing on a Mac before 1.1.0 is released;
1.0.1 stays the stable version for Linux and Windows 11, and the Linux
packages here are the same app as 1.0.1. Characters and user databases
move freely between the three systems and the original programs.

| System | File | Install |
| --- | --- | --- |
| macOS 14 or newer, Apple Silicon (M1 or newer) | `CoreRules-1.1.0-beta1-arm64.dmg` | Open it and drag Core Rules to Applications, then see below |
| Windows 11 | [Microsoft Store](https://apps.microsoft.com/detail/9nr2cgp649jm) | Get it from the Store page |
| Linux | The `.deb`, `.rpm`, pacman package, Flatpak and AppImage below | As for [1.0.1](https://github.com/brahm/corerules/releases/tag/v1.0.1) |

**Opening it the first time.** The Mac app is signed without a
certificate from Apple and is not notarized: the project has no Apple
developer account. So macOS refuses it the first time, saying it could
not check it for malicious software. After copying it to Applications,
run this once in Terminal, and from then on it opens as any app:

```sh
xattr -dr com.apple.quarantine "/Applications/Core Rules.app"
```

Without Terminal: open Core Rules once and press Done; then in System
Settings › Privacy & Security, under Security, press **Open Anyway** next
to "Core Rules" and confirm with your password. Some Macs do not offer
Open Anyway, and a Mac managed by a company or school may not allow the
app at all. Either way is needed again after each update. The `.dmg` has
the same steps in English and Portuguese in `Read Me First.txt`.

**The rules data.** The app opens the data package in
`~/Library/Application Support/Core Rules/data`. Macs have no CD drive,
so File › Extract Data from CD… makes it from the ISO image of a Core
Rules 2.0 or Expansion CD, or from a folder the CD's files were copied
into; File › Open Folder… also opens an installation of the original.

What the app does: it opens a Core Rules 2.0 Expansion installation (or
a data package), creates and edits characters with the Core Rules or
Player's Option, levels them up, manages parties and prints character
sheets; it generates NPCs, treasure and encounters, rolls on the books'
tables, edits every user database and imports and exports DM Toolkit
packages, all in files the original programs still open. The
[README](https://github.com/brahm/corerules#readme) lists every feature,
where to find the disc images if yours are lost (you must own the original
discs), and every difference from the original. It contains no rules
content; you need your own copy of Core Rules. `SHA256SUMS` lists the
checksums of every file.
