# The evaluator

Type: build
Status: in progress — the evaluator runs, the interface ships, and the corrections list is empty

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

### The timeline, edited in place

§9.2's third mode is *"direct editing on the sheet, exposing the timeline"*, and it is now that
rather than a read-only list. A level's class is a select and its die roll is a number, edited on the
row — **no draft and nothing to confirm**, because §6.5 already decided what a correction is: the
old value stops existing.

Driven in the running application on a dwarf fighter, changing the die to 3 and the class to Mage:

```
Dwarf / Wizard / Mage / Clansdwarf · 3 hp · Mage 1

THIS CHARACTER BREAKS A RULE
  Clansdwarf   kit   belongs to the Fighter — Comp. Book of Dwarves

NOT CHECKED
  Mage: its ability requirements have not been rolled yet
```

and the file on disk holds `{"class": "phb:mage", "die": 3}` — history rewritten, not appended to.

**That is §9.2's back door standing open and being walked through, and the sheet naming the book on
the way past.** The edit was not refused: refusing would leave a user looking at a sheet they can
see is wrong with no way to say so, and §5.3 already settled that what gets locked is what *extends*
a Character rather than the reading or the fixing of one.

Every edit reloads both halves. A corrected level changes the values above it — hit points, THAC0,
what the kit granted — and a sheet showing yesterday's numbers beside today's timeline would be
worse than one that refused the edit.

Removing a level takes its choices with it, because they travelled in the event that made them.
§6.3 put them there for exactly this.

### §10, and what building it for real turned up

`app/electron-builder.yml`, `app/build/entitlements.mac.plist` and
`.github/workflows/release.yml` — the artifact matrix, the proven macOS block copied verbatim,
and a tag that builds three artifacts and leaves a draft.

**Every version the spec named checked out, including the odd one.** `actions/checkout@v7.0.1` and
`actions/setup-node@v7.0.0` are current; and npm's `latest` tag for electron-builder is **still
26.15.3 while 26.15.7 exists** — verified a third time, so `npm install electron-builder` silently
gives you the older one and the workflow pins the version explicitly. `actions/download-artifact` is
at **v8**, not v7: upload and download are versioned separately and their numbers have never
matched, which is how you pin a tag that does not exist.

**A Linux AppImage was built and run.** 127 MB, and it finds the real pack:

```
/tmp/tmp.wm9rNj6mTj
CONTENT PACKS
Proving slice (ticket 08)     1,298 records · b3d48ad1ab71
```

The build failed the first time, at the very last step after four minutes of downloading and
packaging: `executableName contains characters that cannot be safely used in file paths:
@corerulesapp`. electron-builder derives the executable from the package **name**, and this one is
scoped. Fixed with an explicit `executableName`, and the comment says why so the next person does
not spend the four minutes.

Two smaller things the build said out loud and are now handled or named: `syncDesktopName`, without
which a desktop environment shows the running window as a second nameless thing beside its own
launcher — and **there is no icon**, so a release today would ship under the Electron logo. That is
a real gap and it is named rather than papered over.

### The icon, which is geometry rather than a drawing

`build/icon.py` draws it and is committed beside the PNG, because an asset nobody can regenerate
is a mystery binary in a repository that otherwise explains itself.

**A twenty-sided die seen face on, from the icosahedron's own vertices** — cyclic permutations of
(0, ±1, ±φ), with the faces derived by asking which vertex triples are mutually one edge apart, so
a typo in a face table cannot happen. Rotated by an orthonormal frame built from the face normal
rather than by two Euler angles, because two rotations in the wrong order gives a die that is
*nearly* face-on, and nearly is exactly what looks wrong without anyone being able to say why.

A d20 is the one shape that says *tabletop role-playing* without saying anyone's name, which
matters for a project that ships no licensed content: no wordmark, no trademark, no borrowed art.
A Platonic solid described by Plato is safe ground. The palette is the application's own, so the
icon looks like the window it opens.

Two things it got wrong first and the geometry said so. The winding of the face tuples is whatever
`combinations` happened to produce, so orienting the normal by the face's own depth **culled the
head-on face** and left the die hollow; it is oriented against the centroid now. And a physically
plausible falloff put the near face and its three neighbours within a few percent of each other,
which at 32 pixels is a grey blob — the three tones are spread hard instead, because an icon is
read at a glance or not at all.

