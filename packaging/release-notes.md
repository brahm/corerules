**Beta release.** Every feature works against real installations and was
checked against the original programs, but the app has had little use
outside those checks. Keep a copy of your `Chars` and `UserDbas` folders,
and report anything that differs from the original.

What's new in 0.8.1:

- Monster types, monsters (through their type), races and characters show
  their picture when its file is in the data folder, as the community
  add-ons' pictures now are. The app reads the BMP files itself. The
  character sheet finds the portrait the same way, and reads its folder
  and file in the right order
- An encounter's Characters Not Generated group shows, under each NPC, why
  the generator could not make it, instead of saying it is not available
- The app's description lists the six languages it speaks, and the package
  tests start the app in each one
- The Core Rules Community Add-ons Collection (`corerules-community-addons`
  on GitHub Packages) leaves out Evermore's monthly goodies, installs the
  668 pictures that came with the add-ons and shows their authors' texts
  on request (see `addons/README.md`)

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
