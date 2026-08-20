/**
 * The content root (§8): **plain files are the source of truth**, and storage splits by who
 * owns the data.
 *
 * Packs and Characters live in a folder the user picks, visible on all three systems, because
 * backup is the user's job by design and `~/.config` is never backed up. Application state and
 * any derived cache go to the OS convention path instead, precisely because those must *not*
 * travel. Only the first half is here; the second belongs to the application.
 *
 * **No persistent index.** The corpus fits in memory, all-or-nothing loading already parses
 * the whole pack, and SQL cannot evaluate a prerequisite predicate — an index would cover the
 * cheap half and miss the expensive one, while being a second source of truth able to go
 * quietly stale.
 *
 *   <root>/<pack>/manifest.json     a directory is a pack if it holds a manifest
 *   <root>/characters/<uuid>.json   and `characters` holds none, so it is never mistaken for one
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Character, type CharacterFile } from "./character.ts";
import { canonical, hashPack } from "./hash.ts";
import { Pack } from "./pack.ts";

export interface PackEntry {
  id: string;
  directory: string;
  hash: string;
}

/** §6.5: on open, if a pack has moved, the Engine re-validates and reports what changed.
 *  **Loading still never fails** — a Character opens fully readable and printable whatever
 *  its packs have done, and what is locked is everything that extends it (§5.3). */
export interface Drift {
  pack: string;
  /** Absent where the pack is simply gone. */
  was?: string;
  now?: string;
  /** Ids the Character names that the pack no longer holds. */
  lost: string[];
}

/**
 * §6.5 trades auditability for **a file that stays legible in a text editor**, and a legible
 * file is one whose keys are in the order a reader wants them: who this is, then what they
 * are, then what the Engine needs to recompute. JSON preserves insertion order, so this is
 * the whole of it — but it has to happen at WRITE time, because a file read back and written
 * again would otherwise keep whatever order it arrived in.
 */
function ordered(file: CharacterFile): CharacterFile {
  const { id, name, race, subrace, kit, kitAbandoned, deity, worn, funds, scores, options, packs, events, ...rest } = file;
  return {
    id, name, race,
    ...(subrace !== undefined ? { subrace } : {}),
    ...(kit !== undefined ? { kit } : {}),
    ...(kitAbandoned !== undefined ? { kitAbandoned } : {}),
    ...(deity !== undefined ? { deity } : {}),
    ...(worn !== undefined ? { worn } : {}),
    ...(funds !== undefined ? { funds } : {}),
    scores, options, packs, events, ...rest,
  };
}

export class Library {
  readonly root: string;

  constructor(root: string) {
    this.root = root;
  }

  /** A directory is a pack if it holds a manifest. Discovery by that test rather than by a
   *  list, because a content root is a place a user puts things — which is the opposite of
   *  §7.1's rule INSIDE a pack, where scanning would let a leftover file join in silence. */
  packs(): PackEntry[] {
    if (!existsSync(this.root)) return [];
    const out: PackEntry[] = [];
    for (const name of readdirSync(this.root, { withFileTypes: true })) {
      if (!name.isDirectory()) continue;
      const directory = join(this.root, name.name);
      if (!existsSync(join(directory, "manifest.json"))) continue;
      const manifest = JSON.parse(readFileSync(join(directory, "manifest.json"), "utf8")) as { id: string };
      out.push({ id: manifest.id, directory, hash: hashPack(directory) });
    }
    return out.sort((a, b) => a.id.localeCompare(b.id));
  }

  load(packId: string): Pack {
    const entry = this.packs().find((p) => p.id === packId);
    if (entry === undefined) throw new Error(`no pack named ${packId} under ${this.root}`);
    return new Pack(entry.directory);
  }

  private characterDir(): string {
    return join(this.root, "characters");
  }

  characterIds(): string[] {
    const dir = this.characterDir();
    if (!existsSync(dir)) return [];
    // Sorted is chronological, because a Character's id is a v7 (§6.5).
    return readdirSync(dir).filter((f) => f.endsWith(".json")).map((f) => f.slice(0, -5)).sort();
  }

  readCharacter(id: string): CharacterFile {
    return JSON.parse(readFileSync(join(this.characterDir(), `${id}.json`), "utf8")) as CharacterFile;
  }

  /**
   * Written whole, through a temporary file and a rename.
   *
   * A Character is a single document and the rename is atomic on every system this ships to,
   * so an interrupted save leaves the previous file rather than half of the new one. That
   * matters more here than it usually would: **corrections rewrite history in place** (§6.5)
   * and the old value stops existing, so a torn write would destroy the only copy.
   */
  writeCharacter(file: CharacterFile): string {
    const dir = this.characterDir();
    mkdirSync(dir, { recursive: true });
    const path = join(dir, `${file.id}.json`);
    const temporary = `${path}.writing`;
    writeFileSync(temporary, canonical(ordered(file)));
    renameSync(temporary, path);
    return path;
  }

  /** Record the hash of every pack a Character was validated against, so §6.5's drift
   *  detection has something to compare. */
  stamp(file: CharacterFile): CharacterFile {
    const known = new Map(this.packs().map((p) => [p.id, p.hash]));
    file.packs = file.packs.map((p) => {
      const hash = known.get(p.id);
      return hash === undefined ? p : { id: p.id, sha256: hash };
    });
    return file;
  }

  /**
   * What changed under a Character since it was last validated.
   *
   * The recorded hash says *whether* a pack moved; it cannot say how, because the old pack is
   * gone. So what the Engine reports is what it can actually check: which of the ids this
   * Character names no longer resolve. That is the half a player can act on.
   */
  drift(file: CharacterFile): Drift[] {
    const known = new Map(this.packs().map((p) => [p.id, p]));
    const out: Drift[] = [];
    for (const recorded of file.packs) {
      const entry = known.get(recorded.id);
      if (entry === undefined) {
        out.push({ pack: recorded.id, ...(recorded.sha256 !== undefined ? { was: recorded.sha256 } : {}), lost: [] });
        continue;
      }
      if (recorded.sha256 === entry.hash) continue;
      const pack = new Pack(entry.directory);
      const named = [
        file.race, file.subrace, file.kit, file.deity,
        ...file.events.flatMap((e) => e.rolls.map((r) => r.class)),
        ...file.events.flatMap((e) => (e.chose ?? []).map((c) => c.ref)),
      ].filter((x): x is string => x !== undefined);
      out.push({
        pack: recorded.id,
        ...(recorded.sha256 !== undefined ? { was: recorded.sha256 } : {}),
        now: entry.hash,
        lost: [...new Set(named)].filter((id) => !pack.byId.has(id)).sort(),
      });
    }
    return out;
  }

  /** Open a Character. Never fails: §5.3 quarantines what EXTENDS a Character, never the
   *  reading of one, and a missing pack is exactly the case no edit in the interface could
   *  repair — the thing to choose is not loaded. */
  open(id: string): { character: Character | undefined; file: CharacterFile; drift: Drift[] } {
    const file = this.readCharacter(id);
    const drift = this.drift(file);
    const first = file.packs[0];
    let character: Character | undefined;
    if (first !== undefined) {
      try {
        character = new Character(this.load(first.id), file);
      } catch {
        character = undefined;
      }
    }
    return { character, file, drift };
  }
}
