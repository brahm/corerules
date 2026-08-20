import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Character } from "../../engine/src/character.ts";
import { canonical } from "../../engine/src/hash.ts";
import { Library } from "../../engine/src/library.ts";
import { check, draftOf } from "../../engine/src/advance.ts";
import { characters, correctEvent, open, packs, removeEvent, timeline, wear } from "../src/main/service.ts";

const fixture = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "engine", "test", "fixtures", "minimal");

function root(): { dir: string; library: Library; id: string } {
  const dir = mkdtempSync(join(tmpdir(), "corerules-app-"));
  cpSync(fixture, join(dir, "minimal"), { recursive: true });
  const library = new Library(dir);
  const c = Character.create(library.load("minimal"), {
    name: "Someone", race: "test:hillfolk", scores: {}, kit: "test:hedge-knight",
  });
  c.advance([{ class: "test:fighter", die: 8 }]);
  library.writeCharacter(library.stamp(c.file));
  return { dir, library, id: c.file.id };
}

test("the packs list says what a user needs to decide whether it is the right pack", () => {
  const { dir, library } = root();
  const [pack] = packs(library);
  assert.equal(pack!.id, "minimal");
  assert.equal(pack!.records, 57);
  assert.match(pack!.hash, /^[0-9a-f]{12}$/);
  assert.deepEqual(pack!.complaints, []);
  rmSync(dir, { recursive: true });
});

test("the characters list carries drift, so a pack that moved is visible before you open it", () => {
  const { dir, library } = root();
  assert.deepEqual(characters(library)[0]!.drift, []);

  const kits = join(dir, "minimal", "kits.json");
  const doc = JSON.parse(readFileSync(kits, "utf8")) as { kits: { id: string }[] };
  doc.kits[0]!.id = "test:hedge-knight-errant";
  writeFileSync(kits, canonical(doc));

  const after = characters(library)[0]!;
  assert.deepEqual(after.drift, [{ pack: "minimal", lost: ["test:hedge-knight"] }]);
  assert.equal(after.hitPoints, 8, "and it still opens, because loading never fails");
  rmSync(dir, { recursive: true });
});

test("a character whose packs are gone still has a name and says so", () => {
  const { dir, library } = root();
  rmSync(join(dir, "minimal"), { recursive: true });
  const [c] = characters(library);
  assert.equal(c!.name, "Someone");
  assert.match(c!.who, /packs are not here/);
  rmSync(dir, { recursive: true });
});

test("opening gives the renderer a display model and never a record", () => {
  const { dir, library, id } = root();
  const view = open(library, id)!;
  assert.equal(view.name, "Someone");
  assert.equal(view.values.find((v) => v.path === "attackRoll")?.value, 2);
  // Nothing in the view is a pack record: the renderer gets names, numbers and books.
  assert.equal(JSON.stringify(view).includes('"effects"'), false);
  rmSync(dir, { recursive: true });
});

test("a level can be corrected in place, and the sheet answers for the result", () => {
  const { dir, library, id } = root();
  const before = library.open(id).character!;
  const eventId = before.file.events[0]!.id;
  assert.equal(before.hitPoints(), 8);

  // §6.5: the old value stops existing. There is no draft and nothing to confirm.
  const said = correctEvent(library, id, eventId, { die: 3 });
  assert.deepEqual(said, [], "a smaller roll breaks no rule");
  assert.equal(library.open(id).character!.hitPoints(), 3, "and the sheet follows");

  // §9.2: the same validation rules on both paths. The Mage wants Intelligence 13 and this
  // character has none, so the edit is APPLIED and answered for.
  const objected = correctEvent(library, id, eventId, { class: "test:mage" });
  assert.equal(library.open(id).file.events[0]!.rolls[0]!.class, "test:mage");
  assert.equal(objected.length + check(library.load("minimal"), draftOf(library.open(id).character!)).caveats.length > 0, true);
  rmSync(dir, { recursive: true });
});

test("removing a level takes its choices with it, because they travelled in it", () => {
  const { dir, library, id } = root();
  const c = library.open(id).character!;
  c.advance([{ class: "test:fighter", die: 6 }], {
    chose: [{ kind: "nonweaponProficiency", ref: "test:riding" }],
  });
  library.writeCharacter(c.file);
  assert.equal(library.open(id).character!.file.events.length, 2);

  const second = library.open(id).file.events[1]!.id;
  removeEvent(library, id, second);
  const after = library.open(id).character!;
  assert.equal(after.file.events.length, 1);
  assert.deepEqual(after.file.events.flatMap((e) => e.chose ?? []), [],
    "the choice went with the level that made it");
  rmSync(dir, { recursive: true });
});

test("armour goes on and comes off without touching the history", () => {
  // Correction 61. §6.3's Level Events are what the rules derive from, and what a character is
  // wearing this afternoon derives nothing about their level — putting it in that list would file
  // a change of clothes beside becoming a 5th-level fighter.
  const { dir, library, id } = root();
  const before = library.open(id).file.events.length;

  wear(library, id, ["test:mail", "test:buckler"]);
  const t = timeline(library, id)!;
  assert.equal(t.derived.armourClass, 4, "mail is 5 alone and 4 with a shield");
  assert.equal(library.open(id).file.events.length, before, "and the timeline is untouched");
  assert.equal(t.wear.find((w) => w.id === "test:mail")?.chosen, true);

  wear(library, id, []);
  assert.equal(timeline(library, id)!.derived.armourClass, 10);
  assert.equal(library.open(id).file.worn, undefined, "and the field goes away rather than emptying");
  rmSync(dir, { recursive: true });
});