Verified by extracting the built AppImage: the shipped `corerules.png` is byte-identical to the
source, the desktop entry carries `StartupWMClass` so a launcher can find the running window, and
its Comment is written for a player rather than left to default into the package description.

### Spells and money the corpus has; armour it does not

`spells.ts`. Three questions were expected to fail together for the same reason — *"equipment, and
what a priest may cast, want a corpus the slice does not have"* — and **two of them turned out to
be answerable and the third to be unanswerable for a different reason than the one assumed.**

**A priest's spells come from the spheres their god grants**, which is [correction
46](../../corpus-v1/map.md)'s deity work paying off in the place it was always for. The Complete
Priest's states the rule at DD05501: major access reaches spells *"of any level from that sphere"*,
minor access *"only 1st through 3rd level"*. The packs have carried `sphere` and `sphereMinor` as
separate grants since the priesthoods were transcribed; this is the first thing that reads them.
A first-level cleric of Agriculture is offered **80 spells** — `{1st: 18, 2nd: 16, 3rd: 18, 4th: 9,
5th: 7, 6th: 10, 7th: 2}` — and the minor spheres stop dead after 3rd, which is the rule holding
rather than the transcription running out. The distinction rides on every offer, because *"you may
have this"* and *"you may have this and never anything above 3rd level in it"* are a career
decision apart, and a list that flattened them would mislead precisely the player who was planning.

**Starting funds are Table 43** and print as the book prints them — `5d4 x 10 gp`, `(1d4+1) x 10
gp`, `2d6 x 10 gp` — a die and not a number, for the same reason hit points are: it is recorded
randomness, and an Engine that rolled it could not record a roll made at the table.

**Spells per day are Table 24 for priests, Table 32 for bards, and for wizards Table 21, which is
in the slice with `rows: []`.** Present and empty, exactly as Table 60 is — so a mage is told *"Table
21 is present in the pack with no rows in it"* and a fighter is told nothing at all, because the
Engine knows which groups cast. A3 keeps those three states apart and this is the first place all
three appear at once.

**Armour class is the refusal, and it is [correction 61](../../corpus-v1/map.md).** Not a missing
equipment list — **Table 46 rates combinations rather than pieces**, and the seven `armor` records
come from two different tables for two different purposes. So `armourClass` reports rather than
computes: 10 unarmoured, because Table 46 has a row that happens to name a single state, and for
anything worn a sentence naming what would have to exist first. The alternative was an AC that
looked right for leather and was silently wrong for leather and shield, which is §5.2's failure
mode with the shield doing the damage.

### Five corrections applied, and what applying them cost

Corrections 14, 11, 25 and 33 were the ones on the list that were both open and buildable; 63 is
what 33 exposed on the way past. They are unrelated rules and they came out with one shape in
common — **each had located its own problem one level away from where the problem was.**

**14 — a list is not a refusal.** The repair is a `listing` declaration, and the direction is the
whole thing: absent means UNDECLARED, so `satisfies()` answers `yes` / `no` / `unknown` and only a
declared `closed` may say no. Three of four states forbid a refusal. Then the measurement:
**77 of 121 `require` effects carry a list, 0 declare what it is**, and they cannot — see
correction 62, the words that would classify them are field prose the transcription drops.
The histogram is the part worth keeping: `{1: 18, 2: 30, …}`, so **18 lists have ONE member**, and
a one-item list read as closed is not a choice at all.

**11 — the clamp.** `set` gains `bound: atMost | atLeast`, and it is not a seventh operation
because §4.3's closure defends *order-independence* while what five records broke was *additivity*.
Min and max commute. Building it produced the piece the correction had not seen: a bounded `set`
must be lifted out of ticket 03's contest before it is judged, since two ceilings on one field are
not two books disagreeing — they compose to the tighter one. And a floor above a ceiling is applied
neither way, with both records named.

**25 — the fall-through.** *"A halfling has a 15% chance of infravision to 60 feet and, failing
that, a 25% chance of it to 30 feet."* The correction called this the first place order-independence
cost something measurable. It cost nothing: **§4.3 guarantees independence BETWEEN LAYERS**, and
this sequence is inside one effect of one record stating one sentence of one book. A guarantee is
about a scope, and nobody had asked this one what its scope was. The Engine shows the chain and
never resolves it — flattening to 15% and 21.25% would put a number on the sheet no book prints.

