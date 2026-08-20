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
 * Armour class, which this corpus **can** compute — correction 61, resolved by reading the table
 * the way the book writes it rather than the way a lookup wants it.
 *
 * **Table 46 rates COMBINATIONS**, and the sentence that makes it look impossible is real:
 * *"Splint mail, banded mail, or bronze plate mail + shield, plate mail"* is one row worth AC 3.
 * The grammar is the whole problem — **`+ shield` attaches BACKWARDS** over the run of
 * alternatives before it, so that row means *(splint or banded or bronze plate) with a shield, or
 * plate mail on its own*. Read left to right, comma by comma, it means something else entirely and
 * produces no error while doing it: splint mail lands on AC 3 from this row and on AC 4 from the
 * one above, and nothing anywhere notices the contradiction.
 *
 * Applied properly the table is **complete**: fourteen armours each with exactly one rating alone
 * and thirteen of them with a shield, no collisions. The pack now carries that as a second record
 * beside the first — the same eleven rows, keyed by what a character wears — and this reads it.
 *
 * **The fifteenth cell is the interesting one.** Brigandine has no row that pairs it with a shield.
 * Scale mail and hide, its neighbours at AC 6, both drop to 5; the arithmetic is obvious and the
 * book does not say it, so this returns no number and names the gap. Guessing 5 would be a rule
 * invented by the Engine, which is the one thing it exists not to do.
 */
export function armourClass(pack: Pack, worn: Id[]): { ac?: number; because: string } {
  const table = pack.records("lookupTables").find((t) => t["supplies"] === "armourClass");
  if (table === undefined) {
    return { because: "no loaded pack has Table 46 as a rule over items" };
  }
  const rows = (table["rows"] as string[][] | undefined) ?? [];

  // Everything worn has to be a thing somebody wears. Silently ignoring what is not turns "you
  // are carrying a sword" into "you are wearing nothing", which is a wrong number with a straight
  // face — and `Metal armour` is the sharper case, because it IS in the armour kind. It is a
  // category the books discriminate on and nobody owns one, which is the whole of correction 61.
  // Wearable is declared, never recognised: a piece says WHERE it is worn, and a heading is
  // wearable because "a shield" is a thing a character can be said to carry without naming a
  // size. Matching `phb:shield` by id here would be a closed enumeration in the Engine, which
  // is the shape §3.4 exists to refuse.
  const wearable = new Map(pack.records("armor")
    .filter((a) => a["worn"] !== undefined)
    .map((a) => [a.id, a] as const));
  const cannot = worn.filter((id) => !wearable.has(id));
  if (cannot.length > 0) {
    return {
      because: cannot.map((id) => {
        const r = pack.byId.get(id);
        if (r === undefined) return `no loaded pack has ${id}`;
        return r["armorKind"] === "category"
          ? `${r.name} is a category a rule discriminates on, not a thing anybody wears`
          : `${r.name} is not armour`;
      }).join("; "),
    };
  }
  const items = worn.map((id) => wearable.get(id));
  const shield = items.some((r) => r?.["worn"] === "shield");
  const body = items.filter((r) => r?.["worn"] === "body");
  if (body.length > 1) {
    return { because: `nobody wears ${body.map((r) => r!.name).join(" and ")} at once` };
  }

  const key = body[0]?.id ?? "(nothing)";
  const row = rows.find((r) => r[0] === key);
  if (row === undefined) {
    return { because: `Table 46 has no rating for ${body[0]?.name ?? "an unarmoured character"}` };
  }
  const cell = row[shield ? 2 : 1] ?? "--";
  const ac = Number.parseInt(cell, 10);
  if (Number.isNaN(ac)) {
    return {
      because: `Table 46 rates ${body[0]?.name ?? "this"} on its own and never with a shield, and `
        + "the neighbouring armours' step is arithmetic the book does not print",
    };
  }
  const what = body[0] === undefined
    ? (shield ? "a shield and nothing else" : "no armour")
    : `${body[0].name}${shield ? " and a shield" : " alone"}`;
  // The equipment list prices a great helm and a basinet; Table 46 has no row for either. Saying
  // so beats dropping them from the sum in silence, which is how a number stops being checkable.
  const ignored = items.filter((r) => r?.["worn"] !== "body" && r?.["worn"] !== "shield").map((r) => r!.name);
  const note = ignored.length > 0 ? ` — Table 46 rates nothing worn there, so ${ignored.join(" and ")} changes nothing` : "";
  return { ac, because: `Table 46: ${what}${note}` };
}
