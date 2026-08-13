#!/usr/bin/env python3
"""First extractor for the proving slice (ticket 13).

Ticket 09 settled the pipeline's shape and this is its first working piece:
WebHelp HTML is the only parse target, one titled page per record, field labels
carried as <I>Label:</I> or <B>Label:</B> markup rather than typographic convention.

Deliberately does only the MECHANICAL half. Field prose comes out as text; turning
that prose into §4.3 effects is judgement and belongs to a later pass.

Usage:  extract.py <webhelp-dir> <book-dir> <pack-id> [kit|deity|subrace] [--json]
"""
import re, sys, json, pathlib, hashlib

TITLE = re.compile(r"<title>(.*?)</title>", re.I | re.S)
LABEL = re.compile(r"<([IB])>\s*([A-Z][A-Za-z0-9 /'&,-]{2,40}):\s*</\1>", re.I)
TAGS = re.compile(r"<[^>]+>")
WS = re.compile(r"\s+")

# Per kind: the label that appears exactly once per record, and the target it attaches to.
# Ticket 07 measured kit and deity at 1:1 with files; subrace is the exception — five in one
# file — which is what exercises the '#n' ordinal.
KINDS = {
    "kit":     {"marker": "Role",                     "array": "kits",     "multi": False},
    "deity":   {"marker": "Duties of the Priest",     "array": "deities",  "multi": False},
    "subrace": {"marker": "Additional Experience Cost", "array": "subraces", "multi": True},
}

# A record's name sits in plain text immediately before its first field label. The HTML
# gives it no markup at all — no heading, no bold, no larger font — so this is a rule a
# human writes once and a parser then applies forever: ticket 04's 'regular but ambiguous'
# bucket, on the parser side.
NAME_TAIL = re.compile(r"([A-Z][A-Za-z'-]+(?:\s+[A-Z][A-Za-z'-]+){0,3})\s*$")
# For records after the first: the name follows the previous record's final field, after
# that field's sentence ends.
NAME_AFTER = re.compile(r"^[^.]{0,60}\.\s*([A-Z][A-Za-z'-]+(?:\s+[A-Z][A-Za-z'-]+){0,2})")


# A name runs up to the first word that opens a field label. The HTML gives the name no
# markup, so nothing but this stops it swallowing the label that follows it.
LABEL_HEADS = {"Ability", "Languages", "Infravision", "Special", "Additional", "Role",
               "Description", "Equipment", "Races", "Weapon", "Nonweapon", "Secondary",
               "Skill", "Wealth", "Duties", "Followers", "Possible", "Minimum", "Other",
               "Powers", "Alignment", "Spheres"}


def trim_name(name: str) -> str:
    words = name.split()
    out = []
    for w in words:
        if out and w in LABEL_HEADS:
            break
        # the record's prose often restarts with the name itself, lower-cased and pluralised
        if out and w.lower().rstrip("s").startswith(out[-1].lower().rstrip("s")):
            break
        out.append(w)
    return " ".join(out)


def clean(s: str) -> str:
    s = TAGS.sub(" ", s)
    s = (s.replace("&quot;", '"').replace("&amp;", "&")
          .replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " "))
    return WS.sub(" ", s).strip()


def split_multi(raw, marker, book=""):
    """Several records in one file (ticket 07's exception: five subraces in one page).

    The delimiter is the marker itself, which is each record's LAST field. A record spans
    from the previous marker to its own. Names carry no markup at all — no heading, no
    bold, no larger font — so the rule is positional: the first record's name is the
    proper noun before its first label, and every later one follows the previous record's
    final field. That rule is written once by a human and applied forever, which is
    ticket 04's 'regular but ambiguous' bucket landing on the parser side.
    """
    marks = list(LABEL.finditer(raw))
    ends = [m for m in marks if m.group(2).strip() == marker]
    if not ends:
        return []

    out, start = [], 0
    for i, e in enumerate(ends):
        span = [m for m in marks if start <= m.start() <= e.start()]
        if i == 0:
            head = clean(raw[:span[0].start()])
            nm = NAME_TAIL.search(head)
            name = trim_name(nm.group(1)) if nm else "UNNAMED"
        else:
            after = clean(raw[ends[i - 1].end():ends[i - 1].end() + 260])
            nm = NAME_AFTER.search(after)
            name = trim_name(nm.group(1)) if nm else "UNNAMED"

        fields = {}
        for j, m in enumerate(span):
            stop = span[j + 1].start() if j + 1 < len(span) else e.end() + 400
            fields[m.group(2).strip()] = clean(raw[m.end():stop])
        out.append({"name": name, "book": book, "fields": fields})
        start = e.end()
    return out


def parse(path: pathlib.Path, marker: str):
    raw = path.read_text(encoding="cp1252", errors="replace")
    t = TITLE.search(raw)
    if not t:
        return None
    title = clean(t.group(1))
    marks = list(LABEL.finditer(raw))
    if not any(m.group(2).strip() == marker for m in marks):
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


def to_record(parsed, pack_id: str, target: str, ordinal=None):
    stem = pathlib.PurePath(parsed["file"]).stem
    suffix = f"#{ordinal}" if ordinal else ""
    return {
        "id": f"{pack_id}:{stem}{suffix}",
        "name": parsed["name"],
        "provenance": {
            "section": [parsed["book"], parsed["name"]],
            "anchor": dict({"rendition": "webhelp", "file": parsed["file"]},
                           **({"ordinal": ordinal} if ordinal else {})),
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
    kind = args[3] if len(args) > 3 else "kit"
    spec = KINDS[kind]
    target = f"{pack_id}:target"

    records, parsed_all, skipped = [], [], 0
    for f in sorted((wh / book).glob("*.HTM")):
        raw = f.read_text(encoding="cp1252", errors="replace")
        if spec["multi"]:
            t = TITLE.search(raw)
            book_name = clean(t.group(1)).partition("(")[2].rstrip(")").strip() if t else book
            found = split_multi(raw, spec["marker"], book_name)
            if not found:
                skipped += 1
                continue
            for n, rec in enumerate(found, 1):
                rec["file"] = f.name
                parsed_all.append(rec)
                records.append(to_record(rec, pack_id, target, ordinal=n))
        else:
            p = parse(f, spec["marker"])
            if p is None:
                skipped += 1
                continue
            parsed_all.append(p)
            records.append(to_record(p, pack_id, target))

    if "--json" in sys.argv:
        json.dump({spec["array"]: records}, sys.stdout, indent=2, ensure_ascii=False)
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
