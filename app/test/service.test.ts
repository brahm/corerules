import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Character } from "../../engine/src/character.ts";
import { canonical } from "../../engine/src/hash.ts";
import { Library } from "../../engine/src/library.ts";
import { characters, open, packs } from "../src/main/service.ts";

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
  assert.equal(pack!.records, 23);
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
