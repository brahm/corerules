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
    ["scores", "ready"], ["race", "ready"], ["class", "waiting"], ["kit", "waiting"],
  ], "with no race chosen there is no subrace step, because no subrace targets nothing");

  const chosen = steps(pack, { scores: { "test:strength": 16 }, race: "test:hillfolk", class: "test:fighter" });
  assert.deepEqual(chosen.map((s) => [s.key, s.state]), [
    ["scores", "done"], ["race", "done"], ["subrace", "ready"], ["class", "done"], ["kit", "ready"],
  ], "and once the race has one, the subrace step appears where it belongs");
});

test("a race with no subraces gets no step to skip past", () => {
  assert.equal(steps(pack, { race: "test:hillfolk" }).some((s) => s.key === "subrace"), true);
  assert.equal(steps(pack, { race: "test:nowhere" }).some((s) => s.key === "subrace"), false);
});
