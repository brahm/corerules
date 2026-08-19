import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Character } from "../src/character.ts";
import { canonical } from "../src/hash.ts";
import { Library } from "../src/library.ts";

const minimal = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal");

/** A content root with one pack in it, the way a user's folder holds one. */
const root = (): string => {
  const dir = mkdtempSync(join(tmpdir(), "corerules-root-"));
  cpSync(minimal, join(dir, "minimal"), { recursive: true });
  return dir;
};

test("a directory is a pack when it holds a manifest, and `characters` never is", () => {
  const dir = root();
  const lib = new Library(dir);
  const c = Character.create(lib.load("minimal"), { name: "Someone", race: "test:hillfolk", scores: {} });
  lib.writeCharacter(c.file);
  assert.deepEqual(lib.packs().map((p) => p.id), ["minimal"]);
  rmSync(dir, { recursive: true });
});

test("a Character is written whole and read back identical", () => {
  const dir = root();
  const lib = new Library(dir);
  const c = Character.create(lib.load("minimal"), { name: "Someone", race: "test:hillfolk", scores: { "test:dexterity": 12 } });
  c.advance([{ class: "test:fighter", die: 8 }]);
  lib.writeCharacter(lib.stamp(c.file));

  assert.deepEqual(lib.characterIds(), [c.file.id]);
  const back = lib.readCharacter(c.file.id);
  assert.deepEqual(back, c.file);
  assert.match(back.packs[0]!.sha256!, /^[0-9a-f]{64}$/, "stamped with the pack's content hash");
  rmSync(dir, { recursive: true });
});

test("a Character opens against its pack and computes the same sheet", () => {
  const dir = root();
  const lib = new Library(dir);
  const c = Character.create(lib.load("minimal"), { name: "Someone", race: "test:hillfolk", scores: {}, kit: "test:hedge-knight" });
  c.advance([{ class: "test:fighter", die: 8 }]);
  lib.writeCharacter(lib.stamp(c.file));

  const opened = lib.open(c.file.id);
  assert.deepEqual(opened.drift, []);
  assert.equal(opened.character?.hitPoints(), 8);
  assert.equal(opened.character?.sheet().view("attackRoll").value, 2);
  rmSync(dir, { recursive: true });
});

test("a pack that moved is reported, and what the Character can no longer find is named", () => {
  const dir = root();
  const lib = new Library(dir);
  const c = Character.create(lib.load("minimal"), { name: "Someone", race: "test:hillfolk", scores: {}, kit: "test:hedge-knight" });
  c.advance([{ class: "test:fighter", die: 8 }]);
  lib.writeCharacter(lib.stamp(c.file));

  // The user re-transcribes and the kit's id changes, which is the ordinary way a pack moves.
  const kits = join(dir, "minimal", "kits.json");
  const doc = JSON.parse(readFileSync(kits, "utf8")) as { kits: { id: string }[] };
  doc.kits[0]!.id = "test:hedge-knight-errant";
  writeFileSync(kits, canonical(doc));

  const opened = lib.open(c.file.id);
  assert.equal(opened.drift.length, 1);
  assert.deepEqual(opened.drift[0]!.lost, ["test:hedge-knight"]);
  assert.notEqual(opened.drift[0]!.was, opened.drift[0]!.now);
  // …and loading still never fails. §5.3 locks what EXTENDS a Character, not reading one.
  assert.equal(opened.character?.hitPoints(), 8);
  rmSync(dir, { recursive: true });
});

test("a pack that is gone entirely is reported, and the Character still opens", () => {
  const dir = root();
  const lib = new Library(dir);
  const c = Character.create(lib.load("minimal"), { name: "Someone", race: "test:hillfolk", scores: {} });
  c.advance([{ class: "test:fighter", die: 8 }]);
  lib.writeCharacter(lib.stamp(c.file));
  rmSync(join(dir, "minimal"), { recursive: true });

  const opened = lib.open(c.file.id);
  assert.deepEqual(opened.drift.map((d) => d.pack), ["minimal"]);
  assert.equal(opened.character, undefined, "there is nothing to compute against");
  assert.equal(opened.file.name, "Someone", "and the file is still readable, which is the point");
  rmSync(dir, { recursive: true });
});

test("a reformatted pack is not drift, because the hash is over the canonical form", () => {
  const dir = root();
  const lib = new Library(dir);
  const c = Character.create(lib.load("minimal"), { name: "Someone", race: "test:hillfolk", scores: {} });
  c.advance([{ class: "test:fighter", die: 8 }]);
  lib.writeCharacter(lib.stamp(c.file));

  const f = join(dir, "minimal", "classes.json");
  writeFileSync(f, JSON.stringify(JSON.parse(readFileSync(f, "utf8")), null, 6));
  assert.deepEqual(lib.open(c.file.id).drift, []);
  rmSync(dir, { recursive: true });
});
