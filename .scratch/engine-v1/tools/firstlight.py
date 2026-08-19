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
        self.vocabulary = set()
        for name in self.manifest["files"]:
            f = self.root / name
            if not f.exists():
                self.complain("manifest", f"declares {name}, which is not there")
                continue
            for kind, recs in json.loads(f.read_text()).items():
                if not isinstance(recs, list):
                    continue
                # Correction 58's `fields` array declares the vocabulary the effects write.
                # It is not a kind and its entries have no id — the first thing in the pack
                # that is an array and not a list of records, which is worth knowing before
                # the Engine assumes otherwise anywhere else.
                if kind == "fields":
                    self.vocabulary = {x["path"] for x in recs}
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
    def __init__(self, pack, race, klass, kit, level, scores, subrace=None, options=False):
        self.pack, self.level, self.scores = pack, level, scores
        # §5.5's catalogue: the rules this campaign plays. A Character records them, because a
        # sheet built under weapon specialization is not the same sheet without it.
        self.options = set(options or ())
        self.race = pack.get(race, "character race")
        self.subrace = pack.get(subrace, "character subrace") if subrace else None
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
        # Ticket 03's decision. Precedence is never inferred from which ARM a layer is;
        # it is either DECLARED by one record about another (a subrace names its race as
        # `target`, an `except` names the limitation it pierces) or it does not exist, and
        # a value nothing declares over is withheld with both books named.
        self.contested = []            # (field, [(rid, value)]) — refused, not guessed
        # Correction 57. An effect the BOOK marks optional is fully modelled; what is unknown is
        # whether this table plays it. Withheld until a campaign says so, and reported — the same
        # channel ticket 02 built, for a fourth reason.
        self.byOption = []             # (field, op, value, src, text)
        # Ticket 05's prototype. A BOUND is a `forbid` pointing at a limitation that
        # carries `members`: the permitted set of that kind is narrowed to those. Bounds
        # INTERSECT, explicit forbids SUBTRACT, and `except` drops a bound by name — all
        # three commute, which is why this needs no seventh operation and does not
        # threaten §4.3's guarantee.
        self.bounds = collections.defaultdict(list)   # kind -> [(src, limitation-id, frozenset)]
        self.lifted = set()                           # limitation ids an `except` removed
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
            # Correction 45: a field, not a string search. Searching the text for the word
            # misread any note that DISCUSSED a marker as being one — a false positive this
            # project introduced twice, both times while repairing markers.
            marked = bool(e.get("unmodelled"))
            if e.get("optional") and e["optional"] not in self.options:
                self.byOption.append((e.get("field"), e["op"],
                                      e.get("by", e.get("to", e.get("ref"))), src,
                                      e.get("text") or "", e["optional"]))
                continue
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
                lim = self.pack.by_id.get(e.get("ref") or "")
                if lim and lim.get("members"):
                    self.bounds[e["kind"]].append((src, lim["id"], frozenset(lim["members"])))
                else:
                    self.forbidden.append((src, e.get("ref") or
                                           f"«{e['defines']['name']}»"))
            elif op == "except":
                self.excepted.append((src, e["ref"]))
                self.lifted.add(e["ref"])
            elif op == "require":
                self.required.append((src, e["kind"], e["count"], e.get("from")))
            if marked and op not in ("adjust", "set"):
                self.riders[src] = e["text"]
            if e.get("ref"):
                self.pack.get(e["ref"], src)

    def impose(self):
        """Correction 48, live. A limitation names the class that imposes it (`imposedBy`) and,
        where the rule bounds a set, carries the members. Nothing has to `forbid` it: being that
        class IS the imposition, which is why the pack has thirteen `except`s and no matching
        `forbid`. An `except` still lifts it by name."""
        mine = {self.klass["id"]} if self.klass else set()
        if self.klass and self.klass.get("group"):
            mine.add(self.klass["group"])
        # NOT `combines`. The PHB states the multi-class rule per class and it is not a uniform
        # intersection: a multi-classed warrior uses everything without restriction, a multi-classed
        # PRIEST keeps his mythos weapons ("a fighter/cleric can use only bludgeoning weapons"), and
        # the thief's restriction is about armour and thieving skills rather than weapons. Composing
        # bounds across the arms of a multi-class would bind a fighter/thief to the thief's twelve
        # weapons, which the book plainly does not. §6.2 says the Engine owns the combination rules;
        # it does not own them yet, so this declines rather than guesses.
        if self.klass and self.klass.get("combines"):
            self.notes.append(f"{self.klass['id']} combines "
                              f"{', '.join(self.klass['combines'])}; the PHB's multi-class rule is "
                              f"stated PER CLASS and is Engine knowledge (§6.2) that does not exist "
                              f"yet, so no class bound is imposed here.")
            return
        for lim in self.pack.by_kind["limitations"]:
            if lim.get("imposedBy") in mine and lim.get("members"):
                kind = "weaponProficiency" if "weapon" in lim["id"] else "armor"
                self.bounds[kind].append((lim["id"], lim["id"], frozenset(lim["members"])))

    @staticmethod
    def commensurate(values):
        """Correction 47. `3- on 1d6` and `50` are the same probability, and the pack recorded a
        contradiction between two books that agree. A rollUnder denotes `n / X`; a bare integer
        on a field another layer writes as a rollUnder is a percentage. The co-occurrence is
        itself the evidence that the field is a probability, so this needs no vocabulary."""
        rolls = [v for v in values if isinstance(v, str) and "- on 1d" in v]
        if not rolls or len(rolls) == len(values):
            return None
        out = set()
        for v in values:
            if isinstance(v, str) and "- on 1d" in v:
                n, _, die = v.partition("- on 1d")
                out.add(round(int(n) / int(die) * 100, 4))
            elif isinstance(v, int):
                out.add(float(v))
            else:
                return None
        return out

    def refines(self, a, b):
        """Does record `a` declare itself a refinement of record `b`? The only such
        declaration in the pack is `target`: a subrace names its race, a kit names its
        class. It is a fact IN the record, never an ordering the Engine supplies."""
        ra, rb = self.pack.by_id.get(a), self.pack.by_id.get(b)
        return bool(ra and rb and ra.get("target") == rb["id"])

    def view(self, path):
        f = self.fields.get(path)
        if not f:
            return None
        if f["set"]:
            layers = [(v, src.rsplit("[", 1)[0], src) for v, src in f["set"]]
            distinct = {str(v) for v, _, _ in layers}
            same = self.commensurate([v for v, _, _ in layers])
            if same is not None and len(same) == 1:
                self.notes.append(f"{path}: two books write this in different notations and they "
                                  f"AGREE — " + " = ".join(sorted(distinct)))
                distinct = {"agreed"}
            if len(distinct) > 1:
                winners = [x for x in layers
                           if all(x[1] == y[1] or self.refines(x[1], y[1]) for y in layers)]
                if len(winners) != 1:
                    # Nothing in the pack says which of these outranks the other, so the
                    # Engine does not pick. §5.3 quarantines a character; this quarantines
                    # one VALUE, which is the same posture at a finer grain.
                    self.contested.append((path, [(x[2], x[0]) for x in layers]))
                    return None
                v = winners[0][0]
            else:
                v = f["set"][-1][0]
            return v if not isinstance(v, int) else v + f["adjust"]
        return f["adjust"]

    def damage(self, weapon):
        """Correction 53. Eleven of Table 44's weapons carry no damage, because in 2e the
        LAUNCHER has the speed factor and the AMMUNITION has the damage. Returns
        (damage, the record that supplied it) — or (None, None) where the book never said,
        which is the short bows."""
        r = self.pack.by_id.get(weapon) or {}
        if r.get("damageSmallMedium"):
            return r["damageSmallMedium"], weapon
        for a in r.get("ammunition") or []:
            am = self.pack.by_id.get(a) or {}
            if am.get("damageSmallMedium"):
                return am["damageSmallMedium"], a
        # No damage and no ammunition is TWO different facts and the columns do not tell them
        # apart — Table 44 prints a dash for both. A weapon whose GROUP-MATES carry ammunition
        # is a launcher the book never gave one (the short bows); a standalone weapon with no
        # damage does none by design (mancatcher, lasso, net).
        for g in self.pack.by_kind["weaponProficiencies"]:
            # A table HEADING only. Correction 54 made that sayable: before it, a heading and
            # one of the Complete Fighter's priced groups were both `isGroup` with `members`,
            # and this test had to infer the difference from `groupKind` being ABSENT. The
            # group "Weapons Not Belonging To Any Group" put the blowgun beside the lasso and
            # made the lasso look like a launcher.
            if g.get("groupKind") != "heading" or weapon not in (g.get("members") or []):
                continue
            if any((self.pack.by_id.get(m) or {}).get("ammunition") for m in g["members"]):
                self.pack.complain("ammunition",
                                   f"{weapon} is a launcher and the book never names what it "
                                   f"fires; nothing can compute its damage")
                return None, None
        return None, "does none"

    def reach(self, weapon):
        """Correction 50. Sixteen kit bounds are stated over 'a melee weapon' or 'a missile
        weapon', and the books answer it in the RANGE tables rather than the weapon tables.
        Three-way, not two — the Complete Fighter's Giant Killer says 'missile or hurled'."""
        r = self.pack.by_id.get(weapon) or {}
        if not r.get("range"):
            return "melee"
        # Within the weapons usable at a distance, a LAUNCHER carries no damage of its own —
        # correction 53's split — and a HURLED weapon carries its own. `ammunition` is not the
        # test: correction 46 left the short bows without one because the book never names
        # their arrow, and they are launchers all the same.
        dmg, src = self.damage(weapon)
        if src == "does none":
            return "hurled"          # the lasso and the net: thrown, and they entangle
        if r.get("damageSmallMedium"):
            return "hurled"
        return "launcher"

    def expand(self, ids):
        """A member may itself be a group — Table 44 nests Bastard sword under Sword, and the
        Complete Fighter's nests Katana under Sword under nothing. Correction 49: a bound that
        names `phb:sword` means the seven swords, and before the groups had members it meant
        nothing at all."""
        out, seen = set(), set()
        stack = list(ids)
        while stack:
            i = stack.pop()
            if i in seen:
                continue
            seen.add(i)
            r = self.pack.by_id.get(i) or {}
            if r.get("groupKind"):
                if not r.get("members"):
                    self.pack.complain("bound",
                                       f"{i} is a group with no members: a bound naming it "
                                       f"permits nothing")
                stack.extend(r.get("members") or [])
            else:
                out.add(i)
        return out

    def permitted(self, kind, universe):
        """(intersection of every bound still standing) minus (everything forbidden).
        Set intersection and union commute, so the layers may be applied in any order —
        the same guarantee §4.3 makes for the six operations, inherited rather than
        re-argued."""
        live = [b for b in self.bounds.get(kind, []) if b[1] not in self.lifted]
        allowed = set(universe)
        for _, _, members in live:
            allowed &= self.expand(members)
        allowed -= {ref for _, ref in self.forbidden}
        return allowed, live

    @property
    def alignment(self):
        return None                     # the sheet has no alignment yet: see the report


