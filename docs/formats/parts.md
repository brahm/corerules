# Parts / Spells / Allowed-classes files (module `parts`)

Reader: `py/parts.py` (registers readers in `arc.READERS` and file framings in `arc.FRAMINGS`;
`py/db.py` consults `FRAMINGS` by lowercase basename before falling back to "one CObList").

Type legend: i32 = 4-byte LE (all three 4-byte helpers 0x40d250/0x40d1b0/0x447560), u8 = 1 byte
(0x40d1f0; "u8 (bool)" = stored as `!= 0` into a BOOL byte, so the file byte may be any value but
the program normalises it), CString / CStringArray / CObArray = MFC archive forms (see arc.py).
`object X*` = `CArchive::ReadObject` (class tag + body, or u16 0 for NULL).

## File framing

| file | document class / Serialize | framing | parses to EOF |
|---|---|---|---|
| Database/Parts.dat | CPartDoc 0x67b10d | **21 consecutive CObArrays**, no outer count | yes |
| UserDbas/PartsU.dat | CPartDoc 0x67b10d | same: 21 empty arrays = 21 x u16 0 = 42 bytes | yes |
| Database/Spells.dat | CSpellsDoc 0x4f9d89 | CObArray +0x58 (wizard spells, 480) then CObArray +0x6c (priest spells, 453) | yes |
| UserDbas/SpellsU.dat | CSpellsDoc 0x4f9d89 | same: 2 empty arrays = 4 bytes | yes |
| UserDbas/AllowedU.dat | CClassesAllowedDoc 0x680e21 | one CObArray (+0x54) of CClassesAllowedOb (30 races) | yes |
| UserDbas/ClassU.dat | CClassDoc 0x68416a | one CObArray (+0x54), same as Class.dat; shipped file is count 0 | yes |
| UserDbas/RaceU.dat | CRaceDoc 0x6849fa | one CObArray (+0x54), same as Race.dat; shipped file is count 0 | yes |

All doc Serialize functions call DeleteContents first when loading and then each array's own
`Serialize` (`call [vtbl+8]`), i.e. `count (u16, 0xffff -> u32) + objects`. The class-tag table
(`R.slots`) is shared across all arrays of one file: in Spells.dat the priest list starts
`c5 01` (count 453) and its first object is tagged `01 80` (class index 1 = CSpellsOb from the
wizard list).

### Parts.dat array index = part type

CPartDoc +0x54 is a CObArray of 21 CObArrays created in DeleteContents (0x677978, 21 calls to
`Add`). The same function fills a name -> type table (+0x324, CPartIDOb-like) whose indices match
the data:

| idx | type (string in 0x677978) | records in Parts.dat |
|---|---|---|
| 0 | Weapons | 733 |
| 1 | Armor | 573 |
| 2 | "NOT" (unused) | 0 |
| 3 | Weapon Proficiencies | 0 |
| 4 | Non-Weapon Proficiencies | 122 |
| 5 | Racial Abilities | 167 |
| 6 | Languages | 61 |
| 7 | Equipment | 306 |
| 8 | Magical Items | 689 |
| 9 | Schools Of Magic | 22 |
| 10 | "NOT" (unused) | 0 |
| 11 | Traits | 28 |
| 12 | Disadvantages | 38 |
| 13 | Gems and Art | 1383 |
| 14 | Class Abilities | 174 |
| 15 | Character Kits | 210 (each carries a CPartKitOb) |
| 16 | Currency Types | 5 |
| 17 | Count (units: "Charge", "Dose", ...) | 45 |
| 18 | Spheres | 28 |
| 19, 20 | (no name) | 0 |

All 4584 CParts are version 7 (`"TSRP3 V7"`); all 210 CPartKitObs are version 30.

## CPart (schema 93, size 0x894, Serialize 0x65c3a0, load at 0x65dd44)

Current version 7 is formatted as `"TSRP3 V%ld"` at entry. After the first block an i32 is read:
if it is < -8888 (writer stores 0xffffdd47) a version CString follows and then `m_0x728`;
otherwise that i32 *is* `m_0x728` and the record is version 0. Fields named by offset; notes
give sample values. "f32 bit pattern" fields are read as 4-byte ints but hold IEEE floats.

