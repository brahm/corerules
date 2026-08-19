import type { SheetView, ValueLine } from "../../../engine/src/present.ts";

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

export function Sheet({ view }: { view: SheetView }): React.JSX.Element {
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
