import { useEffect, useState } from "react";
import type { Api, Draft, Offer, Step } from "../main/api.ts";

/**
 * §9.2's guided creation.
 *
 * **The refusals are the feature.** A generator that hid what a character cannot take would be
 * easier to use and would be lying about the books; §5's posture is that illegal states are
 * unrepresentable *at the point of choice*, and this is that point. So a refused option stays
 * on the screen, greyed, with the rule and the book that refused it — and an option nobody can
 * rule on stays too, saying that nobody can.
 */

const api = window.corerules as Api;

/** §9.1: the tool rolls dice, and entry stays a first-class path. Both end as scores. */
function roll4d6DropLowest(): number {
  const dice = [0, 0, 0, 0].map(() => 1 + Math.floor(Math.random() * 6)).sort((a, b) => a - b);
  return dice[1]! + dice[2]! + dice[3]!;
}

function Scores(
  { abilities, scores, onChange }: { abilities: Offer[]; scores: Record<string, number>; onChange: (s: Record<string, number>) => void },
): React.JSX.Element {
  return (
    <div>
      <div className="offers">
        {abilities.map((a) => {
          const id = a.id;
          return (
            <label className="score" key={a.id}>
              <span>{a.name}</span>
              <input
                type="number" min={3} max={25} value={scores[id] ?? ""}
                onChange={(e) => {
                  const n = Number.parseInt(e.target.value, 10);
                  onChange({ ...scores, ...(Number.isNaN(n) ? {} : { [id]: n }) });
                }}
              />
            </label>
          );
        })}
      </div>
      <button
        type="button"
        onClick={() => { onChange(Object.fromEntries(abilities.map((a) => [a.id, roll4d6DropLowest()]))); }}
      >
        Roll them (4d6, drop the lowest)
      </button>
    </div>
  );
}

function Offers({ offers, onPick }: { offers: Offer[]; onPick: (id: string) => void }): React.JSX.Element {
  const order: Record<Offer["available"], number> = { yes: 0, unknown: 1, no: 2 };
  return (
    <div className="offers">
      {[...offers].sort((a, b) => order[a.available] - order[b.available]).map((o) => (
        <button
          type="button"
          key={o.id}
          className={`offer ${o.available}`}
          disabled={o.available === "no"}
          onClick={() => { onPick(o.id); }}
        >
          <span className="offer-name">{o.name}</span>
          <span className="dim">{o.book}</span>
          {/* The reason travels with the option. §1: which rule refused, and which book. */}
          {o.because !== undefined && <span className="why">{o.because}</span>}
        </button>
      ))}
    </div>
  );
}

export function Wizard(
  { packId, onDone, onCancel }: { packId: string; onDone: () => void; onCancel: () => void },
): React.JSX.Element {
  const [draft, setDraft] = useState<Draft>({});
  const [name, setName] = useState("");
  const [steps, setSteps] = useState<Step[]>([]);

  useEffect(() => { void api.steps(packId, draft).then(setSteps); }, [packId, draft]);

  const pick = (key: Step["key"], id: string): void => {
    setDraft((d) => ({ ...d, [key]: id }));
  };

  const ready = draft.race !== undefined && draft.class !== undefined && name.trim() !== "";

  return (
    <section className="wizard">
      <header>
        <h1>A new character</h1>
        <button type="button" onClick={onCancel}>cancel</button>
      </header>

      <h3>Name</h3>
      <input className="name" value={name} onChange={(e) => { setName(e.target.value); }} />

      {steps.map((step) => (
        <div key={step.key}>
          <h3>
            {step.title}
            {step.state === "waiting" && <span className="dim"> — after the steps above</span>}
            {step.state === "done" && <span className="dim"> — chosen</span>}
          </h3>
          {step.key === "scores"
            ? <Scores abilities={step.offers} scores={draft.scores ?? {}} onChange={(s) => { setDraft((d) => ({ ...d, scores: s })); }} />
            : step.state === "waiting"
              ? null
              : <Offers offers={step.offers} onPick={(id) => { pick(step.key, id); }} />}
        </div>
      ))}

      <div className="finish">
        <button
          type="button"
          disabled={!ready}
          onClick={() => {
            // §6.3: hit points are recorded randomness. The roll happens here, once, and the
            // number is what the file keeps — the Engine is never asked to reproduce it.
            const die = 1 + Math.floor(Math.random() * 10);
            void api.create(packId, { ...draft, name }, die).then(onDone);
          }}
        >
          Create
        </button>
      </div>
    </section>
  );
}
