<p align="center">
  <img src="icon.png" width="128" height="128" alt="Core Rules Community Add-ons Collection icon">
</p>

# Core Rules Community Add-ons Collection

The add-ons that the AD&D Core Rules community made for **AD&D Core Rules 2.0** and its **Expansion**, gathered into one
package that [Core Rules](../README.md), the replica of the original
program for Linux, Windows 11 and macOS, imports in one go. 672 add-on packages became 36,525
items: new races and classes, kits, thousands of spells and monsters,
equipment, encounters, the characters of the published modules and the
AD&D Core Rules User Database. The 668 pictures that came with them come
along, and so do the texts their authors wrote.

> The collection holds what the community wrote and shared as add-ons. It
> leaves out the monthly goodies that Evermore Entertainment, the company
> that made Core Rules 2.0, published in 1998–2000, and the copies of them
> in the User Database. The rules data still comes from a Core Rules
> installation you already own.

## What's inside

Every item is in the Expansion's own format, so it opens in the app and in
the original program alike.

| Kind | Items |
| --- | ---: |
| Wizard spells | 7,479 |
| Priest spells | 3,514 |
| Monsters | 5,307 |
| Monster types | 1,871 |
| Magical items | 2,367 |
| Weapons | 2,112 |
| Equipment | 1,578 |
| Armor | 752 |
| Races | 512 |
| Classes | 558 |
| Kits | 477 |
| Racial and class abilities | 5,449 |
| Nonweapon proficiencies | 1,804 |
| Languages | 672 |
| Encounters and encounter templates | 584 |
| Traps | 173 |
| Psionic powers | 160 |
| Traits and disadvantages | 253 |
| Spheres and schools of magic | 362 |
| Treasure bundles, gems, art and currencies | 336 |
| Characters from the modules | 48 |
| Other parts | 157 |

Once imported, the add-ons sit in the user databases next to your own
records, and the module characters join the character list. Monster types,
races and characters show the picture their author drew or scanned, in the
app and in the original program.

<p>
  <img src="screenshots/picture.png" width="49%" alt="The Achaierai, a monster type with its picture">
  <img src="screenshots/portrait.png" width="49%" alt="Gerard Renier, a character with his portrait">
</p>

<p>
  <img src="screenshots/class.png" width="49%" alt="The Anti-Paladin, one of the add-on classes">
  <img src="screenshots/monster.png" width="49%" alt="A psionic beholder among the add-on monsters">
</p>

![A character from the modules, the Avatar of Bahgtru](screenshots/character.png)

## Importing

