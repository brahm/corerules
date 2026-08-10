#!/usr/bin/env python3
"""Label census over the converted RTF. Counts LINES whose first non-blank
content is `Label: ` — the shape TSR uses for record fields."""
import re, sys, collections, pathlib

RTF = pathlib.Path(sys.argv[1])
LABEL = re.compile(r'^[ \t]*([A-Z][A-Za-z0-9 /\'&,-]{2,36}):[ \t]')

BOOKS = ["phbbk", "dmgbk", "bardbk", "druidbk", "dwarfbk", "elfbk", "fightrbk",
         "gnmhlfbk", "paladnbk", "priestbk", "rangerbk", "thiefbk", "wizardbk"]

per_book = {}
overall = collections.Counter()
for b in BOOKS:
    p = RTF / f"{b}.txt"
    c = collections.Counter()
    for line in p.open(encoding="utf-8", errors="replace"):
        m = LABEL.match(line)
        if m:
            c[m.group(1).strip()] += 1
    per_book[b] = c
    overall.update(c)

if sys.argv[2:] and sys.argv[2] == "global":
    print(f"{'label':<34}{'total':>7}  books")
    for lab, n in overall.most_common(int(sys.argv[3]) if sys.argv[3:] else 40):
        bks = sum(1 for b in BOOKS if per_book[b][lab])
        print(f"{lab:<34}{n:>7}  {bks}")
else:
    labels = sys.argv[2:]
    print(f"{'book':<10}" + "".join(f"{l[:13]:>15}" for l in labels))
    for b in BOOKS:
        print(f"{b:<10}" + "".join(f"{per_book[b][l]:>15}" for l in labels))
    print(f"{'TOTAL':<10}" + "".join(f"{overall[l]:>15}" for l in labels))
