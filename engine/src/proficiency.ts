/**
 * Proficiencies and their slot budgets (§9.1).
 *
 * The rules are exact and the Player's Handbook states them in four sentences:
 *
 *   *"Each character starts with a specific number of nonweapon proficiency slots and then
 *   earns additional slots as he advances."*
 *   *"Initial slots must be assigned immediately; they cannot be saved or held in reserve."*
 *   *"When a player selects a nonweapon proficiency from those categories listed under
 *   'Proficiency Groups' for his character's group, it requires the number of proficiency
 *   slots listed in Table 37."*
 *   *"When a player selects a proficiency from any other category, it requires one additional
 *   proficiency slot beyond the number listed."*
 *
 * So a budget is a number, a cost is a number, and the interesting part is **which groups are
 * open to this character** — because that is what decides whether a proficiency costs one slot
 * or two, and it is the one part the books state in prose the pack cannot fully resolve.
 */
import type { Pack } from "./pack.ts";
import type { Character } from "./character.ts";
import { groupsOf, type Id, type Record_ } from "./types.ts";

export interface Budget {
  /** Table 34's "Initial", and one more every "#Levels". */
  initial: number;
  everyLevels: number;
  /** Weapon proficiencies only: the to-hit penalty for using something you are not
   *  proficient with. It travels with the budget because Table 34 prints it there. */
  penalty?: number;
  total: number;
  spent: number;
  /** What is left, which may be negative where a kit granted more than the budget allows. */
  free: number;
}

export interface Candidate {
  id: Id;
  name: string;
  book: string;
  /** Slots this character would spend. Table 37's number, plus one where the proficiency
   *  belongs to no group open to them. */
  cost: number;
  /** Which open group made it cost what it costs, or why it did not. */
  because: string;
  /** False where the books do not say which group it belongs to, so the crossover surcharge
   *  cannot be decided. The wizard's third state, arriving in a budget. */
  certain: boolean;
  ability?: Id;
  modifier?: number;
}

const bookOf = (r: Record_): string => r.provenance?.section[0] ?? "?";
const norm = (s: string): string => s.toLowerCase().replace(/[^a-z0-9]/g, "");

function table(pack: Pack, id: Id): string[][] {
  return ((pack.byId.get(id)?.["rows"] as string[][] | undefined) ?? []);
}

/**
 * The groups a character may buy from at the listed cost.
 *
 * Always their own class group and General. Table 38 adds the crossovers — and it prints them
 * as PROSE: `phb:paladin` maps to *"Warrior, Priest, General"*. Resolving those names is
 * correction 56's job and this uses it, `alsoPrinted` and all; a name that resolves to nothing
 * is REPORTED rather than dropped, because a silently narrowed list makes a proficiency cost
 * one slot too many and nobody would ever see why.
 */
export function openGroups(pack: Pack, classId: Id): { groups: Set<Id>; unresolved: string[] } {
  const groups = new Set<Id>();
  const unresolved: string[] = [];
  const klass = pack.byId.get(classId);
  for (const g of groupsOf(klass)) groups.add(g);
  if (klass?.isGroup === true) groups.add(klass.id);

  const byName = new Map<string, Id>();
  for (const kind of ["proficiencyGroups", "classes"]) {
    for (const r of pack.records(kind)) {
      for (const n of [r.name, ...(r.alsoPrinted ?? [])]) byName.set(norm(n), r.id);
    }
  }

  const row = table(pack, "phb:DD01539").find((r) => r[0] === classId);
  for (const name of (row?.[1] ?? "").split(",").map((s) => s.trim()).filter((s) => s !== "")) {
    const id = byName.get(norm(name));
    if (id === undefined) unresolved.push(name);
    else groups.add(id);
  }
  // Every character may buy from General, whether or not the crossover row names it.
  const general = byName.get("general");
  if (general !== undefined) groups.add(general);
  return { groups, unresolved };
}

/** Table 34, keyed by the class GROUP's name as the book prints it. */
function slotRow(pack: Pack, classId: Id): string[] | undefined {
  const klass = pack.byId.get(classId);
  const groupId = groupsOf(klass)[0] ?? (klass?.isGroup === true ? klass.id : undefined);
  const groupName = groupId !== undefined ? pack.byId.get(groupId)?.name : undefined;
  if (groupName === undefined) return undefined;
  return table(pack, "phb:DD01524").find((r) => norm(r[0] ?? "") === norm(groupName));
}

const number = (s: string | undefined): number => {
  const n = Number.parseInt((s ?? "").replace("+", ""), 10);
  return Number.isNaN(n) ? 0 : n;
};

/** Table 34's total for a class at a level, without needing a Character — the wizard asks
 *  before there is one. */
