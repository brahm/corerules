import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { advance, check, correct, draftOf, objections } from "../src/advance.ts";
import { Character } from "../src/character.ts";
import { Pack } from "../src/pack.ts";

const pack = new Pack(join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal"));

const someone = (scores: Record<string, number> = { "test:strength": 16 }) => {
  const c = Character.create(pack, { name: "Someone", race: "test:hillfolk", scores });
  c.advance([{ class: "test:fighter", die: 8 }]);
  return c;
};

test("most levels buy nothing, and the ones that do are why the mini-wizard exists", () => {
  const c = someone();
  assert.deepEqual(advance(pack, c, "test:fighter").gains, [], "reaching level 2 gains no slots");
  c.advance([{ class: "test:fighter", die: 5 }]);
  c.advance([{ class: "test:fighter", die: 5 }]);   // now level 3, so the next is 4
  assert.deepEqual(advance(pack, c, "test:fighter").gains, [
    { kind: "weaponProficiency", slots: 1 },
    { kind: "nonweaponProficiency", slots: 1 },
  ], "level 4 crosses both of Table 34's boundaries");
});

test("an advance knows which classes may take it, and which die each rolls", () => {
  const a = advance(pack, someone(), "test:fighter");
  assert.deepEqual(a.classes, [{ id: "test:fighter", name: "Fighter", level: 1, die: "1d10" }]);
  assert.equal(a.die, "1d10");
});

test("a Character built legally has no objections, and carries A3's caveat for ever", () => {
  const said = check(pack, draftOf(someone()));
  assert.deepEqual(said.objections, []);
  // Not noise, and not an error: no pack declares which races may take which classes, so the
  // Engine says every load that it is not checking it. Mixing this into the objections would
  // make a correct Character look broken and teach the user to ignore both.
  assert.equal(said.caveats.length, 1);
  assert.match(said.caveats[0]!.because, /no loaded pack declares which races/);
});

test("a correction is applied and then answered for — §9.2's same rules on both paths", () => {
  const c = someone();
  // The Mage wants Intelligence 13 and this character has none recorded.
  const said = correct(pack, c, c.file.events[0]!.id, { rolls: [{ class: "test:mage", die: 4 }] });
  assert.equal(c.file.events[0]!.rolls[0]!.class, "test:mage", "the correction is applied, not refused");
  const about = said.find((o) => o.step === "class")!;
  assert.equal(about.available, "unknown", "nobody can tell, which is not the same as no");
  assert.match(about.because, /have not been rolled yet/);
});

test("editing the scores reaches the class, because the objections ask every step", () => {
  const c = Character.create(pack, {
    name: "Someone", race: "test:hillfolk", scores: { "test:intelligence": 16 },
  });
  c.advance([{ class: "test:mage", die: 4 }]);
  assert.deepEqual(check(pack, draftOf(c)).objections, []);

  c.file.scores["test:intelligence"] = 8;      // the back door §9.2 names
  const said = check(pack, draftOf(c)).objections;
  assert.equal(said.length, 1);
  assert.match(said[0]!.because, /requires Intelligence 13 — Fixtures/);
});

test("a correction that takes slots away is an objection about the budget", () => {
  const c = someone();
  c.advance([{ class: "test:fighter", die: 4 }], {
    chose: [
      { kind: "nonweaponProficiency", ref: "test:riding" },
      { kind: "nonweaponProficiency", ref: "test:heraldry" },
      { kind: "nonweaponProficiency", ref: "test:tumbling" },  // 1 + 1 + 2 = 4 against 3
    ],
  });
  const said = check(pack, draftOf(c)).objections;
  assert.equal(said.length, 1);
  assert.match(said[0]!.because, /4 slots spent and only 3 to spend/);
});

test("a choice the packs no longer offer is reported, not thrown", () => {
  const c = someone();
  c.file.kit = "test:a-kit-that-went-away";
  const said = check(pack, draftOf(c)).objections;
  assert.equal(said[0]!.because, "no loaded pack offers this any more");
});
