**Core Rules 1.0.1, ready on Linux and Windows 11.** The Windows 11
version has every screen of the Linux one, over the same backend and the
same data files, with the look of a Windows 11 app. It went through a beta
on the Microsoft Store without problems and is now a final release:
[get it from the Microsoft Store](https://apps.microsoft.com/detail/9nr2cgp649jm),
which signs it, installs the Windows App Runtime it needs and brings every
update. Characters and user databases move freely between the two systems
and the original programs.

What's new on Linux since 1.0.0:

- **Flatpak repository**: the Flatpak comes from the project's own signed
  repository, so `flatpak update`, GNOME Software and Discover bring every
  new version
- **Editors**: a priest's quest spell is called a quest spell instead of
  "0th level", the editors' wording is clearer, and Portuguese tells the
  Abilities and Skills tabs apart
- **Characters**: Edit Character shows the social class in the language
  chosen, Back keeps what a creation page had, the points pages say why
  they refuse a choice, and the Psionics page leaves the rogue points to
  the Rogue page
- **Translations**: the app shows the chosen language throughout, and the
  translations lost the original's double spaces

What the app does: it opens a Core Rules 2.0 Expansion installation (or its
CD, or a data package), creates and edits characters with the Core Rules or
Player's Option, levels them up, manages parties and prints character
sheets; it generates NPCs, treasure and encounters, rolls on the books'
tables, edits every user database and imports and exports DM Toolkit
packages, all in files the original programs still open. A Core Rules 2.0
installation opens read-only and is imported into the Expansion. The
[README](https://github.com/brahm/corerules#readme) lists every feature,
where to find the disc images if yours are lost (you must own the original
discs), and every difference from the original. It contains no rules
content; you need your own copy of Core Rules.

| System | File | Install |
| --- | --- | --- |
| Ubuntu 26.04 or newer | `corerules_1.0.1_amd64.deb` | `sudo apt install ./corerules_1.0.1_amd64.deb` |
| Fedora 44 or newer | `corerules-1.0.1-1.fc44.x86_64.rpm` | `sudo dnf install ./corerules-1.0.1-1.fc44.x86_64.rpm` |
| Arch Linux, CachyOS, Omarchy | `corerules-1.0.1-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./corerules-1.0.1-1-x86_64.pkg.tar.zst` |
| Any distribution with Flatpak, such as Bazzite | `CoreRules-1.0.1.flatpak` | `flatpak install --user CoreRules-1.0.1.flatpak` |
| Any distribution (glibc 2.43+) | `CoreRules-1.0.1-x86_64.AppImage` | `chmod +x CoreRules-1.0.1-x86_64.AppImage` and run it |
| Windows 11 | [Microsoft Store](https://apps.microsoft.com/detail/9nr2cgp649jm) | Get it from the Store page |

The Flatpak needs the GNOME 51 runtime from Flathub, which `flatpak install`
offers to fetch. It also installs from the project's own Flatpak repository,
which brings every new version through `flatpak update`, GNOME Software and
Discover:
`flatpak install --user https://brahm.github.io/corerules/io.github.brahm.CoreRules.flatpakref`.
The AppImage bundles the GTK, libadwaita and other libraries of GNOME 51
and runs on Fedora 44, Ubuntu 26.04 and newer. The pacman package needs an
up-to-date system (`sudo pacman -Syu`), and `PKGBUILD` builds it from
source with `makepkg -si`. `SHA256SUMS` lists the checksums of every file.

The [Community Add-ons Collection](https://github.com/brahm/corerules/blob/main/addons/README.md),
672 community add-ons in one self-installing AppImage, has its own
[release](https://github.com/brahm/corerules/releases/tag/addons):
[CoreRules-Community-Addons-x86_64.AppImage](https://github.com/brahm/corerules/releases/download/addons/CoreRules-Community-Addons-x86_64.AppImage).
