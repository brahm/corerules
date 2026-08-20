import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Pack } from "../src/pack.ts";
import { parse, range, roll } from "../src/dice.ts";

const pack = new Pack(join(dirname(fileURLToPath(import.meta.url)), "fixtures", "minimal"));

test("grouping changes the arithmetic, which is why correction 15 widened the grammar twice", () => {
  // `(1d4+1) x 10` is the wizard's standard starting money and is not `1d4+1 x 10`. At the top
  // of the range those are 50 and 14, and a parser that read the first as the second would be
  // wrong by a factor of three about the money every mage in the game starts with.
  assert.deepEqual(parse("(1d4+1) x 10 gp"), { count: 1, sides: 4, plus: 1, times: 10 });
  assert.deepEqual(range("(1d4+1) x 10 gp"), { least: 20, most: 50 });
  assert.deepEqual(range("5d4 x 10 gp"), { least: 50, most: 200 });

  // The corpus writes the multiplier both ways and a bare die with no count.
  assert.deepEqual(parse("4d4x10"), { count: 4, sides: 4, plus: 0, times: 10 });
  assert.deepEqual(parse("d10"), { count: 1, sides: 10, plus: 0, times: 1 });
  assert.equal(parse("not a die"), undefined);
});

test("a roll is a list of dice, not a total", () => {
  // §6.3 records randomness rather than reproducing it, and a player who rolled five 4s wants
  // to see five 4s: 200 gp with no dice behind it is indistinguishable from a number the tool
  // invented. `random` is an argument because a test that cannot fix the dice tests luck.
  const r = roll("5d4 x 10 gp", () => 0.999)!;
  assert.deepEqual(r.dice, [4, 4, 4, 4, 4]);
  assert.equal(r.total, 200);
  assert.equal(roll("5d4 x 10 gp", () => 0)!.total, 50);
});

test("every notation the format treats as a die parses, and the ones that do not are prose", () => {
  // Correction 67: `duration` and `areaOfEffect` hold sentences that START with a die —
  // "1d4 persons in 20-ft. cube" — and nothing in the pack distinguishes them from a value.
  const shaped = /^\(?\s*\d*d\d+/i;
  const bad: string[] = [];
  const walk = (o: unknown): void => {
    if (o === null || typeof o !== "object") return;
    for (const v of Object.values(o)) {
      if (typeof v === "string" && shaped.test(v.trim()) && parse(v) === undefined) bad.push(v);
      else if (typeof v === "object") walk(v);
    }
  };
  for (const r of pack.byId.values()) walk(r);
  assert.deepEqual(bad, [], "the fixture holds no dice-shaped prose");
});
