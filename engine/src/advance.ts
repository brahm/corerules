/**
 * §9.2's other two modes: **advancing** a Character, and **correcting** one.
 *
 * They are one module because §9.2 says the thing that matters about them:
 *
 *   *"The same validation rules must hold on both paths, or sheet editing becomes the back
 *   door that undoes §5."*
 *
 * So neither of these has rules of its own. An advance asks `choice.ts` what may be chosen and
 * a correction asks it the same question about a rewritten past — the difference is only which
 * Character the question is about, and a correction's is a Character that does not exist yet.
 */
import type { Character, LevelEvent } from "./character.ts";
import { steps, type Draft, type Step } from "./choice.ts";
import type { Pack } from "./pack.ts";
import { slots } from "./proficiency.ts";
import { groupsOf, type Id } from "./types.ts";

export interface Advance {
  /** Which classes this Character may put the level into, each with **its own die** — §6.2
   *  averages hit points across hit dice, and a Fighter/Mage rolls a d10 and a d4, not one of
   *  something. A single die on this object read the sheet's `hitDice.perLevel`, which for a
   *  multi-class character is two layers setting one field: contested, and silently absent. */
  classes: { id: Id; name: string; level: number; die?: string }[];
  /** The single-class die, kept for the callers that ask one character one question. */
  die?: string;
  /** What crossing this level buys, which is nought most levels and is the whole reason the
   *  mini-wizard exists on the ones where it is not. */
  gains: { kind: "weaponProficiency" | "nonweaponProficiency"; slots: number }[];
}

/** What the next level in a given class brings. */
export function advance(pack: Pack, character: Character, classId: Id): Advance {
  const levels = character.levels();
  const at = levels[classId] ?? 0;
  const gains: Advance["gains"] = [];
  for (const which of ["weapon", "nonweapon"] as const) {
    const before = slots(pack, classId, Math.max(1, at), which);
    const after = slots(pack, classId, at + 1, which);
    if (after > before) {
      gains.push({
        kind: which === "weapon" ? "weaponProficiency" : "nonweaponProficiency",
        slots: after - before,
      });
    }
  }
  const classes = character.classes().map((id) => {
    const die = dieFor(pack, id);
    return {
      id, name: pack.byId.get(id)?.name ?? id, level: levels[id] ?? 0,
      ...(die !== undefined ? { die } : {}),
    };
  });
  return {
    classes,
    ...(classes.length === 1 && classes[0]!.die !== undefined ? { die: classes[0]!.die } : {}),
    gains,
  };
}

/**
 * The die one class rolls, read off the class group rather than off the sheet.
 *
 * The sheet is the wrong place to ask: a Fighter/Mage applies both class-group layers, both of
 * which `set hitDice.perLevel`, and ticket 03 quite correctly calls that **contested** — two
 * layers writing one field with nothing declaring over them. It is not a contradiction, though.
 * It is two answers to a question that was asked of the wrong subject. The die belongs to a
 * class, not to a character, and asking per class is the whole repair.
 */
export function dieFor(pack: Pack, classId: Id): string | undefined {
  const klass = pack.byId.get(classId);
  const group = groupsOf(klass)[0] ?? (klass?.isGroup === true ? klass.id : undefined);
  for (const source of [klass, group === undefined ? undefined : pack.byId.get(group)]) {
    for (const e of source?.effects ?? []) {
      if (e.op === "set" && e.field === "hitDice.perLevel" && typeof e.to === "string") return e.to;
    }
  }
  return undefined;
}

/**
 * Whether a Character-as-it-would-be still passes the rules it was built under.
 *
 * This is §9.2's whole demand in one function. A correction rewrites history in place (§6.5)
 * and the old value stops existing, so the only defence against sheet editing becoming a back
 * door is to ask the SAME questions of the result — and to ask them of every step, because a
 * change to the ability scores reaches the class and a change to the class reaches the kit.
 */
