/**
 * A pack's content hash (§8).
 *
 * It exists so that **added, removed and edited are all covered without a button anyone has
 * to remember to press** — that is §8's reason for a hash rather than a declared version, and
 * §6.5's for a Character recording one: *"a declared version would miss every typo fix, since
 * nobody bumps a version for one."*
 *
 * **Over parsed content in its canonical form, not over bytes.** Correction 55 gave the pack a
 * canonical serialisation precisely because five of its files disagreed about indentation, and
 * a script that changed five effects reserialised twenty-four thousand lines. Hashing bytes
 * would make every one of those reformats look like a change to every Character built against
 * the pack. Hashing the canonical form makes a reformat invisible and an edit visible, which
 * is what the hash is for.
 */
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Manifest } from "./types.ts";

/** Correction 55's canonical serialisation, and the reason it had to exist. */
export function canonical(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

export function hashPack(root: string): string {
  const manifest = JSON.parse(readFileSync(join(root, "manifest.json"), "utf8")) as Manifest;
  const h = createHash("sha256");
  // The manifest is hashed first and by name, so a file ADDED to or REMOVED from the
  // declaration changes the hash even if no file's contents did.
  h.update("manifest.json ");
  h.update(canonical(manifest));
  for (const name of [...manifest.files].sort()) {
    h.update(`${name} `);
    try {
      h.update(canonical(JSON.parse(readFileSync(join(root, name), "utf8"))));
    } catch {
      // A file the manifest declares and the directory does not have is part of what the
      // pack currently IS, so it hashes as itself rather than throwing.
      h.update(" absent ");
    }
  }
  return h.digest("hex");
}
