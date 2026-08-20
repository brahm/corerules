/**
 * What the sheet says — a display model, computed from a Character and holding no markup.
 *
 * This is where §1's promise becomes a thing you can see: **when the Engine refuses, it says
 * which rule refused and which book that rule came from.** The evaluator already knows both;
 * an interface that only rendered the numbers would throw away the feature the project was
 * built for, which is the same mistake ticket 02 caught the Engine nearly making with the
 * markers.
 *
 * Pure, and deliberately so. Every judgement about what a player is shown is testable without
 * a DOM, and the Electron renderer is left with nothing to decide.
 */
import type { Character } from "./character.ts";
import type { Aside, Contribution, Sheet } from "./sheet.ts";
import type { Id } from "./types.ts";

export interface ValueLine {
  path: string;
  /** Absent when the value is contested — §5.3's quarantine, at the grain of one number. */
  value: number | string | undefined;
  /** One line per layer that contributed, nearest the top last. */
  from: { record: string; book: string; op: "adjust" | "set"; value: number | string | undefined }[];
  contested?: { record: string; book: string; value: number | string | undefined }[];
}

export interface AsideLine {
  because: Aside["because"];
  /** What the player is told, in the Engine's words rather than the transcriber's. */
  headline: string;
  /** The transcriber's words, kept because they are the only place a refusal carries a reason. */
  detail: string;
  record: string;
  book: string;
  value?: number | string | undefined;
  option?: string;
}

export interface SheetView {
  name: string;
  /** Race / class / kit, in the order the layers were applied. */
  who: { role: string; name: string; book: string }[];
  hitPoints: number;
  levels: { class: string; level: number }[];
  values: ValueLine[];
  granted: { name: string; book: string; rider?: string }[];
  /** Correction 14: `bounded` is what a `from` list means once the pack has declared it —
   *  `closed` licenses a refusal, `example` and `undeclared` do not. */
  owed: { kind: string; count: number; from?: string[]; bounded: "closed" | "example" | "undeclared" | "none" }[];
  /** §6.4: those specific proficiencies, not a count, and shown or it becomes a phantom bug. */
  debt: string[];
  aside: AsideLine[];
  /** Whatever the pack could not answer for itself. */
  complaints: string[];
}

const HEADLINE: Record<Aside["because"], string> = {
  marked: "applies in a circumstance the pack could not express",
  undecidable: "asks about something this sheet has no answer for",
  option: "your table has not said whether it plays this rule",
  unresolved: "corerules cannot compute this yet",
};
// `contested` is deliberately absent: it is a property of a VALUE and not of an effect, so it
// lives on the line it spoils rather than in this list. The four here are reasons a single
// contribution never reached a total; a contest is two that did and cannot both stand.

const named = (sheet: Sheet, id: Id): string => sheet.pack.byId.get(id)?.name ?? id;

const line = (c: Contribution): ValueLine["from"][number] => ({
  record: c.source.name, book: c.source.book, op: c.op, value: c.value,
});

export function present(character: Character): SheetView {
  const sheet = character.sheet();
  const levels = character.levels();

  const values: ValueLine[] = [];
  for (const path of [...sheet.fields.keys()].sort()) {
    const v = sheet.view(path);
    values.push({
      path,
      value: v.value,
      from: v.from.map(line),
      ...(v.contested !== undefined
        ? { contested: v.contested.map((c) => ({ record: c.source.name, book: c.source.book, value: c.value })) }
        : {}),
    });
  }

  const aside: AsideLine[] = sheet.aside.map((a) => ({
    because: a.because,
    headline: HEADLINE[a.because],
    detail: a.text,
    record: a.source.name,
    book: a.source.book,
    ...("value" in a ? { value: a.value } : {}),
    ...(a.because === "option" ? { option: a.option } : {}),
  }));

  return {
    name: character.file.name,
    who: sheet.layers.map((l) => ({
      role: l.role, name: l.record.name, book: l.record.provenance?.section[0] ?? "?",
    })),
    hitPoints: character.hitPoints(),
    levels: Object.entries(levels).map(([id, level]) => ({ class: named(sheet, id), level })),
    values,
    granted: sheet.granted.map((g) => ({
      name: g.ref !== undefined ? named(sheet, g.ref) : (g.defines ?? "?"),
      book: g.source.book,
      ...(g.rider !== undefined ? { rider: g.rider } : {}),
    })),
    owed: sheet.owed.map((o) => ({
      kind: o.kind, count: o.count,
      ...(o.from !== undefined ? { from: o.from.map((id) => named(sheet, id)) } : {}),
      bounded: o.from === undefined ? "none" as const : o.listing ?? "undeclared" as const,
    })),
    debt: character.debt().map((id) => named(sheet, id)),
    aside,
    complaints: sheet.pack.complaints.map((c) => `[${c.area}] ${c.message}`),
  };
}

/**
 * The same view, as text. Not a fallback for the desktop application — a way to look at what
 * the interface will be given, without a window, and the thing a bug report can paste.
 */
/** Correction 14: three of these four say the Engine may not refuse an unlisted choice. */
const BOUND: Record<SheetView["owed"][number]["bounded"], string> = {
  closed: "and the pack says that list is all of them",
  example: "which the pack says are examples",
  undeclared: "and the pack does not say whether that list is all of them",
  none: "unbounded",
};

export function render(view: SheetView): string {
  const out: string[] = [];
  out.push(`${view.name} — ${view.hitPoints} hp`);
  out.push(view.who.map((w) => `${w.name} (${w.role})`).join(" / "));
  out.push(view.levels.map((l) => `${l.class} ${l.level}`).join(", "));

  out.push("", "VALUES");
  for (const v of view.values) {
    if (v.contested !== undefined) {
      out.push(`  ${v.path.padEnd(38)}— two books disagree`);
      for (const c of v.contested) out.push(`      ${String(c.value).padEnd(14)}${c.record} — ${c.book}`);
    } else {
      out.push(`  ${v.path.padEnd(38)}${String(v.value)}`);
    }
  }

  if (view.owed.length > 0) {
    out.push("", "CHOICES OWED");
    for (const o of view.owed) {
      out.push(`  ${o.count} x ${o.kind}${o.from !== undefined ? ` from ${o.from.length}` : ""} — ${BOUND[o.bounded]}`);
    }
  }
  if (view.debt.length > 0) {
    out.push("", `DEBT FROM AN ABANDONED KIT (${view.debt.length})`, `  ${view.debt.join(", ")}`);
  }

  const byReason = new Map<string, AsideLine[]>();
  for (const a of view.aside) byReason.set(a.because, [...(byReason.get(a.because) ?? []), a]);
  for (const [because, rows] of byReason) {
    out.push("", `NOT ON THE SHEET — ${rows[0]!.headline} (${rows.length})`);
    for (const a of rows.slice(0, 6)) {
      out.push(`  ${a.record} — ${a.book}${a.option !== undefined ? `  [${a.option}]` : ""}`);
      if (a.detail !== "") out.push(`      ${a.detail.slice(0, 110)}`);
    }
    if (rows.length > 6) out.push(`  …and ${rows.length - 6} more`);
    void because;
  }
  return out.join("\n");
}
