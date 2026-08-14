#!/usr/bin/env python3
"""Extract the PHB's spells (ticket 16 decision 1, taken in ticket 13 session 61).

469 pages, and the most regular thing in the corpus: every one carries Range,
Components, Duration, Casting Time, Area of Effect and Saving Throw, every priest
spell carries Sphere, and the level and caster class are in the page title.

Ticket 16 settled what the record is FOR: a referenceable label plus the six
printed fields AS THE BOOK PRINTS THEM. `Duration: 1 rd./level` stays a string.
Parsing it would mean growing the expression language across 469 records to serve
a question the Engine has not been asked, and A3 makes an unparsed string honest.

Two hazards the markup sets, both of which silently truncate:

  CONTINUATION   the stat block is a two-column <TABLE> and a long value wraps
                 into the NEXT ROW's cell with no label — `Area of Effect: 1
                 creature or` / `   object per 2 rds.`. A cell with no `Label:`
                 belongs to the last label seen IN ITS OWN COLUMN.
  CASE           the corpus writes both `Casting Time:` and `Casting time:`.

Usage:  extract_spells.py <webhelp-dir> [--json]
"""
import re, sys, json, pathlib, collections

TITLE = re.compile(r"<title>(.*?)</title>", re.I | re.S)
# The school (wizard) or the sphere restated (priest) is printed in red. FOUR markup
# variants across 303 pages, and the first regex written for the commonest one missed
# twelve: `<B>(School)</B>`, `<B> <P></P> (School)`, `<B>(School) <P></P> </B>`, and one
# page on a different RoboHELP template using `COLOR="RED"` instead of `#ff0000`.
RED = re.compile(r'COLOR="(?:#ff0000|RED)"[^>]*>(?:(?!COLOR=)[^(]){0,400}?\(([^()]{3,80})\)',
                 re.I | re.S)
REVERSIBLE = re.compile(r"<B>\s*Reversible\s*</B>", re.I)
TABLE = re.compile(r"<TABLE[^>]*>(.*?)</TABLE>", re.I | re.S)
ROW = re.compile(r"<TR[^>]*>(.*?)</TR>", re.I | re.S)
CELL = re.compile(r"<TD[^>]*>(.*?)</TD>", re.I | re.S)
LABEL = re.compile(r"^([A-Z][A-Za-z ]{2,20}):\s*(.*)$", re.S)
TAGS = re.compile(r"<[^>]+>")

# `Casting time` and `Casting Time` are the same field. Keys are lowercased.
FIELD = {
    "range": "range", "components": "components", "duration": "duration",
    "casting time": "castingTime", "area of effect": "areaOfEffect",
    "saving throw": "savingThrow", "sphere": "sphere",
    # Five spells write the label singular. Same field.
    "component": "components",
}

# The spell pages name the schools SHORT and the chapter names them long: `Evocation`
# for Invocation/Evocation, `Conjuration` for Conjuration/Summoning, `Phantasm` for
# Illusion. The nine are fixed by DD01473 and this maps the corpus's own abbreviations
# onto them — a reading, recorded on every record that uses it.
SCHOOL = {
    "abjuration": "phb:abjuration",
    "alteration": "phb:alteration",
    "conjuration": "phb:conjuration-summoning",
    "summoning": "phb:conjuration-summoning",
    "conjuration/summoning": "phb:conjuration-summoning",
    "enchantment": "phb:enchantment-charm",
    "charm": "phb:enchantment-charm",
    "enchantment/charm": "phb:enchantment-charm",
    "illusion": "phb:illusion",
    "phantasm": "phb:illusion",
    "illusion/phantasm": "phb:illusion",
    "invocation": "phb:invocation-evocation",
    "evocation": "phb:invocation-evocation",
    "invocation/evocation": "phb:invocation-evocation",
    "necromancy": "phb:necromancy",
}

# The sixteen spheres are fixed by the priest chapter and the spell pages qualify them:
# `Elemental (Fire)`, `Elemental (Water, Air)`. The four elemental sub-spheres have no
# records — nothing references them and minting four to satisfy a parenthetical would be
# invention — so the parenthetical survives in `sphereAsPrinted` and the id is the sphere.
SPHERE_TYPO = {"necromatic": "necromantic"}      # one page, the corpus's own slip


def spheres(printed):
    """Ids for a printed sphere string. `Elemental (Earth), Divination` is two spheres."""
    out = []
    for part in re.split(r",(?![^(]*\))", printed):
        k = part.strip().partition("(")[0].strip().lower()
        if not k:
            continue
        k = SPHERE_TYPO.get(k, k)
        out.append("phb:" + slug(k))
    seen, uniq = set(), []
    for i in out:
        if i not in seen:
            seen.add(i)
            uniq.append(i)
    return uniq


