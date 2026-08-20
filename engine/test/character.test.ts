import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Pack } from "../src/pack.ts";
import { Character } from "../src/character.ts";

const pack = new Pack(join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal"));

const fresh = (over: Partial<Parameters<typeof Character.create>[1]> = {}) =>
  Character.create(pack, { name: "Someone", race: "test:hillfolk", scores: {}, ...over });

test("a Character is its events; levels are derived from them and never stored", () => {
  const c = fresh();
  c.advance([{ class: "test:fighter", die: 8 }]);
  c.advance([{ class: "test:fighter", die: 6 }]);
  assert.deepEqual(c.levels(), { "test:fighter": 2 });
  assert.deepEqual(c.classes(), ["test:fighter"]);
});

test("the PHB's own multi-class example, which is why an event holds its rolls together", () => {
  // "1d10 (fighter), 1d6 (thief), and 1d4 (mage). The results are 6, 5, and 2. Their sum
  // (13) is divided by three and rounded down to equal 4."
  const c = fresh();
  c.advance([
    { class: "test:fighter", die: 6 },
    { class: "test:thief", die: 5 },
    { class: "test:mage", die: 2 },
  ]);
  assert.equal(c.hitPoints(), 4);
  // Dividing each roll on its own would give 2 + 1 + 0 = 3, which is the wrong answer and
  // the reason the rolls travel in one event.
  assert.notEqual(c.hitPoints(), 3);

  // "He rolls 1d6 ... the result is 4. He divides this by 3 ... Morrison gets 1 more."
  c.advance([{ class: "test:thief", die: 4 }]);
  assert.equal(c.hitPoints(), 5);
  assert.deepEqual(c.levels(), { "test:fighter": 1, "test:thief": 2, "test:mage": 1 });
});

test("events are chronological because their ids are, with no ordering field to disagree", () => {
  const c = fresh();
  const a = c.advance([{ class: "test:fighter", die: 8 }]);
  const b = c.advance([{ class: "test:fighter", die: 3 }]);
  assert.deepEqual(c.file.events.map((e) => e.id), [a.id, b.id].toSorted());
});

test("the sheet is computed from the events, and the class layer arrives with them", () => {
  const c = fresh({ kit: "test:hedge-knight" });
  c.advance([{ class: "test:fighter", die: 8 }]);
  const s = c.sheet();
  assert.deepEqual(s.layers.map((l) => l.role), ["race", "class group", "class", "kit"]);
  assert.equal(s.view("hitDice.perLevel").value, "1d10");
  assert.equal(s.view("attackRoll").value, 2);   // the fighter's 1, the kit's 1
});

test("abandoning a kit drops the layer — benefits and penalties alike — and leaves a debt", () => {
  const c = fresh({ kit: "test:hedge-knight" });
  c.advance([{ class: "test:fighter", die: 8 }]);
  assert.equal(c.sheet().view("attackRoll").value, 2);
  assert.deepEqual(c.debt(), []);

  c.file.kitAbandoned = true;
  assert.equal(c.sheet().view("attackRoll").value, 1);         // the kit's bonus is gone
  assert.deepEqual(c.debt(), ["test:riding", "test:heraldry"]);

  // §6.4: the debt is those specific proficiencies, not a count. Paying one leaves the other.
  c.advance([{ class: "test:fighter", die: 5 }], { chose: [{ kind: "nonweaponProficiency", ref: "test:riding" }] });
  assert.deepEqual(c.debt(), ["test:heraldry"]);
});

test("a Character records the packs it was built against and the options its table plays", () => {
  const c = Character.create(pack, {
    name: "Someone", race: "test:hillfolk", scores: {},
    options: ["cprh:split-prime-requisite-bonus"],
  });
  assert.deepEqual(c.file.packs, [{ id: "minimal" }]);
  c.advance([{ class: "test:fighter", die: 8 }]);
  assert.equal(c.sheet().view("experienceAward.percent").value, 5);
});

test("a Character round-trips through JSON, because the file is the whole of it", () => {
  const c = fresh({ kit: "test:hedge-knight" });
  c.advance([{ class: "test:fighter", die: 8 }]);
  c.advance([{ class: "test:fighter", die: 7 }]);
  const back = new Character(pack, JSON.parse(JSON.stringify(c.file)));
  assert.equal(back.hitPoints(), c.hitPoints());
  assert.deepEqual(back.levels(), c.levels());
  assert.equal(back.sheet().view("attackRoll").value, c.sheet().view("attackRoll").value);
});

test("Constitution enters the hit points twice, and the second way is a floor on the die", () => {
  // Correction 66. Table 3's Hit Point Adjustment column holds two numbers in one cell —
  // `+2 (+4)*`, the parenthetical for warriors only — and the star COUNT is a different rule
  // again: at Constitution 20 every 1 rolled for a Hit Die counts as a 2.
  const mk = (cls: string, con: number, dice: number[]) => {
    const c = Character.create(pack, { name: "x", race: "test:hillfolk", scores: { "test:constitution": con } });
    for (const d of dice) c.advance([{ class: cls, die: d }]);
    return c;
  };
  assert.equal(mk("test:fighter", 10, [8, 8, 8]).hitPoints(), 24, "no bonus, no floor");
  assert.equal(mk("test:fighter", 18, [8, 8, 8]).hitPoints(), 36, "a warrior reads the parenthetical: +4 a level");
  assert.equal(mk("test:mage", 18, [4, 4, 4]).hitPoints(), 18, "everyone else is capped at +2");

  // The floor is applied when the total is computed, never at the roll. §6.3 keeps the 1 the
  // player actually rolled: storing the 2 would destroy the roll and make the number
  // unexplainable the day the character's Constitution changed.
  const lucky = mk("test:fighter", 20, [1, 1, 1]);
  assert.equal(lucky.hitPoints(), 21, "three 1s become three 2s, plus 5 a level");
  assert.deepEqual(lucky.file.events.map((e) => e.rolls[0]!.die), [1, 1, 1], "and the file still says 1");
});

test("a character who is a warrior in one class and not another gets no bonus, and is told why", () => {
  const c = Character.create(pack, { name: "x", race: "test:hillfolk", scores: { "test:constitution": 18 } });
  c.advance([{ class: "test:fighter", die: 8 }, { class: "test:mage", die: 4 }]);
  // Table 3 has two columns and no rule for a character who belongs in both. Picking either
  // would be the Engine inventing one, so it picks neither and says so.
  assert.equal(c.hitPoints(), 6);
  assert.match(c.hitPointsBecause()!, /warrior in one class and not in another/);
});
