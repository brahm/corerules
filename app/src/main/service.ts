/**
 * Everything the main process answers, as functions over a Library.
 *
 * Separated from `main.ts` so that it is testable without a window, which is the same
 * discipline the engine follows: put the logic where it can be tested and leave the shell
 * with nothing to decide. What remains in `main.ts` is Electron wiring — a window, five
 * `ipcMain.handle` lines, and where the settings file lives.
 */
import { Character } from "../../../engine/src/character.ts";
import { advance as whatNext, check, draftOf, type Advance, type Objection } from "../../../engine/src/advance.ts";
import { steps as offerSteps, type Draft, type Step } from "../../../engine/src/choice.ts";
import { derived, type Derived } from "../../../engine/src/derived.ts";
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

export interface Timeline {
  events: { id: string; rolls: { class: string; die: number }[]; chose: { kind: string; ref: string }[] }[];
  next: Advance;
  derived: Derived;
  /** §9.2: the same validation rules on both paths, so the sheet carries what the wizard would
   *  have refused — and separately what nobody can rule on. */
  objections: Objection[];
  caveats: Objection[];
}

export function timeline(library: Library, id: string): Timeline | undefined {
  const opened = library.open(id);
  const c = opened.character;
  if (c === undefined) return undefined;
  const said = check(c.pack, draftOf(c));
  return {
    events: c.file.events.map((e) => ({ id: e.id, rolls: e.rolls, chose: e.chose ?? [] })),
    next: whatNext(c.pack, c, c.classes()[0] ?? ""),
    derived: derived(c.pack, c),
    objections: said.objections,
    caveats: said.caveats,
  };
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
