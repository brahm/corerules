#!/usr/bin/env python3
"""Extract the PHB's remaining referenced kinds (ticket 13, session 51).

Session 50 took the proficiencies and 24% of the pack's references began to
resolve. This takes what else the kits point at and the schema has a home for:

  weaponProficiencies   Table 44's Weapons List (DD01624) — 80 rows with cost,
                        weight, size, type, speed factor and damage. The kind was
                        declared by ticket 14 with NO fields at all, so its shape
                        is set here from the columns the book prints, which is the
                        same move that worked for nonweapon proficiencies.
  races                 the seven player races, a closed list.
  abilities             the six ability scores, a closed list.
  thievingSkills        the eight skills, read off Table 26's own row keys so the
                        ids match the table that scores them.

Four things the kits reference have NO kind to live in — alignments, spheres,
schools of magic and armour — and this tool does not invent one. See ticket 13
finding 99.

Usage:  extract_phb.py <webhelp-dir> [--json]
"""
import re, sys, json, pathlib

TAGS = re.compile(r"<[^>]+>")
WS = re.compile(r"\s+")
CELL = re.compile(r"<TD[^>]*>(.*?)</TD>", re.I | re.S)
ROW = re.compile(r"<TR[^>]*>(.*?)</TR>", re.I | re.S)
TABLE = re.compile(r"<TABLE[^>]*>(.*?)</TABLE>", re.I | re.S)

# Table 44 prints group headings as rows whose every numeric cell is `--`: `Bow`,
# `Crossbow`, `Sword` head a block of variants. They are kept, because the kits
# reference the GROUP — `phb:bow`, `phb:sword` — far more often than a variant.
DASH = {"--", "-", "", "*"}

RACES = ["Dwarf", "Elf", "Gnome", "Half-Elf", "Halfling", "Human"]
ABILITIES = ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"]


def clean(s):
    return WS.sub(" ", TAGS.sub(" ", s).replace("&nbsp;", " ").replace("&amp;", "&")
                  .replace("&#183;", " ")).strip()


def slug(name):
    s = re.sub(r"\s*\d+$", "", name.lower())        # `Arquebus 3` carries a footnote marker
    s = s.replace("/", "-").replace(",", "")
    return re.sub(r"[^a-z0-9]+", "-", s).strip("-")


def prov(name, file):
    return {"section": ["Player's Handbook", name],
            "anchor": {"rendition": "webhelp", "file": file}}


def weapons(wh):
    raw = (wh / "PHB" / "DD01624.HTM").read_text(encoding="cp1252", errors="replace")
    out, seen = [], set()
    for tbl in TABLE.findall(raw):
        for r in ROW.findall(tbl):
            c = [clean(x) for x in CELL.findall(r)]
            if len(c) < 7 or not c[0] or c[0] == "Item":
                continue
            name = re.sub(r"\s*\d+$", "", c[0]).strip()
            s = slug(name)
            if not s or s in seen:
                continue
            seen.add(s)
            rec = {"id": f"phb:{s}", "name": name, "provenance": prov(name, "DD01624.HTM")}
            # A heading row scores nothing; it is a group, and the schema records that
            # rather than pretending the group has a cost and a damage die.
            if all(x in DASH for x in c[1:]):
                rec["isGroup"] = True
            else:
                for key, i in (("cost", 1), ("weight", 2), ("size", 3), ("damageType", 4),
                               ("speedFactor", 5), ("damageSmallMedium", 6)):
                    if i < len(c) and c[i] not in DASH:
                        rec[key] = c[i]
                if len(c) > 7 and c[7] not in DASH:
                    rec["damageLarge"] = c[7]
            out.append(rec)
    return out


def thieving_skills(wh):
    raw = (wh / "PHB" / "DD01501.HTM").read_text(encoding="cp1252", errors="replace")
    out = []
    for tbl in TABLE.findall(raw):
        for r in ROW.findall(tbl):
            c = [clean(x) for x in CELL.findall(r)]
            if len(c) < 2 or not c[0] or not c[1].endswith("%"):
                continue
            out.append({"id": f"phb:{slug(c[0])}", "name": c[0],
                        "provenance": prov(c[0], "DD01501.HTM")})
    return out


def main():
    wh = pathlib.Path(sys.argv[1])
    doc = {
        "weaponProficiencies": weapons(wh),
        "races": [{"id": f"phb:{slug(r)}", "name": r, "provenance": prov(r, "DD01438.HTM")}
                  for r in RACES],
        "abilities": [{"id": f"phb:{slug(a)}", "name": a, "provenance": prov(a, "DD01430.HTM")}
                      for a in ABILITIES],
        "thievingSkills": thieving_skills(wh),
    }
    if "--json" in sys.argv:
        json.dump(doc, sys.stdout, indent=2, ensure_ascii=False)
        return 0
    for k, v in doc.items():
        extra = ""
        if k == "weaponProficiencies":
            extra = f"  ({sum(1 for x in v if x.get('isGroup'))} of them group headings)"
        print(f"  {k:<22}{len(v):>4}{extra}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
