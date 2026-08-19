#!/usr/bin/env python3
"""Rewrite a pack's files in the canonical serialisation (correction 55).

    json.dumps(obj, indent=2, ensure_ascii=False) + "\n"

with keys in the order they were written. Nothing about the CONTENT changes: the
file is parsed and re-serialised, so a run that reports nothing is a run that
found nothing to fix.

§7.1 chose a directory of JSON because diffability is a standing constraint. A
format with no canonical form serves that only by luck, and twice in two sessions
a script that changed five effects reserialised 24,708 lines because five of the
pack's 27 files were written with one space of indentation and the other 22 with
two. `validate.py` reports the drift; this repairs it.

Usage:  normalise.py <pack-dir> [--check]
"""
import sys, json, pathlib, collections


def canonical(obj):
    return json.dumps(obj, indent=2, ensure_ascii=False) + "\n"


def why(raw, want):
    """A one-line diagnosis, because 'this file differs' is not actionable."""
    if raw + "\n" == want or raw == want.rstrip("\n"):
        return "no trailing newline"
    a, b = raw.splitlines(), want.splitlines()
    for i, (x, y) in enumerate(zip(a, b), 1):
        if x == y:
            continue
        ix = len(x) - len(x.lstrip(" "))
        iy = len(y) - len(y.lstrip(" "))
        if x.strip() == y.strip():
            return f"indented {ix} where the canonical form indents {iy}, from line {i}"
        return f"first differs at line {i}"
    return f"{len(a)} lines against {len(b)}"


def main():
    if len(sys.argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    pack = pathlib.Path(sys.argv[1])
    check = "--check" in sys.argv
    drift = []
    for f in sorted(pack.glob("*.json")):
        raw = f.read_text(encoding="utf-8")
        want = canonical(json.loads(raw, object_pairs_hook=collections.OrderedDict))
        if raw == want:
            continue
        drift.append((f.name, why(raw, want)))
        if not check:
            f.write_text(want, encoding="utf-8")
    for name, reason in drift:
        print(f"  {'DRIFT' if check else 'rewrote'}  {name}  ({reason})")
    print(f"{len(drift)} of {len(list(pack.glob('*.json')))} files "
          + ("differ from the canonical form" if check else "rewritten"))
    return 1 if (check and drift) else 0


if __name__ == "__main__":
    sys.exit(main())
