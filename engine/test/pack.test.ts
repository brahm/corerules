import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Pack } from "../src/pack.ts";

const here = dirname(fileURLToPath(import.meta.url));
const fixture = (name: string) => join(here, "fixtures", name);

test("a pack loads from its manifest", () => {
  const pack = new Pack(fixture("minimal"));
  assert.equal(pack.manifest.id, "minimal");
  assert.equal(pack.byId.size, 49);
  assert.deepEqual([...pack.byKind.keys()].sort(), [
    "abilities", "classes", "creatures", "deities", "kits", "limitations", "lookupTables",
    "nonweaponProficiencies", "proficiencyGroups", "races", "spells", "spheres",
    "subraces", "weaponProficiencies",
  ]);
  assert.equal(pack.records("weaponProficiencies").length, 8);
  assert.equal(pack.records("classes").length, 8);
  assert.deepEqual(pack.complaints, []);
});

test("ids are indexed flat, because they are globally scoped and not scoped by kind", () => {
  const pack = new Pack(fixture("minimal"));
  assert.equal(pack.byId.get("test:sabre")?.name, "Sabre");
  assert.equal(pack.byId.get("test:fighter")?.name, "Fighter");
});

test("`fields` is a declaration, not a kind", () => {
  const pack = new Pack(fixture("minimal"));
  assert.deepEqual([...pack.vocabulary].sort(), [
    "asPrinted", "attackRoll", "attackRoll.melee", "detect.slope", "experienceAward.percent", "farsight.range",
    "hitDice.perLevel", "infravision.range", "morale", "reactionCheck", "stealth.bonus", "surefooting",
  ]);
  assert.equal(pack.byKind.has("fields"), false);
});

test("a group expands transitively, because a member may itself be a group", () => {
  const pack = new Pack(fixture("minimal"));
  assert.deepEqual(
    [...pack.expand(["test:blade"])].sort(),
    ["test:greatsword", "test:sabre", "test:short-blade"],
  );
  assert.deepEqual(pack.complaints, []);
});

test("expanding a group with no members complains, because it permits nothing", () => {
  const pack = new Pack(fixture("minimal"));
  assert.deepEqual([...pack.expand(["test:empty-group"])], []);
  assert.equal(pack.complaints.length, 1);
  assert.match(pack.complaints[0]!.message, /permits nothing/);
});

test("a reference that resolves nowhere is reported, not thrown", () => {
  const pack = new Pack(fixture("minimal"));
  assert.equal(pack.get("test:nothing", "a test"), undefined);
  assert.equal(pack.complaints.at(-1)?.area, "reference");
});

test("the defects a pack may carry are reported rather than repaired", () => {
  const pack = new Pack(fixture("broken"));
  const said = (re: RegExp) => pack.complaints.some((c) => re.test(c.message));
  assert.ok(said(/test:collide is defined twice/), "a colliding id");
  assert.ok(said(/missing\.json, which is not there/), "a declared file that is absent");
  assert.ok(said(/undeclared\.json is present .* not in the manifest/), "an undeclared file");
  // …and the undeclared file's records are NOT loaded. §7.1: declaration over discovery.
  assert.equal(pack.byId.has("test:leftover"), false);
});