| field | type | condition | notes |
|---|---|---|---|
| `m_0x198` | CString |  | name (e.g. "Belaying pin") |
| `m_0x300` | CString |  |  |
| `m_0x308` | i32 |  |  |
| `m_0x230` | i32 |  |  |
| `m_0x304` | CString |  | description text |
| `m_0x30c` | i32 |  |  |
| `m_0x310` | i32 |  |  |
| `m_0x314` | i32 |  |  |
| `m_0x318` | i32 |  |  |
| `m_0x31c` | i32 |  |  |
| `m_0x320` | i32 |  |  |
| `m_0x324` | i32 |  |  |
| `m_0x328` | i32 |  |  |
| `m_0x32c` | i32 |  |  |
| `m_0x330` | i32 |  |  |
| `m_0x334` | i32 |  |  |
| `m_0x338` | i32 |  |  |
| `m_0x33c` | i32 |  |  |
| `m_0x340` | i32 |  |  |
| `m_0x344` | i32 |  |  |
| `m_0x348` | i32 |  |  |
| `m_0x34c` | i32 |  |  |
| `m_0x350` | i32 |  |  |
| `m_0x354` | i32 |  |  |
| `m_0x394` | i32 |  |  |
| `m_0x398` | i32 |  |  |
| `m_0x39c` | i32 |  |  |
| `m_0x2ac` | i32 |  |  |
| `m_0x80c` | i32 |  |  |
| `m_0x810` | i32 |  |  |
| `m_0x814` | i32 |  |  |
| `m_0x818` | i32 |  |  |
| `m_0x2a8` | i32 |  | -1 default |
| `m_0x4cc` | i32 |  |  |
| `m_0x470` | i32 |  |  |
| `m_0x474` | i32 |  |  |
| `m_0x478` | i32 |  |  |
| `m_0x47c` | i32 |  |  |
| `m_0x240` | CString |  | equipment category ("Misc","Clothing","Animal") |
| `m_0x460` | CString |  | NWP relevant ability ("Intelligence") |
| `m_0x45c` | CString |  | NWP sub-ability ("Wisdom/Intuition") |
| `m_0x244` | CString |  | equipment source group ("Elven Equipment") |
| `m_0x1b4` | CString |  | school family ("School of Effects") |
| `m_0x464` | CString |  | related NWP name |
| `m_0x1e4` | i32 |  |  |
| `m_0x1e8` | CString |  |  |
| `m_0x1ec` | i32 |  | f32 bit pattern (0x3dcccccd = 0.1 most common) |
| `m_0x1f0` | i32 |  | f32 bit pattern |
| `m_0x1f8` | i32 |  |  |
| `m_0x1fc` | i32 |  |  |
| `m_0x200` | i32 |  |  |
| `m_0x204` | i32 |  |  |
| `m_0x208` | i32 |  |  |
| `m_0x20c` | i32 |  |  |
| `m_0x210` | i32 |  |  |
| `m_0x214` | i32 |  |  |
| `m_0x224` | i32 |  |  |
| `m_0x234` | CString |  | CString holding a number ("209") |
| `m_0x238` | i32 |  |  |
| `m_0x23c` | i32 |  |  |
| `m_0x24c` | i32 |  |  |
| `m_0x248` | i32 |  |  |
| `m_0x250` | i32 |  |  |
| `m_0x254` | i32 |  |  |
| `m_0x258` | i32 |  |  |
| `m_0x25c` | i32 |  |  |
| `m_0x260` | i32 |  |  |
| `m_0x264` | i32 |  |  |
| `m_0x268` | i32 |  |  |
| `m_0x26c` | i32 |  |  |
| `m_0x270` | i32 |  |  |
| `m_0x274` | i32 |  |  |
| `m_0x278` | i32 |  | f32 bit pattern (default -1.0) |
| `m_0x27c` | i32 |  |  |
| `m_0x280` | i32 |  |  |
| `m_0x284` | i32 |  |  |
| `m_0x288` | i32 |  |  |
| `m_0x28c` | i32 |  |  |
| `m_0x290` | i32 |  |  |
| `m_0x294` | i32 |  |  |
| `m_0x298` | i32 |  |  |
| `m_0x29c` | i32 |  |  |
| `m_0x2a0` | i32 |  |  |
| `m_0x2a4` | i32 |  |  |
| `m_0x2b4` | i32 |  |  |
| `m_0x2b8` | i32 |  |  |
| `m_0x2bc` | i32 |  |  |
| `m_0x2c0` | i32 |  |  |
| `m_0x2c4` | i32 |  |  |
| `m_0x2c8` | i32 |  |  |
| `m_0x2cc` | i32 |  |  |
| `m_0x2d0` | i32 |  |  |
| `m_0x2d4` | i32 |  |  |
| `m_0x458` | CString |  | ability/sub-ability ("Dexterity/Aim") |
| `m_0x3b4` | CString |  | weapon damage type ("P","S","B") |
| `m_0x3e4` | CString |  | second-mode damage type |
| `m_0x3b8` | i32 |  |  |
| `m_0x3bc` | i32 |  |  |
| `m_0x3c0` | i32 |  |  |
| `m_0x3c4` | i32 |  |  |
| `m_0x3e8` | i32 |  |  |
| `m_0x3c8` | i32 |  |  |
| `m_0x3ec` | i32 |  |  |
| `m_0x3cc` | i32 |  |  |
| `m_0x3f0` | i32 |  |  |
| `m_0x3d0` | CString |  | weapon size ("S","M","L") |
| `m_0x3f4` | CString |  | second-mode size |
| `m_0x3d4` | i32 |  |  |
| `m_0x3f8` | i32 |  |  |
| `m_0x3d8` | i32 |  |  |
| `m_0x3fc` | i32 |  |  |
| `m_0x3dc` | i32 |  |  |
| `m_0x400` | i32 |  |  |
| `m_0x3e0` | i32 |  |  |
| `m_0x404` | i32 |  |  |
| `m_0x40c` | i32 |  |  |
| `m_0x410` | i32 |  |  |
| `m_0x414` | i32 |  |  |
| `m_0x418` | i32 |  |  |
| `m_0x41c` | CString |  | rate of fire ("2/rnd") |
| `m_0x408` | CString |  | rate of fire (second mode) |
| `m_0x44c` | i32 |  |  |
| `m_0x450` | i32 |  |  |
| `m_0x454` | i32 |  |  |
| `m_0x2d8` | i32 |  |  |
| `m_0x2dc` | i32 |  |  |
| `m_0x2e0` | i32 |  |  |
| `m_0x2e4` | i32 |  |  |
| `m_0x2e8` | i32 |  |  |
| `m_0x2ec` | i32 |  |  |
| `m_0x2f0` | i32 |  |  |
| `m_0x2f4` | i32 |  |  |
| `m_0x2f8` | i32 |  |  |
| `m_0x2fc` | i32 |  |  |
| marker | i32 | | `0xffffdd47` (-8889) = versioned record; if >= -8888 the word **is** `m_0x728` and v=0 |
| version string | CString | marker < -8888 | `"TSRP3 V7"`; v = 7 if equal, else atoi(s[7:]) if len>7, else 6 |
| `m_0x728` | i32 | marker < -8888 | |
| `m_0x72c` | i32 |  |  |
| `m_0x730` | i32 |  |  |
| `m_0x734` | i32 |  |  |
| `m_0x738` | i32 |  |  |
| `m_0x73c` | i32 |  |  |
| `m_0x740` | i32 |  |  |
| `m_0x480` | i32 |  |  |
| `m_0x484` | i32 |  |  |
| `m_0x488` | i32 |  |  |
| `m_0x420` | i32 |  |  |
| `m_0x424` | i32 |  |  |
| `m_0x42c` | i32 |  |  |
| `m_0x4a4` | i32 |  | class flag Bard (pre-v1: converted into m_0x82c by 0x661697) |
| `m_0x4a8` | i32 |  | class flag Cleric (pre-v1) |
| `m_0x4ac` | i32 |  | class flag Druid (pre-v1) |
| `m_0x4b0` | i32 |  | class flag Fighter (pre-v1) |
| `m_0x4b4` | i32 |  | class flag Mage (pre-v1) |
| `m_0x4b8` | i32 |  | class flag Paladin (pre-v1) |
| `m_0x4bc` | i32 |  | class flag Ranger (pre-v1) |
| `m_0x4c0` | i32 |  | class-flag slot not converted (Psionicist?) |
| `m_0x4c4` | i32 |  | class flag Thief (pre-v1) |
| `m_0x4c8` | i32 |  |  |
| (no read) | | marker > -8889 | call 0x661697: pre-v1 class flags -> names appended to `m_0x82c`, flags zeroed |
| `m_0x510` | i32 |  |  |
| `m_0x514` | i32 |  |  |
| `m_0x518` | i32 |  |  |
| `m_0x51c` | i32 |  |  |
| `m_0x520` | i32 |  |  |
| `m_0x524` | i32 |  |  |
| `m_0x528` | i32 |  |  |
| `m_0x52c` | i32 | v > 6 | |
| `m_0x530` | i32 |  |  |
| `m_0x534` | i32 |  |  |
| `m_0x538` | i32 |  |  |
| `m_0x53c` | i32 |  |  |
| `m_0x540` | i32 |  |  |
| `m_0x544` | i32 |  |  |
| `m_0x548` | i32 |  |  |
| `m_0x54c` | i32 |  |  |
| `m_0x550` | i32 |  |  |
| `m_0x554` | i32 |  |  |
| `m_0x558` | i32 |  |  |
| `m_0x55c` | i32 |  |  |
| `m_0x560` | i32 |  |  |
| `m_0x564` | i32 |  |  |
| `m_0x568` | i32 |  |  |
| `m_0x56c` | i32 |  |  |
| `m_0x48c` | i32 |  |  |
| `m_0x490` | i32 |  |  |
| `m_0x494` | i32 |  |  |
| `m_0x498` | i32 |  |  |
| `m_0x49c` | i32 |  |  |
| `m_0x4a0` | i32 |  |  |
| `m_0x584` | i32 |  |  |
| `m_0x588` | i32 |  |  |
| `m_0x58c` | i32 |  |  |
| `m_0x590` | i32 |  |  |
| `m_0x594` | i32 |  |  |
| `m_0x220` | i32 |  |  |
| `m_0x59c` | i32 |  | armor: thief-skill adjustments (-40 ...), block +0x59c..+0x5cc |
| `m_0x5a0` | i32 |  |  |
| `m_0x5a4` | i32 |  |  |
| `m_0x5a8` | i32 |  |  |
| `m_0x5ac` | i32 |  |  |
| `m_0x5b0` | i32 |  |  |
| `m_0x5b4` | i32 |  |  |
| `m_0x5b8` | i32 |  |  |
| `m_0x5bc` | i32 |  |  |
| `m_0x5c0` | i32 |  |  |
| `m_0x5c4` | i32 |  |  |
| `m_0x5c8` | i32 |  |  |
| `m_0x5cc` | i32 |  |  |
| `m_0x598` | i32 |  |  |
| `m_0x4[101]` | 101 x i32 | v > 5 | +0x4..+0x194; data: all 99 except 7 records |
| `m_0x570` | i32 |  |  |
| `m_0x5e4` | i32 |  |  |
| `m_0x5e8` | i32 |  |  |
| `m_0x5ec` | i32 |  |  |
| `m_0x5f0` | i32 |  |  |
| `m_0x5f4` | i32 |  |  |
| `m_0x5f8` | i32 |  |  |
| `m_0x5fc` | i32 |  |  |
| `m_0x600` | i32 |  |  |
| `m_0x604` | i32 |  |  |
| `m_0x608` | i32 |  |  |
| `m_0x60c` | i32 |  |  |
| `m_0x610` | i32 |  |  |
| `m_0x614` | i32 |  |  |
| `m_0x618` | i32 |  |  |
| `m_0x61c` | i32 |  |  |
| `m_0x620` | i32 |  |  |
| `m_0x624` | i32 |  |  |
| `m_0x628` | i32 |  |  |
| `m_0x62c` | i32 |  |  |
| `m_0x630` | i32 |  |  |
| `m_0x634` | i32 |  |  |
| `m_0x638` | i32 |  |  |
| `m_0x63c` | i32 |  |  |
| `m_0x640` | i32 |  |  |
| `m_0x664` | i32 |  |  |
| `m_0x668` | i32 |  |  |
| `m_0x678` | i32 |  |  |
| `m_0x648` | i32 |  |  |
| `m_0x64c` | i32 |  |  |
| `m_0x644` | i32 |  |  |
| `m_0x228` | i32 |  |  |
| `m_0x19c` | i32 |  |  |
| `m_0x2b0` | i32 |  |  |
| `m_0x22c` | i32 |  |  |
| `m_0x218` | i32 |  |  |
| `m_0x75c` | i32 |  |  |
| `m_0x760` | i32 |  |  |
| `m_0x724` | u8 (bool) | v > 4 | |
| `m_0x868` | CString |  |  |
| `m_0x86c` | CString |  |  |
| `m_0x870` | CString |  |  |
| `m_0x874` | CString |  |  |
| `m_0x878` | CString |  |  |
| `m_0x87c` | CString |  |  |
| `m_0x880` | CString |  |  |
| `m_0x884` | CString |  |  |
| `m_0x888` | CString |  |  |
| `m_0x88c` | CString |  |  |
| `m_0x468` | CString |  |  |
| `m_0x46c` | CString |  | NWP group ("GENERAL","PRIEST") |
| `m_0x428` | CString |  | related item name ("Sword, short") |
| `m_0x430` | CString |  | related armor name |
| `m_0x574` | CString |  |  |
| `m_0x578` | CString |  |  |
| `m_0x57c` | CString |  |  |
| `m_0x580` | CString |  | race name (racial abilities) |
| `m_0x21c` | CString |  | race name (traits/disadv.) |
| `m_0x1e0` | CString |  | race name (racial abilities) |
| `m_0x1f4` | i32 |  | f32 bit pattern |
| `m_0x744` | i32 |  |  |
| `m_0x778` | i32 |  |  |
| `m_0x77c` | i32 |  |  |
| `m_0x780` | u8 (bool) |  |  |
| `m_0x781` | u8 (bool) |  |  |
| `m_0x782` | u8 (bool) |  |  |
| `m_0x783` | u8 (bool) |  |  |
| `m_0x784` | u8 (bool) |  |  |
| `m_0x785` | u8 (bool) |  |  |
| `m_0x7a0` | i32 |  |  |
| `m_0x66c` | i32 |  |  |
| `m_0x670` | i32 |  |  |
| `m_0x674` | i32 |  |  |
| `m_0x7b8` | i32 |  |  |
| `m_0x7a4` | i32 |  |  |
| `m_0x7a8` | i32 |  |  |
| `m_0x7ac` | i32 |  |  |
| `m_0x7b0` | i32 |  |  |
| `m_0x7bc` | i32 |  |  |
| `m_0x71c` | i32 |  |  |
| `m_0x78c` | i32 |  |  |
| `m_0x788` | i32 |  |  |
| `m_0x794` | i32 |  |  |
| `m_0x79c` | i32 |  |  |
| `m_0x790` | i32 |  |  |
| `m_0x7cc` | i32 |  |  |
| `m_0x7d0` | i32 |  |  |
| `m_0x7d4` | i32 |  |  |
| `m_0x7d8` | i32 |  |  |
| `m_0x7dc` | i32 |  |  |
| `m_0x7e0` | i32 |  |  |
| `m_0x7e4` | i32 |  |  |
| `m_0x7b4` | i32 |  |  |
| `m_0x4f0` | i32 |  |  |
| `m_0x4e4` | i32 |  |  |
| `m_0x7e8` | i32 |  |  |
| `m_0x7ec` | i32 |  |  |
| `m_0x7f0` | i32 |  |  |
| `m_0x7f4` | i32 |  |  |
| `m_0x7f8` | i32 |  |  |
| `m_0x7fc` | i32 |  |  |
| `m_0x800` | i32 |  |  |
| `m_0x804` | i32 |  |  |
| `m_0x808` | i32 |  |  |
| `m_0x7c0` | i32 |  |  |
| `m_0x7c4` | i32 |  |  |
| `m_0x7c8` | i32 |  |  |
| `m_0x81c` | i32 |  |  |
| `m_0x820` | i32 |  |  |
| `m_0x824` | i32 |  |  |
| `m_0x828` | i32 |  |  |
| `m_0x798` | i32 |  |  |
| `m_0x710` | CString |  |  |
| `m_0x714` | CString |  |  |
| `m_0x650` | i32 |  |  |
| `m_0x67e` | u8 (bool) |  |  |
| `m_0x680` | u8 (bool) |  |  |
| `m_0x681` | u8 (bool) |  |  |
| `m_0x654` | i32 |  |  |
| `m_0x67f` | u8 (bool) |  |  |
| `m_0x718` | CString |  | class ability name ("Tracking") |
| `m_0x660` | i32 |  | 0x80000000 on all non-kit parts, 0 on kits |
| `m_0x67d` | u8 (bool) |  |  |
| `m_0x720` | CString |  |  |
| `m_0x65c` | i32 |  | 100 on non-kits, 0 on kits |
| `m_0x67c` | u8 (bool) |  |  |
| `m_0x658` | i32 |  |  |
| `m_0x4d0` | u8 (bool) | v > 1 | |
| `m_0x448` | u8 (bool) | v > 2 | |
| `m_0x890` | object CPartKitOb* | v > 3 | ReadObject(RUNTIME_CLASS(CPartKitOb)) (0x66ae55); null tag (u16 0) on every non-kit part |
| `m_0x6e8` | CStringArray | |  |
| `m_0x358` | CStringArray | |  |
| `m_0x36c` | CStringArray | |  |
| `m_0x380` | CStringArray | |  |
| `m_0x3a0` | CStringArray | |  |
| `m_0x1a0` | CStringArray | | school names (Schools of Magic) |
| `m_0x1b8` | CStringArray | | historical eras |
| `m_0x1cc` | CStringArray | | weapon groups ("Swords: Large") |
| `m_0x840` | CStringArray | |  |
| `m_0x854` | CStringArray | |  |
| `m_0x434` | CStringArray | | ammo/launcher names |
| `m_0x4fc` | CStringArray | | weapon names |
| `m_0x5d0` | CStringArray | |  |
| `m_0x684` | CStringArray | |  |
| `m_0x698` | CStringArray | |  |
| `m_0x6c0` | CStringArray | |  |
| `m_0x6ac` | CStringArray | |  |
| `m_0x6d4` | CStringArray | |  |
| `m_0x6fc` | CStringArray | |  |
| `m_0x764` | CStringArray | | class ability names |
| `m_0x748` | CObArray<CSpellsOb> | | always empty in data; element class from 0x65ff46 (new 0x74 + CSpellsOb ctor 0x4ff04e) |
| `m_0x82c` | CStringArray | v > 0 | allowed class names ("Cleric","Druid","Mage",...) |