# ---------------------------------------------------------------- report
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pack")
    ap.add_argument("--race", default="phb:dwarf")
    ap.add_argument("--subrace", default=None)
    ap.add_argument("--klass", "--class", dest="klass", default="phb:fighter")
    ap.add_argument("--kit", default="cbd:DD04638")
    ap.add_argument("--deity", default=None)
    ap.add_argument("--level", type=int, default=1)
    ap.add_argument("--optional-rules", default="",
                    help="comma-separated names from §5.5's catalogue, e.g. "
                         "phb:weapon-specialization")
    a = ap.parse_args()

    pack = Pack(a.pack)
    print(f"pack: {len(pack.by_id)} records, {len(pack.by_kind)} kinds\n")

    scores = {"strength": 16, "dexterity": 12, "constitution": 15,
              "intelligence": 10, "wisdom": 11, "charisma": 9}
    c = Character(pack, a.race, a.klass, a.kit, a.level, scores, a.subrace,
                  [x for x in a.optional_rules.split(",") if x])

    # A character IS a race and a class, and MAY carry an attachable. The group a
    # class belongs to is a layer too — the corpus put the hit die there.
    c.apply(c.race, "race")
    c.apply(c.subrace, "subrace")
    if c.klass and c.klass.get("group"):
        c.apply(pack.get(c.klass["group"], "class group"), "class group")
    c.apply(c.klass, "class")
    # A Deity is an Attachable like a Kit, and engine ticket 03 exists because the two
    # can write the same field. Applied AFTER the kit only to have an order at all —
    # which is itself the thing that ticket has to settle.
    if a.deity:
        c.apply(pack.get(a.deity, "character deity"), "deity")
    c.apply(c.kit, "kit")
    c.impose()

    who = " / ".join(x["name"] for x in (c.race, c.subrace, c.klass, c.kit) if x)
    print(f"CHARACTER: {who}, level {a.level}")
    print("  " + ", ".join(f"{k[:3].title()} {v}" for k, v in scores.items()))

    print("\nFIELDS — the total, and nothing in it is approximate")
    for path in sorted(c.fields):
        v = c.view(path)
        if v is not None:
            print(f"    {path:<44}{v}")

    if c.contested:
        print(f"\nCONTESTED — two layers set this and nothing declares a winner "
              f"({len(c.contested)})")
        for path, entries in c.contested:
            print(f"    {path}")
            for src, val in entries:
                r = pack.by_id.get(src.rsplit("[", 1)[0], {})
                bk = (r.get("provenance", {}).get("section") or ["?"])[0]
                print(f"        {str(val):<24}{r.get('name', '?'):<20}{bk}   {src}")

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

    if c.bounds:
        print("\nPERMITTED — bounds intersect, forbids subtract, `except` lifts")
        universe = {"weaponProficiency": [r["id"] for r in pack.by_kind["weaponProficiencies"]
                                          if not r.get("groupKind")],
                    "armor": [r["id"] for r in pack.by_kind["armor"]],
                    "sphere": [r["id"] for r in pack.by_kind["spheres"]]}
        for kind in sorted(c.bounds):
            allowed, live = c.permitted(kind, universe.get(kind, []))
            print(f"    {kind}: {len(allowed)} of {len(universe.get(kind, []))}")
            for src, lim, members in live:
                r = pack.by_id.get(lim, {})
                print(f"        bound by {r.get('name', lim)} ({len(members)})   {src}")
            for src, lim in c.excepted:
                if lim in {b[1] for b in c.bounds.get(kind, [])}:
                    print(f"        LIFTED   {pack.by_id.get(lim, {}).get('name', lim)}   {src}")
            print("        " + ", ".join(sorted(pack.by_id.get(i, {}).get("name", i)
                                                for i in allowed)[:12]))

    if c.byOption:
        print(f"\nBY CAMPAIGN OPTION — §5.5, and this campaign plays "
              f"{', '.join(sorted(c.options)) or 'none of them'} ({len(c.byOption)} withheld)")
        for path, op, val, src, why, opt in c.byOption:
            print(f"    {op} {str(path or val):<28}{opt:<34}{src}")
            print(f"        {why[:150]}")

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
