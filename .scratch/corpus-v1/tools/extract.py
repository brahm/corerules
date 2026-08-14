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
# The terminator is a COLON in eleven books and a PERIOD in the Complete Book of Elves,
# which writes `<B>Role.</B>` after a bullet. Ticket 13 finding 81: with a colon only, CBE
# returned zero kits from 119 pages and the omission was invisible, because a book with no
# kits looks exactly like a book whose kits are elsewhere. Names are safe from the period
# form because CBE sets them in caps without one (`<B>BLADESINGER</B>`), and numbered list
# items are safe because the label must start with a letter.
LABEL = re.compile(r"<([IB])>\s*([A-Z][A-Za-z0-9 /'&,-]{2,40}):\s*</\1>", re.I)

# The Complete Book of Elves terminates its field labels with a PERIOD after a bullet —
# `<SPAN CLASS=Symbol>·</SPAN> <B>Role.</B>` — so the colon form above finds none of them
# and the book returned ZERO kits from 119 pages (ticket 13 finding 81). It is opt-in per
# book and kind rather than global, because the same book's SUBRACE page bolds spell names
# mid-sentence: applied there, `darkness.` becomes a field. One book, two conventions, and
# the bullet is what tells them apart.
LABEL_BULLET = re.compile(r"<SPAN CLASS=Symbol>\W{0,3}</SPAN></FONT>\s*<([IB])>\s*"
                          r"([A-Z][A-Za-z0-9 /'&,-]{2,40})\.\s*</\1>"
                          r"|<([IB])>\s*([A-Z][A-Za-z0-9 /'&,-]{2,40}):\s*</\3>", re.I)
LABELS = {"colon": LABEL, "bullet": LABEL_BULLET}
TAGS = re.compile(r"<[^>]+>")
WS = re.compile(r"\s+")

# --- the field layer, which is the only layer that differs between books -------------
#
# Ticket 13's finding 12: CBGH carries field labels on 3% of its pages where every
# sibling handbook carries them on 15-42%, because its fields are TYPOGRAPHIC — a label
# opens a paragraph, in plain text, with no markup at all. What survives there is the
# page layer: <TITLE> is correct and one record still occupies one titled page.
#
# So the repair is one pluggable layer, not a second program. Both strategies return the
# same thing — the prose before the first label, then ordered (label, value) pairs — and
# everything downstream (names, ids, provenance, records) has one code path.

PARA = re.compile(r"<P>\s*</P>|</?P>|<BR>", re.I)
# A typographic label: begins a paragraph, capitalised, at most four words, then a colon.
# Bounded at four because 'Recommended Nonweapon Proficiencies' is three and the longest
# real field in the book; unbounded, it starts eating sentences that happen to contain a
# colon.
TEXT_LABEL = re.compile(r"^([A-Z][A-Za-z'&/-]*(?:\s+[A-Za-z'&/-]+){0,3}):\s+(.+)$")
NAV = re.compile(r"\s*Table of Contents\s*$", re.I)


def fields_markup(raw, labels="colon"):
    """Twelve of the thirteen v1 books: <I>Label:</I> or <B>Label:</B>."""
    marks = list(LABELS[labels].finditer(raw))
    if not marks:
        return [clean(raw)], []
    out = []
    for i, m in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(raw)
        out.append((((m.group(2) or m.lastindex and m.group(4)) or "").strip(), clean(raw[m.end():end])))
    return [clean(raw[:marks[0].start()])], out


def fields_typographic(raw):
    """CBGH: the label opens a paragraph and carries no markup.

    Worked in paragraph chunks rather than by slicing the raw HTML between matches. The
    obvious alternative — one regex over the raw bytes, anchored on <P> — was tried and
    disagreed with a plain-text census by eleven occurrences, because arbitrary FONT/B/A
    tags interleave between the paragraph break and the first letter. The convention IS
    the paragraph, so the parser splits on paragraphs.
    """
    head, out = [], []
    for chunk in PARA.split(raw):
        t = NAV.sub("", clean(chunk))
        if not t:
            continue
        m = TEXT_LABEL.match(t)
        if m:
            out.append([m.group(1).strip(), m.group(2).strip()])
        elif out:
            out[-1][1] += " " + t   # a field's value running past its own paragraph
        else:
            head.append(t)
    return head, [(a, b) for a, b in out]


