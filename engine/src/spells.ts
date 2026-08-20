/**
 * Spell access, and the two halves of §9.1 that turned out to be transcribed after all.
 *
 * **A priest's spells come from the spheres their god grants**, which is the deity work of
 * correction 46 paying off in the place it was always for. The Complete Priest's states the
 * rule at DD05501:
 *
 *   *"a priesthood can have MAJOR access to one or several spheres, and thus eventually learn
 *   to cast spells of any level from that sphere, and can have MINOR access to one or several
 *   other spheres, and learn to cast spells from only 1st through 3rd level in that sphere."*
 *
 * The pack has carried `sphere` and `sphereMinor` as separate grants since the priesthoods were
 * transcribed. This is the first thing that reads them.
 */
import type { Character } from "./character.ts";
import type { Pack } from "./pack.ts";
import { groupsOf, type Id } from "./types.ts";

/** Minor access reaches 1st through 3rd level and no further. */
export const MINOR_ACCESS_TO = 3;

export interface SpellSlots {
  /** Index 0 is 1st-level spells. */
  perLevel: number[];
  /** Present where no loaded pack can answer, with the table that would have. */
  missing?: string;
}

export interface SpellOffer {
  id: Id;
  name: string;
  level: number;
  book: string;
  /** The sphere that lets this character have it, and on what terms. */
  through: { sphere: string; access: "major" | "minor" };
}

/** Which progression table a class group reads. */
const PROGRESSION: Record<string, Id> = {
  priest: "phb:DD01479",   // Table 24
  bard: "phb:DD01509",     // Table 32
  wizard: "phb:DD01471",   // Table 21
};

const norm = (s: string): string => s.toLowerCase().replace(/[^a-z0-9]/g, "");

export function slots(pack: Pack, classId: Id, level: number): SpellSlots {
  const klass = pack.byId.get(classId);
  const groupId = groupsOf(klass)[0] ?? (klass?.isGroup === true ? klass.id : undefined);
  const key = norm(pack.byId.get(groupId ?? "")?.name ?? "");
  const table = PROGRESSION[key] === undefined ? undefined : pack.byId.get(PROGRESSION[key]!);
  const rows = (table?.["rows"] as string[][] | undefined) ?? [];
  if (rows.length === 0) {
    return {
      perLevel: [],
      // Table 21 is in the slice with no rows at all — present and empty, exactly as Table 60
      // is. Naming the table is the difference between "this class has no spells" and "nobody
      // transcribed how many it gets".
      missing: table === undefined
        ? `no loaded pack has a spell progression for ${pack.byId.get(classId)?.name ?? classId}`
        : `${table.name} is present in the pack with no rows in it`,
    };
  }
  // The first row is the header — spell levels across the top — and the rest are class levels.
  const row = rows.slice(1).find((r) => Number.parseInt(r[0] ?? "", 10) === level);
  if (row === undefined) return { perLevel: [], missing: `${table!.name} has no row for level ${level}` };
  return {
    perLevel: row.slice(1).map((cell) => {
      const n = Number.parseInt(cell, 10);
      return Number.isNaN(n) ? 0 : n;
    }),
  };
}

/**
 * What a priest may learn, and why.
 *
 * Every offer names the sphere it came through and whether the access is major or minor,
 * because that is the difference between *"you may have this"* and *"you may have this until
 * 4th level and then never again"* — and a list that hid it would leave a player planning a
 * career around a spell they cannot reach.
 */
export function available(pack: Pack, character: Character): SpellOffer[] {
  const sheet = character.sheet();
  const access = new Map<Id, "major" | "minor">();
  for (const g of sheet.granted) {
    if (g.ref === undefined) continue;
    if (g.kind === "sphere") access.set(g.ref, "major");
    // A sphere granted both ways keeps the major grant: it is the stronger claim, and the
    // Complete Priest's gives a few priesthoods both when one access is limited in prose.
    else if (g.kind === "sphereMinor" && !access.has(g.ref)) access.set(g.ref, "minor");
  }
  if (access.size === 0) return [];

  const out: SpellOffer[] = [];
  for (const spell of pack.records("spells")) {
    const level = spell["level"] as number | undefined;
    const spheres = (spell["spheres"] as Id[] | undefined) ?? [];
    if (level === undefined) continue;
    // Best access wins, so a spell in two spheres is offered on the better terms.
    let best: { sphere: Id; access: "major" | "minor" } | undefined;
    for (const s of spheres) {
      const how = access.get(s);
      if (how === undefined) continue;
      if (how === "minor" && level > MINOR_ACCESS_TO) continue;
      if (best === undefined || how === "major") best = { sphere: s, access: how };
    }
    if (best === undefined) continue;
    out.push({
      id: spell.id, name: spell.name, level,
      book: spell.provenance?.section[0] ?? "?",
      through: { sphere: pack.byId.get(best.sphere)?.name ?? best.sphere, access: best.access },
    });
  }
  return out.sort((a, b) => a.level - b.level || a.name.localeCompare(b.name));
}

/**
 * Table 43's initial funds, keyed by class group and printed as a die — `5d4 x 10 gp`.
 *
 * §9.1's starting money, and the roll belongs outside for the reason every roll does: it is
 * recorded randomness, and an Engine that rolled it could not record a roll made at the table.
 */
export function startingFunds(pack: Pack, classId: Id): string | undefined {
  const klass = pack.byId.get(classId);
  const groupId = groupsOf(klass)[0] ?? (klass?.isGroup === true ? klass.id : undefined);
  const rows = (pack.byId.get("phb:DD01613")?.["rows"] as string[][] | undefined) ?? [];
  return rows.find((r) => r[0] === groupId)?.[1];
}

/**
 * Armour class, which this corpus cannot compute, and the reason is worth stating precisely
 * because it is not the one it looks like.
 *
 * **Table 46 is not a list of armours. It is a list of COMBINATIONS**: *"Leather or padded armor
 * + shield, studded leather, or ring mail armor"* is one row, worth AC 7, and it names three
 * different ways to arrive there. Nothing in it can be looked up by "what am I wearing".
 *
 * And the pack's seven `armor` records are not an armour vocabulary either. Three were lifted
 * from Table 46's row labels and four from the COLUMN headings of Table 29, which is the
 * thieving-skill adjustment table — two different tables, for two different purposes, neither
 * of them "the armour a character owns". `Metal armour` and `Padded, Hide or Studded Leather`
 * are categories a rule discriminates on, not things anybody wears.
 *
 * So this reports rather than computes. An unarmoured character is 10 because Table 46 says so
 * in a row that happens to name a single state; everything else waits for an equipment list
 * that has not been transcribed.
 */
export function armourClass(pack: Pack, worn: Id[]): { ac?: number; because: string } {
  if (worn.length === 0) {
    const rows = (pack.byId.get("phb:DD01632")?.["rows"] as string[][] | undefined) ?? [];
    const none = rows.find((r) => norm(r[0] ?? "") === "none");
    return none === undefined
      ? { because: "no loaded pack has Table 46" }
      : { ac: Number.parseInt(none[1] ?? "10", 10), because: "Table 46: unarmoured" };
  }
  return {
    because: "Table 46 rates COMBINATIONS of armour and shield rather than pieces, and no loaded "
      + "pack has an equipment list to match one against",
  };
}
