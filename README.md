# Core Rules

A modern Linux replica of **AD&D Core Rules 2.0** (TSR/Evermore, 1998-1999),
written in Rust with GTK4 and libadwaita. It reads and writes the data files
of the original Windows 95 program, so characters made in either program
open in the other.

> This project contains no TSR or Wizards of the Coast content. It reads the
> rules data from a Core Rules installation you already own.

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
- [x] Full character files (`TSRP V27` to `TSRP3 V54`), round-trip tested
- [x] Races and classes, round-trip tested against a real installation
- [x] Survey of the rules databases (`cr survey`)
- [x] Field layouts documented in docs/formats
- [x] Monsters and encounters
- [x] Every shipped database in Rust, round-trip tested

## Layout

| Crate | Purpose |
| --- | --- |
| `cr-formats` | Readers and writers for the original file formats, no UI |
| `cr-store` | Finds and opens an installation (case-insensitive paths) |
| `cr-cli` | `cr` command for inspecting data |
| `cr-gtk` | The GNOME app |

The backend crates have no GTK dependency so macOS and Windows front ends
can reuse them later.

## Building

```sh
# Debian/Ubuntu: sudo apt install libgtk-4-dev libadwaita-1-dev
cargo build
cargo test
```

## Using an original installation

Install Core Rules 2.0 (and the Expansion) with Wine. The tools look in
`$WINEPREFIX/drive_c/Program Files/AD&D Core Rules 2.0`, or take a folder:

```sh
cargo run -p cr-cli -- chars
cargo run -p cr-cli -- survey "/path/to/AD&D Core Rules 2.0"
cargo run -p cr-gtk
```

Format notes live in [`docs/formats`](docs/formats).

## License

MIT, see [LICENSE](LICENSE). AD&D and Advanced Dungeons & Dragons are
trademarks of Wizards of the Coast; this project is not affiliated with them.