# --- the SECOND field level -----------------------------------------------------------
#
# Ticket 13's finding 35: in eight of nine kit books the force of a field — whether it
# binds or advises — is carried by a sub-label INSIDE the field, in plain text, in a small
# closed vocabulary: `Required:`, `Recommended:`, `Bonus Proficiencies:`. CTH and CWH mark
# 100% of their kits this way and CBGH marks none, which is why findings 22 and 24 read
# force as unmarked: they were taken from the one flat book.
#
# So the two strategies are per-LEVEL, not per-book. A markup book is markup at level one
# and typographic at level two. This is that second level, and it is deliberately the same
# shape of rule as `fields_typographic` — a capitalised short phrase, then a colon.
SUBLABEL = re.compile(r"(?:^|(?<=[.;])\s)([A-Z][A-Za-z'&/-]*(?:\s+[A-Za-z'&/-]+){0,2}):\s+")


def split_sublabels(value):
    """(sublabel, text) pairs within one field's value; sublabel is None for the lead-in.

    Returns a single (None, value) pair when the field has no sub-labels, so callers need
    no special case for the flat books.
    """
    marks = list(SUBLABEL.finditer(value))
    if not marks:
        return [(None, value)]
    out = []
    if marks[0].start():
        lead = value[:marks[0].start()].strip()
        if lead:
            out.append((None, lead))
    for i, m in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(value)
        out.append((m.group(1).strip(), value[m.end():end].strip()))
    return out


HEADING_WORDS = 5


def heading_name(head):
    """The record's own name, when the page is titled after its SECTION instead.

    CBGH gives the first kit of each class section the section's page — so DD04865 is
    titled 'Fighter Kits' and is the Breachgnome, and DD04917 is titled 'Fighter Kits'
    and is the Archer. Taking <TITLE> there produces two records called 'Fighter Kits'
    and loses both real names.

    The page is regular about it: chunk 0 is the title line, chunk 1 repeats it as a
    heading, and chunk 2 is EITHER the record's own heading or the first line of prose.
    A heading is short and does not end a sentence, which separates 'Breachgnome' and
    'The Archer' from 'These gnomes are considered eccentric...'. Returns None when the
    title is already the name.
    """
    if len(head) < 3:
        return None
    t = head[2]
    if len(t.split()) <= HEADING_WORDS and not t.endswith((".", "!", "?", ":")):
        return t
    return None


def richer_heading(head, title):
    """The printed heading, when it says more than <TITLE> does.

    Ticket 13 finding 57. On a record-titled page chunk 1 repeats the title — but not
    always verbatim: DD04872's <TITLE> is `Stalker` and its heading is
    `Stalker (Fighter/Thief)`, which is the kit's multiclass target. Taking <TITLE> there
    threw the qualifier away, while DD04871's `Buffoon (Thief/Illusionist)` kept it purely
    because that page is section-titled and so the name came from chunk 2 instead. Two
    multiclass kits in one book, named by two different rules.

    Only fires when the heading EXTENDS the title, so it cannot rename anything.
    """
    if len(head) < 2:
        return None
    h = head[1]
    return h if h != title and h.startswith(title) and len(h) > len(title) else None


STRATEGIES = {"markup": fields_markup, "typographic": fields_typographic}

# Per book, where it departs from the default. The marker lives here too: CBGH's subraces
# are delimited by 'Infravision', not by CBE's 'Additional Experience Cost' — the field
# vocabulary is a property of the BOOK, not of the kind, and assuming otherwise is what
# returned 0 records from 112 pages. So is packing: CBE puts five subraces on one page and
# CBGH gives each its own, so 'multi' is a book's habit too, not a kind's nature.
BOOKS = {
    "CBE":  {"kinds": {"kit": {"labels": "bullet"}}},
    "CBGH": {"strategy": "typographic",
             "kinds": {"subrace": {"marker": "Infravision", "multi": False}}},
}

# Per kind: the label that appears exactly once per record, and the target it attaches to.
# Ticket 07 measured kit and deity at 1:1 with files; subrace is the exception — five in one
# file — which is what exercises the '#n' ordinal.
KINDS = {
    "kit":     {"marker": "Role",                     "array": "kits",     "multi": False},
    "deity":   {"marker": "Duties of the Priest",     "array": "deities",  "multi": False},
    "subrace": {"marker": "Additional Experience Cost", "array": "subraces", "multi": True},
}

