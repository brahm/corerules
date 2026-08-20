/**
 * A Character: **not a snapshot but a sequence of Level Events, from which everything derived
 * is recomputed** (§6.3, §6.5).
 *
 * The reason is not tidiness. Under multi-class, hit points accrue by rolling the advancing
 * class's die and dividing by the number of classes — so a total cannot be reconstructed from
 * *"fighter 5 / mage 4"*. Storing current levels plus a hit point total is **not
 * implementable**: correcting a level would leave the Engine unable to recompute. **Hit points
 * are recorded randomness**, a third category beside a choice and a derivation, and they are
 * the reason the history is the file.
 *
 * Everything else is derived and never stored — levels, quarantine, provenance, dual-class
 * suppression.
 */
import type { Pack } from "./pack.ts";
import { Sheet } from "./sheet.ts";
import { groupsOf, type Id, type Record_ } from "./types.ts";
import { uuidv7 } from "./uuid.ts";

/** One advance. `rolls` is a list because **first level is one event that rolls every class's
 *  die**: the PHB sums 6, 5 and 2 across a fighter/thief/mage and divides the sum by three for
 *  4, where dividing each roll separately would give 3. One rule covers both — floor of the
 *  event's total over the class count — and it only covers both because the event holds the
 *  rolls together. */
export interface LevelEvent {
  id: string;
  /** Which class went up. Several only at creation, where all of them do. */
  rolls: { class: Id; die: number }[];
  /** What was chosen, by kind: proficiencies, spells, a weapon to specialise in. */
  chose?: { kind: string; ref: Id }[];
  note?: string;
}

/** The file, exactly. §6.5: a single JSON document, legible in a text editor. */
export interface CharacterFile {
  id: string;
  name: string;
  /** §6.5: references to pack entries are live, and the file records a content hash of each
   *  pack it was validated against — a declared version would miss every typo fix, because
   *  nobody bumps a version for one. Snapshotting pack DATA in here is forbidden: a Character
   *  carrying pack data is a pack in disguise. */
  packs: { id: string; sha256?: string }[];
  /** §5.5: which optional rules this character's table plays. A sheet built under weapon
   *  specialization is not the same sheet without it. */
  options: string[];
  scores: Record<Id, number>;
  race: Id;
  subrace?: Id;
  /** §6.4: one kit, chosen at creation, never rebound. Abandonment leaves a nominal debt. */
  kit?: Id;
  kitAbandoned?: boolean;
  deity?: Id;
  alignment?: Id;
  /** §9.1's starting money, and it is **recorded randomness** for the same reason hit points
   *  are: the pack says `5d4x10` and the roll happened once. Storing the number is the only
   *  way to have it survive a correction to anything else. */
  startingWealth?: number;
  /** Chronological, because a v7 sorts that way — no separate ordering field to disagree. */
  events: LevelEvent[];
}

export class Character {
  readonly pack: Pack;
  readonly file: CharacterFile;

  constructor(pack: Pack, file: CharacterFile) {
    this.pack = pack;
    this.file = file;
  }

  static create(pack: Pack, init: Omit<CharacterFile, "id" | "events" | "packs" | "options"> &
                Partial<Pick<CharacterFile, "packs" | "options">>): Character {
    return new Character(pack, {
      id: uuidv7(),
      packs: init.packs ?? [{ id: pack.manifest.id }],
      options: init.options ?? [],
      events: [],
      ...init,
    });
  }

  /** Add an advance. The die roll is handed in, never rolled here: it is recorded randomness,
   *  and an Engine that rolled it would be unable to record a roll made at the table. */
  advance(rolls: LevelEvent["rolls"], extra: Omit<LevelEvent, "id" | "rolls"> = {}): LevelEvent {
    const event: LevelEvent = { id: uuidv7(), rolls, ...extra };
    this.file.events.push(event);
    return event;
  }

  /** Derived. The classes this character advances in, in the order first seen. */
  classes(): Id[] {
    const out: Id[] = [];
    for (const e of this.file.events) {
      for (const r of e.rolls) if (!out.includes(r.class)) out.push(r.class);
    }
    return out;
  }

