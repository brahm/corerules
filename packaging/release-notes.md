What's new in 0.3.0:

- Add experience to a character, to one class, to all or split between
  them, with the prime requisite bonus
- Levels go up with hit point rolls, psionic strength points, Skills &
  Powers character points and the Expansion's racial level limits
- Memorizing spells shows the slots per spell level, as the original
  counts them
- Character sheet: view it, print it, save it as PDF, or export it as HTML
  (both programs) or text (Expansion), matching the original's exports

Every change was checked against both original programs.

Preview release: the app opens a Core Rules 2.0 (or Expansion) installation
or its CD, browses characters, races, classes, items, spells, psionics and
monsters, and creates and edits characters in files the original programs
still open.
It contains no rules content; you need your own copy of Core Rules.

| System | File | Install |
| --- | --- | --- |
| Ubuntu 24.04 or newer | `corerules_*_amd64.deb` | `sudo apt install ./corerules_*_amd64.deb` |
| Fedora 44 or newer | `corerules-*.fc44.x86_64.rpm` | `sudo dnf install ./corerules-*.rpm` |
| Any distribution with Flatpak | `CoreRules-*.flatpak` | `flatpak install --user CoreRules-*.flatpak` |
| Any distribution (glibc 2.39+) | `CoreRules-*-x86_64.AppImage` | `chmod +x CoreRules-*.AppImage` and run it |

The Flatpak needs the GNOME 50 runtime from Flathub, which `flatpak install`
offers to fetch. `SHA256SUMS` lists the checksums of every file.
