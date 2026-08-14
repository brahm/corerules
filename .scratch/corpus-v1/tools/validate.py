#!/usr/bin/env python3
"""Validate a pack directory against pack-0.1.schema.json.

This is ticket 10's PACK-VISIBLE checker — the half that needs nothing but the pack
itself. The other half (cross-pack referential integrity: does `phb:halfling` exist?)
belongs to the Engine, because a pack cannot see the packs it points at. Ticket 13's
Acrobat proved that split concrete: its race carve-out inverts into an A3 permit-list
that requires the full race enumeration, which lives in another pack.

A pack is ONE document — manifest plus the record arrays — so it is validated as one,
against the schema's top level. Validating records individually is how you end up
demanding a manifest of every record.

Also reports, without failing, the two states A3 exists to keep apart:
records still awaiting the judgement pass, and files the manifest never declared.

Usage:  validate.py <pack-dir>
"""
import json, sys, pathlib, collections

try:
    import jsonschema
except ImportError:
    print("needs jsonschema:  pip install jsonschema", file=sys.stderr)
    raise SystemExit(2)

SCHEMA = pathlib.Path(__file__).resolve().parent.parent / "schema" / "pack-0.1.schema.json"


def main():
    if len(sys.argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    pack = pathlib.Path(sys.argv[1])
    schema = json.loads(SCHEMA.read_text())
    manifest = json.loads((pack / "manifest.json").read_text())

    # §7.1 is declaration over discovery, so the manifest's file list drives this, not a glob.
    doc, missing = {}, []
    for name in manifest.get("files", []):
        f = pack / name
        if not f.exists():
            missing.append(name)
            continue
        # Two files may contribute to the SAME kind — the PHB's proficiencies and the ones
        # the Complete handbooks add both arrive as `nonweaponProficiencies`. A plain update
        # silently replaces the first with the second, and the record count stays plausible
        # while 65 records vanish. Ticket 13 finding 103: arrays MERGE, they do not overwrite.
        for kind, recs in json.loads(f.read_text()).items():
            doc.setdefault(kind, []).extend(recs)

    doc["manifest"] = manifest
    errors = sorted(jsonschema.Draft202012Validator(schema).iter_errors(doc),
                    key=lambda e: list(e.path))
    total = sum(len(v) for k, v in doc.items() if k != "manifest")
    for e in errors:
        where = "/".join(str(p) for p in e.path) or "(root)"
        print(f"  INVALID  {where}: {e.message[:150]}")

    unmodelled = [r["id"] for arr, recs in doc.items() if arr != "manifest"
                  for r in recs if r.get("effectsModelled") is False]

    # Ticket 13 finding 45. Ticket 10 puts cross-pack referential integrity on the Engine,
    # correctly — a pack cannot see the packs it points at. But nothing was COUNTING the
    # references, so nobody noticed that not one of the proving slice's 75 resolved. Report
    # them, grouped by prefix, without failing: a pack that points outward is normal, and a
    # pack every one of whose references points outward is worth knowing about.
    defined, refs = set(), collections.Counter()

    def walk(conds):
        for c in conds or []:
            for cc in c.get("anyOf", [c]):
                for i in cc.get("anyOfIds", []):
                    refs[i] += 1
                if "ref" in cc:
                    refs[cc["ref"]] += 1

    for arr, recs in doc.items():
        if arr == "manifest":
            continue
        for r in recs:
            defined.add(r["id"])
            # Ticket 13 finding 117: `target` was never walked, and it is an Attachable's
            # MOST repeated reference — every kit names the class it attaches to. 171 of the
            # pack's 177 targets pointed at nothing and no count showed it.
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
    unresolved = collections.Counter(i.split(":")[0] for i in refs if i not in defined)
    undeclared = sorted(p.name for p in pack.glob("*.json")
                        if p.name != "manifest.json" and p.name not in manifest.get("files", []))

    print(f"\n{total} records, {len(errors)} schema errors")
    if missing:
        print(f"  declared but absent: {', '.join(missing)}")
    if undeclared:
        print(f"  present but undeclared (§7.1): {', '.join(undeclared)}")
    if unmodelled:
        print(f"  awaiting the judgement pass: {len(unmodelled)} of {total} "
              f"(effectsModelled: false — not 'no effects')")
    if unresolved:
        n = sum(unresolved.values())
        print(f"  references resolved by the Engine, not here: {n} of {len(refs)} "
              f"({', '.join(f'{k}: {v}' for k, v in unresolved.most_common())})")
    return 1 if errors or missing else 0


if __name__ == "__main__":
    sys.exit(main())
