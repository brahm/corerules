/**
 * The values a sheet computes rather than records (§9.1's "derived values").
 *
 * Everything here is a table read, and the tables are the books' own. What makes it worth a
 * module is that **two of the four are missing from this corpus and the Engine has to say so
 * rather than produce a plausible number** — §5.2's wrong number is the failure mode, and a
 * THAC0 of 20 that came from nowhere is indistinguishable from one that came from Table 53.
 */
import type { Character } from "./character.ts";
import type { Pack } from "./pack.ts";
import { armourClass, slots as spellSlots, startingFunds } from "./spells.ts";
import { groupsOf, type Id } from "./types.ts";

export interface Derived {
  /** Table 43's die, as the book prints it. §9.1's starting money is a roll, not a number. */
  funds?: string;
  /** Armour class, where the corpus can answer — which for now is only the unarmoured case. */
  armourClass?: number;
  armourClassBecause?: string;
  /** Spells per spell level, index 0 being 1st. Empty for a class that casts none, and the
   *  `missing` line says which of the two that is. */
  spells?: number[];
  /** The number to hit Armour Class 0, from Table 53. */
  thac0?: number;
  /** Experience needed for the next level, from the class group's own table. */
  nextLevelAt?: number;
  /** Present only where the Engine could not compute something it was asked for, with the
   *  table that would have answered it named. */
  missing: { value: string; because: string }[];
}

/** Only these groups cast at all, so a fighter is not told his spell table is missing. */
const PROGRESSION_CLASSES = new Set(["priest", "wizard", "bard"]);

const norm = (s: string): string => s.toLowerCase().replace(/[^a-z0-9]/g, "");

function rows(pack: Pack, id: Id): string[][] {
  return (pack.byId.get(id)?.["rows"] as string[][] | undefined) ?? [];
}

/** Table 53 is keyed by class GROUP down the side and by level across the top, with the header
 *  row carrying the levels. */
function thac0(pack: Pack, groupName: string, level: number): number | undefined {
  const table = rows(pack, "phb:DD01679");
  const header = table[0];
  const row = table.find((r) => norm(r[0] ?? "") === norm(groupName));
  if (header === undefined || row === undefined) return undefined;
  const at = header.indexOf(String(level));
  if (at < 1) return undefined;
  const v = Number.parseInt(row[at] ?? "", 10);
  return Number.isNaN(v) ? undefined : v;
}

/** The experience table each class group uses. Keyed by level, with one column per class. */
const EXPERIENCE: Record<string, Id> = {
  warrior: "phb:DD01460",
  wizard: "phb:DD01470",
  priest: "phb:DD01478",
  rogue: "phb:DD01499",
};

function nextLevelAt(pack: Pack, groupName: string, level: number): number | undefined {
  const id = EXPERIENCE[norm(groupName)];
  if (id === undefined) return undefined;
  const row = rows(pack, id).find((r) => Number.parseInt(r[0] ?? "", 10) === level + 1);
  const v = Number.parseInt((row?.[1] ?? "").replace(/,/g, ""), 10);
  return Number.isNaN(v) ? undefined : v;
}

export function derived(pack: Pack, character: Character): Derived {
  const classId = character.classes()[0];
  const klass = classId !== undefined ? pack.byId.get(classId) : undefined;
  const groupId = groupsOf(klass)[0] ?? (klass?.isGroup === true ? klass.id : undefined);
  const groupName = groupId !== undefined ? pack.byId.get(groupId)?.name : undefined;
  const level = classId !== undefined ? (character.levels()[classId] ?? 1) : 1;
  const missing: Derived["missing"] = [];

  if (groupName === undefined) {
    return { missing: [{ value: "everything derived from the class", because: "this character's class names no group" }] };
  }

  const hit = thac0(pack, groupName, level);
  if (hit === undefined) {
    missing.push({ value: "THAC0", because: `no loaded pack has Table 53 for ${groupName} at level ${level}` });
  }
  const next = nextLevelAt(pack, groupName, level);
  if (next === undefined) {
    missing.push({ value: "experience for the next level", because: `no loaded pack has the ${groupName} experience table` });
  }

  const funds = classId !== undefined ? startingFunds(pack, classId) : undefined;
  if (classId !== undefined && funds === undefined) {
    missing.push({ value: "starting funds", because: "no loaded pack has Table 43 for this class" });
  }
  const ac = armourClass(pack, []);
  const casting = classId !== undefined ? spellSlots(pack, classId, level) : undefined;
  if (casting?.missing !== undefined && PROGRESSION_CLASSES.has(norm(groupName))) {
    missing.push({ value: "spells per day", because: casting.missing });
  }

  // Table 60 is in the slice with every cell empty — the rows are there and the numbers are
  // not. That is exactly the state A3 exists to keep apart from "no rule": the pack HAS the
  // table and cannot answer with it, so the sheet says so rather than showing a blank.
  const saving = rows(pack, "phb:DD01724");
  if (saving.length === 0 || saving.every((r) => r.slice(1).every((c) => c === ""))) {
    missing.push({ value: "saving throws", because: "Table 60 is present in the pack with no numbers in it" });
  }

  return {
    ...(funds !== undefined ? { funds } : {}),
    ...(ac.ac !== undefined ? { armourClass: ac.ac } : {}),
    armourClassBecause: ac.because,
    ...(casting !== undefined && casting.perLevel.length > 0 ? { spells: casting.perLevel } : {}),
    ...(hit !== undefined ? { thac0: hit } : {}),
    ...(next !== undefined ? { nextLevelAt: next } : {}),
    missing,
  };
}
