#!/usr/bin/env python3
"""Model the Complete Priest's Handbook's priesthoods (ticket 13, session 62).

The FIRST tool in this effort that does the judgement half rather than the mechanical
one, and it exists because the Complete Priest's is the only book that earns it: its
59 records share ONE ten-field shape, filled in the same order with the same phrasing,
so the mapping from field to effect is a rule rather than a reading.

Every rule below was read off the eight records modelled by hand in sessions 44-46 and
is applied unchanged. Where a field says something the rule does not cover the record
gets an UNMODELLED marker in the same vocabulary the hand pass used — never a guess.

  Minimum Ability Scores  ->  prerequisite, and the experience bonus. `Wisdom or
                              Constitution 16 means +5%; Wisdom and Constitution 16
                              means +10%` becomes TWO `+5` layers, the second
                              conditioned on both — which is §4.4 used exactly right,
                              and is the hand pass's construction, not a new one.
  Alignment               ->  prerequisite. `any lawful alignment` is an axis slice and
                              is expanded through the alignment records' own `ethos`
                              and `morality` (finding 114), never by a name list.
  Races Allowed           ->  prerequisite.
  Spheres of Influence    ->  `grant sphere` / `grant sphereMinor`.
  Powers                  ->  one `grant grantedPower` carrying the field, since its
                              content is a procedure in the Designing Faiths chapter.
  Followers and Strongholds -> `grant follower` at the stated level, marked.
  Weapon and Armor Restrictions -> one representative `grant weaponProficiency`, marked:
                              the field is a permit-list and no operation says `only`.
  Other Limitations       ->  `grant limitation` when it is not `None.`

Usage:  model_deities.py <webhelp-dir> <pack-dir> [--json] [--only DD0nnnn]
"""
import re, sys, json, pathlib, collections

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import extract as E                                        # noqa: E402

MARKER = "Duties of the Priest"
ABILITY = ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"]
RACE = {"dwarves": "phb:dwarf", "elves": "phb:elf", "gnomes": "phb:gnome",
        "half-elves": "phb:half-elf", "halflings": "phb:halfling", "humans": "phb:human"}

PERMIT_NOTE = (
    "UNMODELLED SHAPE: 'Weapons Permitted' is a permit-list (finding 11) — it means ONLY "
    "these, and no operation says 'only'. One weapon is granted as a representative and the "
    "prohibition on everything unnamed is lost. 'Armor Permitted' is worse: phrases like "
    "'all non-metal armor' are a predicate over items, not an enumeration.")
FOLLOWER_NOTE = (
    "UNMODELLED QUANTITY AND NESTING: a roster of followers by level, and often a rule on how "
    "many of each may be taken adventuring and a stronghold cost the priesthood part-pays.")
POWERS_NOTE = (
    "The Powers field, carried as a named ability; its content is a procedure described in the "
    "Designing Faiths chapter.")


# The ten labels the book prints, in the order it prints them. Used to recover a field
# from the PLAIN TEXT when the <I> markup is missing on that one label — 26 field
# instances across 19 of the 59 records, and every one of them present in the text.
# Finding 128. This is finding 82's argument at the field level: it is the same label
# the book printed, so reading it is not a heuristic.
LABELS = ["Alignment", "Minimum Ability Scores", "Races Allowed",
          "Nonweapon and Weapon Proficiencies", "Duties of the Priest",
          "Weapon and Armor Restrictions", "Other Limitations", "Spheres of Influence",
          "Powers", "Followers and Strongholds", "Possible Symbols"]
STRIP = re.compile(r"<[^>]+>")


def fields_from_text(raw):
    txt = re.sub(r"\s+", " ", STRIP.sub(" ", raw).replace("&nbsp;", " ").replace("&amp;", "&"))
    hits = sorted((m.start(), m.end(), lab)
                  for lab in LABELS
                  for m in re.finditer(re.escape(lab) + r"\s*:", txt))
    out = {}
    for i, (s, e, lab) in enumerate(hits):
        stop = hits[i + 1][0] if i + 1 < len(hits) else len(txt)
        out.setdefault(lab, txt[e:stop].strip())
    return out


def sentences(text):
    # Split on the full stop ONLY. The corpus uses the semicolon inside a sentence —
    # `all its priests must be chaotic; they may be chaotic good, ...` — and splitting on
    # it separated the rule from the list that states it.
    return [s.strip() for s in re.split(r"(?<=\.)\s+", text or "") if s.strip()]


def slug(n):
    return re.sub(r"[^a-z0-9]+", "-", n.lower().replace("/", "-")).strip("-")


