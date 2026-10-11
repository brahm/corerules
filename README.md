<p align="center">
  <img src="data/icons/hicolor/256x256/apps/io.github.brahm.CoreRules.png" width="128" height="128" alt="Core Rules icon">
</p>

# Core Rules

A modern Linux replica of **AD&D Core Rules 2.0** (TSR/Evermore, 1998-1999),
written in Rust with GTK4 and libadwaita. It reads and writes the data files
of the original Windows 95 program, so characters made in either program
open in the other.

> This project contains no TSR or Wizards of the Coast content. It reads the
> rules data from a Core Rules installation you already own.

![The app showing a character made in the original program](docs/screenshots/character.png)

## Installing

Every [release](https://github.com/brahm/corerules/releases/latest) has
packages for x86-64 Linux:

| System | Package | Install |
| --- | --- | --- |
| Fedora 44 or newer | `.rpm` | `sudo dnf install ./corerules-*.rpm` |
| Ubuntu 24.04 or newer | `.deb` | `sudo apt install ./corerules_*_amd64.deb` |
| Any distribution | `.flatpak` | `flatpak install --user CoreRules-*.flatpak` |
| Any distribution (glibc 2.39+) | `.AppImage` | `chmod +x CoreRules-*.AppImage`, then run it |

The app is called **Core Rules** in the applications menu (`corerules` on
the command line); the `.deb` and `.rpm` also install the `cr` command-line
tool. The Flatpak uses the GNOME 50 runtime from Flathub and may read and
write your home folder, where Wine keeps Core Rules, and CDs under `/media`
and `/run/media`.

## Testing against your own installation

The rules databases are copyrighted and are not part of this repository.
To check the readers against them, point `CORERULES_DIR` at an installed
copy (the folder that holds `Database/`):

```sh
CORERULES_DIR="$HOME/.wine/drive_c/Program Files/AD&D Core Rules 2.0" cargo test
```

## Status

Early work (phase 0 of the plan): the file formats are being mapped.

- [x] MFC `CArchive` reader and writer, byte for byte
- [x] Character list (`Chars/index.chr`), round-trip tested
- [x] Full character files (`TSRP V27` to `TSRP3 V54`, both programs), round-trip tested
- [x] Races and classes, round-trip tested against a real installation
- [x] Survey of the rules databases (`cr survey`)
- [x] Field layouts documented in docs/formats
- [x] Monsters and encounters
- [x] Every shipped database in Rust, round-trip tested
- [x] Core Rules 2.0 (without the Expansion) databases, round-trip tested
- [x] GNOME app: browse characters, races, classes, items, spells, psionics
      and monsters of an installation or the CD
- [x] 2.0 character files checked against files the 2.0 program wrote
- [x] Character editing: name, player, social class, age, height, weight
      and ability scores, checked by opening the edited files in both
      original programs (close the original program while editing; it
      keeps the character list in memory)
- [x] Editing weapon and nonweapon proficiencies, native languages,
      spells known and items carried, checked in both original programs
- [x] Spellbooks and memorized spells, all nine equipment groups with
      quantities, and Spending Money, checked in both original programs
- [x] Adding a class where the race may dual-class (humans), with the
      Player's Handbook requirements, checked in both original programs;
      race and the classes chosen at creation never change
- [x] Creating characters (New Character, Ctrl+N), following the original
      wizard: campaign setting, ability rolls (Methods I to VI, entered
      scores or Player's Choice), race and subrace, class, details,
      proficiencies and learned languages, rogue skill points, spells and
      starting equipment; checked by opening the new files in both
      original programs. The original has no notion of campaign settings,
      so what each setting allows comes from its campaign book and only
      narrows what the installed databases offer (`crates/cr-store/src/settings.rs`).
- [x] Player's Option (Skills & Powers) creation, as the Expansion's
      Player's Option button: Methods VII to X, racial and class
      abilities bought with character points, subabilities, traits,
      disadvantages, proficiencies paid in points with ranks, weapon of
      choice, expertise and specialization, unarmed combat, shield and
      armor proficiencies, fighting styles and rogue skill points; checked
      against characters the Expansion made and by opening the new files
      in it
- [x] Experience and levels: give or take away experience per class, to
      all classes or split, with the prime requisite bonus, hit point
      rolls for each level gained, PSPs, Skills & Powers character points
      and the Expansion's racial level limits; spell slots per level
      checked against memorized spells; checked against characters the
      originals levelled up
- [x] Character sheet: every section the originals export (abilities and
      their tables, saving throws, THAC0, armor class, weapons, rogue
      skills, turning undead, spells, psionics, inventory, encumbrance),
      shown in the app, printed, saved as PDF, or exported as the
      originals' TXT and HTML files, matching their exports line for line
      (`cr sheet` does the same from the command line)
- [x] Treasure generator (backend and `cr treasure`): treasure types A to
      Z, table rolls, intelligent weapons, spell scrolls, Calculate and the
      saved bundles of `UserDbas/TreaBunU.dat`; with the same seed it
      saves the same bundles as both original programs, byte for byte

## Layout

| Crate | Purpose |
| --- | --- |
| `cr-formats` | Readers and writers for the original file formats, no UI |
| `cr-store` | Finds and opens an installation (case-insensitive paths) and builds the catalog the app shows |
| `cr-cli` | `cr` command for inspecting data |
| `cr-gtk` | The GNOME app |

The backend crates have no GTK dependency so macOS and Windows front ends
can reuse them later.

## Building

You need Rust 1.88 or newer, GTK 4.14 or newer and libadwaita 1.5 or newer.

On Fedora 44 or newer:

```sh
sudo dnf install cargo rust gcc gtk4-devel libadwaita-devel
cargo build --release
cargo test
./target/release/corerules
```

On Ubuntu 24.04 or newer (its packaged Rust is too old, so use
[rustup](https://rustup.rs)):

```sh
sudo apt install build-essential libgtk-4-dev libadwaita-1-dev
cargo build --release
cargo test
```

To install a build system-wide, run `packaging/install.sh / /usr/local`
as root after `cargo build --release`.

### Packages

`packaging/` builds every package (the `.rpm` and Flatpak compile from source,
the others package `target/release`):

```sh
packaging/deb/build.sh 0.1.0        # dist/corerules_0.1.0_amd64.deb (on Ubuntu 24.04)
packaging/rpm/build.sh 0.1.0        # dist/corerules-0.1.0-1.fc44.x86_64.rpm (on Fedora 44, needs rpm-build)
packaging/appimage/build.sh 0.1.0   # dist/CoreRules-0.1.0-x86_64.AppImage (on Ubuntu 24.04)
flatpak-builder --user --install-deps-from=flathub --install build \
    packaging/flatpak/io.github.brahm.CoreRules.yml
```

The Packages workflow in GitHub Actions builds all four, installs each one
on clean Ubuntu 24.04, Ubuntu latest, Fedora 44 and Fedora latest
containers and starts the app. Pushing a `v*` tag publishes them as a
release; `packaging/release-notes.md` is its description.

## Using an original installation

Install Core Rules 2.0 (and the Expansion) with Wine. The tools look in
`$WINEPREFIX/drive_c/Program Files/AD&D Core Rules 2.0`, or take a folder:

```sh
cargo run -p cr-cli -- chars
cargo run -p cr-cli -- survey "/path/to/AD&D Core Rules 2.0"
cargo run -p cr-gtk                                # the Wine prefix
cargo run -p cr-gtk -- "/media/cdrom"               # or any folder with Database/
```

Format notes live in [`docs/formats`](docs/formats).

## License

MIT, see [LICENSE](LICENSE). AD&D and Advanced Dungeons & Dragons are
trademarks of Wizards of the Coast; this project is not affiliated with them.
