#!/usr/bin/env python3
"""The Complete Fighter's new weapons and weapon groups (ticket 13, session 63).

Ticket 16's item 5 said five books had unread proficiencies. Four of them — the Elves',
the Gnomes and Halflings', the Druid's and the Wizard's — turn out to have no proficiency
chapter at all (finding 132). The Fighter's has one, and what is in it is not a nonweapon
proficiency: it is a WEAPON-GROUP system and a list of new weapons.

  DD05367  New Weapons List, printed in Table 44's own eight columns, so it reads with
           the same rules `extract_phb.py` set. Two hazards: footnote markers are a
           <FONT SIZE="1"> span glued to the name, and sub-rows are marked by TWO LEADING
           SPACES — `Dagger` / `  Bone` / `  Stone` — which must qualify the id, because
           `Bone` appears under Dagger AND under Knife.

  DD05200  Tight Groups, 2 slots each.   DD05201  Broad Groups, 3 slots each.
  DD05202  the weapons that belong to no group at all.
           Group names are <B>Name:</B> and members follow as comma-separated fragments.
           A member marked with an asterisk is a weapon THIS book introduces; everything
           else is Table 44's, and the resolver reports anything it cannot place rather
           than inventing an id.

Usage:  extract_cfh_weapons.py <webhelp-dir> <pack-dir> [--json]
"""
import re, sys, json, pathlib, collections

TABLE = re.compile(r"<TABLE[^>]*>(.*?)</TABLE>", re.I | re.S)
ROW = re.compile(r"<TR[^>]*>(.*?)</TR>", re.I | re.S)
CELL = re.compile(r"<TD[^>]*>(.*?)</TD>", re.I | re.S)
FOOTNOTE = re.compile(r'<FONT[^>]*SIZE="1"[^>]*>.*?</FONT>', re.I | re.S)
BOLD = re.compile(r"<B>(.*?)</B>", re.I | re.S)
TAGS = re.compile(r"<[^>]+>")
DASH = {"--", "-", "", "*", "—"}

# The Fighter's spells out what Table 44 abbreviates, and vice versa. Only the differences
# are listed; everything else resolves by slug. Nothing here is a guess — each pair is the
# same weapon under the two books' spellings.
# A member written `A/B` can be ONE weapon under two names or TWO weapons, and only
# reading tells which — `Dagger/Dirk` is Table 44's single entry, `Short sword/Drusus`
# is the Player's Handbook weapon beside this book's variant of it. Both resolve to a
# list, so the pair is never silently halved.
ALIAS = {
    "hand/throwing axe": ["phb:hand-or-throwing-axe"],
    "dagger/dirk": ["phb:dagger-or-dirk"],
    "two-handed sword": ["phb:two-hand-sword"],
    "staff sling": ["phb:staff-sling"],
    "long spear": ["cfh:spear-long"],
    "bola": ["cfh:bolas"],
    "short sword/drusus": ["phb:short-sword", "cfh:sword-drusus"],
    "quarterstaff/bo stick": ["phb:quarterstaff", "cfh:bo-stick"],
    "knife/stiletto": ["phb:knife", "cfh:stiletto"],
}
# <B> labels on these pages that are not a group.
NOT_A_GROUP = {"Special Note"}


def clean(s):
    return re.sub(r"\s+", " ", TAGS.sub(" ", s).replace("&nbsp;", " ")
                  .replace("&amp;", "&")).strip()


def slug(name):
    # The apostrophe becomes a separator, NOT nothing: Table 44's id for `Footman's mace`
    # is `footman-s-mace`, and deleting the apostrophe instead produced `footmans-mace`,
    # which resolved against nothing. Same slug as extract_phb.py, deliberately.
    return re.sub(r"[^a-z0-9]+", "-", name.lower().replace("/", "-")).strip("-")


def weapons(raw):
    """Table 44's columns, with sub-rows qualified by the headings above them.

    The table nests TWO deep: `Sword` / `  Katana` / `    One-handed`. Depth is the
    number of leading spaces in the cell, and a single `parent` variable got it wrong —
    it made Rapier a child of Katana. A stack indexed by depth does not."""
    out, stack = [], {}
    for tbl in TABLE.findall(raw):
        for r in ROW.findall(tbl):
            cells = CELL.findall(r)
            if len(cells) < 7:
                continue
            first = FOOTNOTE.sub("", cells[0])
            raw_first = TAGS.sub("", first).replace("&nbsp;", " ").rstrip()
            depth = (len(raw_first) - len(raw_first.lstrip(" "))) // 2
            name = clean(first)
            if not name or name in ("Item",):
                continue
            c = [clean(FOOTNOTE.sub("", x)) for x in cells]
            prefix = [stack[d] for d in sorted(stack) if d < depth]
            key = "-".join([slug(x) for x in prefix] + [slug(name)])
            full = ", ".join(prefix + [name])
            stack[depth] = name
            for d in [d for d in stack if d > depth]:
                del stack[d]
            if all(x in DASH for x in c[1:]):
                out.append({"name": full, "slug": key, "isGroup": True, "cells": None})
                continue
            out.append({"name": full, "slug": key, "isGroup": False, "cells": c})
    return out


