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
import { range } from "./dice.ts";
import { armourClass, fundsDie, slots as spellSlots } from "./spells.ts";
import { groupsOf, type Id } from "./types.ts";

export interface Derived {
  /** §9.1's starting money: the die as the book prints it, where it came from, what a roll can
   *  produce, and the amount **if somebody has rolled**. A die with no roll behind it is not a
   *  number and must not be shown as one. */
  funds?: { die: string; from: string; least: number; most: number; rolled?: number };
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
  /** §6.2: one line per class where there is more than one, because a Fighter/Mage is two
   *  careers on two tables and a single number hides which half is which. */
  perClass?: { class: string; level: number; thac0?: number; nextLevelAt?: number }[];
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

/** The class group a class belongs to, or itself where it IS one. */
function groupOf(pack: Pack, classId: Id): { id: Id; name: string } | undefined {
  const klass = pack.byId.get(classId);
  const id = groupsOf(klass)[0] ?? (klass?.isGroup === true ? klass.id : undefined);
  const name = id === undefined ? undefined : pack.byId.get(id)?.name;
  return id === undefined || name === undefined ? undefined : { id, name };
}

/**
 * **§6.2, and the reason it is a module concern rather than a lookup.**
 *
 * *"XP split evenly · hit points averaged across hit dice · best saving throw across classes ·
 * best THAC0 · best slot progression."* Those are **shape, not content**: the PHB prints the
 * saving-throw matrix, and *"with two classes, take the best"* is in no table anywhere. So the
 * numbers come from the pack and the combination comes from here, which is §5.2's line drawn
 * through the middle of one sheet.
 *
 * Until now every derived value read `classes()[0]`, so a Fighter/Mage got **the fighter's
 * answers with no sign that half of them were missing** — or, once the class record named no
 * group, nothing at all. The second failure was honest and useless; the first is §5.2's wrong
 * number, and it is the one this exists to stop.
 */
export function derived(pack: Pack, character: Character): Derived {
  const levels = character.levels();
  const arms = character.classes().map((id) => ({
    id, level: levels[id] ?? 1, group: groupOf(pack, id), name: pack.byId.get(id)?.name ?? id,
  }));
  const missing: Derived["missing"] = [];

  if (arms.length === 0) {
    return { missing: [{ value: "everything derived from the class", because: "this character has no class" }] };
  }
  const homeless = arms.filter((a) => a.group === undefined);
  if (homeless.length > 0) {
    return {
      missing: [{
        value: "everything derived from the class",
        because: `${homeless.map((a) => a.name).join(", ")} names no class group in any loaded pack`,
      }],
    };
  }

  // Best THAC0 across the classes — the LOWEST, because THAC0 counts down.
  const hits = arms.map((a) => ({ arm: a, thac0: thac0(pack, a.group!.name, a.level) }));
  const missed = hits.filter((h) => h.thac0 === undefined);
  for (const h of missed) {
    missing.push({ value: `THAC0 as a ${h.arm.name}`, because: `no loaded pack has Table 53 for ${h.arm.group!.name} at level ${h.arm.level}` });
  }
  const known = hits.filter((h) => h.thac0 !== undefined).map((h) => h.thac0!);
  const hit = known.length > 0 ? Math.min(...known) : undefined;

  // Each arm needs its own next threshold, because §6.2 splits experience evenly rather than
  // pooling it: a Fighter/Mage is two careers advancing on two tables at half speed each.
  const perClass = arms.map((a) => {
    const next = nextLevelAt(pack, a.group!.name, a.level);
    if (next === undefined) {
      missing.push({ value: `experience for the next ${a.name} level`, because: `no loaded pack has the ${a.group!.name} experience table` });
    }
    const t = hits.find((h) => h.arm === a)?.thac0;
    return { class: a.name, level: a.level, ...(t !== undefined ? { thac0: t } : {}), ...(next !== undefined ? { nextLevelAt: next } : {}) };
  });
  const nexts = perClass.map((c) => c.nextLevelAt).filter((n): n is number => n !== undefined);
  const next = nexts.length > 0 ? Math.min(...nexts) : undefined;

  const die = fundsDie(pack, character);
  const span = die === undefined ? undefined : range(die.die);
  if (die === undefined) {
    missing.push({ value: "starting funds", because: "no loaded pack has Table 43 for this class" });
  } else if (span === undefined) {
    missing.push({ value: "starting funds", because: `${die.from} gives ${die.die}, which is not a notation this Engine can roll` });
  }
  const funds = die !== undefined && span !== undefined
    ? { die: die.die, from: die.from, least: span.least, most: span.most,
        ...(character.file.funds !== undefined ? { rolled: character.file.funds } : {}) }
    : undefined;
  const ac = armourClass(pack, character.file.worn ?? []);

  // Best slot progression: a caster among the arms casts, and the best table wins.
  const casting = arms
    .map((a) => ({ arm: a, slots: spellSlots(pack, a.id, a.level) }))
    .filter((c) => c.slots.perLevel.length > 0 || PROGRESSION_CLASSES.has(norm(c.arm.group!.name)));
  const best = casting
    .filter((c) => c.slots.perLevel.length > 0)
    .sort((a, b) => b.slots.perLevel.length - a.slots.perLevel.length)[0];
  for (const c of casting) {
    if (c.slots.missing !== undefined) {
      missing.push({ value: `spells per day as a ${c.arm.name}`, because: c.slots.missing });
    }
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
    ...(best !== undefined ? { spells: best.slots.perLevel } : {}),
    ...(hit !== undefined ? { thac0: hit } : {}),
    ...(next !== undefined ? { nextLevelAt: next } : {}),
    ...(arms.length > 1 ? { perClass } : {}),
    missing,
  };
}