You need [Core Rules](https://github.com/brahm/corerules/releases) 1.2.0
or newer, on Linux, Windows 11 or macOS, with your Expansion data in it;
if it is not there yet, open Core Rules and use **Extract Data from CD…**.

1. Download
   [`CoreRules-Community-Addons.zip`](https://github.com/brahm/corerules/releases/download/addons/CoreRules-Community-Addons.zip)
   (the link always gives the newest build; the
   [addons release](https://github.com/brahm/corerules/releases/tag/addons)
   also keeps each version) and put it in a folder of its own. There is no
   need to extract it.
2. In Core Rules, choose **Import Add-ons…**: in the main menu on Linux,
   the More menu on Windows and the File menu on the Mac.
3. Pick the folder that holds the zip. The app reads the folder, its
   subfolders and the ZIP archives in it (also ZIPs inside ZIPs), and tells
   each `.xpx` and `.xpt` package apart by what it holds.
4. Check the list: what comes in, by category, what was found twice and
   what cannot be imported, with why. Nothing is written before you
   confirm.
5. Press **Import**. The whole collection takes about a minute.

Import first copies `UserDbas` and `Chars` to a backup beside the data
folder (`data-before-addons-TIME`). It imports the packages from the
oldest file to the newest, so the newer one wins on a name clash, leaves
out the characters already in the list, so a second run does not duplicate
them, and copies the pictures to the places `pictures.tsv` gives (in
`Bitmaps/Monsters`, `Bitmaps/Portrait` and `UserBMP`), never over a file
that is already there. A log of everything it did is written beside the
data folder (`data-addons-TIME.log`). The same command imports any other
folder of add-ons, loose or zipped, as the community shared them.

The zip holds one folder, `CoreRules-Community-Addons-VERSION`, with:

| File | Contents |
| --- | --- |
| `Comunidade.xpx` | The consolidated add-ons, in the Expansion's export format; the original program's Import reads it too |
| `pictures/` | The 668 pictures (8-bit BMP), in their add-on folders |
| `pictures.tsv` | Where each picture goes in the data folder: picture, a tab, place |
| `texts/` | The authors' 309 texts, as plain text |
| `addons.tsv` | What each add-on folder brought into the collection |
| `README.txt`, `LEIA-ME.txt` | How to import it, in English and Portuguese |

The [Portuguese guide](GUIA.md) also shows how to undo an import.

## How it was built

**Testing every add-on.** The community's archive held 686 packages: 492
Expansion `.xpx` files and 194 Core Rules 2.0 `.xpt` files, including the
AD&D Core Rules User Database. 14 of them were Evermore's monthly goodies,
which the collection leaves out. Each one was read and written back byte for
byte by this project's reader, and the doubtful ones were imported into the
original Expansion program under Wine.

**Converting and repairing.** 90 packages, made with the Expansion after
patches 3 to 5, were refused by the CD's program. Their classes lost the
spell point objects the CD's version does not know, and the User
Database's kits were rewritten in the older kit format. A spell package
damaged by a text-mode transfer had its line endings and lengths restored,
and 103 kits were recovered from a truncated archive. One package held only
a web installer and could not be used. Map Maker II maps and Campaign
Mapper drawings were left out, since the app does not handle those
programs.

**Consolidating.** The packages were imported into a clean Expansion
installation from oldest to newest, the User Database last, and exported
as one file, `Comunidade.xpx`. Where a name appears in more than one
add-on, the newest version wins, as in the original Import; the
[report](relatorio.md) lists every conflict and every package's status.

**Checking in the app.** After an import, every add-on record was opened,
edited and saved with the app's own editors and read back, and none
failed.

**Packaging.** The consolidated file is kept compressed with xz, and
`tools/addons_zip.py VERSION DIR` builds the zip from it and from the
extras below. The `Addons package` workflow (`.github/workflows/addons.yml`)
builds the zip and publishes it on the addons release on every change to
`main`. A Linux installer (an AppImage) carried the add-ons until the app's
1.1.0; Import Add-ons… replaced it in 1.2.0.

**Pictures and texts.** The 668 pictures (8-bit BMP files) and 309 texts
that came with the add-ons, again without Evermore's, are kept in
`Comunidade-extras.xz`. Each picture goes to the place the
records name: a record stores the Windows path the picture had on its
author's computer (`C:\Program Files\AD&D Core Rules 2.0\Bitmaps\Monsters\
Achaierai.BMP`, `C:\AD&D Pics\troll.bmp`), and the app maps that path into
the data folder the same way (`cr-store`'s `pictures` module). The texts,
readme files in plain text, HTML, RTF, Word and PDF, were converted to plain
text.

## Files

| File | Contents |
| --- | --- |
| `Comunidade.xpx.xz` | The consolidated add-ons, as they go into the zip |
| `Comunidade-extras.xz` | The add-ons' pictures and texts, and where each picture goes |
| `tools/` | The scripts that chose each picture's place, built `Comunidade-extras.xz` and build the add-ons' zip |
| [`relatorio.md`](relatorio.md) | The report, in Portuguese: what was converted, repaired or left out |
| [`compatibilidade.csv`](compatibilidade.csv) | One line per add-on package, with its status and item count |
| [`conflitos.csv`](conflitos.csv) | The names found in more than one package, and which version stayed |
| [`GUIA.md`](GUIA.md) | The import guide, in Portuguese |

The add-ons belong to their authors in the Core Rules community.
