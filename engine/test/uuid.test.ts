import { test } from "node:test";
import assert from "node:assert/strict";
import { mintedAt, uuidv7 } from "../src/uuid.ts";

test("a v7 says it is version 7 and variant 10", () => {
  const id = uuidv7();
  assert.match(id, /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
});

test("its timestamp survives the round trip", () => {
  const now = Date.UTC(2026, 7, 19, 12, 0, 0);
  assert.equal(mintedAt(uuidv7(now)), now);
});

test("ids sort lexically in the order they were minted — which is why they are v7", () => {
  const earlier = uuidv7(1_000_000_000_000);
  const later = uuidv7(1_000_000_000_001);
  assert.ok(earlier < later, `${earlier} should sort before ${later}`);
  const many = [5, 1, 4, 2, 3].map((n) => uuidv7(1_700_000_000_000 + n));
  assert.deepEqual([...many].sort(), many.toSorted((a, b) => mintedAt(a) - mintedAt(b)));
});

test("ids minted in the same millisecond still sort in the order they were minted", () => {
  // The reason §6.5 chose v7 is that file order IS chronological order. Two events in one
  // tick is the ordinary case — a Character is created and advanced immediately — and a
  // plain v7 orders those by their random tails, which is to say it does not order them.
  const now = 1_700_000_000_000;
  const ids = Array.from({ length: 50 }, () => uuidv7(now));
  assert.deepEqual([...ids].sort(), ids);
  assert.equal(new Set(ids).size, ids.length);
});
