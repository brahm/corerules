#!/usr/bin/env python3
"""Extract HTML tables as §3.1 lookup-table records (ticket 13).

A different extraction path from extract.py: the record IS a <TABLE>, not a run of
labelled prose, so the mechanical bucket here is genuinely mechanical.

Usage: extract_tables.py <webhelp-dir> <book> <pack-id> <file>[,<file>...] [--json]
"""
import re, sys, json, pathlib

TITLE = re.compile(r"<title>(.*?)</title>", re.I | re.S)
TR = re.compile(r"<tr[^>]*>(.*?)</tr>", re.I | re.S)
TD = re.compile(r"<t[dh][^>]*>(.*?)</t[dh]>", re.I | re.S)
TAGS = re.compile(r"<[^>]+>")
clean = lambda s: " ".join(TAGS.sub(" ", s).split())


def grid(raw):
    return [[clean(c) for c in TD.findall(tr)] for tr in TR.findall(raw)]


def record(path, pack_id):
    raw = path.read_text(encoding="cp1252", errors="replace")
    title = clean(TITLE.search(raw).group(1))
    name, _, book = title.partition("(")
    name = name.strip().rstrip("-").strip()
    # The chapter copy titles itself "Thieving Skill Base Scores-- Table 26" and the
    # appendix copy "Table 26: Thieving Skill Base Scores". Same table, two titles.
    m = re.match(r"(.*?)\s*--\s*(Table \d+)$", name)
    if m:
        name = f"{m.group(2)}: {m.group(1)}"
    g = [r for r in grid(raw) if any(c for c in r)]
    if not g:
        return None
    # Header rows are the ones carrying no data cell; the book splits a header over two
    # lines when a column label is long, so take every leading row without digits.
    head, body = [], []
    for r in g:
        (head if not body and not any(re.search(r"\d|--", c) for c in r) else body).append(r)
    cols = [" ".join(x).strip() for x in zip(*[h + [""] * (max(map(len, g)) - len(h)) for h in head])] if head else []
    return {
        "id": f"{pack_id}:{path.stem}",
        "name": name,
        "provenance": {"section": [book.rstrip(")").strip(), name],
                       "anchor": {"rendition": "webhelp", "file": path.name}},
        # Ticket 13 resolved known unknown #2: a table must declare the FIELD PATH it fills and key
        # its rows by id. Neither can be inferred from the markup, so both are emitted empty and a
        # human fills them — which is the same posture as the apparatus list, and for the same
        # reason: the source does not say.
        "supplies": "",
        "keyedBy": {"kind": "id"},
        "_keyColumn": cols[0] or "key",
        "columns": [c for c in cols[1:] if c] or [f"col{i}" for i in range(1, len(body[0]))],
        "rows": body,
    }


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    wh, book, pack_id, files = pathlib.Path(args[0]), args[1], args[2], args[3].split(",")
    out = [r for r in (record(wh / book / f, pack_id) for f in files) if r]
    if "--json" in sys.argv:
        json.dump({"lookupTables": out}, sys.stdout, indent=2, ensure_ascii=False)
        return 0
    for r in out:
        print(f"{r['id']:<16} {r['name'][:46]:<48} keyedBy={r['keyedBy']} cols={len(r['columns'])} rows={len(r['rows'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
