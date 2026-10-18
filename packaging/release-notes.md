**Beta release.** Every feature works against real installations and was
checked against the original programs, but the app has had little use
outside those checks. Keep a copy of your `Chars` and `UserDbas` folders,
and report anything that differs from the original.

What's new in 0.7.2:

- DM Toolkit packages made with the Expansion after its later patches
  import: the spell points of their classes, which the CD's program cannot
  read, are left out, and their kits are written in the CD's layout.
  Classes with spell-like abilities by level import as well
- A user record that has a standard record's name, as imported packages
  bring, can be edited and saved while it keeps that name
- `cr import` takes the category names `--list` prints and finds names
  with trailing spaces
- `addons/` in the repository holds 686 community add-ons converted to the
  Expansion and consolidated into one package, with an installer and a
  guide in Portuguese

What the app does: it opens a Core Rules 2.0 Expansion installation (or its
CD, or a data package), creates and edits characters with the Core Rules or
Player's Option, levels them up, manages parties and prints character
sheets; it generates NPCs, treasure and encounters, rolls on the books'
tables, edits every user database and imports and exports DM Toolkit
packages, all in files the original programs still open. A Core Rules 2.0
installation opens read-only and is imported into the Expansion. The README
lists every feature.
It contains no rules content; you need your own copy of Core Rules.

| System | File | Install |
| --- | --- | --- |
| Ubuntu 26.04 or newer | `corerules_*_amd64.deb` | `sudo apt install ./corerules_*_amd64.deb` |
| Fedora 44 or newer | `corerules-*.fc44.x86_64.rpm` | `sudo dnf install ./corerules-*.rpm` |
| Any distribution with Flatpak | `CoreRules-*.flatpak` | `flatpak install --user CoreRules-*.flatpak` |
| Any distribution (glibc 2.43+) | `CoreRules-*-x86_64.AppImage` | `chmod +x CoreRules-*.AppImage` and run it |

The Flatpak needs the GNOME 51 runtime from Flathub, which `flatpak install`
offers to fetch. The AppImage bundles the GTK, libadwaita and other
libraries of GNOME 51 and runs on Fedora 44, Ubuntu 26.04 and newer. `SHA256SUMS` lists the checksums of every file.
