import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Character } from "../src/character.ts";
import { Pack } from "../src/pack.ts";
import { armourClass, available, slots, startingFunds } from "../src/spells.ts";

const pack = new Pack(join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal"));

const priest = () => {
  const c = Character.create(pack, {
    name: "Someone", race: "test:hillfolk", scores: {}, deity: "test:harvest",
  });
  c.advance([{ class: "test:cleric", die: 8 }]);
  return c;
};

test("spell slots come from the class group's own progression table", () => {
  assert.deepEqual(slots(pack, "test:cleric", 1).perLevel, [1, 0, 0, 0]);
  assert.deepEqual(slots(pack, "test:cleric", 5).perLevel, [3, 3, 2, 0]);
});

test("a progression table present with no rows names itself rather than reading as zero", () => {
  // Table 21 is in the slice exactly like this. "No spells" and "nobody transcribed how many"
  // are different facts and a caster would notice which one they were living in.
  const said = slots(pack, "test:mage", 1);
  assert.deepEqual(said.perLevel, []);
  assert.match(said.missing!, /Table 21.*present in the pack with no rows in it/);
});

test("major access reaches every level; minor access stops at third", () => {
  // Complete Priest's DD05501: minor access learns "spells from only 1st through 3rd level".
  const offers = available(pack, priest());
  const named = new Map(offers.map((o) => [o.name, o]));

  assert.equal(named.get("Sprout")?.through.access, "major");
  assert.equal(named.get("Furrow")?.through.access, "major", "5th level, and the sphere is major");
  assert.equal(named.get("Glimmer")?.through.access, "minor", "2nd level, minor sphere");
  assert.equal(named.has("Noon"), false, "4th level in a minor sphere is out of reach");
});

test("a spell in two spheres is offered on the better terms", () => {
  // Solstice is 6th level, in one major sphere and one minor. Minor cannot reach it; major can.
  const solstice = available(pack, priest()).find((o) => o.name === "Solstice")!;
  assert.equal(solstice.through.access, "major");
  assert.equal(solstice.through.sphere, "Soil");
});

test("a character with no god is offered nothing rather than everything", () => {
  const c = Character.create(pack, { name: "Someone", race: "test:hillfolk", scores: {} });
  c.advance([{ class: "test:cleric", die: 8 }]);
  assert.deepEqual(available(pack, c), []);
});

test("starting funds are the die Table 43 prints, not a number", () => {
  assert.equal(startingFunds(pack, "test:fighter"), "5d4 x 10 gp");
  assert.equal(startingFunds(pack, "test:thief"), "2d6 x 10 gp");
});

test("armour class is refused for anything worn, and the reason is the table's shape", () => {
  assert.deepEqual(armourClass(pack, []), { ac: 10, because: "Table 46: unarmoured" });
  const worn = armourClass(pack, ["phb:leather"]);
  assert.equal(worn.ac, undefined);
  assert.match(worn.because, /rates COMBINATIONS of armour and shield rather than pieces/);
});
