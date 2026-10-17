What's new in 0.5.0:

- DM Toolkit packages: Import… and Export… in the main menu read and write
  the `.xpx` packages of the Expansion and the `.xpt` packages of Core
  Rules 2.0, with characters and any record you made (races, classes,
  items, spells, monsters, encounters, traps, psionic powers, treasure
  bundles), exactly as the original programs do; a 2.0 package imported
  into the Expansion is converted like the original converts it
- New Package… and Edit Package… create a package or change one, adding
  your records and characters or taking items out
- `cr export`, `cr import` and `cr package` do the same from the command
  line
- There is no `.mon` file: both originals export monsters in packages

Every change was checked against both original programs.

Preview release: the app opens a Core Rules 2.0 (or Expansion) installation
or its CD, browses characters, races, classes, items, spells, psionics and
monsters, and creates and edits characters in files the original programs
still open.
It contains no rules content; you need your own copy of Core Rules.

| System | File | Install |
| --- | --- | --- |
| Ubuntu 26.04 or newer | `corerules_*_amd64.deb` | `sudo apt install ./corerules_*_amd64.deb` |
| Fedora 44 or newer | `corerules-*.fc44.x86_64.rpm` | `sudo dnf install ./corerules-*.rpm` |
| Any distribution with Flatpak | `CoreRules-*.flatpak` | `flatpak install --user CoreRules-*.flatpak` |
| Any distribution (glibc 2.39+) | `CoreRules-*-x86_64.AppImage` | `chmod +x CoreRules-*.AppImage` and run it |

The Flatpak needs the GNOME 50 runtime from Flathub, which `flatpak install`
offers to fetch. `SHA256SUMS` lists the checksums of every file.
