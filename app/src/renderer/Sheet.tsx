import { useEffect, useState } from "react";
import type { Api, Timeline } from "../main/api.ts";
import type { SheetView, ValueLine } from "../../../engine/src/present.ts";

const api = window.corerules as Api;

/**
 * The sheet, and the part of it this project exists for.
 *
 * Most character generators show you numbers. This one also shows you **what is not on the
 * sheet and why**, with the record and the book named — which is §1's promise, and the only
 * reason the transcriber's leftover notes were ever worth carrying. A renderer that dropped
 * the second half would leave the Engine's best feature in a data structure nobody sees.
 */

function Value({ line }: { line: ValueLine }): React.JSX.Element {
  if (line.contested !== undefined) {
    return (
      <div className="value contested">
        <span className="path">{line.path}</span>
        <span className="refused">two books disagree, and neither claims precedence</span>
        <div className="why">
          {line.contested.map((c) => (
            <div key={c.record}>{String(c.value)} — {c.record}, {c.book}</div>
          ))}
        </div>
      </div>
    );
  }
  return (
    <div className="value">
      <span className="path">{line.path}</span>
      <span className="amount">{String(line.value)}</span>
      <div className="why">
        {line.from.map((f, i) => (
          <div key={`${f.record}-${i}`}>{f.op} {String(f.value)} — {f.record}, {f.book}</div>
        ))}
      </div>
    </div>
  );
}

/**
 * §9.2's third mode: the timeline, and the objections beside it.
 *
 * *"The same validation rules must hold on both paths, or sheet editing becomes the back door
 * that undoes §5."* So this shows what the wizard would have refused — and separately what
 * nobody can rule on, because with A3 undeclared that is every class in the corpus and putting
 * the two in one list would teach a reader to ignore both.
 */
function History({ id }: { id: string }): React.JSX.Element {
  const [t, setT] = useState<Timeline | undefined>(undefined);
  const load = (): void => { void api.timeline(id).then(setT); };
  useEffect(load, [id]);
  if (t === undefined) return <></>;

  return (
    <>
      <section>
        <h3>Derived</h3>
        {t.derived.thac0 !== undefined && (
          <div className="value"><span className="path">THAC0</span><span className="amount">{t.derived.thac0}</span></div>
        )}
        {t.derived.nextLevelAt !== undefined && (
          <div className="value">
            <span className="path">Next level at</span>
            <span className="amount">{t.derived.nextLevelAt.toLocaleString()} xp</span>
          </div>
        )}
        {t.derived.missing.map((m) => (
          <div className="value" key={m.value}>
            <span className="path">{m.value}</span>
            <span className="refused">not computed</span>
            <div className="why">{m.because}</div>
          </div>
        ))}
      </section>

      {t.objections.length > 0 && (
        <section>
          <h3>This character breaks a rule</h3>
          {t.objections.map((o, i) => (
            <div className="value contested" key={i}>
              <span className="path">{o.name}</span>
              <span className="refused">{o.step}</span>
              <div className="why">{o.because}</div>
            </div>
          ))}
        </section>
      )}
      {t.caveats.length > 0 && (
        <section>
          <h3>Not checked</h3>
          {t.caveats.map((o, i) => <div className="why" key={i}>{o.name}: {o.because}</div>)}
        </section>
      )}

      <section>
        <h3>Timeline</h3>
        {t.events.map((e, i) => (
          <div className="value" key={e.id}>
            <span className="path">
              level {i + 1} — {e.rolls.map((r) => `${r.class} (${r.die})`).join(", ")}
            </span>
            <span className="amount dim">{e.id.slice(0, 8)}</span>
            {e.chose.length > 0 && <div className="why">chose {e.chose.map((c) => c.ref).join(", ")}</div>}
          </div>
        ))}
        <div className="finish">
          <button
            type="button"
            onClick={() => {
              const cls = t.next.classes[0];
              if (cls === undefined) return;
              // The die is rolled here and recorded, never recomputed (§6.3).
              const die = 1 + Math.floor(Math.random() * Number.parseInt((t.next.die ?? "1d8").split("d")[1] ?? "8", 10));
              void api.levelUp(id, cls.id, die, []).then(load);
            }}
          >
            Advance {t.next.classes[0]?.name} to level {(t.next.classes[0]?.level ?? 0) + 1}
            {t.next.gains.length > 0 && ` — ${t.next.gains.map((g) => `${g.slots} ${g.kind}`).join(", ")}`}
          </button>
        </div>
      </section>
    </>
  );
}

export function Sheet({ view, id }: { view: SheetView; id: string | undefined }): React.JSX.Element {
  const byReason = new Map<string, SheetView["aside"]>();
  for (const a of view.aside) byReason.set(a.because, [...(byReason.get(a.because) ?? []), a]);

  return (
    <article className="sheet">
      <header>
        <h1>{view.name}</h1>
        <p className="dim">
          {view.who.map((w) => w.name).join(" / ")} · {view.hitPoints} hp ·{" "}
          {view.levels.map((l) => `${l.class} ${l.level}`).join(", ")}
        </p>
      </header>

      <section>
        <h3>Values</h3>
        {view.values.map((v) => <Value key={v.path} line={v} />)}
      </section>

      {view.granted.length > 0 && (
        <section>
          <h3>Granted</h3>
          {view.granted.map((g, i) => (
            <div className="value" key={`${g.name}-${i}`}>
              <span className="path">{g.name}</span>
              <span className="amount dim">{g.book}</span>
              {/* Ticket 02: a marked structural effect IS applied, and the marker rides on
                  the entry rather than replacing it. */}
              {g.rider !== undefined && <div className="why">{g.rider}</div>}
            </div>
          ))}
        </section>
      )}

      {view.owed.length > 0 && (
        <section>
          <h3>Choices owed</h3>
          {view.owed.map((o, i) => (
            <div className="value" key={`${o.kind}-${i}`}>
              <span className="path">{o.count} × {o.kind}</span>
              <span className="amount dim">
                {o.from === undefined ? "from a set the book does not enumerate" : `from ${o.from.length}`}
              </span>
            </div>
          ))}
        </section>
      )}

      {view.debt.length > 0 && (
        <section>
          <h3>Owed from an abandoned kit</h3>
          {/* §6.4: those specific proficiencies, not a count — and it may be unpayable, which
              is exactly why it has to be on the screen. */}
          <p>{view.debt.join(", ")}</p>
        </section>
      )}

      {id !== undefined && <History id={id} />}

      {[...byReason].map(([because, rows]) => (
        <section className="aside" key={because}>
          <h3>Not on the sheet — {rows[0]!.headline}</h3>
          {rows.map((a, i) => (
            <div className="value" key={`${a.record}-${i}`}>
              <span className="path">{a.record}</span>
              <span className="amount dim">{a.book}</span>
              <div className="why">{a.detail}</div>
            </div>
          ))}
        </section>
      ))}
    </article>
  );
}
