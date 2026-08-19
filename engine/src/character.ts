/**
 * §4's layer model, and the four reasons a value does not reach the sheet.
 *
 * Nothing is ever overwritten. A Character's view of a value is computed by walking a stack —
 * the base record, then each Attachable, with the Character's own choices topmost — and the
 * provenance of every layer survives to the top, because §1's promise is that a refusal names
 * which rule refused and which book it came from.
 *
 * The four reasons a contribution is set aside are all decided, and each came from a ticket:
 *
 *   marked        the transcriber modelled it as far as the format allowed (ticket 02)
 *   undecidable   its predicate asks something the sheet cannot answer
 *   contested     two layers set it and nothing declares over them (ticket 03)
 *   option        the book marks the rule optional and this table has not chosen (§5.5)
 *
 * The fourth is unlike the other three: **nothing is missing.** The effect is fully modelled,
 * the predicate decidable, no layer disagreeing. What is absent is a decision belonging to
 * neither the pack nor the Engine.
 */
import type { Pack } from "./pack.ts";
import { predicate, scalar, type Sheet, type Truth } from "./predicate.ts";
import type { Effect, Id, Operand, Record_, Scalar } from "./types.ts";

export type Role = "race" | "subrace" | "class group" | "class" | "kit" | "deity" | "choice";

export interface Layer {
  role: Role;
  record: Record_;
}

/** Where a contribution came from, kept so a value can name its cause. */
export interface Source {
  layer: Role;
  record: Id;
  name: string;
  index: number;
  /** The book, from the effect's own provenance where it has one — a record's effects are
   *  not necessarily all from the book the record belongs to. */
  book: string;
}

export type Aside =
  | { because: "marked"; source: Source; value: Value; text: string }
  | { because: "undecidable"; source: Source; text: string }
  | { because: "option"; source: Source; value: Value; option: string; text: string }
  /** A fifth, and it is the Engine's fault rather than the pack's: an operand this version
   *  cannot compute. It exists because the alternative was worse — a contribution whose
   *  value came back undefined was being filtered out of the sum and the field reported as
   *  0, which is §5.2's wrong number produced by the Engine itself. A value the Engine
   *  cannot compute is not zero and must never be added as though it were. */
  | { because: "unresolved"; source: Source; text: string };

export type Value = number | string | undefined;

export interface Contribution {
  source: Source;
  op: "adjust" | "set";
  value: Value;
}

export interface View {
  path: string;
  /** undefined where nothing reached the total, or where the value is contested. */
  value: Value;
  from: Contribution[];
  /** Set only when two layers set this and nothing declares over them. */
  contested?: Contribution[];
}

export interface Granted {
  source: Source;
  kind: string;
  /** A ref, or a name the effect defines inline for a thing that exists nowhere else. */
  ref?: Id;
  defines?: string;
  /** Ticket 02: a marked structural effect IS applied, with the marker riding on it. */
  rider?: string;
}

export interface Owed {
  source: Source;
  kind: string;
  count: number;
  from?: Id[];
}

const book = (r: Record_, e: Effect): string =>
  (e.provenance ?? r.provenance)?.section[0] ?? "?";

export class Character {
  readonly pack: Pack;
  readonly layers: Layer[] = [];
  /** §5.5's catalogue: which optional rules this campaign plays. */
  readonly options: Set<string>;
  readonly scores: Record<string, number>;
  readonly levels: Record<string, number>;

  readonly fields = new Map<string, Contribution[]>();
  readonly aside: Aside[] = [];
  readonly granted: Granted[] = [];
  readonly forbidden: Granted[] = [];
  readonly excepted: { source: Source; kind: string; ref: Id }[] = [];
  readonly owed: Owed[] = [];
  readonly notes: string[] = [];

  constructor(pack: Pack, opts: {
    scores?: Record<string, number>;
    levels?: Record<string, number>;
    options?: Iterable<string>;
  } = {}) {
    this.pack = pack;
    this.scores = opts.scores ?? {};
    this.levels = opts.levels ?? {};
    this.options = new Set(opts.options ?? []);
  }

  private sheet(): Sheet {
    return {
      ability: (id) => this.scores[id],
      level: (id) => this.levels[id] ?? 0,
      field: (path) => this.view(path).value,
      has: (_kind, ref) => this.granted.some((g) => g.ref === ref),
    };
  }

  /** Push a layer and apply its effects in order. Order never changes the answer — the six
   *  operations commute by design — so this is a convenience, not a semantics. */
  apply(record: Record_ | undefined, role: Role): this {
    if (record === undefined) return this;
    this.layers.push({ role, record });
    if (record.effectsModelled === false) {
      this.notes.push(`${record.id} (${record.name}) says its effects are not transcribed — not "no effects".`);
    }
    const effects = record.effects ?? [];
    for (let i = 0; i < effects.length; i++) {
      this.effect(effects[i]!, { layer: role, record: record.id, name: record.name, index: i, book: book(record, effects[i]!) });
    }
    return this;
  }

