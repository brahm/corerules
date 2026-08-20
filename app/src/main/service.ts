/**
 * Everything the main process answers, as functions over a Library.
 *
 * Separated from `main.ts` so that it is testable without a window, which is the same
 * discipline the engine follows: put the logic where it can be tested and leave the shell
 * with nothing to decide. What remains in `main.ts` is Electron wiring — a window, five
 * `ipcMain.handle` lines, and where the settings file lives.
 */
import { Character } from "../../../engine/src/character.ts";
import { advance as whatNext, check, correct, draftOf, type Advance, type Objection } from "../../../engine/src/advance.ts";
import { steps as offerSteps, type Draft, type Step } from "../../../engine/src/choice.ts";
import { derived, type Derived } from "../../../engine/src/derived.ts";
import { available, type SpellOffer } from "../../../engine/src/spells.ts";
import type { Library } from "../../../engine/src/library.ts";
import { present, type SheetView } from "../../../engine/src/present.ts";
import type { CharacterSummary, PackSummary } from "./api.ts";

export function packs(library: Library): PackSummary[] {
  return library.packs().map((entry) => {
    const pack = library.load(entry.id);
    return {
      id: entry.id,
      name: pack.manifest.name,
      directory: entry.directory,
      hash: entry.hash.slice(0, 12),
      records: pack.byId.size,
      // A pack's complaints belong on the screen and not in a log. §1's promise is that the
      // Engine says what it could not do, and a pack that contradicts itself is that case.
      complaints: pack.complaints.map((c) => `[${c.area}] ${c.message}`),
    };
  });
}

export function characters(library: Library): CharacterSummary[] {
  return library.characterIds().map((id) => {
    const opened = library.open(id);
    return {
      id,
      name: opened.file.name,
      // §6.5: loading never fails. A Character whose packs are gone still opens, still has a
      // name, and says what it cannot find rather than refusing to appear.
      who: opened.character === undefined
        ? "— its packs are not here —"
        : opened.character.sheet().layers.map((l) => l.record.name).join(" / "),
      hitPoints: opened.character?.hitPoints() ?? 0,
      drift: opened.drift.map((d) => ({ pack: d.pack, lost: d.lost })),
    };
  });
}

export function open(library: Library, id: string): SheetView | undefined {
  const opened = library.open(id);
  return opened.character === undefined ? undefined : present(opened.character);
}

export function steps(library: Library, packId: string, draft: Draft): Step[] {
  return offerSteps(library.load(packId), draft);
}

/**
 * Write a new Character, at first level, with the die already rolled.
 *
 * The roll arrives from outside because **hit points are recorded randomness** (§6.3): an
 * Engine that rolled them itself could not record a roll made at the table, and §9.1 keeps
 * entry a first-class path beside the dice.
 */
export function create(
  library: Library, packId: string, draft: Draft & { name: string; startingWealth?: number }, hitDie: number,
): string {
  const pack = library.load(packId);
  const character = Character.create(pack, {
    name: draft.name,
    race: draft.race!,
    scores: draft.scores ?? {},
    ...(draft.subrace !== undefined ? { subrace: draft.subrace } : {}),
    ...(draft.kit !== undefined ? { kit: draft.kit } : {}),
    ...(draft.alignment !== undefined ? { alignment: draft.alignment } : {}),
    ...(draft.startingWealth !== undefined ? { startingWealth: draft.startingWealth } : {}),
    packs: [{ id: packId }],
  });
  // §6.3: what was chosen travels in the event that chose it, so a correction to the level
  // corrects the choices with it. And §9.1's initial slots were assigned before this was
  // called, because DD01537 says they cannot be held in reserve.
  character.advance([{ class: draft.class!, die: hitDie }],
    draft.chose !== undefined && draft.chose.length > 0 ? { chose: draft.chose } : {});
  library.writeCharacter(library.stamp(character.file));
  return character.file.id;
}

export interface WearOffer {
  id: string;
  name: string;
  /** Where it goes, which is what decides the column Table 46 is read in. */
  worn: string;
  cost?: string;
  weight?: string;
  chosen: boolean;
}

export interface Timeline {
  events: { id: string; rolls: { class: string; die: number }[]; chose: { kind: string; ref: string }[] }[];
  /** Every class the loaded packs offer, so a correction picks from a list rather than typing
   *  an id. The rules still decide whether the pick stands — this is only the vocabulary. */
  classes: { id: string; name: string }[];
  next: Advance;
  derived: Derived;
  /** §9.2: the same validation rules on both paths, so the sheet carries what the wizard would
   *  have refused — and separately what nobody can rule on. */
  objections: Objection[];
  caveats: Objection[];
  /** What this priest's god lets them learn, and on what terms. Empty for anyone without one. */
  spells: SpellOffer[];
  /** Correction 61: the armour the loaded packs offer, and what this character has on. */
  wear: WearOffer[];
}

