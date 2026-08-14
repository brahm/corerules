#!/usr/bin/env python3
"""Extract the PHB's nonweapon proficiencies (ticket 13, session 50).

The first extractor aimed at the PHB rather than at a Complete handbook, and the
first at a kind that is NOT an Attachable. It exists because the 182 modelled kits
reference 219 `phb:` ids that nothing defines — 147 of them proficiencies — so the
pack validates and could not load.

Two sources, joined:

  Table 37 (DD01538)  five tables, one per proficiency GROUP, giving each
                      proficiency its slot cost, relevant ability and check
                      modifier — exactly the four fields ticket 14 gave the
                      `nonweaponProficiency` kind, written before any of this
                      was transcribed.

  DD01542-DD01606     one titled page per proficiency, carrying the description.
                      These supply PROVENANCE: a proficiency's anchor is its own
                      page, not the table row that scores it.

The join is by name, and the names disagree between the two sources often enough
that the mismatches are reported rather than silently dropped — the corpus writes
`Fire-building` in the table and `Fire-Building` on the page, `Languages, Ancient`
against `Languages-- Ancient`.

Usage:  extract_proficiencies.py <webhelp-dir> [--json]
"""
import re, sys, json, pathlib

TITLE = re.compile(r"<title>(.*?)</title>", re.I | re.S)
TAGS = re.compile(r"<[^>]+>")
WS = re.compile(r"\s+")
CELL = re.compile(r"<TD[^>]*>(.*?)</TD>", re.I | re.S)
ROW = re.compile(r"<TR[^>]*>(.*?)</TR>", re.I | re.S)
TABLE = re.compile(r"<TABLE[^>]*>(.*?)</TABLE>", re.I | re.S)
ABILITY = {"Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"}
# Two proficiencies score `NA NA`: Blind-fighting and Mountaineering have NO ability check
# at all. The first version of this filter required a named ability and silently dropped
# both — finding 82's class of error, in a new extractor, on its first run.
NOCHECK = {"NA", "N/A", "--", "-"}
# The corpus misspells one of its own proficiencies in the scoring table: `Astology` in one
# group and `Astrology` in another, with identical scores. Normalised here so the two rows
# join, and recorded because nothing else would notice.
TYPO = {"Astology": "Astrology"}

# Table 37 prints one table per group and does not repeat the group name inside it;
# the heading sits in the prose above. Order is the book's.
GROUPS = ["general", "priest", "rogue", "warrior", "wizard"]


def clean(s):
    return WS.sub(" ", TAGS.sub(" ", s).replace("&nbsp;", " ").replace("&amp;", "&")).strip()


def slug(name):
    """`Languages, Ancient` -> `languages-ancient`. Punctuation is dropped, not mapped."""
    s = name.lower().replace("/", "-").replace(",", "")
    s = re.sub(r"[^a-z0-9]+", "-", s).strip("-")
    return s


def table37(raw):
    """(name, slots, ability, modifier, group) from the five group tables."""
    out = []
    for gi, tbl in enumerate(TABLE.findall(raw)):
        group = GROUPS[gi] if gi < len(GROUPS) else f"group{gi}"
        for r in ROW.findall(tbl):
            cells = [clean(c) for c in CELL.findall(r)]
            if len(cells) < 4:
                continue
            name, slots, ability, mod = cells[0], cells[1], cells[2], cells[3]
            name = TYPO.get(name, name)
            if not name or not (ability in ABILITY or ability in NOCHECK):
                continue          # a header row, or the two-line column captions
            try:
                slots = int(slots)
            except ValueError:
                continue
            out.append((name, slots, ability, mod.replace("–", "-"), group))
    return out


def pages(wh):
    """slug -> (filename, title) for every `X-- Nonweapon Proficiency` page."""
    found = {}
    for f in sorted((wh / "PHB").glob("*.HTM")):
        raw = f.read_text(encoding="cp1252", errors="replace")
        m = TITLE.search(raw)
        if not m:
            continue
        t = clean(m.group(1)).partition("(")[0].strip()
        if not t.endswith("Nonweapon Proficiency"):
            continue
        name = t[:-len("-- Nonweapon Proficiency")].strip().rstrip("-").strip()
        found[slug(name)] = (f.name, name)
    return found


def main():
    wh = pathlib.Path(sys.argv[1])
    raw = (wh / "PHB" / "DD01538.HTM").read_text(encoding="cp1252", errors="replace")
    rows = table37(raw)
    desc = pages(wh)

    # A proficiency can be scored in more than one group — it is a crossover, and the
    # groups are what a class may buy from, not a property of the proficiency. Merged into
    # one record carrying every group it appears in.
    merged = {}
    for name, slots, ability, mod, group in rows:
        k = slug(name)
        if k in merged:
            merged[k][4].append(group)
        else:
            merged[k] = [name, slots, ability, mod, [group]]

    records, orphans = [], []
    for name, slots, ability, mod, group in merged.values():
        s = slug(name)
        page = desc.get(s)
        if page is None:
            orphans.append(name)
        records.append({
            "id": f"phb:{s}",
            "name": name,
            "provenance": {
                "section": ["Player's Handbook", name],
                # A proficiency's anchor is its own DESCRIPTION page where one exists,
                # and Table 37 otherwise. The scores come from the table either way, so
                # the anchor names where a reviewer should read, not where the numbers are.
                "anchor": {"rendition": "webhelp", "file": page[0] if page else "DD01538.HTM"},
            },
            "group": [f"phb:{g}" for g in group],
            "slotCost": slots,
        })
        # `abilityCheck` is a scalar and `modifier` an integer — the schema said so before any
        # of this existed, and the first run emitted a bare id and a string for them. Both are
        # simply ABSENT for the two proficiencies Table 37 scores `NA`, which is the honest
        # shape: no check at all, rather than a check of zero.
        if ability not in NOCHECK:
            records[-1]["abilityCheck"] = {"ability": f"phb:{ability.lower()}"}
            records[-1]["modifier"] = int(mod.replace("+", "") or 0)

    if "--json" in sys.argv:
        json.dump({"nonweaponProficiencies": records}, sys.stdout, indent=2, ensure_ascii=False)
        return 0

    print(f"Table 37: {len(records)} proficiencies, {len(desc)} description pages")
    by, cross, nocheck = {}, 0, 0
    for r in records:
        gs = r["group"] if isinstance(r["group"], list) else [r["group"]]
        cross += len(gs) > 1
        nocheck += r["abilityCheck"] is None
        for g in gs:
            by[g] = by.get(g, 0) + 1
    print("  by group: " + ", ".join(f"{k} {v}" for k, v in by.items()))
    print(f"  in more than one group: {cross}    with no ability check: {nocheck}")
    if orphans:
        print(f"  scored in Table 37 with no description page: {len(orphans)}")
        print("    " + ", ".join(orphans))
    extra = set(desc) - {r["id"].split(":", 1)[1] for r in records}
    if extra:
        print(f"  described but not scored in Table 37: {len(extra)}")
        print("    " + ", ".join(sorted(extra)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