## CPartKitOb (schema 1, size 0x678, Serialize 0x66ce22, load at 0x66e1a8)

Hung off `CPart::m_0x890` (only for parts of type 15, Character Kits). Self-versioned:
a header CString then a u32 version (current 30). If the header is not `"CPKO"` the program
reports "Attempting to load kit information, found header %s expecting header %s" and returns
without reading further; a version > 30 only produces a warning. Conditions below use v.
(u8 rows without "(bool)" are read as raw BYTEs.)

| field | type | condition | notes |
|---|---|---|---|
| header | CString | | `"CPKO"` |
| v | u32 | | 30 in all data |
| `m_0x4..m_0xe` (11) | u8 (bool) each | | +0x4,+0x5,...,+0xe |
| `m_0x71`, `m_0x72`, `m_0x444` | u8 (bool) | | |
| `m_0x4b4` | u8 (bool) | v > 29 | |
| `m_0x75..m_0x7c` (8), `m_0x84` | u8 (bool) | | |
| `m_0x4c..m_0x70` (37) | u8 | | raw bytes; e.g. 25,25,...,10,...,12 (ability-score-like values) |
| `m_0x80` | i32 | | |
| old_id_type | i32 (local) | v < 3 | type for the old kit-part list below |
| `m_0x88`,`m_0x8c`,`m_0x90`,`m_0x94`,`m_0x98` | i32 | | e.g. 150, 100, -, 0x80000000 |
| `m_0x9c`,`m_0xa0`,`m_0xa4` | CString | | `m_0x9c`/`m_0xa0`: long description texts |
| `m_0x670` | i32 | v > 19 | |
| `m_0x674` | CString | v > 19 | e.g. "Fighter" |
| old_id_names | CStringArray (local) | v < 3 | converted to `CPartIDOb(name, old_id_type)` in `m_0x244` |
| `m_0x10` | CStringArray | | race names ("Deep dwarf","Dwarf") |
| `m_0x24`, `m_0x38` | CStringArray | | |
| `m_0xa8` | CStringArray | | race names (monster races "Aarakocra",...) |
| `m_0x384` | CStringArray | | weapon names ("Quarterstaff") |
| `m_0xbc`, `m_0xd0` | CStringArray | | weapon names |
| `m_0x258` | CStringArray | | NWP names ("Disguise","Etiquette",...) |
| `m_0xe4` | CStringArray | | trait names ("Alertness","Ambidexterity") |
| `m_0xf8` | CStringArray | | ("Net") |
| `m_0x10c` | CStringArray | | 13 social-class strings indexed by roll ("", "", "Lower class", ...) |
| `m_0x244` | CObArray<CPartIDOb> | v >= 3 | |
| `m_0x73`, `m_0x74` | u8 (bool) | v >= 4 | |
| `m_0x120`,`m_0x124`,`m_0x13c`,`m_0x154`,`m_0x16c`,`m_0x170` | i32 | v >= 5 | |
| `m_0x128`,`m_0x140`,`m_0x158` | CStringArray | v >= 5 | thief/bard skill names ("Thief, Move silently* (5)") |
| `m_0x174` | CObArray<CPartIDOb> | v >= 5 | ("Bard, Climb walls...") |
| `m_0x188`,`m_0x189` | u8 (bool) | v >= 6 | |
| `m_0x18c`..`m_0x19c` (5) | i32 | v >= 6 | |
| `m_0x1c8`,`m_0x1dc`,`m_0x208` | CStringArray | v >= 7 | class abilities / schools / spheres |
| `m_0x1f0`,`m_0x1fc` | u8 (bool) | v >= 7 | |
| `m_0x1f4`,`m_0x1f8`,`m_0x200`,`m_0x204` | i32 | v >= 7 | |
| `m_0x26c`,`m_0x280`,`m_0x294`,`m_0x2a8`,`m_0x2bc`,`m_0x2d0`,`m_0x2e4` | CStringArray | v >= 8 | NWP-name lists |
| `m_0x2f8` | CObArray<CPartIDOb> | v >= 8 | |
| `m_0x30c`..`m_0x314` (9) | u8 (bool) | v >= 8 | |
| `m_0x318`,`m_0x320`,`m_0x32c` | i32 | v >= 8 | |
| `m_0x31c`,`m_0x324`,`m_0x328` | CString | v >= 8 | NWP names ("Riding, Land") |
| `m_0x330` | CListOfListsOfOptions | v >= 9 | embedded: CObArray<CNumList> (choose-N-of lists) |
| `m_0x348`,`m_0x380`,`m_0x44c`,`m_0x44d`,`m_0x44e`,`m_0x464`,`m_0x465`,`m_0x4ac`..`m_0x4b3`,`m_0x508`,`m_0x518`,`m_0x520`,`m_0x521`,`m_0x538` (20) | u8 (bool) | v >= 10 | |
| `m_0x34c`,`m_0x350`,`m_0x368`,`m_0x448`,`m_0x450`,`m_0x454`,`m_0x458`,`m_0x45c`,`m_0x460`,`m_0x468`,`m_0x46c`,`m_0x50c`,`m_0x510`,`m_0x514`,`m_0x51c` (15) | i32 | v >= 10 | |
| `m_0x564` | i32 | v >= 10 and v > 23 | |
| `m_0x470` | CObArray<CPartIDOb> | v >= 10 | weapon ids ("Long bow") |
| `m_0x354`,`m_0x36c` | CStringArray | v >= 10 | weapon names |
| `m_0x384` (again) | CStringArray | v >= 10 | the same member is serialized a second time; the second read wins |
| `m_0x484`,`m_0x498`,`m_0x4b8`,`m_0x4cc`,`m_0x4e0`,`m_0x4f4`,`m_0x524`,`m_0x53c` | CStringArray | v >= 10 | weapon-name lists |
| `m_0x550` | CStringArray | v >= 10 and v > 23 | |
| `m_0x568` | CListOfListsOfOptions | v >= 11 | |
| `m_0x1a0`,`m_0x398`,`m_0x3ac`,`m_0x3c0`,`m_0x3d4` | CStringArray | v >= 12 | armor names / weapon names / weapon groups |
| `m_0x3e8`,`m_0x3fc`,`m_0x410`,`m_0x424`,`m_0x1b4` | CStringArray | v >= 13 | |
| `m_0x438`,`m_0x439`,`m_0x5d4`..`m_0x5d9` (8) | u8 (bool) | v >= 13 | |
| `m_0x43c`,`m_0x440`,`m_0x5e0`,`m_0x5e4`,`m_0x5e8`,`m_0x5ec`,`m_0x5f0`,`m_0x5f4`,`m_0x5f8` | i32 | v >= 13 | |
| `m_0x61c` | CString | v >= 13 | |
| `m_0x5fc`,`m_0x600` | i32 | v >= 13 | |
| `m_0x598` | CStringArray | v >= 13 | equipment names ("Quiver") |
| `m_0x608`..`m_0x614` (4) | i32 | v >= 14 | |
| `m_0x634` | CObArray<CSpellsOb> | v >= 15 | bonus spells ("Detect Magic") |
| `m_0x5da` | u8 (bool) | v >= 16 | |
| `m_0x648` | CObArray<CPartKitLevelOb> | v >= 16 | per-level table |
| `m_0x580` | u8 (bool) | v >= 17 | |
| `m_0x584` | CStringArray | v >= 17 | starting equipment ("Backpack") |
| `m_0x5dc` | u8 (bool) | v >= 18 | |
| `m_0x618` | u8 (bool) | v >= 19 | |
| `m_0x604` | i32 | v >= 21 | |
| `m_0x620` | CStringArray | v >= 22 | |
| `m_0x65c` | CObArray<CClassesAllowedHelperOb> | v >= 23 | allowed class / multiclass combinations for the kit |
| `m_0x21c` | CStringArray | v >= 25 | spheres ("Animal") |
| `m_0x230` | CStringArray | v >= 26 | ("Necromantic") |
| `m_0x5dd`,`m_0x5de` | u8 (bool) | v >= 27 | |
| `m_0x5df` | u8 (bool) | v >= 28 | |
| `m_0x5ac`,`m_0x5c0` | CStringArray | v >= 29 | weapon / magic-item names |