**33 — a weapon group is a thing you buy.** 15 tight groups and 4 broad in the slice, each carrying
its own `slotCost`, so the price is the pack's and the arithmetic is the Engine's. Two things had to
move: the wizard **offered no group at all**, and the weapon budget **counted picks instead of
summing costs**, which made a Broad Group of 26 weapons the cheapest thing on the list. The case
worth having is the one no book rules on — the cleric's permit-list reaches 1 of Polearms' 21
weapons — and that offer comes back `unknown`.

**63 — and underneath it, 119 weapons hedged for no reason.** One cost function served both kinds,
so every weapon was asked Table 37's crossover question and answered *"no book says which group this
belongs to"* — correction 60's sentence, true of a nonweapon proficiency and meaningless of a
longsword. Nothing failed and every test passed. **§5.4's third answer costs its own credibility
each time it fires where there is no question**, which makes it unlike a wrong number: the damage is
not local.

### The second subject, and a measurement that was off by forty

**Correction 17 was the largest thing on the corrections list and it took an optional field**, which
is worth recording as a mistake in classification rather than as a small win. *"How does this NPC
react to you"* was filed as a missing SUBJECT, so every proposal was a thing the character has — an
operand, a scalar, a layer. The other party is none of those: **it is an argument to the question**,
not a property of the answer. `Sheet` takes an `against`, the predicate resolves `opponent.creature`
from it, and with nobody named every such rule stays undecidable, which is the honest answer to
*"what is your attack roll"* asked with no one on the other side of it.

The dwarf is the demonstration. **Nine creatures change his numbers and not one had ever reached a
sheet** — `+1` to hit orcs, goblins, hobgoblins and half-orcs; `−4` to the attack rolls of giants,
ogres, ogre magi, titans and trolls. Modelled correctly since the races were transcribed, and
unreachable because the question could not be asked. The sheet now lists them, computed from the
character's own layers rather than from the creature list: a pack with three hundred monsters would
otherwise print three hundred rows of which four differ.

**Correction 62 was not blocked either.** The anchors point into a webhelp rendition still on disk,
so all 61 records carrying a `from` were re-read against their own source pages — matching each
list's members to the sentence that states them and asking whether an exemplary marker attaches to
**that list** rather than to the page. **46 of 77 are now declared `closed`**; 31 stay undeclared
because no single sentence holds their members, which is the state that exists for exactly that.

And the re-read produced **correction 64**, which is the part worth carrying: *"91 of 134 kits carry
`such as`"* is a page-level count, and at clause level **at most 2 of 77 lists are exemplary**. Off
by a factor of forty, in the same shape as correction 7. Better still, the case correction 14
quoted — *"a concealable hand weapon such as a dagger, knife, or hand axe"* — turns out to be a
requirement the transcriber deliberately recorded **with no list at all**, four years before anyone
worried about it. The practice was already right. It was simply never written down, and a convention
nothing states is one nobody can rely on.

### Armour class, which was never blocked on what everyone said it was blocked on

`armourClass` computes. Correction 61 had it filed as the one entry on the corrections list that
neither the format nor the Engine could reach — *"there is no equipment list"* — and both halves of
that were wrong.

**The equipment list was in the source under a table number already spent.** `DD01623.HTM` is
*Armor Costs*, which the book **also numbers Table 44**; the corpus had transcribed a different
Table 44 — the clothing list — and never looked for a second. Twenty pieces with cost and weight,
four shields under a `Shield` heading and two helmets under `Helmet`. And that table carries no AC
at all: it says *"See table 46"*, so the book itself points from the item to the combination table.
**They were always one vocabulary.**

**The real obstacle is a grammar, and it fails silently.** In *"Splint mail, banded mail, or bronze
plate mail + shield, plate mail"* the `+ shield` attaches **BACKWARDS** over the run before it.
Parse the row left to right, comma by comma — the obvious reading of a comma-separated row — and
splint mail lands on AC 3 here and AC 4 one row up, **with no error raised anywhere**. That is the
danger correction 61 half-saw: not a table that cannot be indexed, but one that indexes wrong and
says nothing. Read properly it is complete and collision-free — fourteen armours alone, thirteen
with a shield, checked for contradictions before a line was written.

**Brigandine with a shield is the gap, and it is the book's.** Scale mail and hide sit beside it at
AC 6 and both drop to 5; the answer is obvious and unprinted, which is exactly the number the
Engine must not supply. It reports the gap by name.

