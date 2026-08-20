import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Character } from "../src/character.ts";
import { Pack } from "../src/pack.ts";
import { present, render } from "../src/present.ts";

const pack = new Pack(join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal"));

const someone = (over: Record<string, unknown> = {}) => {
  const c = Character.create(pack, {
    name: "Someone", race: "test:hillfolk", scores: { "test:dexterity": 12 }, ...over,
  });
  c.advance([{ class: "test:fighter", die: 8 }]);
  return c;
};

test("a value carries the layers that made it, each naming its book", () => {
  const view = present(someone({ kit: "test:hedge-knight" }));
  const attack = view.values.find((v) => v.path === "attackRoll")!;
  assert.equal(attack.value, 2);
  assert.deepEqual(attack.from.map((f) => f.record), ["Fighter", "Hedge Knight"]);
  assert.deepEqual([...new Set(attack.from.map((f) => f.book))], ["Fixtures"]);
});

test("every reason a value is missing reaches the player in the Engine's words", () => {
  const view = present(someone());
  const said = new Map(view.aside.map((a) => [a.because, a]));

  assert.match(said.get("marked")!.headline, /circumstance the pack could not express/);
  assert.match(said.get("marked")!.detail, /only with other hillfolk/,
    "and the transcriber's words survive, because they are the only place a refusal carries a reason");

  assert.match(said.get("undecidable")!.headline, /no answer for/);
  assert.match(said.get("option")!.headline, /your table has not said/);
  assert.equal(said.get("option")!.option, "cprh:split-prime-requisite-bonus");
  assert.match(said.get("unresolved")!.headline, /cannot compute this yet/);
});

test("a contested value spoils its own line rather than joining the list of reasons", () => {
  const c = someone();
  // A kit targets a class, so neither record declares anything about the other.
  c.pack.byId.set("test:stranger", {
    id: "test:stranger", name: "Stranger", target: "test:fighter",
    provenance: { section: ["Elsewhere"] },
    effects: [{ op: "set", field: "infravision.range", to: 15 }],
  });
  c.file.kit = "test:stranger";
  const view = present(c);
  const line = view.values.find((v) => v.path === "infravision.range")!;
  assert.equal(line.value, undefined);
  assert.deepEqual(line.contested!.map((x) => x.book).sort(), ["Elsewhere", "Fixtures"]);
  assert.equal(view.aside.some((a) => String(a.because) === "contested"), false);
});

test("a marked structural grant keeps its rider, because the thing IS on the sheet", () => {
  const view = present(someone());
  const inline = view.granted.find((g) => g.rider !== undefined)!;
  assert.equal(inline.name, "A thing that exists nowhere else");
  assert.match(inline.rider!, /UNMODELLED SHAPE/);
});

test("an abandoned kit's debt is shown, or it becomes a phantom bug", () => {
  const c = someone({ kit: "test:hedge-knight" });
  c.file.kitAbandoned = true;
  assert.deepEqual(present(c).debt, ["Riding", "Heraldry"]);
});

test("the text rendering says the same things, and is what a bug report pastes", () => {
  const text = render(present(someone({ kit: "test:hedge-knight" })));
  assert.match(text, /Someone — 8 hp/);
  assert.match(text, /Hillfolk \(race\) \/ Warrior \(class group\) \/ Fighter \(class\) \/ Hedge Knight \(kit\)/);
  assert.match(text, /NOT ON THE SHEET — your table has not said/);
  assert.match(text, /2 x language — unbounded/);
});

test("the sheet says which foes change its numbers, and stays silent when none do", () => {
  // Correction 17. *"How does this NPC react to you"* has no meaning without the NPC, so every
  // rule naming one was permanently undecidable and the dwarf's +1 against orcs reached no
  // sheet. The candidates come from the character's own layers rather than from the creature
  // list — a pack with three hundred monsters would otherwise print three hundred rows, of
  // which four differ, and the four are the rules the character actually has.
  const c = Character.create(pack, { name: "Someone", race: "test:hillfolk", scores: {} });
  c.advance([{ class: "test:fighter", die: 8 }]);
  const v = present(c).versus;
  assert.equal(v.length, 1, "the bear is in the pack and no rule of this character names it");
  assert.equal(v[0]!.name, "Wolf");
  assert.deepEqual(v[0]!.changes, [{ path: "attackRoll.melee", value: 1 }]);

  // The bear is a record in the same pack and no row mentions it, which is the half that keeps
  // this readable: the section is the character's rules, not the pack's bestiary.
  assert.equal(v.some((x) => x.creature === "test:bear"), false);
});
