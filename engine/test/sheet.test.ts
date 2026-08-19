import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Pack } from "../src/pack.ts";
import { Sheet } from "../src/sheet.ts";

const here = dirname(fileURLToPath(import.meta.url));
const pack = new Pack(join(here, "fixtures", "minimal"));

const hillfolk = (opts: { subrace?: boolean; options?: string[] } = {}) => {
  const c = new Sheet(pack, { options: opts.options ?? [] });
  c.apply(pack.byId.get("test:hillfolk"), "race");
  if (opts.subrace === true) c.apply(pack.byId.get("test:highlander"), "subrace");
  c.apply(pack.byId.get("test:warrior"), "class group");
  c.apply(pack.byId.get("test:fighter"), "class");
  return c;
};

test("a value carries the layers that made it, and each names its book", () => {
  const v = hillfolk().view("attackRoll");
  assert.equal(v.value, 1);
  assert.equal(v.from.length, 1);
  assert.equal(v.from[0]!.source.record, "test:fighter");
  assert.equal(v.from[0]!.source.book, "Fixtures");
});

test("a marked numeric effect never reaches the total, and is kept beside it", () => {
  const c = hillfolk();
  assert.equal(c.view("reactionCheck").value, undefined);
  const aside = c.aside.filter((a) => a.because === "marked");
  assert.equal(aside.length, 1);
  assert.match(aside[0]!.text, /only with other hillfolk/);
});

test("a marked STRUCTURAL effect is applied, with the marker riding on the entry", () => {
  const c = hillfolk();
  const inline = c.granted.find((g) => g.defines !== undefined);
  assert.equal(inline?.defines, "A thing that exists nowhere else");
  assert.match(inline!.rider!, /UNMODELLED SHAPE/);
});

test("an undecidable predicate sets the effect aside — it is not treated as false", () => {
  const c = hillfolk();
  assert.equal(c.aside.filter((a) => a.because === "undecidable").length, 1);
});

test("an optional rule is withheld until the campaign plays it, and named when it is", () => {
  const off = hillfolk();
  assert.equal(off.view("experienceAward.percent").value, undefined);
  const a = off.aside.find((x) => x.because === "option");
  assert.equal(a?.because === "option" ? a.option : undefined, "cprh:split-prime-requisite-bonus");

  const on = hillfolk({ options: ["cprh:split-prime-requisite-bonus"] });
  assert.equal(on.view("experienceAward.percent").value, 5);
  assert.equal(on.aside.filter((x) => x.because === "option").length, 0);
});

test("a subrace refines its race, because its record declares the race as its target", () => {
  assert.equal(hillfolk().view("infravision.range").value, 60);
  assert.equal(hillfolk({ subrace: true }).view("infravision.range").value, 90);
});

test("two notations for one probability are an agreement, not a contradiction", () => {
  const c = hillfolk({ subrace: true });
  assert.equal(c.view("detect.slope").value, 50);
  assert.ok(c.notes.some((n) => /AGREE/.test(n)), "the sheet says the two books agree");
  assert.equal(c.view("detect.slope").contested, undefined);
});

test("two layers that set one value with nothing declaring over them contest it", () => {
  const c = new Sheet(pack);
  c.apply(pack.byId.get("test:hillfolk"), "race");
  // A kit targets a CLASS, so neither record declares anything about the other.
  c.apply({ id: "test:stranger", name: "Stranger", target: "test:fighter",
            provenance: { section: ["Elsewhere"] },
            effects: [{ op: "set", field: "infravision.range", to: 15 }] }, "kit");
  const v = c.view("infravision.range");
  assert.equal(v.value, undefined);
  assert.equal(v.contested?.length, 2);
  assert.deepEqual(v.contested!.map((x) => x.source.book).sort(), ["Elsewhere", "Fixtures"]);
});

test("a require becomes a choice the character owes", () => {
  const owed = hillfolk().owed;
  assert.equal(owed.length, 1);
  assert.equal(owed[0]!.kind, "language");
  assert.equal(owed[0]!.count, 2);
});

test("a table read finds its table by the field path the table says it supplies", () => {
  const c = new Sheet(pack, { scores: { "test:dexterity": 12 } });
  c.apply(pack.byId.get("test:hillfolk"), "race");
  assert.equal(c.view("surefooting").value, 1);
});

test("an adjust that does not resolve to a number is set aside, never summed as zero", () => {
  const c = new Sheet(pack, { scores: { "test:dexterity": 12 } });
  c.apply(pack.byId.get("test:hillfolk"), "race");
  // The table prints its value the way the book does. That is a real mismatch with the
  // contract for an `adjust`, and reporting it is the whole point: a field the Engine
  // cannot compute must not come back as 0.
  assert.equal(c.view("asPrinted").value, undefined);
  const a = c.aside.find((x) => x.because === "unresolved");
  assert.match(a!.text, /resolved to "\+4", which cannot be summed/);
});

test("a limitation imposes itself on the class it names, and an except lifts it", () => {
  const bound = new Sheet(pack);
  bound.apply(pack.byId.get("test:fighter"), "class");
  const weapons = pack.records("weaponProficiencies").filter((r) => r.groupKind === undefined).map((r) => r.id);
  const a = bound.permitted("weaponProficiency", weapons);
  assert.deepEqual([...a.allowed].sort(), ["test:greatsword", "test:sabre", "test:short-blade"]);
  assert.equal(a.bounds.length, 1);

  const free = new Sheet(pack);
  free.apply(pack.byId.get("test:fighter"), "class");
  free.apply({ id: "test:duellist", name: "Duellist", target: "test:fighter",
               provenance: { section: ["Fixtures"] },
               effects: [{ op: "except", kind: "weaponProficiency", ref: "test:fighter-weapon-restriction" }] }, "kit");
  assert.equal(free.permitted("weaponProficiency", weapons).allowed.size, weapons.length);
});