def groups(raw):
    """[(name, [member strings])] from a <B>Name:</B> ... list page."""
    out = []
    for m in BOLD.finditer(raw):
        label = clean(m.group(1))
        if not label.endswith(":"):
            continue
        nxt = BOLD.search(raw, m.end())
        body = clean(raw[m.end():nxt.start() if nxt else len(raw)])
        body = body.split("Table of Contents")[0]
        # The list ends at its own full stop; on the last group of a page a paragraph of
        # prose follows it, and swallowing that turned commentary into thirty members.
        body = body.split(". ")[0]
        members = [x.strip().rstrip(".") for x in body.split(",") if x.strip(" .")]
        if members:
            out.append((label.rstrip(":").strip(), members))
    return out


def main():
    wh = pathlib.Path(sys.argv[1]) / "CFH"
    pack = pathlib.Path(sys.argv[2])
    known = set()
    for f in pack.glob("*.json"):
        if f.name == "manifest.json":
            continue
        for kind, recs in json.loads(f.read_text()).items():
            if isinstance(recs, list):
                known.update(r["id"] for r in recs if isinstance(r, dict) and "id" in r)

    prov = lambda n, p: {"section": ["Comp. Fighter's Handbook", n],
                         "anchor": {"rendition": "webhelp", "file": p}}
    records, new_ids = [], set()
    for w in weapons((wh / "DD05367.HTM").read_text(encoding="cp1252", errors="replace")):
        rec = collections.OrderedDict([("id", f"cfh:{w['slug']}"), ("name", w["name"]),
                                       ("provenance", prov(w["name"], "DD05367.HTM"))])
        if w["isGroup"]:
            rec["isGroup"] = True
        else:
            for key, i in (("cost", 1), ("weight", 2), ("size", 3), ("damageType", 4),
                           ("speedFactor", 5), ("damageSmallMedium", 6), ("damageLarge", 7)):
                if i < len(w["cells"]) and w["cells"][i] not in DASH:
                    rec[key] = w["cells"][i]
        records.append(rec)
        new_ids.add(rec["id"])

    def resolve(member):
        star = member.endswith("*")
        name = member.rstrip("*").strip()
        k = name.lower()
        if k in ALIAS:
            return [i for i in ALIAS[k] if i in new_ids or i in known] or None
        for cand in (f"cfh:{slug(name)}", LEAF.get(slug(name)), f"phb:{slug(name)}"):
            if cand is None:
                continue
            if cand in new_ids or cand in known:
                if star and cand.startswith("phb:") and f"cfh:{slug(name)}" in new_ids:
                    return [f"cfh:{slug(name)}"]
                return [cand]
        return None

    # A new weapon printed as a sub-row carries its heading in the id — `cfh:polearm-naginata`
    # — and the group lists name the leaf alone. This indexes the leaf so the two meet.
    LEAF = {}
    for r in records:
        leaf = r["id"].split(":", 1)[1].rsplit("-", 1)[-1]
        LEAF.setdefault(slug(r["name"].split(",")[-1].strip()), r["id"])

    unresolved = collections.Counter()
    for page, kind, cost in (("DD05200", "tight", 2), ("DD05201", "broad", 3),
                             ("DD05202", "none", None)):
        raw = (wh / f"{page}.HTM").read_text(encoding="cp1252", errors="replace")
        for name, members in groups(raw):
            if name in NOT_A_GROUP:
                continue
            ids, lost = [], []
            for m in members:
                i = resolve(m)
                (ids.extend(i) if i else lost.append(m))
            unresolved.update(lost)
            rec = collections.OrderedDict([
                ("id", f"cfh:group-{slug(name)}"), ("name", name),
                ("provenance", prov(name, f"{page}.HTM")),
                ("isGroup", True), ("groupKind", kind), ("members", ids)])
            if cost:
                rec["slotCost"] = cost
            if lost:
                rec["interpretation"] = {"confidence": "reading", "note":
                    "UNMODELLED SUBJECT: members this pack has no weapon record for — "
                    + ", ".join(lost) + "."}
            records.append(rec)

    if "--json" in sys.argv:
        json.dump({"weaponProficiencies": records}, sys.stdout, indent=2, ensure_ascii=False)
        return 0
    w = [r for r in records if not r["id"].startswith("cfh:group-")]
    g = [r for r in records if r["id"].startswith("cfh:group-")]
    print(f"{len(w)} new weapons ({sum(1 for x in w if x.get('isGroup'))} of them headings)")
    print(f"{len(g)} weapon groups: "
          + ", ".join(f"{k} {sum(1 for x in g if x['groupKind'] == k)}"
                      for k in ("tight", "broad", "none")))
    print(f"  members resolved: {sum(len(x['members']) for x in g)}")
    if unresolved:
        print("  members with no record: "
              + ", ".join(f"{k} ({v})" for k, v in unresolved.most_common()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