export interface Objection {
  step: Step["key"];
  choice: Id;
  name: string;
  because: string;
  /** `no` is a refusal; `unknown` is *nobody can tell any more*, which after a correction is
   *  a different and quieter kind of bad news. Collapsing the two would either block an edit
   *  the books allow or wave through one they may not. */
  available: "no" | "unknown";
}

/**
 * Two lists, because the third state means something different here than it does in a wizard.
 *
 * An **objection** is a choice the books refuse. A **caveat** is a choice nobody can rule on —
 * and with A3 undeclared, that is *every class this corpus offers*, permanently. Putting the
 * two in one list would make a legally built Character look broken on every load, which is the
 * fastest way to teach a user to ignore the warnings that matter.
 */
export function check(pack: Pack, draft: Draft): { objections: Objection[]; caveats: Objection[] } {
  const all = objections(pack, draft);
  return {
    objections: all.filter((o) => o.available === "no"),
    caveats: all.filter((o) => o.available === "unknown"),
  };
}

export function objections(pack: Pack, draft: Draft): Objection[] {
  const out: Objection[] = [];
  for (const step of steps(pack, draft)) {
    const chosen: Id | undefined = step.key === "race" ? draft.race
      : step.key === "subrace" ? draft.subrace
      : step.key === "class" ? draft.class
      : step.key === "kit" ? draft.kit
      : step.key === "deity" ? draft.deity
      : step.key === "alignment" ? draft.alignment
      : undefined;
    if (chosen === undefined) continue;
    const offer = step.offers.find((o) => o.id === chosen);
    if (offer === undefined) {
      out.push({
        step: step.key, choice: chosen, name: chosen, available: "no",
        // A choice that is not on the list at all is the drift case: the pack moved under a
        // Character that already had it. §6.5 says loading never fails, so this is reported
        // and not thrown.
        because: "no loaded pack offers this any more",
      });
    } else if (offer.available !== "yes") {
      out.push({
        step: step.key, choice: chosen, name: offer.name,
        because: offer.because ?? "refused", available: offer.available,
      });
    }
  }
  // The budgets are rules too, and DD01537's is the one a correction is most likely to break:
  // lowering a level takes slots away from choices already made.
  for (const step of steps(pack, draft)) {
    if (step.budget !== undefined && step.budget.free < 0) {
      out.push({
        step: step.key, choice: "", name: step.title, available: "no",
        because: `${step.budget.spent} slots spent and only ${step.budget.total} to spend`,
      });
    }
  }
  return out;
}

/** The draft a Character amounts to, so a correction can be checked before it is written. */
export function draftOf(character: Character): Draft {
  const f = character.file;
  return {
    scores: f.scores,
    race: f.race,
    ...(f.subrace !== undefined ? { subrace: f.subrace } : {}),
    ...(f.kit !== undefined && f.kitAbandoned !== true ? { kit: f.kit } : {}),
    ...(f.alignment !== undefined ? { alignment: f.alignment } : {}),
    ...(f.deity !== undefined ? { deity: f.deity } : {}),
    ...(character.classes()[0] !== undefined ? { class: character.classes()[0]! } : {}),
    chose: f.events.flatMap((e) => e.chose ?? []),
  };
}

/**
 * Rewrite one event in place, then say what the result breaks.
 *
 * §6.5 chose this over an audit trail deliberately: *"the old value stops existing;
 * auditability is deliberately traded for a file that stays legible in a text editor."* So the
 * correction is applied and the objections come back beside it — the caller decides whether to
 * keep it, and nothing is silently prevented or silently allowed.
 */
export function correct(
  pack: Pack, character: Character, eventId: string, replacement: Partial<LevelEvent>,
): Objection[] {
  const at = character.file.events.findIndex((e) => e.id === eventId);
  if (at < 0) return [{ step: "class", choice: eventId, name: eventId, because: "no such event", available: "no" }];
  const event = character.file.events[at]!;
  character.file.events[at] = { ...event, ...replacement, id: event.id };
  return objections(pack, draftOf(character));
}
