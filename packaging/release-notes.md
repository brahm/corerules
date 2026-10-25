**Core Rules 1.0, the first stable release.** Core Rules does everything
the original AD&D Core Rules 2.0 Expansion does, and every feature was
checked against the original programs. Players and Dungeon Masters ran the
release candidate on **Fedora 45 beta, Ubuntu 26.10, Bazzite, Arch Linux
and CachyOS**, and it worked as it should, so 1.0.0 is the code of
1.0.0-rc2 with the new About window. Characters and user databases made
with the release candidates open as they are.

What's new since 1.0.0-rc2:

- **About**: the About window has the credits of the original program and
  of this one, the legal notice, the details a bug report needs (version,
  package, GTK and libadwaita, language and data folder) and these release
  notes, and links to the new website, https://brahm.github.io/corerules/
- **Packages**: the `.deb`, `.rpm` and pacman package are plain 1.0.0, and
  replace a release candidate on update

What the release candidates brought since 0.9.0:

- **Saving with the CD** (rc2): opening the mounted Expansion CD or ISO
  image with Open Folder… copies the disc to `~/.local/share/corerules/data`
  (in the Flatpak, its own data folder), opens that copy and saves there
- **Importing packages** (rc2): Import brings in, in one go, the race/class
  combinations that name two classes of the package
- **The test round of 0.9.0** (rc1): Edit Character keeps an elf's subrace,
  a large package (36,525 items) imports in the background without
  crashing, Extract Data from CD copies the pictures, New Character and
  Enter Character have a Psionics step, and more than 40 smaller fixes
- **Packages** (rc1): the `.deb` refuses to install on Ubuntu 24.04, which
  is not supported; the Flatpak runs there; a pacman package installs on
  Arch Linux, CachyOS and Omarchy

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
| Ubuntu 26.04 or newer | `corerules_1.0.0_amd64.deb` | `sudo apt install ./corerules_1.0.0_amd64.deb` |
| Fedora 44 or newer | `corerules-1.0.0-1.fc44.x86_64.rpm` | `sudo dnf install ./corerules-1.0.0-1.fc44.x86_64.rpm` |
| Arch Linux, CachyOS, Omarchy | `corerules-1.0.0-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./corerules-1.0.0-1-x86_64.pkg.tar.zst` |
| Any distribution with Flatpak, such as Bazzite | `CoreRules-1.0.0.flatpak` | `flatpak install --user CoreRules-1.0.0.flatpak` |
| Any distribution (glibc 2.43+) | `CoreRules-1.0.0-x86_64.AppImage` | `chmod +x CoreRules-1.0.0-x86_64.AppImage` and run it |

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
