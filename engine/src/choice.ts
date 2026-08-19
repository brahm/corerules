/**
 * What may be chosen next, and for everything that may not, **which rule refused and which
 * book that rule came from** (§1, §5).
 *
 * This is the whole of §9.2's guided creation that is worth testing. The wizard's screens are
 * an arrangement of these offers; the offers are where the rules live. §5's posture is that
 * illegal states are unrepresentable **at the point of choice, not flagged afterwards**, so an
 * option a character cannot take is not presented and then rejected — it arrives already
 * carrying the reason.
 *
 * **An offer has three states, not two**, and the third is the one that makes this honest:
 *
 *   yes        the rule holds
 *   no         the rule refuses, and says which rule and which book
 *   unknown    nobody can say — either the predicate asks something a half-built character
 *              cannot answer yet, or A3 (§5.1) means the pack never claimed to cover this
 *
 * A tool that collapsed `unknown` into `no` would forbid what the books allow; one that
 * collapsed it into `yes` would be the "flagged afterwards" §5 exists to prevent. So it stays
 * a third state, and the interface has to show it.
 */
import type { Pack } from "./pack.ts";
import { cost as costOf, slots as slotsFor } from "./proficiency.ts";
import { predicate, type Subject } from "./predicate.ts";
import { groupsOf, type Id, type Record_ } from "./types.ts";

export type Availability = "yes" | "no" | "unknown";

export interface Offer {
  id: Id;
  name: string;
  book: string;
  available: Availability;
  /** Present unless the answer is a plain yes. The rule, in the book's own terms where the
   *  pack gives them, and always the book. */
  because?: string;
}

/** What a character-in-progress knows about itself. Everything is optional, because a wizard
 *  asks in an order and each answer narrows the next. */
export interface Draft {
  scores?: Record<Id, number>;
  race?: Id;
  subrace?: Id;
  class?: Id;
  kit?: Id;
  /** What has been spent so far. A list rather than a set because the same kind may be
   *  chosen several times, and order is the order the player picked. */
  chose?: { kind: string; ref: Id }[];
}

const bookOf = (r: Record_): string => r.provenance?.section[0] ?? "?";

function subject(pack: Pack, draft: Draft): Subject {
  return {
    ability: (id) => draft.scores?.[id],
    level: () => 1,
    field: (path) => (path === "race" ? draft.race : path === "subrace" ? draft.subrace : undefined),
    has: () => false,
  };
}

/** The classes a record targets, expanded through groups and multi-class arms — the same walk
 *  a kit's `target` needs, because a kit for `phb:fighter` is offered to a Fighter/Thief. */
function targets(pack: Pack, target: Id): Set<Id> {
  const out = new Set<Id>();
  for (const c of pack.records("classes")) {
    if (c.isGroup === true) continue;
    const chain = new Set<Id>([c.id]);
    for (const g of groupsOf(c)) chain.add(g);
    for (const m of c.combines ?? []) {
      chain.add(m);
      for (const g of groupsOf(pack.byId.get(m))) chain.add(g);
    }
    if (c.variantOf !== undefined) chain.add(c.variantOf);
    if (chain.has(target)) out.add(c.id);
  }
  return out;
}

/**
 * §5.1's A3, at the point of choice.
 *
 * A pack that declares a rule-set makes absence from its permit-list a hard block; a pack that
 * declares nothing about a rule means **the Engine does not validate that rule and says so
 * visibly**. The proving slice declares nothing at all, so every race-and-class combination is
 * offered and every one of them says why it was not checked. That is not a bug to be tidied
 * away later — it is the difference between *"the books allow this"* and *"nobody has told me
 * whether the books allow this"*, and losing it is what A3 exists to prevent.
 */
export function undeclared(pack: Pack, ruleSet: string): boolean {
  return !(pack.manifest.declares ?? []).includes(ruleSet);
}

export function races(pack: Pack): Offer[] {
  return pack.records("races").map((r) => ({
    id: r.id, name: r.name, book: bookOf(r), available: "yes" as const,
  }));
}

export function subraces(pack: Pack, draft: Draft): Offer[] {
  return pack.records("subraces")
    .filter((s) => s.target === draft.race)
    .map((s) => ({ id: s.id, name: s.name, book: bookOf(s), available: "yes" as const }));
}

export function classes(pack: Pack, draft: Draft): Offer[] {
  const on = subject(pack, draft);
  return pack.records("classes")
    .filter((c) => c.isGroup !== true)
    .map((c): Offer => {
      const minimums = (c["abilityMinimums"] as { ability: Id; minimum: number }[] | undefined) ?? [];
      const short = minimums.filter((m) => {
        const score = draft.scores?.[m.ability];
        return score !== undefined && score < m.minimum;
      });
      if (short.length > 0) {
        const named = short
          .map((m) => `${pack.byId.get(m.ability)?.name ?? m.ability} ${m.minimum}`)
          .join(", ");
        return { id: c.id, name: c.name, book: bookOf(c), available: "no", because: `requires ${named} — ${bookOf(c)}` };
      }
      if (minimums.some((m) => draft.scores?.[m.ability] === undefined)) {
        return { id: c.id, name: c.name, book: bookOf(c), available: "unknown", because: "its ability requirements have not been rolled yet" };
      }
      // Race and class permission is a rule-set of its own, and this pack has not claimed it.
      if (draft.race !== undefined && undeclared(pack, "race-class-permission")) {
        return {
          id: c.id, name: c.name, book: bookOf(c), available: "unknown",
          because: "no loaded pack declares which races may take which classes, so corerules is not checking it",
        };
      }
      return { id: c.id, name: c.name, book: bookOf(c), available: "yes" };
    });
}