# ---------------------------------------------------------------- alignment
def alignment_ids(field, ALIGN):
    """Ids for the PRIESTS' alignment sentence, or (None, why) when the rule does not cover it."""
    sent = next((s for s in sentences(field)
                 if re.search(r"\b(priests?|priesthood)\b", s, re.I)
                 and not re.search(r"^\s*The flock", s, re.I)), None)
    if not sent:
        return None, "no sentence in the Alignment field speaks about the priests"
    low = sent.lower()
    # `So must be his priests and their flock.` — the priests take the deity's alignment,
    # which the field's first sentence states.
    if re.search(r"so must be (his|her|its|the) priests", low):
        deity = sentences(field)[0].lower()
        named = [i for i, r in ALIGN.items() if r["name"].lower() in deity]
        if named:
            return sorted(named), None
    named = [i for i, r in ALIGN.items() if r["name"].lower() in low]
    axis = None
    for field_name in ("ethos", "morality"):
        for value in {r[field_name] for r in ALIGN.values()}:
            if re.search(rf"(?:any(?: sort of| kind of)?|must be|all)\s+{value}\b", low) \
                    and value != "neutral":
                axis = {i for i, r in ALIGN.items() if r[field_name] == value}
    if re.search(r"any alignment", low):
        return sorted(ALIGN), None
    if axis:
        # An axis slice, expanded through the records' own axes. A named alignment in the
        # same sentence usually qualifies a sect and is not the requirement, so the axis wins.
        return sorted(axis), None
    if named:
        return sorted(set(named)), None
    return None, f"the priests' alignment is stated as prose: {sent[:90]!r}"


# ---------------------------------------------------------------- fields
def ability_scores(field):
    """(prerequisites, xp effects)."""
    head, _, tail = (field or "").partition(".")
    mins = [(a, int(n)) for a, n in re.findall(rf"({'|'.join(ABILITY)})\s+(\d+)", head)]
    pre = [{"subject": {"ability": f"phb:{a.lower()}"}, "op": "gte", "value": n}
           for a, n in mins]

    fx = []
    both = re.search(rf"({'|'.join(ABILITY)}) and ({'|'.join(ABILITY)}) 16 means \+(\d+)%", tail)
    either = re.search(rf"({'|'.join(ABILITY)}) or ({'|'.join(ABILITY)}) 16 means \+(\d+)%", tail)
    single = re.search(rf"({'|'.join(ABILITY)}) 16 means \+(\d+)%", tail)
    if both and either:
        fx.append({"op": "adjust", "field": "experienceAward.percent", "by": int(either.group(3)),
                   "text": f"UNMODELLED CONDITION: '{either.group(1)} OR {either.group(2)} 16'. "
                           "Disjunction inside an effect's `when` does not exist, so the "
                           "condition is carried as text."})
        fx.append({"op": "adjust", "field": "experienceAward.percent", "by": int(both.group(3)) // 2,
                   "when": [{"subject": {"ability": f"phb:{both.group(1).lower()}"},
                             "op": "gte", "value": 16},
                            {"subject": {"ability": f"phb:{both.group(2).lower()}"},
                             "op": "gte", "value": 16}]})
    elif single:
        fx.append({"op": "adjust", "field": "experienceAward.percent", "by": int(single.group(2)),
                   "when": [{"subject": {"ability": f"phb:{single.group(1).lower()}"},
                             "op": "gte", "value": 16}]})
    return pre, fx


def spheres(field, SPHERE):
    fx, unknown, qualified = [], [], []
    for kind, pat in (("sphere", r"Major Access to ([^.]+)"),
                      ("sphereMinor", r"Minor Access to ([^.]+)")):
        m = re.search(pat, field or "")
        if not m:
            continue
        # Split on commas OUTSIDE parentheses: the book qualifies an access in place —
        # `Elemental (the priest may only use spells whose names include Fire, Flame, Heat)`
        # — and splitting inside it turned one sphere into four nonexistent ones.
        for name in re.split(r",(?![^(]*\))", m.group(1)):
            name = re.sub(r"^\s*and\s+", "", name.strip()).rstrip(".")
            qualifier = re.search(r"\((.+)", name)
            name = re.sub(r"\s*\(.*", "", name).strip()
            if not name:
                continue
            if qualifier:
                qualified.append(f"{name}: {qualifier.group(1).rstrip(')')}")
            i = f"phb:{slug(name)}"
            if i in SPHERE:
                fx.append({"op": "grant", "kind": kind, "ref": i})
            else:
                unknown.append(name)
    extra = [s for s in sentences(field)
             if not re.match(r"(Major|Minor) Access", s)]
    return fx, unknown, extra, qualified


def weapons(field, WEAPON):
    m = re.search(r"Weapons? Permitted:\s*([^.]+)", field or "")
    if not m:
        return []
    for name in re.split(r",", m.group(1)):
        name = re.sub(r"\(.*?\)", "", name).strip().rstrip(".")
        for cand in (slug(name), slug(name.replace("/", " ")), slug(name.split("/")[0])):
            if f"phb:{cand}" in WEAPON:
                return [{"op": "grant", "kind": "weaponProficiency", "ref": f"phb:{cand}",
                         "text": PERMIT_NOTE}]
    return [{"op": "grant", "kind": "weaponProficiency",
             "defines": {"name": "Permitted weapons", "text": m.group(1).strip()},
             "text": PERMIT_NOTE}]


