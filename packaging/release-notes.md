**Beta release.** Every feature works against real installations and was
checked against the original programs, but the app has had little use
outside those checks. Keep a copy of your `Chars` and `UserDbas` folders,
and report anything that differs from the original.

What's new in 0.9.0, which brings the rest of the original's screens:

- **Update Character**: gender, alignment, kit, background, hit points,
  subrace, racial and class abilities, schools and spheres, weapon and
  nonweapon proficiencies, weapon groups, combat options, traits and
  disadvantages, rogue skills, psionics and wild talents, all charged and
  checked as the original does; the Levels dialog with hit point and PSP
  arrows; buying and selling equipment with the spending money
- **Enter Character**, for a character that already exists on paper:
  typed-in scores, any race and classes, levels set in the Levels dialog,
  and later pages with no limits checked
- **Kits** followed everywhere: the Name page's filter and warnings,
  nonweapon and weapon proficiencies (with Core Rules specialization and
  Player's Option weapon groups), rogue skills, schools, spheres and
  spells, experience, hit points, starting money and equipment, in New
  Character and in Update
- **New Character**: a psionicist's 1st-level PSPs, a subrace's package at
  its price, a random name, the Background step with Select Randomly, the
  Mongrelman's point to any ability, the chance to learn each spell with
  the original's questions, Test for Wild Talents and the Table 83 roll,
  and the Expansion's "Core Rules Vs Player's Option" message, which
  Preferences turns back on
- **Editors**: the ten remaining part types in Other Parts, the encounter
  template editor, a class's free spells, NWPs and abilities by level with
  Auto Fill, a monster's spells, inventory, attacks and Suggest XP Value,
  a treasure table entry's details, item icons on General Info, Schools…
  on any spell, and the parts lists' Magic Only filter
- **DM Toolkit**: Group Encounter from a party, hit points by hand,
  Re-roll and New Monster in an encounter, View NPC and Print NPC, View
  and Print on templates, 2-20 tables and treasure tables, copies of saved
  encounters and treasure bundles, Next Image and Full Screen on monster
  pictures, the 2-20 table's template chosen by frequency as the original
  does, and New NPC Group's Standard and Allowed races, kits and alignments
- **Help** on every screen (Help button and F1), right-click help and
  summaries, the help window's printing, the Online Manual, the Tome of
  Help and About
- **Character sheet** options (sections, order, page breaks and border),
  Markdown export, and Convert to NPC and back
- **Name Conflict** asks for another name when an imported part's name is
  taken, and the app offers to rebuild a damaged character list

The README lists the few things that work differently from the original
on purpose.

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
| Any distribution with Flatpak | `CoreRules-*.flatpak` | `flatpak install --user CoreRules-*.flatpak` |
| Any distribution (glibc 2.43+) | `CoreRules-*-x86_64.AppImage` | `chmod +x CoreRules-*.AppImage` and run it |

The Flatpak needs the GNOME 51 runtime from Flathub, which `flatpak install`
offers to fetch. The AppImage bundles the GTK, libadwaita and other
libraries of GNOME 51 and runs on Fedora 44, Ubuntu 26.04 and newer. `SHA256SUMS` lists the checksums of every file.