  /** Derived. Never stored — correcting an event has to change this, which is the whole
   *  argument for keeping the history. */
  levels(): Record<Id, number> {
    const out: Record<Id, number> = {};
    for (const e of this.file.events) {
      for (const r of e.rolls) out[r.class] = (out[r.class] ?? 0) + 1;
    }
    return out;
  }

  /**
   * Derived. Each event contributes `floor(sum of its rolls / number of classes)`.
   *
   * The PHB's worked example is the test: a fighter/thief/mage rolls 6, 5 and 2 at first
   * level and begins with **4** — the sum divided by three — and a later thief advance
   * rolling 4 adds **1**. Dividing each roll separately would have given 3 at creation, so the
   * event must hold its rolls together.
   *
   * Constitution is not in here yet: the bonus is a table read with a per-class cap, and
   * pretending otherwise would be the kind of quiet wrong number the rest of this refuses.
   */
  hitPoints(): number {
    const n = Math.max(1, this.classes().length);
    let total = 0;
    for (const e of this.file.events) {
      total += Math.floor(e.rolls.reduce((s, r) => s + r.die, 0) / n);
    }
    return total;
  }

  /** The layer stack §4 walks, in order. Order never changes the answer — the six operations
   *  commute — so this is for reading, not for semantics. */
  /** `against` is correction 17's second subject: who the question is about, never a layer. */
  sheet(against?: Id): Sheet {
    const s = new Sheet(this.pack, {
      scores: this.file.scores,
      levels: this.levels(),
      options: this.file.options,
      ...(against !== undefined ? { against } : {}),
    });
    s.apply(this.pack.get(this.file.race, "the character's race"), "race");
    if (this.file.subrace !== undefined) {
      s.apply(this.pack.get(this.file.subrace, "the character's subrace"), "subrace");
    }
    for (const id of this.classes()) {
      const c: Record_ | undefined = this.pack.get(id, "a class the character advances in");
      for (const g of groupsOf(c)) s.apply(this.pack.get(g, "a class group"), "class group");
      s.apply(c, "class");
    }
    if (this.file.alignment !== undefined) {
      // Alignment is a layer with no effects in this corpus, and it is applied all the same:
      // a kit's prerequisite reads `{field: "alignment"}`, and the predicate reads it off the
      // sheet rather than off the file.
      s.apply(this.pack.byId.get(this.file.alignment), "choice");
    }
    if (this.file.deity !== undefined) {
      s.apply(this.pack.get(this.file.deity, "the character's deity"), "deity");
    }
    // §6.4: an abandoned kit removes all standing modifiers, benefits and penalties alike,
    // and leaves its granted proficiencies as a debt. Dropping the layer is the whole of the
    // first half — which is what §4 said layering was for.
    if (this.file.kit !== undefined && this.file.kitAbandoned !== true) {
      s.apply(this.pack.get(this.file.kit, "the character's kit"), "kit");
    }
    return s;
  }

  /**
   * §6.4's nominal debt: **those specific proficiencies, not a count.**
   *
   * A Character carrying debt is valid and never quarantined — quarantine would deadlock,
   * because the debt is only payable by levelling and quarantine locks levelling. The debt
   * may also be **unpayable**, if the kit granted something the class cannot take, and the
   * sheet has to show it or it becomes a phantom bug.
   */
  debt(): Id[] {
    if (this.file.kit === undefined || this.file.kitAbandoned !== true) return [];
    const kit = this.pack.byId.get(this.file.kit);
    const owed = (kit?.effects ?? [])
      .filter((e) => e.op === "grant" && e.kind === "nonweaponProficiency" && e.ref !== undefined)
      .map((e) => e.ref!);
    const paid = this.file.events.flatMap((e) => e.chose ?? [])
      .filter((c) => c.kind === "nonweaponProficiency").map((c) => c.ref);
    const remaining = [...owed];
    for (const p of paid) {
      const at = remaining.indexOf(p);
      if (at >= 0) remaining.splice(at, 1);
    }
    return remaining;
  }
}