  private effect(e: Effect, source: Source): void {
    const text = e.text ?? "";

    // §5.5. Checked before the predicate: a rule the table does not play is not a rule whose
    // condition failed, and saying so is the difference between "your Wisdom is too low" and
    // "your table doesn't use that rule".
    if (e.optional !== undefined && !this.options.has(e.optional)) {
      this.aside.push({ because: "option", source, value: this.operand(e), option: e.optional, text });
      return;
    }

    const fires: Truth = predicate(e.when, this.sheet());
    if (fires === null) {
      this.aside.push({ because: "undecidable", source, text });
      return;
    }
    if (fires === false) return;

    // Ticket 02. The OPERATION decides, not the marker's declared category: a structural
    // operation is applied with the marker riding on the entry, because the thing is right
    // and only its edges are under-described; a numeric one is withheld from the total,
    // because the number is right and the circumstance is missing.
    const marked = e.unmodelled === true;

    switch (e.op) {
      case "adjust":
      case "set": {
        const value = this.operand(e);
        // Every `adjust` operand in the corpus is an integer, a computed value or a table
        // read — never a bare string — so an adjust that does not produce a number is a
        // defect somewhere, and the sheet has to say so. Silently leaving it out of the sum
        // reports the field as 0, which is §5.2's wrong number produced by the Engine
        // itself. A `set` may legitimately be a string: `5d4x10`, `3- on 1d6`.
        if (value === undefined || (e.op === "adjust" && typeof value !== "number")) {
          this.aside.push({ because: "unresolved", source,
            text: value === undefined
              ? `${e.op} ${e.field}: the operand did not resolve`
              : `adjust ${e.field}: the operand resolved to ${JSON.stringify(value)}, which cannot be summed` });
          return;
        }
        if (marked) {
          this.aside.push({ because: "marked", source, value, text });
          return;
        }
        const into = this.fields.get(e.field!) ?? [];
        into.push({ source, op: e.op, value });
        this.fields.set(e.field!, into);
        return;
      }
      case "grant":
      case "forbid": {
        const entry: Granted = { source, kind: e.kind ?? "?", ...(e.ref !== undefined ? { ref: e.ref } : {}), ...(e.defines !== undefined ? { defines: e.defines.name } : {}), ...(marked ? { rider: text } : {}) };
        (e.op === "grant" ? this.granted : this.forbidden).push(entry);
        if (e.ref !== undefined) this.pack.get(e.ref, `${source.record}[${source.index}]`);
        return;
      }
      case "except":
        this.excepted.push({ source, kind: e.kind ?? "?", ref: e.ref! });
        return;
      case "require":
        this.owed.push({ source, kind: e.kind ?? "?", count: e.count ?? 1, ...(e.from !== undefined ? { from: e.from } : {}) });
        return;
    }
  }

  private operand(e: Effect): Value {
    const v: Operand | undefined = e.op === "set" ? e.to : e.by;
    if (v === undefined) return undefined;
    if (typeof v === "number" || typeof v === "string") return v;
    if ("rollAtMost" in v) return `${v.rollAtMost}- on ${v.on}`;
    // Discriminated on `supplies`, not on `of`: BOTH the computed operand and the table read
    // carry an `of`, and they mean different things — the computed one reads a scalar, the
    // table one names the record whose table to read. Checking `of` first types the table
    // arm's id as a scalar and would have compiled in a language that let it.
    if ("supplies" in v) return this.table(v.supplies, v.at, v.of);
    if ("of" in v) {
      const base = scalar(v.of, this.sheet());
      if (typeof base !== "number") return undefined;
      const n = (base * (v.multiplyBy ?? 1)) / (v.divideBy ?? 1);
      return v.round === "down" ? Math.floor(n) : Math.ceil(n);
    }
    return undefined;
  }

  /**
   * A table read. The pack names the FIELD PATH a table fills rather than a role, so the
   * Engine finds the table it needs by naming the value it is computing — corpus finding
   * 138's `supplies`, chosen over a second closed enumeration because §3.4 allows exactly
   * one. `of` names a specific table where two supply the same path.
   *
   * Values come back as the book prints them: Table 9's bonus is `"+4"`, not 4. Correction
   * from ticket 04 — a table read returning a string where the contract says an integer is
   * a real mismatch and it is the pack's to fix, not something to paper over here.
   */
  private table(supplies: string, at: Scalar, of?: Id): Value {
    const tables = this.pack.records("lookupTables").filter((t) => t["supplies"] === supplies);
    const t = of !== undefined ? tables.find((x) => x.id === of) ?? tables[0] : tables[0];
    if (t === undefined) {
      this.pack.complain("reference", `no table supplies ${supplies}`);
      return undefined;
    }
    const key = scalar(at, this.sheet());
    if (key === undefined) return undefined;
    for (const row of (t["rows"] as [string, string][] | undefined) ?? []) {
      if (Character.rowMatches(row[0], key)) return row[1];
    }
    this.pack.complain("reference", `${t.id} has no row for ${String(key)}`);
    return undefined;
  }

