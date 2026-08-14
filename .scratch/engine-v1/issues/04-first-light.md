# First light: the smallest program that loads the pack

Type: task
Status: open

## Question

Nothing to decide. **This is the experiment the corpus map closed by naming and could not perform**:
its last sentence is *"nothing has ever loaded this pack"*, and everything both prior maps concluded
about the format rests on artifacts that no program has ever consumed.

`validate.py` proves the pack conforms to a schema. `verdict.py` proves things about its contents.
**Neither is a program building a character**, and the distinction is the same one A3 exists to keep:
a pack that validates is not a pack that works.

## What it must produce

1. **A program that loads the pack** — the manifest, the 20 kinds, the references between them — and
   fails loudly on anything it cannot make sense of. Not the Engine. A hundred lines that read JSON
   and build an object graph.
2. **One character, computed.** A dwarf fighter with a kit is enough: it exercises a Race with
   effects, a Class, an Attachable, the layer model, a `tableValue` read and at least one marked
   effect. **The sheet does not have to be right. It has to exist**, and the ways it is wrong are the
   finding.
3. **A list of everything the pack turned out not to say.** The prediction, stated in advance so it
   can be scored: the gaps will not be in the operations — those were measured over 1,910 effects —
   but in **the joins between kinds**, which nothing has ever traversed.

## Why now, before the Electron app

Because it is cheap and because the alternative is worse. The specified application is Electron +
TypeScript + React with packaging, persistence and a product surface; a wrong assumption about the
pack discovered *there* is discovered under three layers of scaffolding. **Discovering it in a
throwaway script costs an afternoon.**

There is a second reason, and it is the stronger one. Ticket 01 has to decide what an implementation
session reads on day one, and ticket 02 has to decide what the Engine does with 380 marked effects.
**Both of those decisions get better with one running example in hand** — the first character will
show whether the corrections list is a day-one document or a reference, and how many markers a real
character actually touches.

## What it must not do

**Do not let it become the Engine.** The temptation, once JSON is loading and a sheet is printing, is
to keep going — add a UI, add persistence, and quietly skip the map. This produces a character in a
terminal and then stops.

**Do not fix the pack while writing it.** Everything the loader stumbles on is evidence. Record it;
the corpus map's own method was that a finding recorded is worth more than a defect quietly repaired,
and this is the first time anything has looked at the pack from the consuming side.

## Its result feeds

Tickets [01](./01-which-spec-does-the-engine-implement.md), [02](./02-what-the-engine-does-with-an-unmodelled-effect.md)
and [03](./03-precedence-when-two-attachables-contradict.md) — all three ask questions that a single
worked character makes concrete. **Ticket 03 especially**: a dwarf priest with a kit is the exact case
where two Attachables contradict, and it can be built on purpose.