# Chapter apparatus wearing a record's clothes. A chapter's template page carries the same
# field labels as the records it describes — including the marker — so it parses perfectly
# and is not a record. Ticket 13's finding 1 found two by hand in CTH; finding 9 found a
# third in CPRH, in a different kind, which is what makes this a property of the WebHelp
# rather than one book's quirk.
#
# THIS LIST IS HUMAN-MAINTAINED, and that is a measured conclusion, not laziness. A
# detector keyed on the giveaway — an apparatus page *describes* its fields ("This
# paragraph describes the usual alignment...", "Ability score minimums are listed here")
# instead of filling them — finds 5 of the 12 pages below, across four books, with no false
# positives. It misses the other seven because CTH and CBGH write their templates as
# ordinary prose. So it is a good HINT GENERATOR for a book nobody has read yet, and never
# a gate: run it, read what it flags, and read the book anyway.
# Keyed by (book, name) and NOT by name alone. Ticket 13's finding 32 measured 10 kit
# names shared across two books each, so a bare name is not a safe key for dropping a
# record: an apparatus name in one book could be a real kit in another and would vanish
# silently. Twelve pages, nine books — 'Creating New Kits' alone appears in four.
# The mirror of EXCLUDE, and needed for the same reason. Finding 82: two records carry no
# `Role` field AT ALL — not unmarked, absent — and are otherwise complete kits. EXCLUDE drops
# apparatus that parses; INCLUDE keeps records that do not. Both are human-maintained because
# finding 1 established that record boundaries are not mechanical.
INCLUDE = {
    ("CBD", "DD04642"),   # Outcast
    ("CBE", "DD04787"),   # Spellfilcher
}

EXCLUDE = {
    ("CTH",  "Kits and Thief Types"),      # chapter preamble
    ("CTH",  "Creating New Kits"),
    ("CBD",  "Creating New Kits"),
    ("CFH",  "Creating New Kits"),
    ("CWH",  "Creating New Kits"),
    ("CBGH", "Structure of the Kits"),     # the gnome chapter's template
    ("CBGH", "The Structure of the Kits"),  # and the halfling chapter's, worded differently
    ("CFH",  "Kits and Warriors"),
    ("CWH",  "The Wizard Kits"),
    ("CWH",  "The Kits"),          # the chapter list; its Description is the Academician's
    ("CPAH", "Kit Subsections"),
    ("CRH",  "Kit Subsections"),
    ("CRH",  "List of Kits"),      # the chapter list; its Description is the Beastmaster's
    ("CBE",  "Elf PC Kits"),        # the chapter template
    ("CPRH", "Priesthoods"),               # the Designing Faiths template
}
# Five of these were found not by reading but by the name-collision check of finding 32:
# 'Kit Subsections' appearing in two books is what a template page looks like from the
# outside. The detector described above then confirmed them.


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


def parse(path: pathlib.Path, marker: str, strategy="markup", labels="colon"):
    raw = path.read_text(encoding="cp1252", errors="replace")
    t = TITLE.search(raw)
    if not t:
        return None
    title = clean(t.group(1))
    head, pairs = (fields_markup(raw, labels) if strategy == "markup"
                   else STRATEGIES[strategy](raw))
    parts = {label: split_sublabels(value) for label, value in pairs}
    # The marker test reads BOTH levels. Ticket 13 finding 82: the CPAH Equerry and the CRH
    # Warden print `Role:` in their text and the <I> markup fails on it, so a marker test
    # that looks only at top-level labels drops a whole record for one bad tag. Reading the
    # sub-label layer here is not a heuristic — it is the same label the book printed.
    if not (any(label == marker for label, _ in pairs)
            or any(sub == marker for ps in parts.values() for sub, _ in ps)
            or (path.parent.name, path.stem) in INCLUDE):
        return None
    fields = dict(pairs)

    # Title carries the book: "Bounty Hunter (Comp. Thief's Handbook)"
    name, _, book = title.partition("(")
    if strategy == "typographic":
        name = heading_name(head) or richer_heading(head, name.strip()) or name
    return {
        "name": name.strip(),
        "book": book.rstrip(")").strip(),
        "file": path.name,
        "fields": fields,
        "parts": parts,
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
    profile = BOOKS.get(book, {})
    strategy = profile.get("strategy", "markup")
    spec = {**KINDS[kind], **profile.get("kinds", {}).get(kind, {})}
    target = f"{pack_id}:target"

    records, parsed_all, skipped, excluded = [], [], 0, 0
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
                if (book, rec["name"]) in EXCLUDE:
                    excluded += 1
                    continue
                rec["file"] = f.name
                parsed_all.append(rec)
                records.append(to_record(rec, pack_id, target, ordinal=n))
        else:
            p = parse(f, spec["marker"], strategy, spec.get("labels", "colon"))
            if p is None:
                skipped += 1
                continue
            if (book, p["name"]) in EXCLUDE:
                excluded += 1
                continue
            parsed_all.append(p)
            records.append(to_record(p, pack_id, target))

    if "--json" in sys.argv:
        json.dump({spec["array"]: records}, sys.stdout, indent=2, ensure_ascii=False)
        return 0

    print(f"{book}: {len(records)} records, {skipped} pages skipped, "
          f"{excluded} apparatus pages excluded by hand")
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
