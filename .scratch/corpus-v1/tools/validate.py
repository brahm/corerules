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

Also reports, without failing: the two states A3 exists to keep apart — records
still awaiting the judgement pass, and files the manifest never declared — and,
since correction 55, any file that is not in the canonical serialisation.

Correction 6: the manifest's `provenanceMode` is checked here rather than in the schema,
because the condition spans two files — the mode is in the manifest and the anchors are in
the records — and a JSON Schema validates one document at a time. Same reason duplicate ids
are checked here (correction 40).

Usage:  validate.py <pack-dir>
"""
import re, json, sys, pathlib, collections

# Ticket 13 finding 135. This walked a HAND-LISTED set of paths — target, prerequisite,
# effects.ref, effects.from, when — and every kind added since put ids somewhere the list
# did not mention: `members` on a weapon group, `group` on a proficiency, `schools` and
# `spheres` on a spell, `combines` on a class. The checker was seeing 1,985 of the pack's
# 4,110 references. It now walks EVERY string shaped like an id, which is mechanical and
# cannot fall behind a new kind.
#
# `vocabulary` is excluded because it names a KIND, not a record — the one place an
# id-shaped string is deliberately not a reference.
ID_SHAPED = re.compile(r"^[a-z][a-z0-9]*:[a-z0-9][a-z0-9-]*$")
NOT_A_REFERENCE = {"vocabulary"}


def collect(node, refs, key=None):
    if isinstance(node, str):
        if key not in NOT_A_REFERENCE and ID_SHAPED.match(node):
            refs[node] += 1
    elif isinstance(node, list):
        for x in node:
            collect(x, refs, key)
    elif isinstance(node, dict):
        for k, v in node.items():
            if k != "id":
                collect(v, refs, k)


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

    # Correction 6. §7.1 made book-and-page mandatory on every record; §5.1 says the
    # house-rule escape hatch IS the pack. A hand-authored record has no rendition and no
    # source file, so the two rules could not both hold and nobody noticed. A3 applied to
    # provenance: the pack DECLARES which kind it is, and the requirement follows.
    mode = manifest.get("provenanceMode")
    provenance = []
    for arr, recs in doc.items():
        if arr == "manifest":
            continue
        for r in recs:
            pv = r.get("provenance")
            if mode == "extracted":
                if not pv:
                    provenance.append(f"{r['id']}  no provenance, in an `extracted` pack")
                elif not pv.get("anchor"):
                    provenance.append(f"{r['id']}  no anchor, in an `extracted` pack")
            elif mode == "hand-authored" and pv and pv.get("anchor"):
                provenance.append(f"{r['id']}  carries an anchor, in a `hand-authored` pack — "
                                  f"it names a rendition of a source the manifest does not claim")
    if mode == "extracted" and not manifest.get("sources"):
        provenance.append("manifest  declares `extracted` and names no sources")
    if mode == "hand-authored" and manifest.get("sources"):
        provenance.append("manifest  declares `hand-authored` and names sources anyway")

    # Correction 51. A limitation declares the `kind` it bounds; an effect that names it with a
    # different one is a disagreement nothing could previously see. `phb:thief-weapon-restriction`
    # was lifted as `weaponProficiency` three times and as `weapon` once, and the odd one out was
    # the Spy, whose book is restating the general non-proficiency rule rather than making an
    # exception. Checked here because the limitation and the effects naming it are in different
    # files, which a JSON Schema validates one at a time.
    bounds = {r["id"]: r.get("bounds") for arr, recs in doc.items() if arr == "limitations"
              for r in recs}
    kinds = []
    for arr, recs in doc.items():
        if arr == "manifest":
            continue
        for r in recs:
            for i, e in enumerate(r.get("effects") or []):
                ref = e.get("ref")
                if ref in bounds and bounds[ref] and e.get("kind") != bounds[ref]:
                    kinds.append(f"{r['id']}[{i}] {e['op']}s {ref} as `{e.get('kind')}`, "
                                 f"which bounds `{bounds[ref]}`")

    # Correction 45. The marker convention was typographic and had already drifted — 286
    # markers, six of them punctuated differently, and prose that merely MENTIONS the word
    # reads as a marker to any regex. `unmodelled` is now the statement; this keeps the text
    # and the field from parting company, which is how the convention drifted in the first
    # place. Reported, not failed: a text that discusses markers is legitimate.
    markers = []
    for arr, recs in doc.items():
        if arr == "manifest":
            continue
        for r in recs:
            for i, e in enumerate(r.get("effects") or []):
                says = "UNMODELLED" in (e.get("text") or "")
                flag = bool(e.get("unmodelled"))
                if says != flag:
                    markers.append(f"{r['id']}[{i}] "
                                   + ("text says UNMODELLED and `unmodelled` is absent"
                                      if says else "`unmodelled` is set and the text does not say so"))

    # Correction 56. `alsoPrinted` exists to resolve prose into ids, so an alias that resolves
    # to two records is worse than no alias at all: it turns a visible ambiguity into a silent
    # wrong answer. Checked within a KIND — a record's own name may legitimately repeat across
    # books (nine kits are called the same thing in two handbooks, and every priest/wizard spell
    # pair shares a name), so only ALIASES are held to uniqueness, and only against other names
    # and aliases of the same kind.
    aliases = []
    for arr, recs in doc.items():
        if arr == "manifest":
            continue
        claim = collections.defaultdict(list)
        for r in recs:
            for nm in [r["name"]] + list(r.get("alsoPrinted") or []):
                claim[re.sub(r"[^a-z0-9]", "", nm.lower())].append(r["id"])
        for r in recs:
            for nm in r.get("alsoPrinted") or []:
                k = re.sub(r"[^a-z0-9]", "", nm.lower())
                other = [i for i in claim[k] if i != r["id"]]
                if other:
                    aliases.append(f"{r['id']} is also printed {nm!r}, which {arr} already "
                                   f"claims for {', '.join(other)}")

    # Correction 55. §7.1 chose a directory of JSON because diffability is a standing
    # constraint, and a format with no canonical form serves that only by luck: twice in two
    # sessions a script that changed five effects reserialised 24,708 lines, because five of
    # the pack's 27 files were written with one space of indentation and the other 22 with
    # two. REPORTED, not failed — a file's formatting is a fact about the pack's hygiene and
    # not an error in its contents, and a hand-authored pack should not be rejected for having
    # been written in somebody's editor. `tools/normalise.py` repairs it.
    canon = []
    for f in sorted(pack.glob("*.json")):
        raw = f.read_text(encoding="utf-8")
        if raw != json.dumps(json.loads(raw, object_pairs_hook=collections.OrderedDict),
                             indent=2, ensure_ascii=False) + "\n":
            canon.append(f.name)

    # Ticket 13 finding 45. Ticket 10 puts cross-pack referential integrity on the Engine,
    # correctly — a pack cannot see the packs it points at. But nothing was COUNTING the
    # references, so nobody noticed that not one of the proving slice's 75 resolved. Report
    # them, grouped by prefix, without failing: a pack that points outward is normal, and a
    # pack every one of whose references points outward is worth knowing about.
    defined, refs = set(), collections.Counter()
    duplicates, dup_kind = [], {}
    for arr, recs in doc.items():
        if arr == "manifest":
            continue
        for r in recs:
            # Ticket 04 of the Engine map. `defined` was a SET, so an id claimed by two
            # kinds was invisible to the one tool whose job is identity — eight of them,
            # found by the first program that indexed the pack by id. A collision is not
            # a warning: a consumer silently loses one of the two records. Counted apart
            # from schema errors because the schema CANNOT express it: uniqueness spans
            # arrays, and a JSON Schema sees one array at a time.
            if r["id"] in defined:
                duplicates.append(f"{r['id']}  ({dup_kind[r['id']]} and {arr})")
            dup_kind[r["id"]] = arr
            defined.add(r["id"])
            collect(r, refs)
    unresolved = collections.Counter(i.split(":")[0] for i in refs if i not in defined)
    undeclared = sorted(p.name for p in pack.glob("*.json")
                        if p.name != "manifest.json" and p.name not in manifest.get("files", []))

    for d in duplicates:
        print(f"  DUPLICATE  {d}")
    for d in provenance[:20]:
        print(f"  PROVENANCE  {d}")
    for d in kinds[:20]:
        print(f"  KIND  {d}")
    for d in aliases[:20]:
        print(f"  ALIAS  {d}")
    print(f"\n{total} records, {len(errors)} schema errors"
          + (f", {len(duplicates)} duplicate ids" if duplicates else "")
          + (f", {len(provenance)} provenance violations" if provenance else "")
          + (f", {len(kinds)} kind disagreements" if kinds else "")
          + (f", {len(aliases)} ambiguous aliases" if aliases else "")
          + f"  [provenanceMode: {mode}]")
    if missing:
        print(f"  declared but absent: {', '.join(missing)}")
    if undeclared:
        print(f"  present but undeclared (§7.1): {', '.join(undeclared)}")
    if markers:
        print(f"  text and `unmodelled` disagree ({len(markers)}):")
        for m in markers[:8]:
            print(f"    {m}")
    if canon:
        print(f"  not in the canonical serialisation ({len(canon)} of "
              f"{len(list(pack.glob('*.json')))}): {', '.join(canon)}"
              f"\n    — `tools/normalise.py <pack>` rewrites them; correction 55")
    if unmodelled:
        print(f"  awaiting the judgement pass: {len(unmodelled)} of {total} "
              f"(effectsModelled: false — not 'no effects')")
    if unresolved:
        n = sum(unresolved.values())
        print(f"  references resolved by the Engine, not here: {n} of {len(refs)} "
              f"({', '.join(f'{k}: {v}' for k, v in unresolved.most_common())})")
    return 1 if errors or missing or duplicates or provenance or kinds or aliases else 0


if __name__ == "__main__":
    sys.exit(main())
