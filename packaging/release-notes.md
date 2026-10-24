**Release candidate for 1.0.** Core Rules now does everything the
original AD&D Core Rules 2.0 Expansion does, and every feature was checked
against the original programs and went through a full round of manual
tests. This release goes to real users: after a testing period, the last
release candidate becomes 1.0. Keep a copy of your `Chars` and `UserDbas`
folders, and [report](https://github.com/brahm/corerules/issues) anything
that is wrong or works differently from the original.

What's new since 0.9.0, the fixes from its test round:

- **Critical**: Edit Character keeps an elf's subrace when it saves, and a
  large package (36,525 items) imports without crashing, in the background
- **High**: Extract Data from CD copies the portraits, icons and monster
  pictures; New Character and Enter Character have a Psionics step for a
  psionicist; a subrace follows its parent race; a kit's negative rogue
  skill modifier reads correctly; slots and points no longer go negative
- **Medium and low**: more than 40 smaller fixes across rogue skills, equipment, the
  database editors, the NPC generator, encounters, treasure, the sheet,
  printing, the books, the help and the `cr` command
- **Encounters**: Re-roll keeps the hit points set by hand, as the
  original's later patch does
- **Packages**: the `.deb` refuses to install on Ubuntu 24.04, which is not
  supported; the Flatpak runs there; a new pacman package installs on Arch
  Linux, CachyOS and Omarchy

The README lists everything the app does, where to find the disc images if
yours are lost (you must own the original discs), and every difference
from the original.

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
| Arch Linux, CachyOS, Omarchy | `corerules-*-x86_64.pkg.tar.zst` | `sudo pacman -U ./corerules-*.pkg.tar.zst` |
| Any distribution with Flatpak | `CoreRules-*.flatpak` | `flatpak install --user CoreRules-*.flatpak` |
| Any distribution (glibc 2.43+) | `CoreRules-*-x86_64.AppImage` | `chmod +x CoreRules-*.AppImage` and run it |

The Flatpak needs the GNOME 51 runtime from Flathub, which `flatpak install`
offers to fetch. The AppImage bundles the GTK, libadwaita and other
libraries of GNOME 51 and runs on Fedora 44, Ubuntu 26.04 and newer. The
pacman package needs an up-to-date system (`sudo pacman -Syu`), and
`PKGBUILD` builds it from source with `makepkg -si`. `SHA256SUMS` lists the checksums of every file.
