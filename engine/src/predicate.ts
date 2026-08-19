/**
 * §6.1's predicate, and the one thing about it that matters most: **it is three-valued.**
 *
 * A clause can hold, fail, or be *undecidable* — the pack asks about something the sheet has
 * no answer for. Undecidable is not false. Treating it as false silently drops an effect that
 * may well apply, which is exactly the class of wrong number §5.2 exists to name, so it
 * travels as its own value and the caller has to say what it does about it.
 */
import type { Clause, Condition, Id, Operand, Predicate, Scalar } from "./types.ts";

/** null is UNDECIDABLE. */
export type Truth = boolean | null;

/** What a predicate may ask about. Named for §6.1's own word: a condition has a `subject`.
 *  Deliberately an interface of readers rather than a data object, so the Sheet can answer
 *  `field` by computing a view — a predicate that reads a field is reading the layer stack. */
export interface Subject {
  /** Ability scores by id — `phb:strength`. */
  ability(id: Id): number | undefined;
  /** Levels in a class by id, 0 where the character has none of it. */
  level(id: Id): number;
  /** A field the layers have already computed, or undefined where nothing has. */
  field(path: string): number | string | undefined;
  /** Whether the character has been granted a thing. */
  has(kind: string, ref: Id): boolean;
}

export function scalar(s: Scalar, on: Subject): number | string | undefined {
  if ("ability" in s) return on.ability(s.ability);
  if ("level" in s) return on.level(s.level);
  return on.field(s.field);
}

function compare(op: "gte" | "lte" | "eq" | "neq", left: number | string, right: Operand): Truth {
  // A comparison the corpus makes is always against a plain number: `Wisdom >= 16`,
  // `level >= 9`. The other operand shapes exist for what an effect WRITES, not for what a
  // predicate tests, and a predicate that reaches one is asking a question this cannot
  // answer rather than a question with the answer false.
  if (typeof right !== "number" || typeof left !== "number") return null;
  switch (op) {
    case "gte": return left >= right;
    case "lte": return left <= right;
    case "eq": return left === right;
    case "neq": return left !== right;
  }
}

export function holds(c: Condition, on: Subject): Truth {
  if ("anyOfIds" in c) {
    const v = scalar(c.member, on);
    if (v === undefined) return null;
    return c.anyOfIds.includes(String(v));
  }
  if ("has" in c) return on.has(c.has, c.ref);
  const v = scalar(c.subject, on);
  if (v === undefined) return null;
  return compare(c.op, v, c.value);
}

/** Ticket 13 finding 10: one level of disjunction, no recursion, which is what keeps the
 *  evaluator a loop rather than a recursive descent. */
export function clause(cl: Clause, on: Subject): Truth {
  if ("anyOf" in cl) {
    let undecided = false;
    for (const c of cl.anyOf) {
      const t = holds(c, on);
      if (t === true) return true;      // a disjunction that has found a witness is settled
      if (t === null) undecided = true;
    }
    return undecided ? null : false;
  }
  return holds(cl, on);
}

/** Every clause must hold. Absent means yes. */
export function predicate(p: Predicate | undefined, on: Subject): Truth {
  if (p === undefined || p.length === 0) return true;
  let undecided = false;
  for (const cl of p) {
    const t = clause(cl, on);
    if (t === false) return false;      // a conjunction that has found a falsifier is settled
    if (t === null) undecided = true;
  }
  return undecided ? null : true;
}
