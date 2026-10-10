Preview release: the app opens a Core Rules 2.0 (or Expansion) installation
or its CD, browses characters, races, classes, items, spells, psionics and
monsters, and edits characters in files the original programs still open.
It contains no rules content; you need your own copy of Core Rules.

| System | File | Install |
| --- | --- | --- |
| Ubuntu 24.04 or newer | `corerules_*_amd64.deb` | `sudo apt install ./corerules_*_amd64.deb` |
| Fedora 44 or newer | `corerules-*.fc44.x86_64.rpm` | `sudo dnf install ./corerules-*.rpm` |
| Any distribution with Flatpak | `CoreRules-*.flatpak` | `flatpak install --user CoreRules-*.flatpak` |
| Any distribution (glibc 2.39+) | `CoreRules-*-x86_64.AppImage` | `chmod +x CoreRules-*.AppImage` and run it |

The Flatpak needs the GNOME 50 runtime from Flathub, which `flatpak install`
offers to fetch. `SHA256SUMS` lists the checksums of every file.
