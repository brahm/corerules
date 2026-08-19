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
| `src/character.ts` | the layer stack, the five reasons a value is set aside, bounds, table reads |
| `src/smoke.ts` | not a test — it takes whatever pack path you give it |

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

## What is left

- **The Character is not yet a sequence of Level Events** (§6.3). It is a stack of layers with
  scores handed in, which is what a view of a value needs and not what a Character is.
- **Nothing persists** (§8).
- **No interface** (§9, §10). Electron and React arrive there, not here.
