#!/usr/bin/env python3
"""Re-measure ticket 13's verdict over a pack (ticket 13, session 59).

The verdict was measured by hand at session 48 and was stale within two sessions:
it still said none of the pack's references resolve, on a pack where 86% of them
do. A number that took an afternoon to produce and goes wrong by itself is a tool
that was never written. This is that tool.

Every figure in the ticket's VERDICT section comes from here. Two populations are
reported separately and never mixed:

  ATTACHABLES   kits, deities and subraces — the proving slice ticket 08 defined,
                and the only population the original verdict measured.
  MODELLED      every record carrying an `effects` array, which now includes races
                and will include whatever else grows one.

Records in kinds with NO effects array — proficiencies, weapons, alignments — are
counted as records and never as incomplete ones: they have nothing to express.

Usage:  verdict.py <pack-dir> [--markers]
"""
import re, sys, json, pathlib, collections

ATTACHABLE = ("kits", "deities", "subraces")
MARKER = "UNMODELLED"

# Ticket 13 finding 115: the markers classify THEMSELVES. A marker is written
# `UNMODELLED CONDITION:` or `UNMODELLED VALUE AND SUBJECT:` — a category named by
# the transcriber at the moment of the refusal, when the source was in front of
# them, and 232 of 262 carry one. So this reads the declared label rather than
# guessing from the prose after the fact. A compound label counts once in EACH of
# its categories, which is why the classified total exceeds the marker count.
DECLARED = re.compile(r"UNMODELLED[ ]+([A-Z][A-Z -]*[A-Z])")
CATEGORY = {
    "CONDITION": "conditions", "CONDITIONS": "conditions",
    "VALUE": "operands and values", "OPERAND": "operands and values",
    "QUANTITY": "operands and values", "FRACTION": "operands and values",
    "PROPORTION": "operands and values", "PARAMETER": "operands and values",
    "ANTI-SCALING": "operands and values",
    "SUBJECT": "subjects",
    "SHAPE": "shapes",
    "SCOPE": "scopes and earmarks", "EARMARK": "scopes and earmarks",
    "CAP": "caps and clamps", "CLAMP": "caps and clamps",
    "REDUCTION": "reductions and substitutions", "SUBSTITUTION": "reductions and substitutions",
    "SHIFT": "reductions and substitutions", "MUTATION": "reductions and substitutions",
    "CHOICE": "choices", "OPTION": "choices", "ELECTION": "choices",
    "TRADE": "choices", "FORFEIT": "choices", "SPLIT": "choices",
    "FREQUENCY": "frequencies and triggers", "TRIGGER": "frequencies and triggers",
    "EXPIRY": "frequencies and triggers", "SUSTAIN": "frequencies and triggers",
    "TEMPORARY STATE": "frequencies and triggers", "DEFINITION": "shapes",
    "NESTING": "composition", "LINK": "composition", "RELATION": "composition",
    "PRECEDENCE": "composition", "PROCEDURE": "composition",
}


def load(pack):
    doc = {}
    for f in sorted(pack.glob("*.json")):
        if f.name == "manifest.json":
            continue
        for kind, recs in json.loads(f.read_text()).items():
            if isinstance(recs, list):
                doc.setdefault(kind, []).extend(recs)
    return doc


def book(rec):
    return rec["id"].split(":", 1)[0]


def measure(records):
    """(records with effects, effects, markers, complete records)"""
    withfx = [r for r in records if "effects" in r]
    fx = [e for r in withfx for e in r["effects"]]
    marked = [e for e in fx if MARKER in (e.get("text") or "")]
    complete = [r for r in withfx
                if not any(MARKER in (e.get("text") or "") for e in r["effects"])]
    return withfx, fx, marked, complete


def references(doc):
    defined, refs = set(), collections.Counter()

    def walk(conds):
        for c in conds or []:
            for cc in c.get("anyOf", [c]):
                for i in cc.get("anyOfIds", []):
                    refs[i] += 1
                if "ref" in cc:
                    refs[cc["ref"]] += 1

    for recs in doc.values():
        for r in recs:
            defined.add(r["id"])
            # Ticket 13 finding 117: `target` was never walked, and it is an Attachable's
            # MOST repeated reference — every kit names the class it attaches to.
            t = r.get("target")
            for i in ([t] if isinstance(t, str) else
                      list(t.values()) if isinstance(t, dict) else t or []):
                if isinstance(i, str):
                    refs[i] += 1
                elif isinstance(i, list):
                    for x in i:
                        refs[x] += 1
            walk(r.get("prerequisite"))
            for e in r.get("effects", []):
                if "ref" in e:
                    refs[e["ref"]] += 1
                for i in e.get("from", []):
                    refs[i] += 1
                walk(e.get("when"))
    return defined, refs


