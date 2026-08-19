#!/usr/bin/env python3
"""First light: load the pack and compute one character (engine ticket 04).

Not the Engine. A throwaway that answers one question the corpus map could not:
what happens when a program actually consumes this pack.

The prediction, recorded before running it: the gaps will NOT be in the six
operations — those were measured over 1,910 effects — but in the JOINS between
kinds, which nothing has ever traversed.

Everything it stumbles on is printed rather than repaired. The corpus map's own
method was that a finding recorded is worth more than a defect quietly fixed.

Usage:  firstlight.py <pack-dir> [--race phb:dwarf] [--class phb:fighter]
                      [--kit cbd:DD04638] [--level 1]
"""
import sys, json, pathlib, collections, argparse


# ---------------------------------------------------------------- the pack
class Pack:
    def __init__(self, root):
        self.root = pathlib.Path(root)
        self.manifest = json.loads((self.root / "manifest.json").read_text())
        self.by_kind = collections.defaultdict(list)
        self.by_id = {}
        self.complaints = []
        for name in self.manifest["files"]:
            f = self.root / name
            if not f.exists():
                self.complain("manifest", f"declares {name}, which is not there")
                continue
            for kind, recs in json.loads(f.read_text()).items():
                if not isinstance(recs, list):
                    continue
                for r in recs:
                    if r["id"] in self.by_id:
                        self.complain("identity", f"{r['id']} defined twice")
                    self.by_id[r["id"]] = r
                    self.by_kind[kind].append(r)

    def complain(self, area, msg):
        self.complaints.append((area, msg))

    def get(self, i, why):
        r = self.by_id.get(i)
        if r is None:
            self.complain("reference", f"{why} points at {i}, which is not in the pack")
        return r

    # Written expecting to need it, and never used: the pack keys arrays by KIND NAME
    # (`races`, `grantedAbilities`) and effects name the CONCEPT (`ability`, `stronghold`),
    # so a consumer looks like it must map one to the other. It does not, because ids are
    # GLOBALLY scoped — `by_id[ref]` finds the record whatever array it lives in. Kept as
    # the record of a join that turned out to be free, and note that the same flat namespace
    # is exactly what let eight ids collide.


