# The evaluator

Type: build
Status: in progress — the evaluator runs; the interface has not started

## What this builds

**The thing every other part of the Engine sits on**: load a Content Pack, apply the layers, and
answer *what is this Character's view of this value, and which rule and which book said so.*

No Electron, no interface, no persistence. Those are specified (§8, §9, §10) and none of them can be
built against nothing.

## Why this first

[Ticket 04](./04-first-light.md)'s `firstlight.py` already does most of it, and that is the argument
rather than against it. It was written to *find* things and it found 26 corrections; what it is not
is an Engine — it is a Python script that reads a private pack from an absolute path, has no tests,
and lives in a map's tools directory. **Everything it learned is worth keeping and nothing it is
worth shipping.**

The five grillings are resolved and their decisions are all *behavioural*: what happens to a marked
effect, what happens when two layers disagree, what a bound is, what an optional rule is. **Those
decisions exist only as prose and one throwaway.** Writing them down as code that runs is the point.

## The stack, and why it is nearly nothing

**TypeScript on Node, with no dependencies.** Node 24 runs `.ts` directly and `node --test` is in the
standard library, so the evaluator needs no bundler, no transpiler and no test framework. That is
worth having on purpose and not only for tidiness: a project whose posture is *never silently wrong*
should be able to say what every line it ships does.

