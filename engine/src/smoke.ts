/**
 * Load a pack and say what came back. Not a test — the tests run against hand-authored
 * fixtures, because the corpus does not circulate and a test that needs it is a test
 * nobody else can run. This takes whatever path you give it.
 *
 *   node src/smoke.ts <pack-dir>
 */
import { Pack } from "./pack.ts";

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