def main():
    pack = pathlib.Path(sys.argv[1])
    doc = load(pack)
    allrecs = [r for recs in doc.values() for r in recs]

    att = [r for k in ATTACHABLE for r in doc.get(k, [])]
    for label, pop in (("ATTACHABLES (the proving slice)", att),
                       ("MODELLED (every record with effects)", allrecs)):
        withfx, fx, marked, complete = measure(pop)
        print(f"\n{label}")
        print(f"  records carrying effects   {len(withfx):>6}")
        print(f"  effects                    {len(fx):>6}")
        print(f"  expressed without a marker {len(fx)-len(marked):>6}"
              f"  ({100*(len(fx)-len(marked))/max(len(fx),1):.0f}%)")
        print(f"  carrying UNMODELLED        {len(marked):>6}"
              f"  ({100*len(marked)/max(len(fx),1):.0f}%)")
        print(f"  records complete           {len(complete):>6} of {len(withfx)}"
              f"  ({100*len(complete)/max(len(withfx),1):.0f}%)")

    print("\nRECORDS BY KIND")
    for k, v in sorted(doc.items(), key=lambda kv: -len(kv[1])):
        fx = sum(len(r.get("effects", [])) for r in v)
        print(f"  {k:<26}{len(v):>5}{('  ' + str(fx) + ' effects') if fx else '  (no effects)'}")
    print(f"  {'TOTAL':<26}{len(allrecs):>5}")

    print("\nCOMPLETENESS BY BOOK (records carrying effects)")
    per = collections.defaultdict(list)
    for r in allrecs:
        if "effects" in r:
            per[book(r)].append(r)
    for b, rs in sorted(per.items(), key=lambda kv: -len(kv[1])):
        c = sum(1 for r in rs
                if not any(MARKER in (e.get("text") or "") for e in r["effects"]))
        print(f"  {b:<8}{len(rs):>5}{c:>6} complete  ({100*c/len(rs):.0f}%)")

    print("\nOPERATIONS")
    ops = collections.Counter(e["op"] for r in allrecs for e in r.get("effects", []))
    tot = sum(ops.values())
    for op, n in ops.most_common():
        print(f"  {op:<10}{n:>6}  {100*n/tot:.1f}%")

    defined, refs = references(doc)
    occ, bad = sum(refs.values()), sum(v for k, v in refs.items() if k not in defined)
    dbad = sum(1 for k in refs if k not in defined)
    print("\nREFERENCES")
    print(f"  occurrences  {occ - bad:>5} of {occ} resolve  ({100*(occ-bad)/occ:.0f}%)")
    print(f"  distinct ids {len(refs)-dbad:>5} of {len(refs)} resolve  ({100*(len(refs)-dbad)/len(refs):.0f}%)")
    by = collections.Counter(k.split(":")[0] for k in refs if k not in defined)
    print("  unresolved by prefix: " + ", ".join(f"{k} {v}" for k, v in by.most_common()))

    print("\nWHAT THE MARKERS SAY (the category each marker declares for itself)")
    cls, undeclared, unknown, total = collections.Counter(), 0, collections.Counter(), 0
    residue = []
    for r in allrecs:
        for e in r.get("effects", []):
            t = e.get("text") or ""
            if MARKER not in t:
                continue
            total += 1
            m = DECLARED.search(t)
            if not m:
                undeclared += 1
                residue.append((r["id"], t))
                continue
            for word in re.split(r"\s+AND\s+", m.group(1)):
                word = word.strip()
                if word in CATEGORY:
                    cls[CATEGORY[word]] += 1
                else:
                    unknown[word] += 1
    for name, n in cls.most_common():
        print(f"  {name:<30}{n:>5}")
    print(f"  {'— declaring no category':<30}{undeclared:>5}")
    if unknown:
        print("  labels with no mapping: " + ", ".join(f"{k} {v}" for k, v in unknown.most_common()))
    print(f"  {total} markers, {total-undeclared} declaring a category")
    if "--markers" in sys.argv:
        print("\nDECLARING NO CATEGORY")
        for i, t in residue:
            print(f"  {i}  {t[:140]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
