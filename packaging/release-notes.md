**Core Rules 1.2.0: Import Add-ons.** A new command, **Import Add-ons…**
(the main menu on Linux, the More menu on Windows, the File menu on the
Mac), imports every add-on in a folder at once. It reads the folder, its
subfolders and its ZIP archives (also ZIPs inside ZIPs, as the community
shared them), tells each `.xpx` and `.xpt` package apart by what it holds,
and lists what comes in, what was found twice and what cannot be imported,
with why, before anything is written. Then it backs up the user databases
and characters, imports from the oldest file to the newest, leaves out the
characters already in the list, copies the add-ons' pictures to where
their records look for them and writes a log.

**The Community Add-ons Collection as a .zip.** The 672 community add-ons
(36,525 items, 668 pictures and the authors' texts) now come as one file,
[`CoreRules-Community-Addons.zip`](https://github.com/brahm/corerules/releases/download/addons/CoreRules-Community-Addons.zip),
on the [addons release](https://github.com/brahm/corerules/releases/tag/addons).
Put it in a folder of its own, choose Import Add-ons… and pick that folder;
there is no need to extract it. The whole collection comes in in about a
minute, on Linux, Windows 11 and macOS. The Linux add-ons installer (an
AppImage) is no longer published: the zip and Import Add-ons replace it.

| System | File | Install |
| --- | --- | --- |
| macOS 14 or newer, Apple Silicon (M1 or newer) | `CoreRules-1.2.0-arm64.dmg` | Open it and drag Core Rules to Applications, then see below |
| Windows 11 | [Microsoft Store](https://apps.microsoft.com/detail/9nr2cgp649jm) | Get it from the Store page |
| Fedora 44 or newer | `corerules-1.2.0-1.fc44.x86_64.rpm` | `sudo dnf install ./corerules-1.2.0-1.fc44.x86_64.rpm` |
| Ubuntu 26.04 or newer | `corerules_1.2.0_amd64.deb` | `sudo apt install ./corerules_1.2.0_amd64.deb` |
| Arch Linux, CachyOS, Omarchy | `corerules-1.2.0-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./corerules-1.2.0-1-x86_64.pkg.tar.zst` |
| Any distribution with Flatpak, such as Bazzite | The [Flatpak repository](https://brahm.github.io/corerules/io.github.brahm.CoreRules.flatpakref) | `flatpak install --user https://brahm.github.io/corerules/io.github.brahm.CoreRules.flatpakref` |
| Any distribution (glibc 2.43+) | `CoreRules-1.2.0-x86_64.AppImage` | `chmod +x CoreRules-1.2.0-x86_64.AppImage`, then run it |

**Opening it on a Mac the first time.** The Mac app is signed without a
certificate from Apple and is not notarized, because the project has no
Apple developer account. So the first time you open it, macOS refuses
it and says it could not check it for malicious software. After copying
it to Applications, run this once in Terminal. From then on it opens
like any other app:

```sh
xattr -dr com.apple.quarantine "/Applications/Core Rules.app"
```

Without Terminal: open Core Rules once and press Done. Then go to System
Settings › Privacy & Security, and under Security press **Open Anyway**
next to "Core Rules" and confirm with your password. Some Macs do not
offer Open Anyway, and a Mac managed by a company or school may not
allow the app at all. You need to do this again after each update. The
`.dmg` has the same steps in English and Portuguese in
`Read Me First.txt`.

**The rules data on a Mac.** The app opens the data package in
`~/Library/Application Support/Core Rules/data`. Macs have no CD drive,
so File › Extract Data from CD… builds the data package from the ISO
image of a Core Rules 2.0 or Expansion CD, or from a folder the CD's
files were copied into. File › Open Folder… also opens an installation
of the original.

What the app does:

- Opens a Core Rules 2.0 Expansion installation or a data package.
- Creates and edits characters with the Core Rules or Player's Option,
  levels them up, manages parties and prints character sheets.
- Generates NPCs, treasure and encounters, and rolls on the books'
  tables.
- Edits every user database, and imports and exports DM Toolkit
  packages.

Everything it saves can still be opened by the original programs. The
[README](https://github.com/brahm/corerules#readme) lists every feature
and every difference from the original. It also says where to find the
disc images if yours are lost (you must own the original discs). The
app contains no rules content: you need your own copy of Core Rules.
`SHA256SUMS` lists the checksums of every file.
