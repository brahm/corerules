**Core Rules 1.1.0: now on macOS.** Core Rules now runs on Linux,
Windows 11 and macOS, with the same features on all three, the same
backend and the same data files. The Mac version is native. It looks and
works like a Mac app: the sections, the entries and the details in three
columns, every action in the menu bar, and the sheets, the help and the
generators in their own windows. It went through a beta (1.1.0-beta1) on
an Apple Silicon Mac, where it worked as it should. Nothing changes in
what the Linux and Windows 11 versions do. More of their code is now
shared by the three versions. Characters and user databases work on all
three systems and in the original programs.

| System | File | Install |
| --- | --- | --- |
| macOS 14 or newer, Apple Silicon (M1 or newer) | `CoreRules-1.1.0-arm64.dmg` | Open it and drag Core Rules to Applications, then see below |
| Windows 11 | [Microsoft Store](https://apps.microsoft.com/detail/9nr2cgp649jm) | Get it from the Store page |
| Fedora 44 or newer | `corerules-1.1.0-1.fc44.x86_64.rpm` | `sudo dnf install ./corerules-1.1.0-1.fc44.x86_64.rpm` |
| Ubuntu 26.04 or newer | `corerules_1.1.0_amd64.deb` | `sudo apt install ./corerules_1.1.0_amd64.deb` |
| Arch Linux, CachyOS, Omarchy | `corerules-1.1.0-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./corerules-1.1.0-1-x86_64.pkg.tar.zst` |
| Any distribution with Flatpak, such as Bazzite | The [Flatpak repository](https://brahm.github.io/corerules/io.github.brahm.CoreRules.flatpakref) | `flatpak install --user https://brahm.github.io/corerules/io.github.brahm.CoreRules.flatpakref` |
| Any distribution (glibc 2.43+) | `CoreRules-1.1.0-x86_64.AppImage` | `chmod +x CoreRules-1.1.0-x86_64.AppImage`, then run it |

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
