import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Character } from "../src/character.ts";
import { Pack } from "../src/pack.ts";
import { budget, candidates, cost, openGroups } from "../src/proficiency.ts";

const pack = new Pack(join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal"));
const fighter = () => {
  const c = Character.create(pack, { name: "Someone", race: "test:hillfolk", scores: {} });
  c.advance([{ class: "test:fighter", die: 8 }]);
  return c;
};

test("Table 34 gives the budget, and the penalty travels with the weapon half", () => {
  const c = fighter();
  assert.deepEqual(budget(pack, c, "weapon"), {
    initial: 4, everyLevels: 3, total: 4, spent: 0, free: 4, penalty: -2,
  });
  assert.deepEqual(budget(pack, c, "nonweapon"), {
    initial: 3, everyLevels: 3, total: 3, spent: 0, free: 3,
  });
});

test("slots accrue with level, one every #Levels", () => {
  const c = fighter();
  for (const die of [5, 5, 5]) c.advance([{ class: "test:fighter", die }]);   // level 4
  assert.equal(budget(pack, c, "nonweapon")!.total, 4);
});

test("a character's own group and General are open; Table 38 adds the crossovers", () => {
  assert.deepEqual([...openGroups(pack, "test:fighter").groups].sort(), ["phb:general", "test:warrior"]);
  assert.deepEqual([...openGroups(pack, "test:thief").groups].sort(), ["phb:general", "test:rogue"]);
  assert.deepEqual(openGroups(pack, "test:fighter").unresolved, []);
});

test("a proficiency outside every open group costs one slot more, and says which group", () => {
  assert.deepEqual(cost(pack, "test:riding", "test:fighter"),
    { cost: 1, because: "in Warrior, which is open to this class", certain: true });
  const outside = cost(pack, "test:tumbling", "test:fighter");
  assert.equal(outside.cost, 2);
  assert.match(outside.because, /in Rogue, which is not open to this class — one slot more/);
});

test("a proficiency whose group no book states is NOT surcharged, and says so", () => {
  // The Complete Thief's prints "1 slot, Wisdom, -1 modifier" and stops. Charging the
  // crossover anyway would invent a rule against the character where nobody would look.
  const unknown = cost(pack, "cth:begging", "test:fighter");
  assert.equal(unknown.cost, 1);
  assert.equal(unknown.certain, false);
  assert.match(unknown.because, /no book says which group this belongs to/);
});

test("spending is counted from the events, at the cost that applied", () => {
  const c = fighter();
  c.advance([{ class: "test:fighter", die: 4 }], {
    chose: [{ kind: "nonweaponProficiency", ref: "test:tumbling" }],   // outside, so 2
  });
  const b = budget(pack, c, "nonweapon")!;
  assert.equal(b.spent, 2);
  assert.equal(b.free, b.total - 2);
});

test("what is already taken is not offered again", () => {
  const c = fighter();
  assert.equal(candidates(pack, c).some((x) => x.id === "test:riding"), true);
  c.advance([{ class: "test:fighter", die: 4 }], { chose: [{ kind: "nonweaponProficiency", ref: "test:riding" }] });
  assert.equal(candidates(pack, c).some((x) => x.id === "test:riding"), false);
});
