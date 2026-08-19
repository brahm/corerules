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

### §9.2, and the third answer

`engine/src/choice.ts` is the whole of guided creation that is worth testing. The wizard's screens
are an arrangement of **offers**; the offers are where the rules live, because §5 puts validation
**at the point of choice, not flagged afterwards** — so an option a character cannot take is not
presented and then rejected, it arrives already carrying the reason.

**An offer has three states, and the third is what makes it honest.** *Yes*, the rule holds. *No*,
and here is the rule and the book. *Unknown* — either the predicate asks something a half-built
character cannot answer yet, or **A3 means no pack ever claimed to cover this**. Collapsing unknown
into no would forbid what the books allow; collapsing it into yes would be the *flagged afterwards*
§5 exists to prevent.

Against the real slice, a dwarf with Charisma 9:

```
CLASS
  no        Paladin    requires Wisdom 13, Charisma 17 — Player's Handbook
  no        Ranger     requires Dexterity 13, Wisdom 14 — Player's Handbook
  no        Bard       requires Intelligence 13, Charisma 15 — Player's Handbook
  unknown   Fighter    no loaded pack declares which races may take which classes,
                       so corerules is not checking it
KIT for a dwarf fighter: 24 offered, 140 refused by name
```

**Nineteen classes come back unknown and every one says why.** The proving slice declares nothing,
so §5.1's A3 holds: the Engine does not validate race-and-class permission and **says so visibly**.
That is not a gap to tidy away later — it is the difference between *"the books allow this"* and
*"nobody has told me whether the books allow this"*, which is the whole of what A3 is for.

The abilities come from the pack, not from a prefix the interface knows: `phb:strength` is the
Player's Handbook's and another book's would be its own.

### Proficiencies, and a third state arriving in a budget

The Player's Handbook states the whole rule in four sentences, and the interesting part is not the
arithmetic but **which groups are open to this character**, because that is what decides whether a
proficiency costs one slot or two.

Against the real slice, a dwarf fighter: **4 weapon slots and 3 nonweapon, one more of each every 3
levels, −2 to hit with anything he is not proficient in** — Table 34, read by the class's group name.
Table 38's crossovers resolve through correction 56's `alsoPrinted`, so a Paladin's *"Warrior,
Priest, General"* becomes three ids and a Thief's becomes two.

And then the third state turns up somewhere new. Of 120 candidates, 36 cost one slot and 30 cost more
— and **54 carry a cost the books cannot decide**:

```
1?  Acting          no book says which group this belongs to,
                    so whether it costs one slot more is undecided
```

Every one of the PHB's 65 proficiencies names its group. **One** of the 55 from the Complete
handbooks does, and the books do not say either. So the crossover rule has nothing to test, and the
Engine charges the listed cost while saying the surcharge is undecided — because charging it anyway
would invent a rule **against the character**, in the one place a player would never think to check.
Sent back as correction 60.

One type wrinkle worth keeping: `group` is **an id on a class and a list on a proficiency**, because
a class belongs to one class group and Table 37 puts fifteen proficiencies in two. Both readings now
go through `groupsOf`, which is the only place that knows.

The budget is now a step in the wizard, and **it is a gate rather than a nag**: PHB DD01537 says
initial slots *"must be assigned immediately; they cannot be saved or held in reserve"*, so Create
stays disabled until they are spent. A candidate that costs more than remains is refused **in the
budget's own terms** — *"2 slots, and 1 left"* — because that is not a rule in a book and saying it
was would be a lie about which book.

```
NONWEAPON PROFICIENCIES
  3 of 3 slots left — they cannot be held in reserve
  Slow Respiration   Comp. Book of Dwarves
      2 slots — in Special Background (dwarven), which is not open to this class — one slot more
  Agriculture        Player's Handbook
      1 slot — in General Proficiency Group, which is open to this class
```

### The rest of §9.1, and two things the corpus cannot answer

The pipeline now runs **scores → race → subrace → class → alignment → weapon proficiencies →
nonweapon proficiencies → kit**, and the order is not cosmetic: **six kits and fifty-nine
priesthoods carry an alignment prerequisite**, so asking for alignment after the kit would turn a
decidable rule into an undecidable one — honestly reported and useless.

Weapon slots come from Table 34's other half and are bounded by what the class may take at all,
through the same `imposedBy` correction 48 found. A dwarf fighter gets **4 weapon slots and 3
nonweapon**, and the sheet says so.

Derived values are read from the books' own tables and **two of the four are not there**:

```
Fighter 1:  THAC0 20   next level at 2,000    missing: saving throws
Fighter 5:  THAC0 16   next level at 32,000   missing: saving throws
Thief 3:    THAC0 19   next level at 5,000    missing: saving throws
```

Table 53 and the four experience tables index cleanly. **Table 60 is in the pack with every cell
empty** — the rows are there and the numbers are not — so the Engine names the table and says it
cannot answer, which is precisely the state A3 exists to keep apart from *"no rule"*.

**Equipment and encumbrance are not built, and the reason is the corpus rather than the effort.**
Weapons carry a cost and a weight, but `armor` is seven CATEGORY records — *Metal armour*, *Leather
or padded armour* — and nobody can buy a category. Table 47 keys on prose. Building a shop against
that would produce a plausible screen over data that cannot support it, which is the one thing this
Engine has refused at every other turn.

### §9.2's other two modes, and the sentence they turn on

*"The same validation rules must hold on both paths, or sheet editing becomes the back door that
undoes §5."* So neither advancing nor correcting has rules of its own: an advance asks `choice.ts`
what may be chosen, and **a correction asks it the same question about a rewritten past.**

A correction is **applied and then answered for**, never refused — §6.5 traded auditability for a
legible file, and refusing the edit would hide a state instead of naming it:

```
after dropping Strength to 6 on the sheet:
   class: Fighter — requires Strength 9 — Player's Handbook
```

**Two lists, because the third state means something different here.** An *objection* is a choice
the books refuse; a *caveat* is one nobody can rule on — and with A3 undeclared that is **every
class in this corpus, permanently**. Putting them in one list would make a correctly built
character look broken on every load, which is the fastest way to teach someone to ignore the
warnings that matter.

Advancing shows what the level actually buys, which most levels is nothing:

```
at level 1: next rolls 1d10, gains []
at level 3: next rolls 1d10, gains [1 weaponProficiency, 1 nonweaponProficiency]
```

Driven in the running application, a dwarf fighter advanced to level 2 and the sheet followed:
**THAC0 19** from Table 53, **next level at 4,000 xp** from Table 14, saving throws *"not computed —
Table 60 is present in the pack with no numbers in it"*, and the timeline with both events by id.

## What is left
- **Editing the timeline in the interface.** The engine corrects an event and answers for it;
  the sheet shows the timeline and advances, and does not yet let you rewrite a past level.
- **Equipment, encumbrance and spell selection**, which want a corpus the slice does not have.
- **Constitution is not in the hit points.** The bonus is a table read with a per-class cap, and
  a plausible-looking total that quietly omits it is the kind of wrong number the rest of this
  refuses.

