/**
 * The whole surface between the two processes, in one file so it can be read in one sitting.
 *
 * **The renderer never touches the file system**, and that is not only Electron hygiene here.
 * A Content Pack is WotC-derived content the user transcribed from books they own; §1's
 * posture only holds while it stays on their machine, so the process that talks to the world
 * is not the process that holds the pack. The renderer is given a display model — names,
 * numbers and the books they came from — and never the records.
 */
import type { SheetView } from "../../../engine/src/present.ts";

export interface PackSummary {
  id: string;
  name: string;
  directory: string;
  /** §8's per-pack content hash, short enough to read. */
  hash: string;
  records: number;
  complaints: string[];
}

export interface CharacterSummary {
  id: string;
  name: string;
  /** Race and class by name, for a list you can scan. */
  who: string;
  hitPoints: number;
  /** §6.5: what moved under this Character since it was last validated. */
  drift: { pack: string; lost: string[] }[];
}

export interface Api {
  root(): Promise<string>;
  chooseRoot(): Promise<string | undefined>;
  packs(): Promise<PackSummary[]>;
  characters(): Promise<CharacterSummary[]>;
  open(id: string): Promise<SheetView | undefined>;
}

/** One place for the channel names, so a typo is a build error rather than a silent no-op. */
export const CHANNEL = {
  root: "corerules:root",
  chooseRoot: "corerules:chooseRoot",
  packs: "corerules:packs",
  characters: "corerules:characters",
  open: "corerules:open",
} as const;