def followers(field):
    m = re.search(r"received at (\d+)(?:st|nd|rd|th) level", field or "", re.I)
    e = {"op": "grant", "kind": "follower", "ref": "phb:priest", "text": FOLLOWER_NOTE}
    if m:
        e["when"] = [{"subject": {"level": "phb:priest"}, "op": "gte", "value": int(m.group(1))}]
    return [e]


def main():
    wh = pathlib.Path(sys.argv[1]) / "CPRH"
    pack = pathlib.Path(sys.argv[2])
    doc = {}
    for f in pack.glob("*.json"):
        if f.name == "manifest.json":
            continue
        for kind, recs in json.loads(f.read_text()).items():
            if isinstance(recs, list):
                doc.setdefault(kind, []).extend(recs)
    ALIGN = {r["id"]: r for r in doc.get("alignments", [])}
    SPHERE = {r["id"] for r in doc.get("spheres", [])}
    WEAPON = {r["id"] for r in doc.get("weaponProficiencies", [])}
    have = {r["id"] for r in doc.get("deities", [])}
    only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None

    out, notes = [], collections.Counter()
    for f in sorted(wh.glob("*.HTM")):
        if only and f.stem != only:
            continue
        p = E.parse(f, MARKER)
        if not p or (E.EXCLUDE and ("CPRH", p["name"]) in E.EXCLUDE):
            continue
        rid = f"cprh:{f.stem}"
        if rid in have and not only:
            continue
        # Markup first, plain text for whatever the markup lost.
        fields = dict(fields_from_text(f.read_text(encoding="cp1252", errors="replace")))
        fields.update({k: v for k, v in p["fields"].items() if v})
        pre, fx = ability_scores(fields.get("Minimum Ability Scores", ""))
        marks = []

        ids, why = alignment_ids(fields.get("Alignment", ""), ALIGN)
        if ids:
            pre.insert(0, {"member": {"ability": "phb:alignment"}, "anyOfIds": ids})
        else:
            marks.append(f"UNMODELLED CONDITION: {why}.")
            notes["alignment"] += 1

        races = [i for name, i in RACE.items()
                 if re.search(rf"\b{name}\b", fields.get("Races Allowed", ""), re.I)]
        if races:
            pre.append({"member": {"ability": "phb:race"}, "anyOfIds": sorted(races)})
        else:
            marks.append("UNMODELLED CONDITION: the Races Allowed field names no race this "
                         "pack holds.")
            notes["races"] += 1

        sfx, unknown, extra, qualified = spheres(fields.get("Spheres of Influence", ""), SPHERE)
        fx += sfx
        if qualified:
            marks.append("UNMODELLED SCOPE: access to a sphere restricted in place — "
                         + "; ".join(qualified)[:260] + ".")
            notes["sphere-scope"] += 1
        if unknown:
            marks.append("UNMODELLED SUBJECT: sphere access the pack has no record for — "
                         + ", ".join(unknown) + ".")
            notes["sphere"] += 1
        if extra:
            marks.append("UNMODELLED OPTION: the Spheres field carries a further clause — "
                         + " ".join(extra)[:200])
            notes["sphere-option"] += 1

        if fields.get("Powers", "").strip().rstrip(".").lower() not in ("", "none"):
            fx.append({"op": "grant", "kind": "grantedPower", "text": POWERS_NOTE,
                       "defines": {"name": "Granted Powers", "text": POWERS_NOTE}})
        fx += followers(fields.get("Followers and Strongholds", ""))
        fx += weapons(fields.get("Weapon and Armor Restrictions", ""), WEAPON)

        lim = fields.get("Other Limitations", "").strip()
        if lim and lim.rstrip(".").lower() != "none":
            fx.append({"op": "grant", "kind": "limitation",
                       "defines": {"name": "Other Limitations", "text": lim[:400]},
                       "text": "UNMODELLED SHAPE: the Other Limitations field is a rule in "
                               "prose — celibacy, a vow, a taboo — and none of it is a value."})

        rec = collections.OrderedDict([
            ("id", rid), ("name", p["name"]),
            ("provenance", {"section": ["Comp. Priest's Handbook", p["name"]],
                            "anchor": {"rendition": "webhelp", "file": f.name}}),
            ("target", "phb:priest"), ("cardinality", "one-per-target"),
            ("effects", fx), ("effectsModelled", True), ("prerequisite", pre)])
        if marks:
            rec["interpretation"] = {"confidence": "reading", "note": " ".join(marks)[:900]}
        out.append(rec)

    if "--json" in sys.argv:
        json.dump({"deities": out}, sys.stdout, indent=2, ensure_ascii=False)
        return 0
    print(f"{len(out)} priesthoods modelled")
    print(f"  effects: {sum(len(r['effects']) for r in out)}"
          f"   prerequisites: {sum(len(r['prerequisite']) for r in out)}")
    print(f"  records carrying an interpretation note: {sum(1 for r in out if 'interpretation' in r)}")
    if notes:
        print("  fields the rules did not cover: "
              + ", ".join(f"{k} {v}" for k, v in notes.most_common()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
