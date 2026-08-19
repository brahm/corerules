/**
 * The whole surface between the two processes, in one file so it can be read in one sitting.
 *
 * **The renderer never touches the file system**, and that is not only Electron hygiene here.
 * A Content Pack is WotC-derived content the user transcribed from books they own; §1's
 * posture only holds while it stays on their machine, so the process that talks to the world
 * is not the process that holds the pack. The renderer is given a display model — names,
 * numbers and the books they came from — and never the records.
 */
import type { Draft, Step } from "../../../engine/src/choice.ts";
import type { SheetView } from "../../../engine/src/present.ts";

export type { Draft, Offer, Step } from "../../../engine/src/choice.ts";

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
  /** §9.2's guided creation. The draft goes over the wire each time rather than living in the
   *  main process: the rules are a pure function of it, so there is no session to lose. */
  steps(packId: string, draft: Draft): Promise<Step[]>;
  /** The tool rolls dice (§9.1), and entry stays a first-class path — so both arrive here as
   *  scores already decided, and the Engine never learns which it was. */
  create(packId: string, draft: Draft & { name: string; startingWealth?: number }, hitDie: number): Promise<string>;
}

/** One place for the channel names, so a typo is a build error rather than a silent no-op. */
export const CHANNEL = {
  root: "corerules:root",
  chooseRoot: "corerules:chooseRoot",
  packs: "corerules:packs",
  characters: "corerules:characters",
  open: "corerules:open",
  steps: "corerules:steps",
  create: "corerules:create",
} as const;
