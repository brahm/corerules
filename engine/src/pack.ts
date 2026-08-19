/**
 * Loading a Content Pack.
 *
 * §7.1 is declaration over discovery: the manifest's file list drives this, never a scan of
 * the directory. A file present but unlisted is reported and not loaded, because scanning
 * would let a leftover from an earlier extraction join the pack in silence.
 *
 * Nothing here repairs anything. Every surprise becomes a `Complaint` the caller can show,
 * which is the corpus map's own method — a finding recorded is worth more than a defect
 * quietly fixed — and it is also §1's product promise: when the Engine cannot do something,
 * it says which rule and which book.
 */
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import type { FieldDeclaration, Id, Manifest, Record_ } from "./types.ts";

export interface Complaint {
  area: "manifest" | "identity" | "reference" | "vocabulary";
  message: string;
}

/** Not kinds. `manifest` describes the pack; `fields` (correction 58) declares the
 *  vocabulary its effects write. Neither holds records, and `fields` is the first array in
 *  a pack that is not a list of records — the checker found that out by crashing. */
const NOT_A_KIND = new Set(["manifest", "fields"]);

export class Pack {
  readonly root: string;
  readonly manifest: Manifest;
  readonly byId = new Map<Id, Record_>();
  readonly byKind = new Map<string, Record_[]>();
  /** Correction 58: the field paths this pack says it writes. */
  readonly vocabulary = new Set<string>();
  readonly complaints: Complaint[] = [];

  // Not `constructor(readonly root: string)`. Node runs TypeScript by STRIPPING types, so
  // any construct that emits runtime code is refused — parameter properties, enums,
  // namespaces, decorators. `erasableSyntaxOnly` in tsconfig makes the typechecker say so
  // instead of leaving Node to say it at run time. The constraint is worth keeping on
  // purpose: everything this package ships is either a value you can read or a type that
  // vanishes, and nothing is generated behind your back.
  constructor(root: string) {
    this.root = root;
    this.manifest = JSON.parse(readFileSync(join(root, "manifest.json"), "utf8")) as Manifest;

    for (const name of this.manifest.files) {
      let doc: Record<string, unknown>;
      try {
        doc = JSON.parse(readFileSync(join(root, name), "utf8")) as Record<string, unknown>;
      } catch {
        this.complain("manifest", `declares ${name}, which is not there`);
        continue;
      }
      for (const [kind, value] of Object.entries(doc)) {
        if (!Array.isArray(value)) continue;
        if (kind === "fields") {
          for (const f of value as FieldDeclaration[]) this.vocabulary.add(f.path);
          continue;
        }
        if (NOT_A_KIND.has(kind)) continue;
        // Two files may contribute to the SAME kind — the PHB's proficiencies and the ones
        // the Complete handbooks add both arrive as `nonweaponProficiencies`. Arrays merge;
        // a plain assignment silently drops 65 records and leaves the count plausible.
        const into = this.byKind.get(kind) ?? [];
        for (const record of value as Record_[]) {
          if (this.byId.has(record.id)) {
            // Ids are globally scoped, not scoped by kind, so this is not a warning: a
            // consumer indexing by id silently loses one of the two records.
            this.complain("identity", `${record.id} is defined twice`);
          }
          this.byId.set(record.id, record);
          into.push(record);
        }
        this.byKind.set(kind, into);
      }
    }

    const declared = new Set(this.manifest.files);
    for (const f of readdirSync(root)) {
      if (f.endsWith(".json") && f !== "manifest.json" && !declared.has(f)) {
        this.complain("manifest", `${f} is present in the directory and not in the manifest`);
      }
    }
  }

  complain(area: Complaint["area"], message: string): void {
    this.complaints.push({ area, message });
  }

  /** Resolve a reference, complaining rather than throwing. A pack that points outward is
   *  normal — cross-pack referential integrity belongs to whatever loads several — so this
   *  reports and returns undefined. */
  get(id: Id, why: string): Record_ | undefined {
    const r = this.byId.get(id);
    if (r === undefined) this.complain("reference", `${why} points at ${id}, which is not in the pack`);
    return r;
  }

  records(kind: string): readonly Record_[] {
    return this.byKind.get(kind) ?? [];
  }

  /**
   * Expand a set of ids through group membership.
   *
   * A member may itself be a group: Table 44 nests Bastard sword under Sword, and the
   * Complete Fighter's nests Katana under Sword. Correction 49 gave the sixteen headings
   * their members, which had been sitting in the source as three spaces of indentation —
   * before that, a bound naming `phb:sword` permitted nothing at all.
   */
  expand(ids: Iterable<Id>): Set<Id> {
    const out = new Set<Id>();
    const seen = new Set<Id>();
    const stack = [...ids];
    while (stack.length > 0) {
      const id = stack.pop()!;
      if (seen.has(id)) continue;
      seen.add(id);
      const r = this.byId.get(id);
      if (r?.groupKind !== undefined) {
        if (r.members === undefined || r.members.length === 0) {
          this.complain("reference", `${id} is a group with no members: a bound naming it permits nothing`);
        }
        stack.push(...(r.members ?? []));
      } else {
        out.add(id);
      }
    }
    return out;
  }
}