Post-load (no read): if `m_0x30e` and `m_0x44d` are both set, `m_0x30e` is cleared.

## CPartIDOb (schema 94, size 12, Serialize 0x6589ff)

| field | type | notes |
|---|---|---|
| `m_0x4` | CString | part name ("Tracking") |
| `m_0x8` | i32 | part type index (see table above) |

## CPartKitLevelOb (schema 0x80000001 VERSIONABLE, size 44, Serialize 0x66aadd)

| field | type | notes |
|---|---|---|
| version | u32 (local) | 1 in data; read but not tested |
| `m_0x4`..`m_0x28` | 10 x i32 | |

## CNumList (schema 1, size 28, Serialize 0x4b2a5f)

| field | type | notes |
|---|---|---|
| `m_0x4` | i32 | how many to pick (1, 2) |
| `m_0x8` | CStringArray | option names |

## CListOfListsOfOptions (not a tagged object here; embedded member, vtable 0x79ea60, Serialize 0x4b290a)

`CObArray m_0x4` (count + objects); elements are CNumList in all data.

## CClassesAllowedHelperOb (schema 95, size 40, Serialize 0x671f50)

One class or multiclass combination; each class group has its own slot.

| field | type | condition | notes |
|---|---|---|---|
| version | i32 (local) | | 1 in data |
| `m_0x8` | CString | | warrior class ("Fighter","Ranger","Paladin") |
| `m_0xc` | CString | | rogue class ("Thief","Bard") |
| `m_0x10` | CString | | priest class ("Cleric","Druid") |
| `m_0x14` | CString | | wizard class ("Mage","Illusionist",...) |
| `m_0x18` | CString | | psionicist ("Psionicist") |
| `m_0x1c` | i32 | version > 0 | level limit in AllowedU.dat (Aarakocra Cleric 7, Fighter 11; Psionicist 0); 0 in kits |
| `m_0x20` | CString | | always "" |
| `m_0x4` | u8 (bool) | | 1 in AllowedU.dat, 0 in kits |
| `m_0x24` | u8 (bool) | | 0 in data |

## CClassesAllowedOb (schema 95, size 36, Serialize 0x68954b)

| field | type | notes |
|---|---|---|
| version | i32 (local) | 0 in data; not tested |
| `m_0x4` | CString | race name ("Aarakocra") |
| `m_0x1c` | CString | "" in data |
| `m_0x20` | u8 (bool) | 0 in data |
| `m_0x8` | CObArray<CClassesAllowedHelperOb> | allowed classes / combinations for the race |

## CSpellsOb

Reader already in `py/common.py`; used unchanged for Spells.dat and for `CPartKitOb::m_0x634`.

## Unresolved / notes

* `CPart::m_0x748` (CObArray) is empty in every record; its element class is CSpellsOb (copy code
  at 0x65ff46 allocates 0x74 bytes and calls the CSpellsOb constructor 0x4ff04e).
* Most CPart / CPartKitOb fields are named only by offset; semantics are inferred from sample
  values only where noted.
* The CPartKitOb "v < 3" path reads a local i32 and a local CStringArray; the reader returns them
  as `old_id_type` / `old_id_names` (the program rebuilds `m_0x244` from them).
* Parts.dat array 17 is labelled "Count" by the type table; its records are units ("Charge", "Dose").
