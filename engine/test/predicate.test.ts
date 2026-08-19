import { test } from "node:test";
import assert from "node:assert/strict";
import { predicate, type Subject } from "../src/predicate.ts";
import type { Predicate } from "../src/types.ts";

const sheet = (scores: Record<string, number>, fields: Record<string, number | string> = {}): Subject => ({
  ability: (id) => scores[id],
  level: (id) => scores[id] ?? 0,
  field: (path) => fields[path],
  has: (_kind, ref) => ref === "test:granted",
});

const wis16: Predicate = [{ subject: { ability: "test:wisdom" }, op: "gte", value: 16 }];

test("a conjunction holds when every clause does", () => {
  const p: Predicate = [
    { subject: { ability: "test:wisdom" }, op: "gte", value: 16 },
    { subject: { ability: "test:charisma" }, op: "gte", value: 16 },
  ];
  assert.equal(predicate(p, sheet({ "test:wisdom": 16, "test:charisma": 16 })), true);
  assert.equal(predicate(p, sheet({ "test:wisdom": 16, "test:charisma": 10 })), false);
});

test("an absent predicate holds", () => {
  assert.equal(predicate(undefined, sheet({})), true);
  assert.equal(predicate([], sheet({})), true);
});

test("a clause the sheet cannot answer is UNDECIDABLE, which is not false", () => {
  assert.equal(predicate(wis16, sheet({})), null);
  // …and it must not be confused with a clause the sheet answers with no.
  assert.equal(predicate(wis16, sheet({ "test:wisdom": 9 })), false);
});

test("a conjunction that has found a falsifier is settled, undecidable clauses or not", () => {
  const p: Predicate = [
    { subject: { ability: "test:wisdom" }, op: "gte", value: 16 },   // undecidable
    { subject: { ability: "test:charisma" }, op: "gte", value: 16 }, // false
  ];
  assert.equal(predicate(p, sheet({ "test:charisma": 3 })), false);
});

test("a disjunction that has found a witness is settled", () => {
  const p: Predicate = [{ anyOf: [
    { subject: { ability: "test:wisdom" }, op: "gte", value: 16 },     // true
    { subject: { ability: "test:charisma" }, op: "gte", value: 16 },   // undecidable
  ] }];
  assert.equal(predicate(p, sheet({ "test:wisdom": 18 })), true);
});

test("a disjunction with no witness and an undecidable arm is undecidable", () => {
  const p: Predicate = [{ anyOf: [
    { subject: { ability: "test:wisdom" }, op: "gte", value: 16 },     // false
    { subject: { ability: "test:charisma" }, op: "gte", value: 16 },   // undecidable
  ] }];
  assert.equal(predicate(p, sheet({ "test:wisdom": 3 })), null);
});

test("membership reads a scalar and tests it against a list of ids", () => {
  const p: Predicate = [{ member: { field: "race" }, anyOfIds: ["test:dwarf", "test:gnome"] }];
  assert.equal(predicate(p, sheet({}, { race: "test:dwarf" })), true);
  assert.equal(predicate(p, sheet({}, { race: "test:elf" })), false);
  assert.equal(predicate(p, sheet({})), null);
});

test("a comparison against something that is not a number is undecidable, not false", () => {
  const p: Predicate = [{ subject: { field: "startingWealth" }, op: "gte", value: 100 }];
  assert.equal(predicate(p, sheet({}, { startingWealth: "5d4x10" })), null);
});