  /** A key cell is a value or a printed range: `14-17`. */
  private static rowMatches(cell: string, key: number | string): boolean {
    if (typeof key !== "number") return cell === key;
    const range = /^(\d+)-(\d+)$/.exec(cell);
    if (range !== null) return Number(range[1]) <= key && key <= Number(range[2]);
    return Number(cell) === key;
  }

  /**
   * Correction 47: `3- on 1d6` and `50` are the same probability, and a pack that records
   * both is recording two books that AGREE. The co-occurrence of the two notations on one
   * field is itself the evidence that the field is a probability, so this needs no
   * field-dimension vocabulary. Returns the shared value, or undefined where they are not
   * commensurable.
   */
  private static agreed(values: Value[]): number | undefined {
    const rolls = values.filter((v) => typeof v === "string" && /^\d+- on 1d\d+$/.test(v));
    if (rolls.length === 0 || rolls.length === values.length) return undefined;
    const seen = new Set<number>();
    for (const v of values) {
      if (typeof v === "string") {
        const m = /^(\d+)- on 1d(\d+)$/.exec(v);
        if (m === null) return undefined;
        seen.add(Number(m[1]) / Number(m[2]) * 100);
      } else if (typeof v === "number") {
        seen.add(v);
      } else return undefined;
    }
    return seen.size === 1 ? [...seen][0] : undefined;
  }

  /** Ticket 03: precedence is never inferred from which arm a layer is. It is declared by
   *  one record about another — a subrace names its race as `target` — or it does not exist,
   *  and a value nothing declares over is refused rather than guessed. */
  private refines(a: Id, b: Id): boolean {
    const ra = this.pack.byId.get(a);
    return ra?.target === b;
  }

  view(path: string): View {
    const from = this.fields.get(path) ?? [];
    if (from.length === 0) return { path, value: undefined, from };

    const sets = from.filter((c) => c.op === "set");
    const adjust = from
      .filter((c) => c.op === "adjust" && typeof c.value === "number")
      .reduce((n, c) => n + (c.value as number), 0);

    if (sets.length === 0) return { path, value: adjust, from };

    let winner = sets[sets.length - 1]!;
    const distinct = new Set(sets.map((c) => String(c.value)));
    if (distinct.size > 1) {
      const same = Character.agreed(sets.map((c) => c.value));
      if (same !== undefined) {
        this.notes.push(`${path}: two books write this in different notations and they AGREE — ${[...distinct].join(" = ")}`);
      } else {
        const winners = sets.filter((x) =>
          sets.every((y) => x.source.record === y.source.record || this.refines(x.source.record, y.source.record)));
        if (winners.length !== 1) return { path, value: undefined, from, contested: sets };
        winner = winners[0]!;
      }
    }
    const v = winner.value;
    return { path, value: typeof v === "number" ? v + adjust : v, from };
  }

  /** Ticket 05. `permitted = (∩ bounds still standing) \ (∪ explicit forbids)`. Intersection
   *  and union commute, so §4.3's guarantee is inherited rather than re-argued. */
  permitted(kind: string, universe: Iterable<Id>): { allowed: Set<Id>; bounds: Granted[]; lifted: Id[] } {
    const lifted = this.excepted.map((x) => x.ref);
    const bounds = this.forbidden.filter((f) => {
      const lim = f.ref !== undefined ? this.pack.byId.get(f.ref) : undefined;
      return lim?.members !== undefined && !lifted.includes(f.ref!);
    });
    // A limitation naming the character's class imposes itself: `imposedBy` IS the
    // imposition, which is why the corpus has 72 `except`s and no matching `forbid`.
    const classes = new Set(this.layers.filter((l) => l.role === "class" || l.role === "class group").map((l) => l.record.id));
    for (const lim of this.pack.records("limitations")) {
      if (lim.imposedBy !== undefined && classes.has(lim.imposedBy) && lim.members !== undefined
          && lim.bounds === kind && !lifted.includes(lim.id)) {
        bounds.push({ source: { layer: "class", record: lim.id, name: lim.name, index: 0, book: lim.provenance?.section[0] ?? "?" }, kind, ref: lim.id });
      }
    }
    let allowed = new Set(universe);
    for (const b of bounds) {
      const members = this.pack.expand(this.pack.byId.get(b.ref!)!.members!);
      allowed = new Set([...allowed].filter((i) => members.has(i)));
    }
    for (const f of this.forbidden) {
      if (f.ref !== undefined && !bounds.some((b) => b.ref === f.ref)) allowed.delete(f.ref);
    }
    return { allowed, bounds, lifted };
  }
}
