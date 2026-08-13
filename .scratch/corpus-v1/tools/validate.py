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
import json, sys, pathlib

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
    doc, missing = {"manifest": manifest}, []
    for name in manifest.get("files", []):
        f = pack / name
        if not f.exists():
            missing.append(name)
            continue
        doc.update(json.loads(f.read_text()))

    errors = sorted(jsonschema.Draft202012Validator(schema).iter_errors(doc),
                    key=lambda e: list(e.path))
    total = sum(len(v) for k, v in doc.items() if k != "manifest")
    for e in errors:
        where = "/".join(str(p) for p in e.path) or "(root)"
        print(f"  INVALID  {where}: {e.message[:150]}")

    unmodelled = [r["id"] for arr, recs in doc.items() if arr != "manifest"
                  for r in recs if r.get("effectsModelled") is False]
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
    return 1 if errors or missing else 0


if __name__ == "__main__":
    sys.exit(main())
