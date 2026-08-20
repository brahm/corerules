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

test("armour class is a rule over what you wear, once the combination table is read as one", () => {
  // Correction 61. Table 46 rates COMBINATIONS — "Splint mail, banded mail, or bronze plate mail
  // + shield, plate mail" is one row — and the `+ shield` attaches BACKWARDS over the run before
  // it. Read left to right that row and the one above it contradict each other about splint mail
  // and nothing notices. Read correctly the table is complete, and this is what it computes.
  assert.equal(armourClass(pack, []).ac, 10);
  assert.equal(armourClass(pack, ["test:shield"]).ac, 9);
  assert.equal(armourClass(pack, ["test:jerkin"]).ac, 8);
  assert.equal(armourClass(pack, ["test:jerkin", "test:buckler"]).ac, 7, "a shield by its own name");
  assert.equal(armourClass(pack, ["test:mail", "test:shield"]).ac, 4);
});

test("armour class refuses the four ways it can be asked a question the table cannot answer", () => {
  // A cell the book never prints. Its neighbours make the arithmetic obvious, which is exactly
  // why guessing would be a rule invented by the Engine.
  const gap = armourClass(pack, ["test:oddments", "test:shield"]);
  assert.equal(gap.ac, undefined);
  assert.match(gap.because, /never with a shield/);

  // A category, which is the whole of correction 61: it IS in the armour kind, and nobody owns one.
  assert.match(armourClass(pack, ["test:heavy-armor"]).because, /category a rule discriminates on/);

  // Two suits at once, and something that is not armour at all.
  assert.match(armourClass(pack, ["test:jerkin", "test:mail"]).because, /nobody wears/);
  assert.match(armourClass(pack, ["test:sabre"]).because, /not armour/);
});