export function kits(pack: Pack, draft: Draft): Offer[] {
  const on = subject(pack, draft);
  return pack.records("kits").map((k): Offer => {
    const book = bookOf(k);
    if (draft.class !== undefined && k.target !== undefined && !targets(pack, k.target).has(draft.class)) {
      const target = pack.byId.get(k.target)?.name ?? k.target;
      return { id: k.id, name: k.name, book, available: "no", because: `belongs to the ${target} — ${book}` };
    }
    const holds = predicate(k.prerequisite as Parameters<typeof predicate>[0], on);
    if (holds === false) {
      return { id: k.id, name: k.name, book, available: "no", because: `its prerequisite does not hold — ${book}` };
    }
    if (holds === null) {
      return { id: k.id, name: k.name, book, available: "unknown", because: "its prerequisite asks something this character has not decided yet" };
    }
    return { id: k.id, name: k.name, book, available: "yes" };
  });
}

/** The steps, and whether each is answerable yet. §9.2's wizard is guided, which means it
 *  knows what it is waiting for. */
export interface Step {
  key: "scores" | "race" | "subrace" | "class" | "kit" | "proficiencies";
  title: string;
  /** Answered, ready to answer, or waiting on an earlier step. */
  state: "done" | "ready" | "waiting";
  offers: Offer[];
  /** Only the proficiency step. What may still be spent, and on what — §9.1's slot budgets,
   *  and PHB DD01537's rule that **initial slots must be assigned immediately; they cannot be
   *  saved or held in reserve**, which is why this step is not `done` until `free` is nought. */
  budget?: { total: number; spent: number; free: number };
}

/**
 * The proficiency step, which is a budget rather than a list of yes-and-no.
 *
 * A candidate that would cost more than remains is refused **by the budget**, not by a rule in
 * a book — so it says so in those terms. And a cost the books cannot decide (correction 60)
 * arrives as `unknown`, because spending a slot you may not owe is exactly as wrong as being
 * refused one you could afford.
 */
function proficiencyStep(pack: Pack, draft: Draft): Omit<Step, "key" | "title"> {
  if (draft.class === undefined) return { state: "waiting", offers: [] };
  const total = slotsFor(pack, draft.class);
  const chosen = (draft.chose ?? []).filter((c) => c.kind === "nonweaponProficiency");
  const spent = chosen.reduce((n, c) => n + costOf(pack, c.ref, draft.class!).cost, 0);
  const free = total - spent;
  const taken = new Set(chosen.map((c) => c.ref));

  const offers = pack.records("nonweaponProficiencies")
    .filter((p) => !taken.has(p.id))
    .map((p): Offer => {
      const { cost, because, certain } = costOf(pack, p.id, draft.class!);
      const label = `${cost} slot${cost === 1 ? "" : "s"}${certain ? "" : ", perhaps one more"} — ${because}`;
      if (cost > free) {
        return { id: p.id, name: p.name, book: bookOf(p), available: "no",
                 because: `${cost} slots, and ${free} left` };
      }
      return { id: p.id, name: p.name, book: bookOf(p),
               available: certain ? "yes" : "unknown", because: label };
    });

  return { state: free === 0 ? "done" : "ready", offers, budget: { total, spent, free } };
}

export function steps(pack: Pack, draft: Draft): Step[] {
  const has = (v: unknown): boolean => v !== undefined;
  const sub = subraces(pack, draft);
  return [
    {
      key: "scores", title: "Ability scores",
      state: has(draft.scores) ? "done" : "ready",
      // The abilities come from the PACK, because their ids do: the Player's Handbook's are
      // `phb:strength` and another book's are its own. An interface that assembled them from
      // a known prefix would be the Engine deciding what a pack contains, which is the one
      // thing §3.4's open enumerations exist to prevent.
      offers: pack.records("abilities").map((a) => ({
        id: a.id, name: a.name, book: bookOf(a), available: "yes" as const,
      })),
    },
    { key: "race", title: "Race", state: has(draft.race) ? "done" : "ready", offers: races(pack) },
    // A race with no subraces in any loaded pack does not get a step to skip past.
    ...(sub.length > 0
      ? [{ key: "subrace" as const, title: "Subrace",
           state: has(draft.subrace) ? "done" as const : "ready" as const, offers: sub }]
      : []),
    {
      key: "class", title: "Class",
      state: has(draft.class) ? "done" : has(draft.scores) && has(draft.race) ? "ready" : "waiting",
      offers: has(draft.scores) || has(draft.race) ? classes(pack, draft) : [],
    },
    {
      key: "proficiencies", title: "Nonweapon proficiencies",
      ...proficiencyStep(pack, draft),
    },
    {
      key: "kit", title: "Kit",
      // §6.4: one kit, chosen at creation, never rebound. A step you can decline, not skip.
      state: has(draft.kit) ? "done" : has(draft.class) ? "ready" : "waiting",
      offers: has(draft.class) ? kits(pack, draft) : [],
    },
  ];
}