Electron and React arrive with the interface (§10's decision, unchanged). They do not arrive here.

## The first question it has to answer

The map's first fog: **is the schema the Engine's internal model, or only its wire format?** It could
not be asked sharply until a pack had been loaded; it has now been loaded a few hundred times, and the
answer should fall out of the first file rather than be argued about.

## What "done" looks like

- A pack loads **from the manifest**, never by scanning (§7.1), and every complaint is reported
  rather than repaired.
- A Character's view of a value is a **stack of layers with provenance surviving to the top** (§4).
- The four reasons a value is withheld are all implemented and distinguishable: a marked effect
  ([02](./02-what-the-engine-does-with-an-unmodelled-effect.md)), an undecidable predicate, a
  contradiction nothing declares over ([03](./03-precedence-when-two-attachables-contradict.md)), and
  a campaign option no table has chosen (§5.5).
- **Tests run against hand-authored fixtures**, never against the corpus. The corpus does not
  circulate, so a test that needs it is a test nobody else can run. `fixtures/house-rules/` is the
  precedent and the shape.

## What it must not do

- **Not reimplement `firstlight.py` feature for feature.** That script grew a `--deity` flag, an
  ammunition rule and a reach classifier because each answered a question. The Engine takes the
  decisions, not the scaffolding.
- **Not read the private pack in anything committed.** A local smoke test against `~/corerules/slice`
  is fine and is not a test.

---

## Where it stands

`engine/`, TypeScript on Node 24, **27 tests, no runtime dependencies.** Two dev dependencies and
both are the typechecker: `typescript` and `@types/node`. Node runs `.ts` by stripping types and
`node --test` is in the standard library, so nothing bundles, transpiles or frameworks.

| | |
|---|---|
| `src/types.ts` | the pack's shapes, as the schema defines them |
| `src/pack.ts` | manifest-driven loading, the two indexes, transitive group expansion |
| `src/predicate.ts` | §6.1, **three-valued** |
| `src/sheet.ts` | the layer stack, the five reasons a value is set aside, bounds, table reads |
| `src/character.ts` | §6.3's sequence of Level Events, and everything derived from it |
| `src/uuid.ts` | UUIDv7, monotonic within a millisecond |
| `src/hash.ts` | a pack's content hash, over the canonical form |
| `src/library.ts` | §8's content root: packs, Characters, and drift |
| `src/smoke.ts` | not a test — it takes whatever pack path you give it |

**`Character` and `Sheet` are different things and the names now say so.** A Character is *"not
a snapshot but a sequence of Level Events, from which everything derived is recomputed"*; a
Sheet is the everything-derived. The first version called the layer stack a Character, which
would have been the wrong noun in every conversation after it.

### The fog is answered, and by the first file

**The schema is the internal model, not only the wire format.** Nothing is reshaped on load. §4
requires the provenance of every layer to survive to the top, so a normalised model would have to
keep the original record beside it and pay for two representations to answer one question; and the
six operations read `op`, `field`, `by` and `when` exactly as the pack writes them. What the Engine
adds is an index by id, an index by kind, and one traversal — all derived views of the same objects.

### It reproduces the pack, and then found something

Against the private slice, the TypeScript evaluator gives the same answers the Python throwaway did:
a thief permits **12 of 119** weapons, an Assassin **119** with the lifted rule named, a priest of
Agriculture **6** — the book's own six.

And then it produced a report `firstlight.py` never could:

```
SET ASIDE — unresolved (2)
    phb:dwarf[2]   adjust savingThrow.vsWandsStavesRodsSpells: the operand resolved to "+4", which cannot be summed
    phb:dwarf[3]   adjust savingThrow.vsPoison: the operand resolved to "+4", which cannot be summed
```

**A fifth reason a value is set aside, and the only one that is the Engine's fault rather than the
pack's.** It exists because the first version did what every evaluator does by default: filtered the
uncomputable contribution out of the sum and reported the field as **0**. Every `adjust` operand in
the corpus is an integer, a computed value or a table read — never a bare string — so an adjust that
does not produce a number is a defect somewhere, and **0 is the wrong number §5.2 names, produced by
the Engine itself.** Ticket 04 recorded `"+4"` as a finding; this is the first time it has been
*reported* rather than written down.

### §6.3, and the example that forced the shape

**Hit points are recorded randomness** — a third category beside a choice and a derivation — and
they are the reason the history is the file: under multi-class a total cannot be reconstructed from
*"fighter 5 / mage 4"*, so storing levels plus a total is not merely worse but **not implementable**.

A Level Event holds a **list** of rolls, and the PHB's own worked example is why. A
fighter/thief/mage rolls 6, 5 and 2 and begins with **4** — the sum divided by three — where
dividing each roll on its own gives 3. One rule covers creation and every later advance, *floor of
the event's total over the class count*, and it only covers both because the rolls travel together.
The test is the book's paragraph, including the later thief advance that rolls 4 and adds 1.

### The second id ever minted was out of order

§6.5 chose UUIDv7 so that *"file order is chronological order with no extra field"*. **A plain v7
does not do that**: two ids minted in the same millisecond are ordered by their random tails, and
creating a Character and advancing it happen in the same tick as a matter of course. `rand_a` is now
a counter, RFC 9562's monotonic method — without it the reason for choosing v7 was simply untrue,
and a test caught it on the second event this Engine ever created.

### §8, and correction 55 paying for itself

**Plain files are the source of truth**, and a directory is a pack when it holds a manifest —
discovery by that test, because a content root is *a place a user puts things*, which is the
opposite of §7.1's rule INSIDE a pack, where scanning would let a leftover file join in silence.
`characters/` holds no manifest and so is never mistaken for one.

The hash is **over parsed content in its canonical form, not over bytes**, and that is correction 55
paying a dividend nobody expected. Byte-hashing would make every reformat look like a change to
every Character built against the pack — and this project reformatted five files twice in two days.
A reformat is now invisible and a one-character typo fix is not, which is exactly what §6.5 asked
for when it chose a hash over a declared version.

Drift reports what it can actually check. A recorded hash says *whether* a pack moved and can never
say how, because the old pack is gone; what the Engine can check is **which of the ids this
Character names no longer resolve**, and that is the half a player can act on. A pack that vanished
entirely is reported and **the Character still opens** — §5.3 locks what extends a Character, never
the reading of one.

A real Character against the real slice is **781 bytes**, and the keys are ordered at write time
rather than at construction, because a file read back and written again would otherwise keep
whatever order it arrived in. §6.5 traded auditability for a file that stays legible; legibility is
a thing you have to do, not a thing you get.

### The interface, and where the promise finally shows

`app/` — Electron, one window, and the sheet. The main process owns the file system, the Library and
every pack; the renderer holds **no Node**, shares no context, runs sandboxed, and is handed a
**display model** rather than records. That is not only Electron hygiene: a pack is content the user
transcribed from books they own, and §1's posture holds only while it stays on their machine, so the
process that draws pixels is not the process that holds the pack. The window's CSP allows nothing
but its own two files, because there is no network in this application.

The main process's answers live in `service.ts`, testable without a window, and `main.ts` is left
with a window, five `ipcMain.handle` lines and where the settings file goes. Same discipline as the
engine: put the logic where it can be tested and leave the shell with nothing to decide.

It boots on a machine with no display, which is how it was checked here — `CORERULES_SMOKE` loads
the window, reads the text back and quits:

```
corerules
/tmp/…
CONTENT PACKS
Proving slice (ticket 08)          1,298 records · 8bb8178c2fe8
CHARACTERS
Thorin                             Dwarf / Warrior / Fighter / Clansdwarf · 8 hp
```

and clicking through to the sheet shows every value **with the layers that made it underneath, in
small type** — provenance is not a tooltip, because §1's promise is that it is there without being
asked for — and then the three sections that are the reason any of this exists:

```
NOT ON THE SHEET — CORERULES CANNOT COMPUTE THIS YET
  Dwarf — Player's Handbook
      adjust savingThrow.vsPoison: the operand resolved to "+4", which cannot be summed
NOT ON THE SHEET — APPLIES IN A CIRCUMSTANCE THE PACK COULD NOT EXPRESS
  Clansdwarf — Comp. Book of Dwarves
      UNMODELLED CONDITION: with others of his own clan.
NOT ON THE SHEET — ASKS ABOUT SOMETHING THIS SHEET HAS NO ANSWER FOR
```

## What is left
- **Constitution is not in the hit points.** The bonus is a table read with a per-class cap, and
  a plausible-looking total that quietly omits it is the kind of wrong number the rest of this
  refuses.