export function slots(pack: Pack, classId: Id, level = 1, which: "weapon" | "nonweapon" = "nonweapon"): number {
  const row = slotRow(pack, classId);
  if (row === undefined) return 0;
  const initial = number(which === "weapon" ? row[1] : row[4]);
  const everyLevels = Math.max(1, number(which === "weapon" ? row[2] : row[5]));
  return initial + Math.floor((level - 1) / everyLevels);
}

export function budget(pack: Pack, character: Character, which: "weapon" | "nonweapon"): Budget | undefined {
  const classId = character.classes()[0];
  if (classId === undefined) return undefined;
  const row = slotRow(pack, classId);
  if (row === undefined) return undefined;
  // Table 34 prints weapon and nonweapon side by side: Group, Initial, #Levels, Penalty,
  // Initial, #Levels.
  const initial = number(which === "weapon" ? row[1] : row[4]);
  const everyLevels = Math.max(1, number(which === "weapon" ? row[2] : row[5]));
  const level = character.levels()[classId] ?? 1;
  const total = initial + Math.floor((level - 1) / everyLevels);

  const kind = which === "weapon" ? "weaponProficiency" : "nonweaponProficiency";
  const spent = character.file.events
    .flatMap((e) => e.chose ?? [])
    .filter((c) => c.kind === kind)
    .reduce((n, c) => n + cost(pack, c.ref, classId).cost, 0);

  return {
    initial, everyLevels, total, spent, free: total - spent,
    ...(which === "weapon" ? { penalty: number(row[3]) } : {}),
  };
}

/** Table 37's slot count, plus one where no open group holds it. */
export function cost(pack: Pack, proficiencyId: Id, classId: Id): { cost: number; because: string; certain: boolean } {
  const p = pack.byId.get(proficiencyId);
  const listed = (p?.["slotCost"] as number | undefined) ?? 1;
  const { groups } = openGroups(pack, classId);
  const belongs = groupsOf(p);
  const mine = belongs.filter((g) => groups.has(g));
  if (mine.length > 0) {
    const name = pack.byId.get(mine[0]!)?.name ?? mine[0]!;
    return { cost: listed, because: `in ${name}, which is open to this class`, certain: true };
  }
  if (belongs.length === 0) {
    // Every one of the Player's Handbook's sixty-five names its group. Fifty-four of the
    // fifty-five from the Complete handbooks name none, and the books do not either — the
    // Complete Thief's prints "1 slot, Wisdom, -1 modifier" and stops. So the surcharge
    // cannot be decided, and charging it anyway would invent a rule against the character
    // in the one place a player would never think to check.
    return {
      cost: listed,
      because: "no book says which group this belongs to, so whether it costs one slot more is undecided",
      certain: false,
    };
  }
  const where = belongs.map((g) => pack.byId.get(g)?.name ?? g).join(", ");
  return { cost: listed + 1, because: `in ${where}, which is not open to this class — one slot more`, certain: true };
}

/** Everything this character could spend a nonweapon slot on, with what it would cost. */
export function candidates(pack: Pack, character: Character): Candidate[] {
  const classId = character.classes()[0];
  if (classId === undefined) return [];
  const taken = new Set(character.file.events.flatMap((e) => e.chose ?? []).map((c) => c.ref));
  return pack.records("nonweaponProficiencies")
    .filter((p) => !taken.has(p.id))
    .map((p): Candidate => {
      const { cost: slots, because, certain } = cost(pack, p.id, classId);
      const check = p["abilityCheck"] as { ability?: Id } | undefined;
      return {
        id: p.id, name: p.name, book: bookOf(p), cost: slots, because, certain,
        ...(check?.ability !== undefined ? { ability: check.ability } : {}),
        ...(typeof p["modifier"] === "number" ? { modifier: p["modifier"] } : {}),
      };
    });
}

/**
 * The weapons a class may take proficiency in, before there is a Character to ask.
 *
 * `Sheet.permitted` answers the same question for a built character, where a kit's `forbid`
 * and `except` are also in play; this answers it for a draft, where the only bound is the one
 * the class imposes on itself. Correction 48 found that `imposedBy` IS the imposition, which
 * is why neither needs an effect to look for.
 */
export function permittedWeapons(pack: Pack, classId: Id): { allowed: Set<Id>; bound?: string } {
  const universe = new Set(pack.records("weaponProficiencies").filter((w) => w.groupKind === undefined).map((w) => w.id));
  const mine = new Set<Id>([classId, ...groupsOf(pack.byId.get(classId))]);
  for (const lim of pack.records("limitations")) {
    if (lim.imposedBy === undefined || !mine.has(lim.imposedBy)) continue;
    if (lim.members === undefined || lim.bounds !== "weaponProficiency") continue;
    const members = pack.expand(lim.members);
    return { allowed: new Set([...universe].filter((i) => members.has(i))), bound: lim.name };
  }
  return { allowed: universe };
}
