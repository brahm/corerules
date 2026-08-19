/**
 * Load a pack and say what came back. Not a test — the tests run against hand-authored
 * fixtures, because the corpus does not circulate and a test that needs it is a test
 * nobody else can run. This takes whatever path you give it.
 *
 *   node src/smoke.ts <pack-dir> [race] [class] [kit] [deity]
 */
import { Pack } from "./pack.ts";
import { Sheet } from "./sheet.ts";
import { Character } from "./character.ts";

const root = process.argv[2];
if (root === undefined) {
  console.error("usage: node src/smoke.ts <pack-dir>");
  process.exit(2);
}

const pack = new Pack(root);
console.log(`${pack.manifest.name} — ${pack.manifest.id} ${pack.manifest.version}`);
console.log(`  ${pack.byId.size} records across ${pack.byKind.size} kinds`);
console.log(`  ${pack.vocabulary.size} field paths declared`);
const kinds = [...pack.byKind].sort((a, b) => b[1].length - a[1].length);
for (const [kind, records] of kinds.slice(0, 8)) {
  console.log(`    ${kind.padEnd(24)}${records.length}`);
}
if (pack.complaints.length > 0) {
  console.log(`\n  ${pack.complaints.length} complaints:`);
  for (const c of pack.complaints.slice(0, 10)) console.log(`    [${c.area}] ${c.message}`);
} else {
  console.log("\n  nothing to complain about");
}

// ---------------------------------------------------------------- a character
const [, , , race, klass, kit, deity] = process.argv;
if (race === undefined) process.exit(0);

const scores = { "phb:strength": 16, "phb:dexterity": 12, "phb:constitution": 15,
                 "phb:intelligence": 10, "phb:wisdom": 16, "phb:charisma": 9 };
const character = Character.create(pack, {
  name: "Smoke", race, scores,
  ...(kit !== undefined && kit !== "" ? { kit } : {}),
  ...(deity !== undefined && deity !== "" ? { deity } : {}),
  options: process.env["OPTIONS"]?.split(",").filter((x) => x !== "") ?? [],
});
character.advance([{ class: klass ?? "", die: 8 }]);
const c = character.sheet();

const who = c.layers.map((l) => l.record.name).join(" / ");
console.log(`\nCHARACTER: ${who} — ${character.hitPoints()} hp, level ${Object.values(character.levels())[0]}`);
console.log(`  ${character.file.id}`);

console.log("\nFIELDS — the total, and nothing in it is approximate");
for (const path of [...c.fields.keys()].sort()) {
  const v = c.view(path);
  if (v.contested !== undefined) {
    console.log(`    ${path.padEnd(40)}CONTESTED`);
    for (const x of v.contested) console.log(`        ${String(x.value).padEnd(14)}${x.source.name} — ${x.source.book}`);
  } else if (v.value !== undefined) {
    console.log(`    ${path.padEnd(40)}${v.value}`);
  }
}

for (const because of ["marked", "undecidable", "option", "unresolved"] as const) {
  const rows = c.aside.filter((a) => a.because === because);
  if (rows.length === 0) continue;
  console.log(`\nSET ASIDE — ${because} (${rows.length})`);
  for (const a of rows.slice(0, 4)) {
    console.log(`    ${(a.source.record + "[" + a.source.index + "]").padEnd(20)}${a.text.slice(0, 96)}`);
  }
}

const weapons = pack.records("weaponProficiencies").filter((r) => r.groupKind === undefined).map((r) => r.id);
const { allowed, bounds, lifted } = c.permitted("weaponProficiency", weapons);
console.log(`\nPERMITTED weaponProficiency: ${allowed.size} of ${weapons.length}`);
for (const b of bounds) console.log(`    bound by ${pack.byId.get(b.ref!)?.name}`);
for (const l of lifted) console.log(`    LIFTED   ${pack.byId.get(l)?.name ?? l}`);
if (allowed.size <= 20) {
  console.log("        " + [...allowed].map((i) => pack.byId.get(i)?.name).sort().join(", "));
}
