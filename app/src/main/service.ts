/**
 * Everything the main process answers, as functions over a Library.
 *
 * Separated from `main.ts` so that it is testable without a window, which is the same
 * discipline the engine follows: put the logic where it can be tested and leave the shell
 * with nothing to decide. What remains in `main.ts` is Electron wiring — a window, five
 * `ipcMain.handle` lines, and where the settings file lives.
 */
import type { Library } from "../../../engine/src/library.ts";
import { present, type SheetView } from "../../../engine/src/present.ts";
import type { CharacterSummary, PackSummary } from "./api.ts";

export function packs(library: Library): PackSummary[] {
  return library.packs().map((entry) => {
    const pack = library.load(entry.id);
    return {
      id: entry.id,
      name: pack.manifest.name,
      directory: entry.directory,
      hash: entry.hash.slice(0, 12),
      records: pack.byId.size,
      // A pack's complaints belong on the screen and not in a log. §1's promise is that the
      // Engine says what it could not do, and a pack that contradicts itself is that case.
      complaints: pack.complaints.map((c) => `[${c.area}] ${c.message}`),
    };
  });
}

export function characters(library: Library): CharacterSummary[] {
  return library.characterIds().map((id) => {
    const opened = library.open(id);
    return {
      id,
      name: opened.file.name,
      // §6.5: loading never fails. A Character whose packs are gone still opens, still has a
      // name, and says what it cannot find rather than refusing to appear.
      who: opened.character === undefined
        ? "— its packs are not here —"
        : opened.character.sheet().layers.map((l) => l.record.name).join(" / "),
      hitPoints: opened.character?.hitPoints() ?? 0,
      drift: opened.drift.map((d) => ({ pack: d.pack, lost: d.lost })),
    };
  });
}

export function open(library: Library, id: string): SheetView | undefined {
  const opened = library.open(id);
  return opened.character === undefined ? undefined : present(opened.character);
}
