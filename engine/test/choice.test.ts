import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { classes, kits, steps, type Offer } from "../src/choice.ts";
import { Pack } from "../src/pack.ts";

const pack = new Pack(join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal"));
const find = (offers: Offer[], id: string): Offer => offers.find((o) => o.id === id)!;

test("a class the scores cannot reach is refused, and names the rule and the book", () => {
  const offers = classes(pack, { scores: { "test:strength": 16, "test:intelligence": 8 } });
  const mage = find(offers, "test:mage");
  assert.equal(mage.available, "no");
  assert.match(mage.because!, /requires Intelligence 13 — Fixtures/);
  assert.equal(find(offers, "test:fighter").available, "yes");
});

test("before the scores are rolled, a gated class is unknown rather than offered or refused", () => {
  const mage = find(classes(pack, {}), "test:mage");
  assert.equal(mage.available, "unknown");
  assert.match(mage.because!, /have not been rolled yet/);
});

test("A3: a rule no pack declares is not checked, and the offer says so", () => {
  // The fixture declares nothing, exactly as the proving slice declares nothing. So every
  // race-and-class pairing is offered WITH the reason it was not checked — which is the
  // difference between "the books allow this" and "nobody told me whether they do".
  const offers = classes(pack, { scores: { "test:strength": 16, "test:intelligence": 16 }, race: "test:hillfolk" });
  const fighter = find(offers, "test:fighter");
  assert.equal(fighter.available, "unknown");
  assert.match(fighter.because!, /no loaded pack declares which races may take which classes/);
});

test("a kit belonging to another class is refused by name", () => {
  const rover = find(kits(pack, { class: "test:fighter", race: "test:hillfolk" }), "test:sea-rover");
  assert.equal(rover.available, "no");
  assert.match(rover.because!, /belongs to the Thief — Fixtures/);
});

test("a kit whose prerequisite fails is refused; one whose prerequisite holds is offered", () => {
  const draft = { class: "test:fighter", race: "test:hillfolk", scores: {} };
  assert.equal(find(kits(pack, draft), "test:hedge-knight").available, "yes");
  assert.equal(find(kits(pack, { ...draft, race: "test:lowlander" }), "test:hedge-knight").available, "no");
});

test("a prerequisite the draft cannot answer yet is unknown, not a refusal", () => {
  // The Sea Rover wants Dexterity 13 and this character has not rolled.
  const rover = find(kits(pack, { class: "test:thief" }), "test:sea-rover");
  assert.equal(rover.available, "unknown");
  assert.match(rover.because!, /has not decided yet/);
});

test("the wizard knows what it is waiting for", () => {
  const empty = steps(pack, {});
  assert.deepEqual(empty.map((s) => [s.key, s.state]), [
    ["scores", "ready"], ["race", "ready"], ["class", "waiting"], ["alignment", "ready"],
    ["weapons", "waiting"], ["proficiencies", "waiting"], ["kit", "waiting"],
  ], "with no race chosen there is no subrace step, because no subrace targets nothing");

  const chosen = steps(pack, { scores: { "test:strength": 16 }, race: "test:hillfolk", class: "test:fighter" });
  assert.deepEqual(chosen.map((s) => [s.key, s.state]), [
    ["scores", "done"], ["race", "done"], ["subrace", "ready"], ["class", "done"],
    ["alignment", "ready"], ["weapons", "ready"], ["proficiencies", "ready"], ["kit", "ready"],
  ], "and once the race has one, the subrace step appears where it belongs");
});

test("a race with no subraces gets no step to skip past", () => {
  assert.equal(steps(pack, { race: "test:hillfolk" }).some((s) => s.key === "subrace"), true);
  assert.equal(steps(pack, { race: "test:nowhere" }).some((s) => s.key === "subrace"), false);
});

test("the proficiency step is a budget, and it is not done until the slots are spent", () => {
  const bare = steps(pack, { class: "test:fighter" }).find((s) => s.key === "proficiencies")!;
  assert.deepEqual(bare.budget, { total: 3, spent: 0, free: 3 });
  assert.equal(bare.state, "ready");

  // PHB DD01537: initial slots must be assigned immediately; they cannot be saved.
  const spent = steps(pack, {
    class: "test:fighter",
    chose: [
      { kind: "nonweaponProficiency", ref: "test:riding" },     // 1, in Warrior
      { kind: "nonweaponProficiency", ref: "test:tumbling" },   // 2, outside every open group
    ],
  }).find((s) => s.key === "proficiencies")!;
  assert.deepEqual(spent.budget, { total: 3, spent: 3, free: 0 });
  assert.equal(spent.state, "done");
});

test("a candidate the budget cannot afford is refused in the budget's own terms", () => {
  const step = steps(pack, {
    class: "test:fighter",
    chose: [{ kind: "nonweaponProficiency", ref: "test:heraldry" }, { kind: "nonweaponProficiency", ref: "test:riding" }],
  }).find((s) => s.key === "proficiencies")!;
  const tumbling = step.offers.find((o) => o.id === "test:tumbling")!;
  assert.equal(tumbling.available, "no");
  assert.match(tumbling.because!, /2 slots, and 1 left/);
});

test("a cost the books cannot decide arrives as unknown, not as a yes", () => {
  const step = steps(pack, { class: "test:fighter" }).find((s) => s.key === "proficiencies")!;
  const begging = step.offers.find((o) => o.id === "cth:begging")!;
  assert.equal(begging.available, "unknown");
  assert.match(begging.because!, /perhaps one more/);
});

test("alignment comes before the kit, because kits ask about it", () => {
  // Six kits and fifty-nine priesthoods carry an alignment prerequisite. Asking in the wrong
  // order turns a decidable rule into an undecidable one, which the three-valued predicate
  // would report honestly and uselessly.
  const order = steps(pack, {}).map((s) => s.key);
  assert.ok(order.indexOf("alignment") < order.indexOf("kit"));
});

test("the weapon budget is bounded by what the class may take at all", () => {
  const step = steps(pack, { class: "test:thief" }).find((s) => s.key === "weapons")!;
  assert.deepEqual(step.budget, { total: 2, spent: 0, free: 2 });
  const refused = step.offers.filter((o) => o.available === "no");
  assert.ok(refused.some((o) => /Fighter Weapon Restriction does not permit it/.test(o.because ?? "")) === false,
    "the thief is not bound by the fighter's rule");
});

test("a weapon group is a thing you buy, and it costs what the book charges for it", () => {
  // Correction 33. Table 44's headings are typography; the Complete Fighter's makes grouping a
  // purchase — a Tight Group two slots, a Broad Group three — and until now the wizard offered
  // neither, so the one weapon rule that costs a player something was missing entirely.
  const step = steps(pack, { class: "test:fighter" }).find((s) => s.key === "weapons")!;
  const tight = step.offers.find((o) => o.id === "test:group-blades")!;
  assert.equal(tight.available, "yes");
  assert.match(tight.because!, /tight group of 2 weapons, 2 slots/);

  // And the budget spends by cost, not by pick. Counting picks made a Broad Group the cheapest
  // thing on the list, which is exactly backwards.
  const after = steps(pack, { class: "test:fighter", chose: [{ kind: "weaponProficiency", ref: "test:group-blades" }] })
    .find((s) => s.key === "weapons")!;
  assert.equal(after.budget!.spent, 2);
});

test("a group the class may only partly use is unknown, because no book rules on it", () => {
  // §5.4's third answer, arriving in a place nobody designed for it: the thief may take one of
  // the Blades group's two weapons. Refusing the group would invent a rule; allowing it would
  // invent a different one. Against the real slice a cleric's Polearms is 1 of 21.
  const step = steps(pack, { class: "test:thief" }).find((s) => s.key === "weapons")!;
  const partly = step.offers.find((o) => o.id === "test:group-blades")!;
  assert.equal(partly.available, "unknown");
  assert.match(partly.because!, /permits 1 of its 2 weapons/);
});