def schools(printed, level):
    """Ids for a printed school string, or None where the corpus says something the
    nine cannot hold. `Divination` is split BY THE BOOK'S OWN RULE: DD01473 says lesser
    divination is every divination spell of 4th level or less and greater divination is
    5th or higher, so the spell's level decides it and nothing is guessed."""
    out = []
    for part in [p.strip() for p in printed.split(",")]:
        k = part.lower()
        if k.startswith("divination"):
            out.append("phb:greater-divination" if level >= 5 else "phb:divination")
            k = k.partition("/")[2].strip()      # `Divination/Illusion` is TWO schools
            if not k:
                continue
        if k in SCHOOL:
            out.append(SCHOOL[k])
        elif "/" in k and all(x.strip() in SCHOOL for x in k.split("/")):
            out.extend(SCHOOL[x.strip()] for x in k.split("/"))
        else:
            return None                          # `All Schools`, and anything unforeseen
    seen, uniq = set(), []
    for i in out:
        if i not in seen:
            seen.add(i)
            uniq.append(i)
    return uniq or None


def clean(s):
    return re.sub(r"\s+", " ", TAGS.sub(" ", s).replace("&nbsp;", " ")
                  .replace("&amp;", "&").replace("&#183;", " ")).strip()


def slug(name):
    s = name.lower().replace("/", "-").replace("'", "")
    return re.sub(r"[^a-z0-9]+", "-", s).strip("-")


def statblock(raw):
    """Label -> value, joining continuation cells to the label above them."""
    out, last = {}, {}
    for tbl in TABLE.findall(raw)[:1]:          # the stat block is the first table
        for r in ROW.findall(tbl):
            for col, cell in enumerate(CELL.findall(r)):
                text = clean(cell)
                if not text:
                    continue
                m = LABEL.match(text)
                if m and m.group(1).lower() in FIELD:
                    key = FIELD[m.group(1).lower()]
                    out[key] = m.group(2).strip()
                    last[col] = key
                elif col in last:
                    # A wrapped value. The book indents it; the join is a space.
                    out[last[col]] = (out[last[col]] + " " + text).strip()
    return out


def main():
    wh = pathlib.Path(sys.argv[1]) / "PHB"
    records, missing = [], collections.Counter()
    unmapped = collections.Counter()
    seen = {}
    for f in sorted(wh.glob("*.HTM")):
        raw = f.read_text(encoding="cp1252", errors="replace")
        m = TITLE.search(raw)
        if not m:
            continue
        title = clean(m.group(1))
        t = re.match(r"(.+?)--\s*(\d+)(?:st|nd|rd|th) Level (Wizard|Priest) Spell", title)
        if not t:
            continue
        name, level, cls = t.group(1).strip(), int(t.group(2)), t.group(3)

        # 30 of 440 spell names are BOTH a wizard and a priest spell — Know Alignment,
        # Detect Magic, Gate — and none collides inside a class. So the id is qualified
        # by the caster ALWAYS, not only on collision: ticket 07 wants an id that
        # survives re-extraction, and a scheme that renames `phb:bless` the day a wizard
        # Bless is transcribed is the instability it warned about. Both halves come from
        # the page title, so the id is derivable from the page and nothing else.
        key = f"{slug(name)}-{cls.lower()}"
        assert key not in seen, key
        seen[key] = True

        rec = collections.OrderedDict([
            ("id", f"phb:{key}"),
            ("name", name),
            ("provenance", {"section": ["Player's Handbook", title.split("(")[0].strip()],
                            "anchor": {"rendition": "webhelp", "file": f.name}}),
            ("level", level),
            ("casterClass", f"phb:{cls.lower()}"),
        ])
        block = statblock(raw)
        red = RED.search(raw)
        if cls == "Wizard" and red:
            printed = clean(red.group(1))
            rec["schoolAsPrinted"] = printed
            ids = schools(printed, level)
            if ids:
                rec["schools"] = ids
            else:
                unmapped[printed] += 1
        if "sphere" in block:
            printed = block.pop("sphere")
            rec["sphereAsPrinted"] = printed
            rec["spheres"] = spheres(printed)
        if REVERSIBLE.search(raw):
            rec["reversible"] = True
        for k in ("range", "components", "duration", "castingTime",
                  "areaOfEffect", "savingThrow"):
            if k in block:
                rec[k] = block[k]
            else:
                missing[k] += 1
        records.append(rec)

    if "--json" in sys.argv:
        json.dump({"spells": records}, sys.stdout, indent=2, ensure_ascii=False)
        return 0

    by = collections.Counter(r["casterClass"] for r in records)
    print(f"{len(records)} spells: " + ", ".join(f"{k} {v}" for k, v in by.most_common()))
    print(f"  with a school: {sum(1 for r in records if 'schools' in r)}"
          f"   multi-school: {sum(1 for r in records if len(r.get('schools', [])) > 1)}"
          f"   with a sphere: {sum(1 for r in records if 'spheres' in r)}"
          f"   multi-sphere: {sum(1 for r in records if len(r.get('spheres', [])) > 1)}"
          f"   reversible: {sum(1 for r in records if r.get('reversible'))}")
    if missing:
        print("  MISSING a printed field: " + ", ".join(f"{k} {v}" for k, v in missing.most_common()))
    if unmapped:
        print("  printed school the nine cannot hold: "
              + ", ".join(f"{k!r} {v}" for k, v in unmapped.most_common()))
    lens = sorted(records, key=lambda r: -len(r.get("areaOfEffect", "")))
    print(f"  longest areaOfEffect: {lens[0]['name']} — {lens[0].get('areaOfEffect','')[:70]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
