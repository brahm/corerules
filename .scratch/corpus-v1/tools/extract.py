#!/usr/bin/env python3
"""First extractor for the proving slice (ticket 13).

Ticket 09 settled the pipeline's shape and this is its first working piece:
WebHelp HTML is the only parse target, one titled page per record, field labels
carried as <I>Label:</I> or <B>Label:</B> markup rather than typographic convention.

Deliberately does only the MECHANICAL half. Field prose comes out as text; turning
that prose into §4.3 effects is judgement and belongs to a later pass.

Usage:  extract.py <webhelp-dir> <book-dir> <pack-id> [--json]
"""
import re, sys, json, pathlib, hashlib

TITLE = re.compile(r"<title>(.*?)</title>", re.I | re.S)
LABEL = re.compile(r"<([IB])>\s*([A-Z][A-Za-z0-9 /'&,-]{2,40}):\s*</\1>", re.I)
TAGS = re.compile(r"<[^>]+>")
WS = re.compile(r"\s+")

# One per record, in every book measured (ticket 07: 133 kits in 133 files).
KIT_MARKER = "Role"


def clean(s: str) -> str:
    s = TAGS.sub(" ", s)
    s = (s.replace("&quot;", '"').replace("&amp;", "&")
          .replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " "))
    return WS.sub(" ", s).strip()


def parse(path: pathlib.Path):
    raw = path.read_text(encoding="cp1252", errors="replace")
    t = TITLE.search(raw)
    if not t:
        return None
    title = clean(t.group(1))
    marks = list(LABEL.finditer(raw))
    if not any(m.group(2).strip() == KIT_MARKER for m in marks):
        return None

    fields = {}
    for i, m in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(raw)
        fields[m.group(2).strip()] = clean(raw[m.end():end])

    # Title carries the book: "Bounty Hunter (Comp. Thief's Handbook)"
    name, _, book = title.partition("(")
    return {
        "name": name.strip(),
        "book": book.rstrip(")").strip(),
        "file": path.name,
        "fields": fields,
    }


def to_record(parsed, pack_id: str, target: str):
    stem = pathlib.PurePath(parsed["file"]).stem
    return {
        "id": f"{pack_id}:{stem}",
        "name": parsed["name"],
        "provenance": {
            "section": [parsed["book"], parsed["name"]],
            "anchor": {"rendition": "webhelp", "file": parsed["file"]},
        },
        "target": target,
        "cardinality": "one-per-target",
        # MECHANICAL HALF ONLY. Turning field prose into §4.3 effects is judgement.
        # `effectsModelled: false` is what makes that state honest rather than
        # indistinguishable from a modelled record that has no effects — A3's
        # distinction, one level down. Ticket 13 found this by running the extractor.
        "effects": [],
        "effectsModelled": False,
    }
    # Field prose is deliberately NOT carried into the record. It is book text, and
    # the record already points at it: ticket 05's anchor is how the review page
    # (ticket 12) fetches the source. Carrying it here would duplicate the corpus
    # inside the pack — the shape §6.5 forbids.



def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) < 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    wh, book, pack_id = pathlib.Path(args[0]), args[1], args[2]
    target = f"{pack_id}:thief"

    records, parsed_all, skipped = [], [], 0
    for f in sorted((wh / book).glob("*.HTM")):
        p = parse(f)
        if p is None:
            skipped += 1
            continue
        parsed_all.append(p)
        records.append(to_record(p, pack_id, target))

    if "--json" in sys.argv:
        json.dump({"kits": records}, sys.stdout, indent=2, ensure_ascii=False)
        return 0

    print(f"{book}: {len(records)} records, {skipped} pages skipped")
    labels = {}
    for p in parsed_all:
        for k in p["fields"]:
            labels[k] = labels.get(k, 0) + 1
    print("field labels found, with record counts:")
    for k, v in sorted(labels.items(), key=lambda kv: -kv[1]):
        print(f"   {k:<28}{v:>4}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