# ---------------------------------------------------------------- character
class Character:
    def __init__(self, pack, race, klass, kit, level, scores):
        self.pack, self.level, self.scores = pack, level, scores
        self.race = pack.get(race, "character race")
        self.klass = pack.get(klass, "character class")
        self.kit = pack.get(kit, "character kit") if kit else None
        self.fields = collections.defaultdict(lambda: {"base": None, "adjust": 0, "set": [],
                                                       "from": []})
        self.granted, self.forbidden, self.excepted, self.required = [], [], [], []
        # Ticket 02's decision, applied. A marked effect does not reach the total.
        # On a STRUCTURAL op the thing is right and its edges are under-described, so it
        # is applied and the marker rides along on that entry. On a NUMERIC op the number
        # is right and the circumstance is missing, so it never joins the sum — it becomes
        # a named situational line the player applies when the circumstance holds.
        self.riders = {}               # src -> marker text, on an applied entry
        self.situational = []          # (field, op, value, src, text) — NOT in the total
        self.notes = []

    # -- predicate ------------------------------------------------------
    def scalar(self, s):
        """§6.1's scalar, all three arms."""
        if "ability" in s:
            return self.scores.get(s["ability"].split(":")[1])
        if "level" in s:
            return self.level if s["level"] in (self.klass or {}).get("id") else 0
        if "field" in s:
            path = s["field"]
            if path == "race":
                return self.race["id"] if self.race else None
            if path == "alignment":
                return self.alignment
            if path in ("subrace", "druidBranch"):
                return None
            return self.view(path)
        raise ValueError(s)

    def holds(self, cond):
        if "anyOf" in cond:
            return any(self.holds(c) for c in cond["anyOf"])
        if "member" in cond:
            v = self.scalar(cond["member"])
            return v in cond["anyOfIds"]
        if "has" in cond:
            return cond["ref"] in [r for _, r in self.granted]
        v = self.scalar(cond["subject"])
        w = cond["value"]
        if not isinstance(w, int) or v is None:
            return None                 # undecidable, not false
        return {"gte": v >= w, "lte": v <= w, "eq": v == w, "neq": v != w}[cond["op"]]

    def when(self, clauses):
        """None means UNDECIDABLE — the pack asks about something the sheet has no
        answer for. That is not the same as False and must not be silently treated
        as one."""
        if not clauses:
            return True
        out = [self.holds(c) for c in clauses]
        if any(r is None for r in out):
            return None
        return all(out)

    # -- operands -------------------------------------------------------
    def operand(self, v, source):
        if isinstance(v, int):
            return v
        if isinstance(v, str):
            return v                    # a die, kept as printed
        if "rollAtMost" in v:
            return f"{v['rollAtMost']}- on {v['on']}"
        if "supplies" in v:
            return self.table(v, source)
        if "of" in v:
            base = self.scalar(v["of"])
            if base is None:
                self.pack.complain("operand",
                                   f"{source}: computed operand reads {v['of']}, which the "
                                   f"sheet has no value for")
                return None
            n = base * v.get("multiplyBy", 1) / v.get("divideBy", 1)
            return int(n) if v["round"] == "down" else -int(-n)
        raise ValueError(v)

    def table(self, v, source):
        t = None
        if v.get("of"):
            owner = self.pack.get(v["of"], f"{source} table owner")
            t = next((x for x in self.pack.by_kind["lookupTables"]
                      if x.get("supplies") == v["supplies"]), None)
        else:
            t = next((x for x in self.pack.by_kind["lookupTables"]
                      if x.get("supplies") == v["supplies"]), None)
        if t is None:
            self.pack.complain("table",
                               f"{source}: no table supplies {v['supplies']!r}")
            return None
        key = self.scalar(v["at"])
        for row in t["rows"]:
            if self.row_matches(row[0], key):
                return row[1]
        self.pack.complain("table",
                           f"{source}: {t['id']} has no row for {key!r}")
        return None

    @staticmethod
    def row_matches(cell, key):
        if isinstance(key, int):
            if "-" in str(cell):
                lo, _, hi = str(cell).partition("-")
                try:
                    return int(lo) <= key <= int(hi)
                except ValueError:
                    return False
            try:
                return int(cell) == key
            except ValueError:
                return False
        return cell == key

    # -- the layer stack ------------------------------------------------
    def apply(self, record, why):
        if record is None:
            return
        if record.get("effectsModelled") is False:
            self.notes.append(f"{record['id']} ({record['name']}) says its effects are not "
                              f"transcribed — nothing to apply.")
        for i, e in enumerate(record.get("effects", [])):
            src = f"{record['id']}[{i}]"
            marked = "UNMODELLED" in (e.get("text") or "")
            fires = self.when(e.get("when"))
            if fires is None:
                self.pack.complain("predicate",
                                   f"{src}: `when` asks about something the sheet cannot "
                                   f"answer; effect skipped")
                continue
            if fires is False:
                continue
            op = e["op"]
            if op in ("adjust", "set"):
                val = self.operand(e.get("by") if op == "adjust" else e["to"], src)
                if marked:
                    # Withheld from the total, kept in full beside it. This is what stops
                    # the 36 priesthoods paying +10% where the book says +5%: the unmarked
                    # conditional sibling stays in the sum, the marked twin stands apart.
                    self.situational.append((e["field"], op, val, src, e["text"]))
                    continue
                f = self.fields[e["field"]]
                if op == "adjust":
                    if isinstance(val, int):
                        f["adjust"] += val
                    else:
                        f["set"].append((val, src))
                else:
                    f["set"].append((val, src))
                f["from"].append(src)
            elif op == "grant":
                self.granted.append((src, e.get("ref") or
                                     f"«{e['defines']['name']}»"))
            elif op == "forbid":
                self.forbidden.append((src, e.get("ref") or
                                       f"«{e['defines']['name']}»"))
            elif op == "except":
                self.excepted.append((src, e["ref"]))
            elif op == "require":
                self.required.append((src, e["kind"], e["count"], e.get("from")))
            if marked and op not in ("adjust", "set"):
                self.riders[src] = e["text"]
            if e.get("ref"):
                self.pack.get(e["ref"], src)

    def view(self, path):
        f = self.fields.get(path)
        if not f:
            return None
        if f["set"]:
            vals = {v for v, _ in f["set"]}
            if len(vals) > 1:
                self.pack.complain("layers",
                                   f"{path}: two `set` layers disagree — "
                                   + "; ".join(f"{v} from {s}" for v, s in f["set"]))
            v = f["set"][-1][0]
            return v if not isinstance(v, int) else v + f["adjust"]
        return f["adjust"]

    @property
    def alignment(self):
        return None                     # the sheet has no alignment yet: see the report


