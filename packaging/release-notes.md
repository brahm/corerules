**Beta release.** Every feature works against real installations and was
checked against the original programs, but the app has had little use
outside those checks. Keep a copy of your `Chars` and `UserDbas` folders,
and report anything that differs from the original.

What's new in 0.8.0:

- **Extract Data from CD…** in the main menu (and on the start page when
  no data is found) copies what the app uses from the Core Rules 2.0 or
  Expansion CD into the app's data folder. It reads the CD's ISO image
  directly, with no mounting or other tools, or takes the mounted CD, an
  installation or a Wine prefix. `cr extract-data` does the same from the
  command line
- The Expansion takes the data folder over from Core Rules 2.0 data, which
  moves next to it for Import Core Rules 2.0…
- `extract-data.sh` is gone: the app does its job, and its copies from a
  CD are writable

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
