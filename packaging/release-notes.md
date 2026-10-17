What's new in 0.6.0:

- Parties: Manage Parties… puts characters in a party, takes them out or
  deletes every member, and a character page can delete its character,
  with Undo (`cr party` and `cr delete` from the command line)
- New content follows the Expansion: a Core Rules 2.0 installation opens
  read-only, and Import Core Rules 2.0… brings its characters and records
  into an Expansion installation, converted as the Expansion converts a
  2.0 package; `.xpt` packages are only imported
- Lists sort exactly as Windows sorts them
- Player's Option NPCs choose the languages their kit offers and pay the
  base prices, as the original does
- Table Rolls: the first roll of each dialog compares with what the last
  one left, as in the original
- The class editor offers the original's weapon categories
- The Flatpak uses the GNOME 51 runtime, and the AppImage carries the
  GNOME 51 libraries

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
| Any distribution (glibc 2.43+) | `CoreRules-*-x86_64.AppImage` | `chmod +x CoreRules-*.AppImage` and run it |

The Flatpak needs the GNOME 51 runtime from Flathub, which `flatpak install`
offers to fetch. The AppImage bundles the GTK, libadwaita and other
libraries of GNOME 51 and runs on Fedora 44, Ubuntu 26.04 and newer. `SHA256SUMS` lists the checksums of every file.
