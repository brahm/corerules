# archive: MFC `CArchive` serialization

Every `.dat`, `.chr`, `.xpt` and `.xpx` file of Core Rules 2.0 and the
Expansion is written by MFC 4.x `CArchive` (Visual C++ 4/5), the way each
class's `Serialize` stores its members. This document is the encoding those
files share; the other documents in this folder describe what each class
stores with it.

Reader and writer: [`crates/cr-formats/src/archive.rs`](../../crates/cr-formats/src/archive.rs)
(`ArchiveReader`, `ArchiveWriter`), with the value types in
[`field.rs`](../../crates/cr-formats/src/field.rs) and the polymorphic
objects in [`object.rs`](../../crates/cr-formats/src/object.rs).

An archive has no header, no magic number and no checksum: the first byte of
a file is the first member the document's `Serialize` writes. The layout is
only known from the code that reads it, so a reader must know which document
class a file belongs to (see the table in [README.md](README.md)).

## Scalars

| type | bytes | notes |
|---|---|---|
| `u8` | 1 | `BYTE` |
| `bool` | 1 | a `BYTE` or `BOOL` member stored as one byte; the programs test it with `!= 0` and only ever write 0 or 1 |
| `u32`, `i32` | 4 | little-endian `DWORD`, `int`, `long`, `UINT`, `BOOL` stored with `<<` on a 4-byte type |
| `f32 bits` | 4 | a `float` stored through a 4-byte integer helper: the IEEE 754 bit pattern, little-endian |

The programs read most 4-byte members with the same helper whether they are
signed or not (0x40d250 and 0x40d1b0 in TSRP3.EXE), so the documents give
`i32` or `u32` after the values found in the data. There are no 2-byte or
8-byte members in any Core Rules class.

## Counts

`WriteCount` / `ReadCount`, used by every collection:

| value | bytes |
|---|---|
| below `0xFFFF` | `u16` |
| `0xFFFF` or more | `FF FF`, then `u32` |

Some classes store a count of their own with a plain `u32` instead
(`CCharacterDocRogueSkillsOb`, the class list of a character); the documents
call that `u32 count`.

## Strings (`CString`)

ANSI `CString`, length in bytes, then the bytes with no terminator:

| length | prefix |
|---|---|
| 0 to 254 | `u8` length |
| 255 to 65533 | `FF`, then `u16` length |
| 65534 or more | `FF FF FF`, then `u32` length |

`FF FE FF` introduces a Unicode string in MFC; neither program writes one,
and the reader rejects it. The text is Windows-1252. The programs compare
names with `strcmp` or `lstrcmpi` on those bytes, so a name must survive the
round trip through Windows-1252 unchanged; the writer refuses characters the
code page lacks rather than replacing them.

Many strings are padded or trimmed by the program before use (names are
trimmed and compared without regard to case on import, for example); the
file keeps them as typed.

## Collections

Each MFC collection class serializes as a count followed by its elements:

| type | elements |
|---|---|
| `CStringArray`, `CStringList` | `CString`s |
| `CDWordArray`, `CUIntArray` stored as such | `u32`s |
| `CByteArray` | `u8`s |
| `CObArray<C>`, `CObList<C>` | objects (see below), each with its tag |
| `CMapStringToOb` | pairs of `CString` key and object |

`T[N]` in the documents is a C array member written element by element in a
loop: `N` values with no count. `(A, B)` is two values written one after the
other, typically two members stored in the same loop step.

A `CObArray` and a `CObList` of the same objects are byte for byte the same,
so the documents name the one the program declares.

`CMapStringToOb` writes its pairs in hash-bucket order and rebuilds each
bucket's chain in reverse on load. The order is therefore not meaningful,
and only the order as read round-trips; `Convert.dat` ([misc.md](misc.md))
is the only map in the data.

## Objects

`WriteObject` (a `CObject*` member, or an element of a `CObArray`/`CObList`)
writes a tag, then, for a new object, the object's own `Serialize`:

| tag (`u16`) | meaning |
|---|---|
| `0x0000` | `NULL` pointer |
| `0xFFFF` | new class: `u16` schema, `u16` name length, the class name in ASCII, then the object |
| `0x8000 \| n` | new object of the class stored at index `n` |
| `0x0001` to `0x7FFE` | a reference to the object already stored at that index |
| `0x7FFF` | big index: a `u32` follows; with bit 31 set it is a class index, otherwise an object reference |

Classes and objects share one index counter per archive, which starts at 1:
a new class takes the next index, then the object takes the one after it,
and each later object of a known class takes one index. A reader that does
not keep the counter exactly will resolve the next `0x8000 | n` to the
wrong class.

The counter lives as long as the archive, not the collection: when a
document writes several collections into one file (the 21 part arrays of
`Parts.dat`, the wizard and priest lists of `Spells.dat`, the sections of an
export package, the eight maps of `Convert.dat`), a class first seen in one
collection is referred to as `0x8000 | n` in the next. A file can only be
read from its start.

Object references (a tag below `0x8000`) never occur in the shipped data:
every object is stored once. The reader decodes the tag but treats a
reference where an object is expected as an error, and the writer never
produces one.

### Schemas

The schema in a new-class tag is the class's `IMPLEMENT_SERIAL` schema.
Each class has one schema in both programs (88, 93, 94, 95, 96 or 1); the
documents give it in each class heading, and the reader rejects an object
whose schema differs. Classes declared with `VERSIONABLE_SCHEMA` (shown as
"versionable schema 1") store the schema without the flag bit, so the file
holds 1.

Most classes keep their real layout version elsewhere: a version string or
number at the start of their own `Serialize` (`TSRP3 V54` for characters,
`TSRP3 V39` for classes, `TSRP3 V7` for parts), and the load code skips the
members an older version lacks. Those versions and conditions are in each
class's document.

### Inline members

A member that is a C++ object rather than a pointer (an embedded
`CInventoryBundle`, a monster's arsenal, a character inside an encounter) is
written by calling its `Serialize` directly: its fields with no tag. The
documents link such members to the section that describes them.

## Version strings

The documents that start with a version string (characters, classes, parts,
kits, export packages, frequency tables) store it as an ordinary `CString`
(`"TSRP3 V54"`). Each program has its own rule for turning it into a
number, copied into the matching `version_of` function: the documents give
the rule with the class. Programs only ever write their current version;
older versions are read and, when saved again, written in the current
layout.

## Empty files

A user database the program has never written may be 0 bytes long. The
programs read that as an empty document, and so does this reader
([`database::survey`](../../crates/cr-formats/src/database.rs)). An empty
`CObList` file written by the program is `00 00`; an empty `Parts` file is
21 such counts (42 bytes) and an empty `Spells` file two (4 bytes).

## Round trip

Every reader in `cr-formats` keeps every field, named or not, and its
writer produces the same bytes it read, so the original programs keep
working on files this app has saved. The tests read every shipped and
fixture file, write it back and compare byte for byte. Fields whose meaning
is not known are named after their member offset in the original object
(`unknown_0x2c` is `this + 0x2c`), so they can be traced in the
disassembly.

## Saving

Before it replaces a file the app copies the current one to `<name>.bak`,
then writes `<name>.tmp` next to it and renames that over the original, so
a failed save leaves the old file in place and the previous contents stay
in the backup (deleting a party also keeps each member as `.bak` until the
deletion is final, see [character.md](character.md)). A file that does not
exist yet goes through the temporary file too, with no backup. Files are matched
without regard to case (`Database` and `DATABASE`, `index.chr` and
`INDEX.CHR`), and an existing file keeps the case it has.
