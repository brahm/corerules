#!/usr/bin/env python3
"""Extract HTML tables as §3.1 lookup-table records (ticket 13, extended in session 65).

A different extraction path from extract.py: the record IS a <TABLE>, not a run of
labelled prose, so the mechanical bucket here is genuinely mechanical.

Session 65 took it over the v1 tier's remaining 90 tables and gave it the two things it
had been leaving to a human:

  keyedBy   inferred from the KEYS THEMSELVES. All integers is `integer`; `4-6` and
            `1-4` are `range`; keys that all resolve to records of one kind are `id`,
            with that kind as the vocabulary; anything else is `text`, which says the
            key is the book's prose and the Engine cannot index it (finding 138).

  supplies  still never guessed, and now ABSENT rather than empty — the schema's own
            rule is that a table supplying no field is not consumed, and an empty
            string claimed a field path called "".

Usage: extract_tables.py <webhelp-dir> <book> <pack-id> <file>[,<file>...] [--json]
                         [--pack <dir>]   resolve row keys against this pack
"""
import re, sys, json, pathlib

TITLE = re.compile(r"<title>(.*?)</title>", re.I | re.S)
TR = re.compile(r"<tr[^>]*>(.*?)</tr>", re.I | re.S)
TD = re.compile(r"<t[dh][^>]*>(.*?)</t[dh]>", re.I | re.S)
TAGS = re.compile(r"<[^>]+>")
clean = lambda s: " ".join(TAGS.sub(" ", s).split())


def grid(raw):
    return [[clean(c) for c in TD.findall(tr)] for tr in TR.findall(raw)]


def singular(kind):
    """`classes` -> `class`, `abilities` -> `ability`. The vocabulary names the KIND in
    the singular, which is the convention the hand-made tables set."""
    if kind.endswith("ies"):
        return kind[:-3] + "y"
    if kind.endswith(("sses", "xes", "ches", "shes")):
        return kind[:-2]
    if kind.endswith("s") and not kind.endswith("ss"):
        return kind[:-1]
    return kind


def kebab(kind):
    return re.sub(r"(?<!^)(?=[A-Z])", "-", singular(kind)).lower()


def slug(name):
    # A key often carries a footnote marker the book set in a smaller font — `Paladin*`,
    # `Elf1` — which is not part of the name and stopped every such key from resolving.
    name = re.sub(r"[*†‡]+$|\s*\d+$", "", name.strip())
    return re.sub(r"[^a-z0-9]+", "-", name.lower().replace("/", "-")).strip("-")


# Table 13 abbreviates its own row label; the pack's record is the chapter's full name.
ALIAS = {"specialist": "phb:specialist-wizard"}


def axis(keys, by_kind):
    """What the row keys ARE, decided by looking at them.

    Resolution is PER KIND, not over one global name map. A global map made Table 7's
    `Strength` resolve to the Complete Priest's priesthood of Strength — §7.3's point,
    demonstrated by a tool that ignored it. A table's keys are drawn from ONE
    enumeration, so the axis is accepted only when a single kind covers all of them.
    """
    keys = [k for k in keys if k]
    if not keys:
        return {"kind": "text"}, None
    if all(re.fullmatch(r"-?\d+", k) for k in keys):
        return {"kind": "integer"}, None
    if all(re.fullmatch(r"-?\d+\s*(-|–|to)\s*\d+|-?\d+\+?|\d+ or (less|more)", k) for k in keys):
        return {"kind": "range"}, None
    for kind, names in by_kind.items():
        hits = [ALIAS.get(slug(k)) or names.get(slug(k)) for k in keys]
        if all(hits):
            return {"kind": "id", "vocabulary": f"phb:{kebab(kind)}"}, hits
    return {"kind": "text"}, None


def record(path, pack_id, by_kind=None):
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
    # Header rows are the ones carrying no data cell; the book splits a header over two
    # lines when a column label is long, so take leading rows without digits — but AT MOST
    # TWO. Table 38's rows are all words, and an uncapped rule made the whole table a
    # header and emitted a record with zero rows.
    head, body = [], []
    for r in g:
        is_head = not body and len(head) < 2 and not any(re.search(r"\d|--", c) for c in r)
        (head if is_head else body).append(r)
    body = [r for r in body if r and r[0]]
    cols = [" ".join(x).strip() for x in zip(*[h + [""] * (max(map(len, g)) - len(h)) for h in head])] if head else []
    key, ids = axis([r[0] for r in body], by_kind or {})
    rows = [([ids[i]] + r[1:]) if ids else r for i, r in enumerate(body)]
    return {
        "id": f"{pack_id}:{path.stem}",
        "name": name,
        "provenance": {"section": [book.rstrip(")").strip(), name],
                       "anchor": {"rendition": "webhelp", "file": path.name}},
        # `supplies` is still never guessed: which field a table fills is a modelling
        # decision, and the schema's own rule is that a table without it is not consumed.
        "keyedBy": key,
        "columns": [c for c in cols[1:] if c] or [f"col{i}" for i in range(1, len(body[0]))],
        "rows": rows,
    }


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    wh, book, pack_id, files = pathlib.Path(args[0]), args[1], args[2], args[3].split(",")
    by_kind = {}
    if "--pack" in sys.argv:
        pack = pathlib.Path(sys.argv[sys.argv.index("--pack") + 1])
        for f in sorted(pack.glob("*.json")):
            if f.name == "manifest.json":
                continue
            for kind, recs in json.loads(f.read_text()).items():
                if not isinstance(recs, list):
                    continue
                for r in recs:
                    if isinstance(r, dict) and "name" in r:
                        by_kind.setdefault(kind, {}).setdefault(slug(r["name"]), r["id"])
    out = [r for r in (record(wh / book / f, pack_id, by_kind) for f in files) if r]
    if "--json" in sys.argv:
        json.dump({"lookupTables": out}, sys.stdout, indent=2, ensure_ascii=False)
        return 0
    for r in out:
        print(f"{r['id']:<16} {r['name'][:44]:<46} {r['keyedBy']['kind']:<8}"
              f"{len(r['columns'])} cols {len(r['rows'])} rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
