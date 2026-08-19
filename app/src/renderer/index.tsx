import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Api, CharacterSummary, PackSummary } from "../main/api.ts";
import type { SheetView } from "../../../engine/src/present.ts";
import { Sheet } from "./Sheet.tsx";
import { Wizard } from "./Wizard.tsx";

declare global {
  interface Window { corerules: Api }
}

const api = window.corerules;

function Empty({ onPick }: { onPick: () => void }): React.JSX.Element {
  return (
    <div className="empty">
      <h2>Nothing here yet.</h2>
      <p>
        corerules ships no game content. It supplies the rules logic; you supply the data, from
        books you own, as Content Packs — and a fresh install opens empty because of it.
      </p>
      <p>Point it at the folder where your packs live, or where you would like them to.</p>
      <button type="button" onClick={onPick}>Choose that folder</button>
    </div>
  );
}

function Packs({ packs }: { packs: PackSummary[] }): React.JSX.Element {
  return (
    <section>
      <h3>Content Packs</h3>
      {packs.map((p) => (
        <div className="row" key={p.id}>
          <div>
            <strong>{p.name}</strong>
            <div className="dim">{p.records.toLocaleString()} records · {p.hash}</div>
            {p.complaints.map((c) => <div className="warn" key={c}>{c}</div>)}
          </div>
        </div>
      ))}
    </section>
  );
}

function Characters(
  { characters, onOpen }: { characters: CharacterSummary[]; onOpen: (id: string) => void },
): React.JSX.Element {
  return (
    <section>
      <h3>Characters</h3>
      {characters.length === 0 && <p className="dim">None yet.</p>}
      {characters.map((c) => (
        <button type="button" className="row link" key={c.id} onClick={() => { onOpen(c.id); }}>
          <div>
            <strong>{c.name}</strong>
            <div className="dim">{c.who} · {c.hitPoints} hp</div>
            {/* §6.5: if a pack has moved, say what changed. Loading never fails. */}
            {c.drift.map((d) => (
              <div className="warn" key={d.pack}>
                {d.pack} has changed
                {d.lost.length > 0 ? ` — this character can no longer find ${d.lost.join(", ")}` : ""}
              </div>
            ))}
          </div>
        </button>
      ))}
    </section>
  );
}

function App(): React.JSX.Element {
  const [root, setRoot] = useState("");
  const [packs, setPacks] = useState<PackSummary[]>([]);
  const [characters, setCharacters] = useState<CharacterSummary[]>([]);
  const [sheet, setSheet] = useState<SheetView | undefined>(undefined);
  const [openId, setOpenId] = useState<string | undefined>(undefined);
  const [creating, setCreating] = useState(false);

  const refresh = async (): Promise<void> => {
    setRoot(await api.root());
    setPacks(await api.packs());
    setCharacters(await api.characters());
  };
  useEffect(() => { void refresh(); }, []);

  const pick = async (): Promise<void> => {
    if ((await api.chooseRoot()) !== undefined) await refresh();
  };

  if (creating && packs[0] !== undefined) {
    return (
      <main>
        <Wizard
          packId={packs[0].id}
          onCancel={() => { setCreating(false); }}
          onDone={() => { setCreating(false); void refresh(); }}
        />
      </main>
    );
  }

  if (sheet !== undefined) {
    return (
      <main>
        <button type="button" className="back" onClick={() => { setSheet(undefined); void refresh(); }}>← back</button>
        <Sheet view={sheet} id={openId} />
      </main>
    );
  }

  return (
    <main>
      <header>
        <h1>corerules</h1>
        <button type="button" onClick={() => { void pick(); }}>{root}</button>
      </header>
      {packs.length === 0
        ? <Empty onPick={() => { void pick(); }} />
        : (
          <>
            <Packs packs={packs} />
            <Characters characters={characters} onOpen={(id) => { setOpenId(id); void api.open(id).then(setSheet); }} />
            <div className="finish">
              <button type="button" onClick={() => { setCreating(true); }}>Create a character</button>
            </div>
          </>
        )}
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<StrictMode><App /></StrictMode>);
