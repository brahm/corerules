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


# Corrections 49 and 52. `clean()` strips leading whitespace, and Table 44's ONLY statement
# that a long bow is a Bow is three spaces of indentation in the source. Reading depth before
# cleaning recovers two things at once: a group's `members`, and the fact that a row nested
# TWO deep carries a name that is meaningless on its own — `One-handed`, under `Bastard sword`.
#
# Table 44's convention is NOT the Complete Fighter's. That book writes every sub-row relative
# to its heading (`Bone` under `Dagger`), so `extract_cfh_weapons.py` composes at every level.
# Table 44 writes depth-1 rows absolutely (`Long bow` under `Bow`) and only depth-2 rows
# relatively. Composing everything here would turn `phb:long-bow` into `phb:bow-long-bow`.
# So: compose from the second nesting level down, which is this table's rule and not a
# general one.
INDENT = re.compile(r"^\s*")


def depth_of(cell):
    inner = TAGS.sub("", cell).replace("&nbsp;", " ")
    return len(INDENT.match(inner).group(0).replace("\n", "").replace("\r", ""))


def weapons(wh):
    raw = (wh / "PHB" / "DD01624.HTM").read_text(encoding="cp1252", errors="replace")
    out, seen, by_name = [], set(), {}
    stack = {}                       # depth -> the row name at that depth
    levels = []                      # the distinct depths seen, in order of first appearance
    for tbl in TABLE.findall(raw):
        for r in ROW.findall(tbl):
            cells = CELL.findall(r)
            c = [clean(x) for x in cells]
            if len(c) < 7 or not c[0] or c[0] == "Item":
                continue
            d = depth_of(cells[0])
            if d not in levels:
                levels.append(d)
                levels.sort()
            name = re.sub(r"\s*\d+$", "", c[0]).strip()
            stack[d] = name
            for deeper in [k for k in stack if k > d]:
                del stack[deeper]
            # ancestors above this row, nearest last
            anc = [stack[k] for k in sorted(stack) if k < d]
            if len(anc) >= 2 or (anc and levels.index(d) >= 2):
                name = ", ".join(anc[-1:] + [name])
            s = slug(name)
            if not s or s in seen:
                continue
            seen.add(s)
            rec = {"id": f"phb:{s}", "name": name, "provenance": prov(name, "DD01624.HTM")}
            # A heading row scores nothing; it is a group, and the schema records that
            # rather than pretending the group has a cost and a damage die. Correction 54:
            # `groupKind` alone says a record is a group and which kind — `heading` here is
            # a PRINTING convention, against the Complete Fighter's `tight`/`broad`, which
            # are a purchase. `isGroup` no longer exists on this kind.
            if all(x in DASH for x in c[1:]):
                rec["groupKind"] = "heading"
            else:
                for key, i in (("cost", 1), ("weight", 2), ("size", 3), ("damageType", 4),
                               ("speedFactor", 5), ("damageSmallMedium", 6)):
                    if i < len(c) and c[i] not in DASH:
                        rec[key] = c[i]
                if len(c) > 7 and c[7] not in DASH:
                    rec["damageLarge"] = c[7]
            out.append(rec)
            # Correction 49: a group's members are the rows indented directly beneath it.
            # Two rows share that indentation and are NOT members: AMMUNITION carries a
            # damage and no speed factor, where a launcher carries a speed factor and no
            # damage. A character is not proficient in an arrow.
            if anc:
                parent = by_name.get(anc[-1])
                ammunition = ("speedFactor" not in rec and "damageSmallMedium" in rec)
                if parent is not None and parent.get("groupKind") and not ammunition:
                    parent.setdefault("members", []).append(rec["id"])
            by_name[stack[d]] = rec        # keyed by the row's OWN name, which is what
                                           # `anc` holds — the composed name is for display
    # Table 44 indents `Dagger or dirk` under Crossbow. It is the row that follows the
    # crossbow block alphabetically and it is not a crossbow — a slip in the table or its
    # rendition, and the only one in 79 rows.
    for r in out:
        if r["id"] == "phb:crossbow":
            r["members"] = [m for m in r.get("members", []) if m != "phb:dagger-or-dirk"]
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
            extra = f"  ({sum(1 for x in v if x.get('groupKind'))} of them group headings)"
        print(f"  {k:<22}{len(v):>4}{extra}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