export function timeline(library: Library, id: string): Timeline | undefined {
  const opened = library.open(id);
  const c = opened.character;
  if (c === undefined) return undefined;
  const said = check(c.pack, draftOf(c));
  return {
    events: c.file.events.map((e) => ({ id: e.id, rolls: e.rolls, chose: e.chose ?? [] })),
    classes: c.pack.records("classes").filter((k) => k.isGroup !== true)
      .map((k) => ({ id: k.id, name: k.name })),
    next: whatNext(c.pack, c, c.classes()[0] ?? ""),
    derived: derived(c.pack, c),
    objections: said.objections,
    caveats: said.caveats,
    spells: available(c.pack, c),
    wear: c.pack.records("armor")
      .filter((a) => a["worn"] !== undefined && a["armorKind"] === "item")
      .map((a) => ({
        id: a.id, name: a.name, worn: a["worn"] as string,
        ...(a["cost"] !== undefined ? { cost: a["cost"] as string } : {}),
        ...(a["weight"] !== undefined ? { weight: a["weight"] as string } : {}),
        chosen: (c.file.worn ?? []).includes(a.id),
      })),
  };
}

/**
 * §9.2's third mode, doing something at last: **rewrite one level in place.**
 *
 * §6.5 is explicit that this is what a correction is — *"corrections rewrite history in place;
 * the old value stops existing"* — and §9.2 is explicit that it may not be a way around the
 * rules. So the edit is applied, the whole Character is re-checked by the same `choice.ts` the
 * wizard uses, and the objections come back with it. Nothing is refused: refusing would leave
 * the user with a sheet they can see is wrong and no way to say so.
 */
export function correctEvent(
  library: Library, id: string, eventId: string, replacement: { class?: string; die?: number },
): Objection[] {
  const opened = library.open(id);
  const c = opened.character;
  if (c === undefined) return [];
  const event = c.file.events.find((e) => e.id === eventId);
  if (event === undefined) return [];
  const first = event.rolls[0];
  if (first !== undefined) {
    // Only the first roll: a multi-class creation event rolls several dice at once, and
    // editing one of them is a finer operation than this screen offers yet.
    const said = correct(c.pack, c, eventId, {
      rolls: [{ class: replacement.class ?? first.class, die: replacement.die ?? first.die },
              ...event.rolls.slice(1)],
    });
    library.writeCharacter(library.stamp(c.file));
    return said.filter((o) => o.available === "no");
  }
  return [];
}

/**
 * Remove a level. Also a correction — §6.5 does not distinguish, and an advance made by
 * mistake is the commonest kind there is.
 *
 * The choices made at that level go with it, because they travelled in the event that chose
 * them. That is the whole reason §6.3 put them there.
 */
export function removeEvent(library: Library, id: string, eventId: string): Objection[] {
  const opened = library.open(id);
  const c = opened.character;
  if (c === undefined) return [];
  c.file.events = c.file.events.filter((e) => e.id !== eventId);
  library.writeCharacter(library.stamp(c.file));
  return check(c.pack, draftOf(c)).objections;
}

/** One level, with the die rolled outside — §6.3's recorded randomness, again. */
export function levelUp(
  library: Library, id: string, classId: string, die: number, chose: { kind: string; ref: string }[],
): Objection[] {
  const opened = library.open(id);
  const c = opened.character;
  if (c === undefined) return [];
  c.advance([{ class: classId, die }], chose.length > 0 ? { chose } : {});
  const said = check(c.pack, draftOf(c));
  // §5.3: a Character that breaks a rule is still written. What is locked is what EXTENDS it,
  // and the sheet shows why — refusing the save would hide the state instead of naming it.
  library.writeCharacter(library.stamp(c.file));
  return said.objections;
}

/**
 * Correction 61: put armour on, or take it off.
 *
 * Not a Level Event and not a correction to one. §6.3's history is what the RULES derive from,
 * and what a character is wearing this afternoon derives nothing about their level — recording it
 * as an event would put a change of clothes in the same list as becoming a 5th-level fighter.
 */
export function wear(library: Library, id: string, worn: string[]): void {
  const opened = library.open(id);
  const c = opened.character;
  if (c === undefined) return;
  if (worn.length === 0) delete c.file.worn;
  else c.file.worn = worn;
  library.writeCharacter(library.stamp(c.file));
}
