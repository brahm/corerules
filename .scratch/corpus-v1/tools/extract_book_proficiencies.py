#!/usr/bin/env python3
"""Proficiencies the Complete handbooks introduce (ticket 13, session 53).

After the PHB was transcribed, 37 proficiency references still failed to resolve —
Intimidation, Camouflage, Trail Signs, Falconry — because each Complete handbook
adds its own, and the kits that require them had been given `phb:` ids by a
transcriber guessing at which book owns a name (finding 99).

Three books print a COMPILED table in exactly PHB Table 37's four columns, with the
ability abbreviated: the Bard's, the Paladin's and the Ranger's. Those parse with the
same reader. The compiled tables restate the PHB's own proficiencies alongside the new
ones, so a row already defined by the PHB is skipped rather than duplicated under a
second id — which is the whole point: one thing, one id, in the book that introduced it.

Usage:  extract_book_proficiencies.py <webhelp-dir> <phb-proficiencies.json> [--json]
"""
import re, sys, json, pathlib

TAGS = re.compile(r"<[^>]+>")
WS = re.compile(r"\s+")
CELL = re.compile(r"<TD[^>]*>(.*?)</TD>", re.I | re.S)
ROW = re.compile(r"<TR[^>]*>(.*?)</TR>", re.I | re.S)
TABLE = re.compile(r"<TABLE[^>]*>(.*?)</TABLE>", re.I | re.S)

# The compiled tables abbreviate. `NA` means no ability check at all, as in the PHB's
# Blind-fighting and Mountaineering (finding 97).
ABBR = {"Str": "strength", "Dex": "dexterity", "Con": "constitution",
        "Int": "intelligence", "Wis": "wisdom", "Cha": "charisma"}
NOCHECK = {"NA", "N/A", "--", "-", ""}

# book -> (pack id, page carrying the compiled table)
COMPILED = {
    "CBH":  ("cbh",  "DD04974.HTM"),
    "CPAH": ("cpah", "DD05417.HTM"),
    "CRH":  ("crh",  "DD05730.HTM"),
}


def clean(s):
    return WS.sub(" ", TAGS.sub(" ", s).replace("&nbsp;", " ").replace("&amp;", "&")).strip()


def slug(name):
    s = name.lower().replace("/", "-").replace(",", "")
    return re.sub(r"[^a-z0-9]+", "-", s).strip("-")


def read(raw):
    for tbl in TABLE.findall(raw):
        for r in ROW.findall(tbl):
            c = [clean(x) for x in CELL.findall(r)]
            if len(c) < 4 or not c[0]:
                continue
            name, slots, ability, mod = c[0], c[1], c[2], c[3]
            if ability not in ABBR and ability not in NOCHECK:
                continue                      # a caption row, or a second column block
            try:
                slots = int(slots)
            except ValueError:
                continue
            yield name, slots, ability, mod.replace("–", "-")


def main():
    wh = pathlib.Path(sys.argv[1])
    phb = {r["id"].split(":", 1)[1]
           for r in json.load(open(sys.argv[2]))["nonweaponProficiencies"]}

    out, restated = {}, 0
    for book, (pack, page) in COMPILED.items():
        f = wh / book / page
        if not f.exists():
            continue
        for name, slots, ability, mod in read(f.read_text(encoding="cp1252", errors="replace")):
            s = slug(name)
            if s in phb:
                restated += 1
                continue                      # the PHB owns it; do not mint a second id
            key = s
            if key in out:
                continue                      # two books adding the same proficiency: first wins
            rec = {
                "id": f"{pack}:{s}",
                "name": name,
                "provenance": {"section": [book, name],
                               "anchor": {"rendition": "webhelp", "file": page}},
                "group": [f"{pack}:new"],
                "slotCost": slots,
            }
            if ability in ABBR:
                rec["abilityCheck"] = {"ability": f"phb:{ABBR[ability]}"}
                try:
                    rec["modifier"] = int(mod.replace("+", "") or 0)
                except ValueError:
                    pass
            out[key] = rec

    records = list(out.values())
    if "--json" in sys.argv:
        json.dump({"nonweaponProficiencies": records}, sys.stdout, indent=2, ensure_ascii=False)
        return 0
    by = {}
    for r in records:
        b = r["id"].split(":")[0]
        by[b] = by.get(b, 0) + 1
    print(f"{len(records)} proficiencies the Complete handbooks add")
    print("  " + ", ".join(f"{k} {v}" for k, v in sorted(by.items())))
    print(f"  PHB proficiencies restated in those tables and skipped: {restated}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
