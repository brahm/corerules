What's new in 0.4.0:

- Treasure generator: treasure types A to Z, table rolls, intelligent
  weapons, spell scrolls and saved treasure bundles
- Encounter generator: encounters from templates, saved encounters and the
  2-20 tables, NPC lines included
- NPC generator: New NPC and New NPC Group, for Core Rules and Player's
  Option (Skills & Powers) NPCs, with kits, proficiencies, equipment, spells,
  psionics and rogue skills, and Convert to Character
- The Core Rules books in a reader, Table Rolls, and editors for every user
  database
- Run without Wine: `packaging/extract-data.sh` makes a data package from
  your own installation (keep it for yourself, do not share it)
- The `.deb` now targets Ubuntu 26.04; on Ubuntu 24.04 use the AppImage or
  the Flatpak

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
