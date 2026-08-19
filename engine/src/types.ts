/**
 * The shapes a Content Pack holds, as `pack-0.1.schema.json` defines them.
 *
 * **The schema is the internal model, not only the wire format.** That was this map's first
 * fog and the answer falls out of what the Engine actually needs: indexes, and one
 * traversal. Nothing here is reshaped on load, for two reasons that both come from §4 —
 * a Character's view of a value must carry the provenance of every layer that touched it,
 * so a normalised model would have to keep the original record beside it and would be
 * paying for two representations to answer one question; and the six operations read
 * `op`, `field`, `by` and `when` exactly as the pack writes them, so there is nothing to
 * normalise except the names.
 *
 * What the Engine adds is in `pack.ts`: an index by id, an index by kind, and the
 * transitive expansion of a weapon group. All three are derived views of the same objects.
 */

/** `book:slug` — §7.3. Globally scoped: an id is unique across the whole pack, not within
 *  its kind, which is why eight of them once collided and why `Pack` indexes them flat. */
export type Id = string;

export interface Provenance {
  /** Heading chain, outermost first. Replaces §7.1's book-and-page, which this corpus
   *  cannot supply. */
  section: string[];
  /** Absent in a `hand-authored` pack: correction 6. */
  anchor?: { rendition: "webhelp" | "rtf"; file: string; ordinal?: number };
}

export interface Interpretation {
  confidence: string;
  note?: string;
}

/** §6.1's scalar: the three things a predicate may read. */
export type Scalar =
  | { ability: Id }
  | { level: Id }
  | { field: string };

export type RollUnder = { rollAtMost: number; on: string };

/** Correction 47: a rollUnder denotes a probability, `rollAtMost / sides`. */
export type Operand =
  | number
  | string
  | RollUnder
  | { of: Scalar; multiplyBy?: number; divideBy?: number; round: "up" | "down" }
  | { supplies: string; at: Scalar; of?: Id };

export type Condition =
  | { subject: Scalar; op: "gte" | "lte" | "eq" | "neq"; value: Operand }
  | { member: Scalar; anyOfIds: Id[] }
  | { has: string; ref: Id };

/** Ticket 13 finding 10: one level of disjunction, no recursion. */
export type Clause = Condition | { anyOf: Condition[] };

/** A flat list; every clause must hold. */
export type Predicate = Clause[];

export type Op = "adjust" | "set" | "grant" | "forbid" | "except" | "require";

export interface Effect {
  op: Op;
  when?: Predicate;
  text?: string;
  /** Correction 45: a field, because a string search over `text` is not a classifier. */
  unmodelled?: true;
  /** Correction 59: the name of a campaign option from §5.5's catalogue. */
  optional?: string;
  /** Present when this effect comes from a different book than its record. */
  provenance?: Provenance;

  field?: string;
  by?: Operand;
  to?: Operand;
  kind?: string;
  ref?: Id;
  defines?: { name: string; text?: string };
  count?: number;
  from?: Id[];
}

export interface Record_ {
  id: Id;
  name: string;
  provenance?: Provenance;
  interpretation?: Interpretation;
  effects?: Effect[];
  /** False where the record's effects are not transcribed — which is not "no effects". */
  effectsModelled?: boolean;
  /** Correction 56: the other names the books print. For resolving prose into ids, never
   *  for display. */
  alsoPrinted?: string[];

  /** Attachables: what this binds to, and how many may hold at once. */
  target?: Id;
  cardinality?: string;
  /** Classes: the group a class belongs to, and the arms a multi-class combines. */
  group?: Id;
  isGroup?: boolean;
  combines?: Id[];
  variantOf?: Id;
  /** Weapon proficiencies: correction 54 made `groupKind` the whole statement that a
   *  record is a group, and correction 49 gave the headings their members. */
  groupKind?: "heading" | "tight" | "broad" | "none";
  members?: Id[];
  /** Correction 53: a launcher carries no damage; its ammunition does. */
  ammunition?: Id[];
  /** Limitations: the class that imposes the rule, and the kind it bounds. */
  imposedBy?: Id;
  bounds?: string;

  [k: string]: unknown;
}

export interface Manifest {
  id: string;
  name: string;
  version: string;
  formatVersion: "0.1";
  compatibility: { minimum: string; verified: string; maximum?: string };
  /** Correction 6: A3 applied to provenance. */
  provenanceMode: "extracted" | "hand-authored";
  sources?: { path: string; sha256: string; rendition: string }[];
  /** §5.1: the rule-sets this pack provides. */
  declares?: string[];
  files: string[];
  dependencies?: { id: string; minimum?: string; maximum?: string }[];
}

/** Correction 58: every field path the pack's effects write. Declaration over discovery,
 *  which is §7.1's argument for the manifest, one level down. */
export interface FieldDeclaration {
  path: string;
  note?: string;
}