Shipped as twenty item records carrying `armorKind` and `worn` — **declared, never recognised**,
because an Engine matching `phb:shield` by id is a closed enumeration in the consumer — the five
surviving categories marked as categories, `phb:DD01632#2` holding Table 46 keyed by item with the
interpretation note that admits it is a reading, a `worn` list on the Character, and an armour
picker on the sheet rather than in the wizard. Armour is bought and swapped; §6.3's history is what
the rules derive from, and a change of clothes belongs nowhere near becoming a 5th-level fighter.

**And correction 65 fell out of it.** Correction 58 declared the field paths the pack's EFFECTS
write; a path also enters through a table's `supplies`, and that half was never covered — **eight
of the slice's ten table paths sat outside the vocabulary** and nothing checked. `Pack` complains
now, and caught two more in the Engine's own fixture within a minute of being written.

### The priesthood step, and a feature that had been finished and unreachable

**Nothing had ever offered a deity.** 59 priesthoods in the slice, every one targeting
`phb:priest`, each with alignment and ability prerequisites — and no step, no field on the draft,
no argument to `create`. So correction 46's sphere work and everything `spells.ts` reads off it
were **finished and unreachable**: a cleric of Agriculture is offered 80 spells, a cleric of
nothing is offered none, and every cleric this application could make was a cleric of nothing.
The only way to attach one was to edit the character file by hand, which is what the last two
sessions did without noticing what that meant.

It is not a kit and does not go where kits go — §3.1 makes it an Attachable of its own,
`one-per-target`. The step is offered only where some priesthood in a loaded pack could take this
class at all, **decided by asking the records rather than by knowing that priests have gods**: a
fighter gets no step to decline.

Against the real slice the three answers all appear and each is right. A lawful-good cleric with
Wisdom 16: **10 available, 34 refused, 15 unknown** — and the fifteen are the priesthoods that ask
for a second ability nobody has rolled yet, which is precisely the case §5.4's third answer exists
for. Drop the alignment and it is 58 unknown, because the question cannot be asked at all. Wisdom 9
and chaotic evil leaves two.

### Constitution, and the third table that hides a dimension in punctuation

`hitPoints()` is complete. Constitution enters **twice** and the second way is the interesting one.

Table 3's Hit Point Adjustment column reads `+2 (+4)*` — two numbers in one cell, the parenthetical
for warriors only — and the **count of asterisks is a separate rule**: at Constitution 20 every 1
rolled for a Hit Die counts as a 2, at 21 every 1 and 2 counts as a 3, at 23 every 1, 2 and 3 counts
as a 4. One column, a per-group bonus, a cap, and a floor on the die, told apart by parentheses and
by counting punctuation.

That is **the third table in a row with this shape** — Table 46's backwards `+ shield`, Table 44
meaning two different tables, and now this. Worth stating as a rule of thumb for whoever transcribes
the remaining books: *these tables compress a second dimension into typography*, they read as flat
lookups until someone needs the second dimension, and two of the three produce a plausible wrong
number rather than an error.

**The die floor is where §6.3 pays off in a way nobody planned.** The rule turns a rolled 1 into a
2, and the Engine applies it when the total is computed rather than at the roll — so the file keeps
the 1 the player actually rolled. Storing the 2 would have destroyed the roll and made the number
unexplainable the first time the character's Constitution changed. *"Hit points are recorded
randomness"* was argued for multi-class arithmetic and turns out to be what lets a rule modify a
roll without eating it.

Checked against the book: a Constitution 18 warrior gains +4 a level and a Constitution 18 mage
gains +2, which is the cap doing its whole job. The PHB's worked example — 6, 5 and 2 across three
classes for 4 hit points — is unmoved, because that character has no Constitution recorded and the
Engine adds nothing rather than assuming an average. A fighter/mage with Constitution 18 gets **no
bonus at all** and is told why: Table 3 has two columns and no rule for a character in both.

### §6.2, and the shape of a rule that is in no book

*"XP split evenly · hit points averaged across hit dice · best saving throw across classes · best
THAC0 · best slot progression."* The spec calls these **shape, not content**, and building them
shows why the distinction had to be drawn: the PHB prints Table 34 and Table 53 and the experience
tables, and *"with two classes, take the best"* is **in no table anywhere**. So the numbers come
from the pack and the combination comes from the Engine — §5.2's line, drawn through the middle of
one sheet.