# ---------------------------------------------------------------- report
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pack")
    ap.add_argument("--race", default="phb:dwarf")
    ap.add_argument("--klass", "--class", dest="klass", default="phb:fighter")
    ap.add_argument("--kit", default="cbd:DD04638")
    ap.add_argument("--deity", default=None)
    ap.add_argument("--level", type=int, default=1)
    a = ap.parse_args()

    pack = Pack(a.pack)
    print(f"pack: {len(pack.by_id)} records, {len(pack.by_kind)} kinds\n")

    scores = {"strength": 16, "dexterity": 12, "constitution": 15,
              "intelligence": 10, "wisdom": 11, "charisma": 9}
    c = Character(pack, a.race, a.klass, a.kit, a.level, scores)

    # A character IS a race and a class, and MAY carry an attachable. The group a
    # class belongs to is a layer too — the corpus put the hit die there.
    c.apply(c.race, "race")
    if c.klass and c.klass.get("group"):
        c.apply(pack.get(c.klass["group"], "class group"), "class group")
    c.apply(c.klass, "class")
    # A Deity is an Attachable like a Kit, and engine ticket 03 exists because the two
    # can write the same field. Applied AFTER the kit only to have an order at all —
    # which is itself the thing that ticket has to settle.
    if a.deity:
        c.apply(pack.get(a.deity, "character deity"), "deity")
    c.apply(c.kit, "kit")

    who = " / ".join(x["name"] for x in (c.race, c.klass, c.kit) if x)
    print(f"CHARACTER: {who}, level {a.level}")
    print("  " + ", ".join(f"{k[:3].title()} {v}" for k, v in scores.items()))

    print("\nFIELDS — the total, and nothing in it is approximate")
    for path in sorted(c.fields):
        print(f"    {path:<44}{c.view(path)}")

    print(f"\nGRANTED ({len(c.granted)})")
    for src, ref in c.granted:
        r = pack.by_id.get(ref)
        print(f"    {(r['name'] if r else ref):<40}{src}")
        if src in c.riders:
            print(f"        \u2937 {c.riders[src]}")
    if c.forbidden:
        print(f"\nFORBIDDEN ({len(c.forbidden)})")
        for src, ref in c.forbidden:
            r = pack.by_id.get(ref)
            print(f"    {(r['name'] if r else ref):<40}{src}")
            if src in c.riders:
                print(f"        \u2937 {c.riders[src]}")
    if c.required:
        print(f"\nCHOICES OWED ({len(c.required)})")
        for src, kind, n, frm in c.required:
            print(f"    {n} x {kind:<34}{'from ' + str(len(frm)) + ' options' if frm else 'unbounded'}"
                  f"   {src}")

    if c.situational:
        print(f"\nSITUATIONAL — withheld from the total, applied when the circumstance "
              f"holds ({len(c.situational)})")
        for path, op, val, src, why in c.situational:
            shown = f"{val:+d}" if isinstance(val, int) else str(val)
            print(f"    {path:<30}{shown:<12}{src}")
            print(f"        {why}")

    if c.notes:
        print("\nNOTES")
        for n in c.notes:
            print(f"    {n}")

    if pack.complaints:
        print(f"\nWHAT THE PACK DID NOT SAY ({len(pack.complaints)})")
        by = collections.defaultdict(list)
        for area, msg in pack.complaints:
            by[area].append(msg)
        for area in sorted(by):
            print(f"  [{area}] {len(by[area])}")
            for m in sorted(set(by[area]))[:12]:
                print(f"    {m}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
