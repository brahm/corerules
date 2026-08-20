/**
 * Rolling dice, which this Engine has specified twice and never done.
 *
 * §9.1 says *"the tool rolls dice, and entry stays a first-class path"*, and correction 15 put a
 * `dice` pattern in the schema and widened it twice — once for `NdM x k` (4.7% of 2,574 corpus
 * notations, which the stated grammar rejected) and once for `(NdM ± J) x k`, because grouping
 * changes the arithmetic. **Nothing ever executed it.** Every roll in this project so far is
 * `1 + Math.floor(Math.random() * sides)` written inline in a React component, which cannot
 * express `5d4 x 10` and silently would not try.
 *
 * So the grammar has been validated against the corpus for months and run against nothing. This
 * is the other half.
 *
 * **The result is a list, not a number.** §6.3 records randomness rather than reproducing it, and
 * a player who rolled five 4s wants to see five 4s — a total of 200 gp with no dice behind it is
 * indistinguishable from a number the tool made up.
 */

export interface Roll {
  /** What each die showed, in the order thrown. */
  dice: number[];
  /** The number the notation produces: `(sum + plus) * times`. */
  total: number;
  /** The notation this came from, as the pack prints it. */
  notation: string;
}

export interface Notation {
  count: number;
  sides: number;
  plus: number;
  times: number;
}

/**
 * Both shapes the corpus writes, and the difference between them is not cosmetic:
 * `1d4+1 x 10` and `(1d4+1) x 10` are 14 and 50 at the extreme, and the wizard's standard
 * starting money is the second one.
 */
const FLAT = /^(\d*)d(\d+)(?:\s*([+-])\s*(\d+))?(?:\s*[x*]\s*(\d+))?$/i;
const GROUPED = /^\(\s*(\d*)d(\d+)(?:\s*([+-])\s*(\d+))?\s*\)\s*[x*]\s*(\d+)$/i;

/** Strips what the books print around the notation — `5d4 x 10 gp`, `1d6 turns`. */
const bare = (s: string): string => s.trim().replace(/\s*[a-z]{2,}\.?$/i, "").trim();

export function parse(notation: string): Notation | undefined {
  const text = bare(notation);
  const m = GROUPED.exec(text) ?? FLAT.exec(text);
  if (m === null) return undefined;
  const sign = m[3] === "-" ? -1 : 1;
  return {
    // `d10` with no count is one die: the corpus writes it and correction 15 admits it.
    count: m[1] === "" || m[1] === undefined ? 1 : Number.parseInt(m[1], 10),
    sides: Number.parseInt(m[2]!, 10),
    plus: m[4] === undefined ? 0 : sign * Number.parseInt(m[4], 10),
    // A flat `NdM+J x K` multiplies only the modifier's side of nothing — the corpus does not
    // write it, and the grouped form is the one that means "multiply the whole thing".
    times: m[5] === undefined ? 1 : Number.parseInt(m[5], 10),
  };
}

/**
 * Roll it. `random` is an argument because a test that cannot fix the dice is a test of luck,
 * and because §6.3's entry path hands in numbers somebody else threw.
 */
export function roll(notation: string, random: () => number = Math.random): Roll | undefined {
  const n = parse(notation);
  if (n === undefined) return undefined;
  const dice: number[] = [];
  for (let i = 0; i < n.count; i++) dice.push(1 + Math.floor(random() * n.sides));
  const sum = dice.reduce((a, b) => a + b, 0);
  return { dice, total: (sum + n.plus) * n.times, notation: notation.trim() };
}

/** The most and least a notation can produce, for a sheet that wants to say what a roll was against. */
export function range(notation: string): { least: number; most: number } | undefined {
  const n = parse(notation);
  if (n === undefined) return undefined;
  return {
    least: (n.count + n.plus) * n.times,
    most: (n.count * n.sides + n.plus) * n.times,
  };
}
