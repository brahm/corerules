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
  /** Correction 61: what the character wears, which Table 46 can now be asked about. Armour
   *  only — the rest of §9.1's equipment is still a corpus away. */
  worn?: Id[];
  /** §9.1's starting money, **as rolled**. Not to be confused with the `startingWealth` FIELD,
   *  which 38 kits write and which holds a die — `4d4x10`. One is the question and the other is
   *  the answer, and they had the same name until something finally rolled one. */
  funds?: number;
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
   * Derived. Each event contributes `floor(sum of its rolls / number of classes)`, plus what
   * Constitution says, and **the roll may be raised before it is counted**.
   *
   * The PHB's worked example is the test: a fighter/thief/mage rolls 6, 5 and 2 at first
   * level and begins with **4** — the sum divided by three — and a later thief advance
   * rolling 4 adds **1**. Dividing each roll separately would have given 3 at creation, so the
   * event must hold its rolls together.
   *
   * **Constitution enters twice**, which is correction 66 and is why this took so long to be
   * safe to add. Table 3's Hit Point Adjustment column holds two numbers in one cell —
   * `+2 (+4)*`, where the parenthetical is warriors only — and the star COUNT is a different
   * rule again: at Constitution 20 every 1 rolled for a Hit Die counts as a 2, at 21 every 1
   * and 2 counts as a 3, at 23 every 1, 2 and 3 counts as a 4.
   *
   * The die floor is applied **here and not at the roll**, and that is §6.3 doing exactly what
   * it was built for: the file records the 1 the player actually rolled, and the rule turns it
   * into a 2 every time the total is computed. Storing the 2 would have destroyed the roll and
   * made the number unexplainable the day the character's Constitution changed.
   */
  hitPoints(): number {
    const n = Math.max(1, this.classes().length);
    const { bonus, floor } = this.constitution();
    let total = 0;
    for (const e of this.file.events) {
      const rolled = e.rolls.reduce((s, r) => s + Math.max(r.die, floor), 0);
      // The bonus is per Hit Die and a multi-class character rolls one die per class for a
      // level — so it is added once per level, like the dice it accompanies.
      total += Math.floor(rolled / n) + bonus;
    }
    return total;
  }

  /**
   * What Table 3 says about this character, or nothing where it cannot say.
   *
   * Warriors read a different column, so this needs the class GROUP — and a multi-class
   * fighter/mage is a warrior for one of its halves and not the other. The book does not rule
   * on that combination's hit point bonus, and the Engine will not invent one: where the
   * classes disagree about which column to read, **neither is used** and the sheet says so.
   */
  private constitution(): { bonus: number; floor: number; because?: string } {
    const score = Object.entries(this.file.scores)
      .find(([id]) => id.endsWith(":constitution"))?.[1];
    const table = this.pack.records("lookupTables").find((t) => t["supplies"] === "hitPoints.perLevel");
    if (score === undefined || table === undefined) return { bonus: 0, floor: 0 };
    const row = ((table["rows"] as string[][] | undefined) ?? [])
      .find((r) => Number.parseInt(r[0] ?? "", 10) === score);
    if (row === undefined) return { bonus: 0, floor: 0, because: `${table.name} has no row for Constitution ${score}` };

    const groups = new Set(this.classes().flatMap((id) => {
      const c = this.pack.byId.get(id);
      const g = groupsOf(c)[0] ?? (c?.isGroup === true ? c.id : undefined);
      return g === undefined ? [] : [(this.pack.byId.get(g)?.name ?? "").toLowerCase()];
    }));
    const warrior = groups.has("warrior");
    if (warrior && groups.size > 1) {
      return { bonus: 0, floor: 0, because: "this character is a warrior in one class and not in another, and Table 3 does not say which column a multi-class character reads" };
    }
    const n = Number.parseInt(row[warrior ? 1 : 2] ?? "0", 10);
    const min = Number.parseInt(row[3] ?? "", 10);
    return { bonus: Number.isNaN(n) ? 0 : n, floor: Number.isNaN(min) ? 0 : min };
  }

  /** Why the hit points are what they are, for a sheet that has to explain itself. */
  hitPointsBecause(): string | undefined {
    return this.constitution().because;
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