Every derived value read `classes()[0]`. For a Fighter/Mage that meant one of two failures
depending on how the character was built: the fighter's answers with **no sign that half of them
were missing**, or — once the compound class record named no group — nothing at all. The second is
honest and useless. The first is the wrong number §5.2 exists to name.

Now: THAC0 is the lowest across the arms, the slot budgets are the best of each (a Fighter/Mage
gets the warrior's **four** weapon slots and the wizard's **four** nonweapon ones, which is not the
same class winning twice), spell progression is the best caster's, and experience is **per arm**,
because §6.2 splits it rather than pooling it — a Fighter/Mage is two careers on two tables at half
speed each. The sheet prints one line per class beside the combined figures, since a single number
hides which half earned it.

**And the die stopped being a property of the character.** `advance()` had read `hitDice.perLevel`
off the sheet; a Fighter/Mage applies both class-group layers, both `set` that field, and ticket 03
quite correctly calls that **contested** — so the die came back `undefined` and the interface rolled
a d8 by default. It was never a contradiction. It was two right answers to a question asked of the
wrong subject: a die belongs to a class, not to a character. `advance()` now reports one per class
and the wizard rolls a d10 and a d4.

§6.1's sum type resolves at the one point it has to: `phb:fighter-wizard` is a single record that
`combines` two classes, and a Level Event holds **one roll per arm** — the only shape from which
`floor(sum / count)` reproduces the PHB's own worked example. `service.arms()` does that expansion
once, where a player is about to roll, so the renderer never learns what `combines` means.

**Dual-class is still open and is not the same job.** §6.1 makes `Dual(original, new)` a distinct
arm with a suppression threshold, and no record in this corpus describes one — so there is nothing
to test a shape against, which is the wrong condition under which to invent one.

### The dice grammar had been validated for months and executed never

`dice.ts`. §9.1 says *"the tool rolls dice"*, and correction 15 put a `dice` pattern in the schema
and widened it twice — once for `NdM x k`, 4.7% of 2,574 corpus notations that the stated grammar
rejected, and once for `(NdM ± J) x k`, because **grouping changes the arithmetic**. Every roll in
this project until now was `1 + Math.floor(Math.random() * sides)` written inline in a React
component, which cannot express `5d4 x 10` and would not have tried.

The grouping case is not pedantry: `(1d4+1) x 10` and `1d4+1 x 10` top out at **50 and 14**, and
the first is the money every mage in the game starts with.

**A roll returns its dice, not a total.** §6.3 records randomness rather than reproducing it, and a
player who threw five 4s wants to see five 4s — 200 gp with nothing behind it is indistinguishable
from a number the tool made up. `random` is an argument, because a test that cannot fix the dice is
a test of luck.

Starting money is the last unrolled die in §9.1, and it turned out not to be Table 43's alone:
**38 kits `set startingWealth` outright**, so the Animal Master starts on `4d4x10` where his class
group says `5d4 x 10 gp`. The kit layer speaks where it speaks and Table 43 answers where it does
not — §4 doing its job, needing a function only because the base is a table read rather than an
effect, so the two never meet on the sheet by themselves.

Two names had to come apart on the way: the **field** `startingWealth`, which 38 kits write and
which holds a DIE, and the Character's recorded **amount**, which is a number. One is the question
and the other is the answer, and they had the same name until something finally rolled one. The
amount is `funds` now.

And pointing the parser at the whole pack produced **correction 67**: 19 of 61 distinct dice-shaped
strings are prose — *"1d4 persons in 20-ft. cube"* — all of them in `duration` or `areaOfEffect`.
The grammar is right; a string that begins with a die is not a die.

## What is left
- **Dual-class**, per §6.1 and §6.2's suppression threshold, once something in a pack describes one.
- **Encumbrance**, which wants the rest of §9.1's equipment: the armour list carries weights and
  Table 47 rates them, but a character carries more than armour.
- **Spell selection**, as opposed to spell access: which of the 80 a priest prepares today. That is
  a per-day record on the Character, not a rules question, and it wants a screen more than an engine.
- **Constitution is not in the hit points.** The bonus is a table read with a per-class cap, and
  a plausible-looking total that quietly omits it is the kind of wrong number the rest of this
  refuses.

