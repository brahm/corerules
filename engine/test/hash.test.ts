import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { canonical, hashPack } from "../src/hash.ts";

const minimal = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal");
const copy = (): string => {
  const dir = join(mkdtempSync(join(tmpdir(), "corerules-")), "pack");
  cpSync(minimal, dir, { recursive: true });
  return dir;
};

test("the same pack hashes the same twice", () => {
  assert.equal(hashPack(minimal), hashPack(minimal));
});

test("a reformat is invisible, because the hash is over the canonical form", () => {
  const dir = copy();
  const before = hashPack(dir);
  const f = join(dir, "classes.json");
  writeFileSync(f, JSON.stringify(JSON.parse(readFileSync(f, "utf8")), null, 8));
  assert.equal(hashPack(dir), before, "indentation is not content");
  rmSync(dirname(dir), { recursive: true });
});

test("an edit is visible, however small", () => {
  const dir = copy();
  const before = hashPack(dir);
  const f = join(dir, "classes.json");
  const doc = JSON.parse(readFileSync(f, "utf8")) as { classes: { name: string }[] };
  doc.classes[0]!.name = "Warrior of the Fixtures";  // the typo fix §6.5 names
  writeFileSync(f, canonical(doc));
  assert.notEqual(hashPack(dir), before);
  rmSync(dirname(dir), { recursive: true });
});

test("adding a file changes the hash, even before anyone reads it", () => {
  const dir = copy();
  const before = hashPack(dir);
  const m = join(dir, "manifest.json");
  const manifest = JSON.parse(readFileSync(m, "utf8")) as { files: string[] };
  writeFileSync(join(dir, "extra.json"), canonical({ armor: [] }));
  manifest.files = [...manifest.files, "extra.json"].sort();
  writeFileSync(m, canonical(manifest));
  assert.notEqual(hashPack(dir), before);
  rmSync(dirname(dir), { recursive: true });
});

test("a declared file that has gone missing is part of what the pack currently is", () => {
  const dir = copy();
  const before = hashPack(dir);
  rmSync(join(dir, "tables.json"));
  assert.notEqual(hashPack(dir), before, "and it does not throw");
  rmSync(dirname(dir), { recursive: true });
});
