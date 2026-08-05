# Which Complete handbooks are in v1

Type: grilling
Status: resolved
Blocked by: —

## Question

The PHBR "Complete" series runs to roughly two dozen volumes. Which of them does v1's spec
have to accommodate?

The series is not homogeneous:

- **Class handbooks** (Fighter's, Thief's, Priest's, Wizard's, Bard's, Ranger's, Paladin's,
  Druid's…) — mostly kits, proficiencies, equipment. Additive.
- **Racial handbooks** (Dwarves, Elves, Humanoids, Gnomes & Halflings, Book of Necromancers…)
  — add races, racial kits, and in the Humanoids' case playable monster races that break
  assumptions about level limits and ability ranges.
- **The Complete Psionics Handbook** — an entire parallel subsystem, not kits.

Does "the red books" mean all of them, the class ones only, or a specific list Wagner owns and
actually uses at his table? And does the answer change what "v1" must ship versus what the
format must merely *permit*?

## Answer

**The book list is not a usable filter.** Wagner owns 100% of the AD&D 1st and 2nd Edition
material ever published — print, boxed sets, and the official TSR PDFs distributed free between
1997 and 2001 — and his group has played together for over thirty years, actively exploring every
combination the books allow. Any of the ~24 PHBR volumes can reach the table today. Scoping v1 by
"which handbooks" was therefore the wrong axis.

**Clarified while resolving [ticket 04](./04-validate-or-record.md):** "every combination" means
every *legal* combination. The table plays by the book and expects the tool to hold them to it —
which is why 04 landed on hard validation.

The right axis is **mechanical shape**. The whole series reduces to three:

1. **Kit attaching to a class** — every class handbook (Fighter's, Thief's, Wizard's, Bard's…).
   One mechanism, replicated a dozen times. See [ticket 10](./10-kit-modifies-parent-class.md).
2. **Kit attaching to a race** — the racial handbooks (Dwarves, Elves, Gnomes & Halflings). Same
   design, different target. Cheap if the kit mechanism names its attachment point; expensive to
   retrofit if the engine is born assuming a kit is a class thing.
3. **Parallel subsystem** — only the Complete Psionics Handbook. Not a kit on top of anything.

### Decision 1 — the engine is native to AD&D 2e, not generic

Rejected: an engine that knows only declared object kinds, tables, predicates and effects, with
2e itself as data (which would make psionics content rather than a feature). Chosen: the engine
knows what a class, a class group, a kit, a proficiency slot, a sphere, a school, a power score
and a PSP *are*; packs supply instances and numbers, never new kinds.

Wagner's reason: serving AD&D was always the whole intent — corerules is not a generic RPG engine
that happens to run 2e.

The supporting reasons, both from [ticket 01's research](./01-prior-art-2e-content-modelling.md):

- **PCGen is a field report on the generic path, and it failed.** It went generic and *still*
  cannot express 2e — no per-class XP table, no race×class matrix — and had to retrofit
  data-declared fields (`FACTDEF`) when a closed token set did not survive contact with new books.
  Generic bought it three live expression evaluators, an undeclared type namespace, and a method
  still in production called `processBrokenParser`.
- **AD&D 2e is a dead edition, and that is a rare advantage.** The concept set is closed *by
  history*. The fear that justifies a generic engine — "what about rules I did not foresee?" —
  does not apply. The engine's concept inventory is exactly the union of what those books
  introduce, and that union is enumerable today.

**Critical qualifier: closed kinds, open enumerations.** What lives in the engine is the *shape*,
not the *contents*. The engine knows there is such a thing as "a saving-throw matrix keyed by
class group and category, indexed by level" — but *which categories exist* remains data. Ravenloft
bolts `fear`, `horror` and `madness` onto the standard five-group structure (the Roll20 sheet
carries them), and a hardcoded enum would break on contact.

### Decision 2 — v1 covers the additive shapes; the non-additive ones are v2

Three concentric rings were put to Wagner:

| Ring | Books | Verdict |
|---|---|---|
| 1 | PHB, DMG + the PHBR "Complete" series | **v1**, except the Psionics Handbook |
| 2 | Non-PHBR rule supplements (*Tome of Magic*, *Arms & Equipment Guide*) | mechanically covered by v1; *which* books get packs is a content question, not a spec question |
| 3 | Campaign settings (Dark Sun, Dragonlance, Forgotten Realms, Ravenloft, Planescape) | **v2** |

Ring 3 was scoped out *despite* being in active play — the group runs five settings. The reason it
is expensive is that a setting is not more content: it is a set of **overrides to the central
model**. Dark Sun makes this concrete. Almost everything it brings (thri-kreen, half-giant, mul,
gladiator, templar, defiler/preserver, elemental clerics, Athasian level limits) is pure data
under Decision 1. Only two things are structural: **ability scores ranging 5–24 instead of 3–18**,
and **a wild psionic talent for every character**.

The Psionics Handbook goes to v2 with the settings, on Wagner's call. The concern raised and
overruled: psionics is the format's hardest load test, and deferring it risks discovering in v2
that the format cannot express it — precisely PCGen's failure. A worked psionics example in the
v1 spec was proposed as a load test and declined; everything psionics-related is out of v1.

**Roadmap, in Wagner's words:** v1 = core + PHBR kits (class and racial). v2 = settings, psionics,
and further books to be decided. v3 = Player's Option. This supersedes the map's earlier framing
of Player's Option as merely "a separate effort".

### Decision 3 — v2 and v3 get the map's non-foreclosure treatment

The same standing constraint the map already applies to sync: the v1 spec does not describe
psionics or settings, but **no v1 format decision may make them impossible**. The lighter of two
options — the alternative was to accept a format break with a converter and a version bump, on
PCGen's precedent that this is survivable.

It binds very little, and what it binds already fell out of the decisions above:

- **enumerations stay open** (otherwise Ravenloft breaks the engine);
- **the patch mechanism takes scope as a parameter** (otherwise a setting cannot be expressed —
  a kit patches the character's view of *its class*, a setting patches the character's view of
  *the base tables*; neither ever touches the shared record, which is `.MOD`'s error);
- **the character records which packs it was built against** — the Roll20 sheet's 21 per-character
  book toggles are evidence that "which books are in play" is a property of the character in this
  genre, not a global setting. Note this does *not* pull the Campaign aggregate into v1; the
  aggregate stays out, only the character's record of its own pack set comes in.

### Open, deferred to v2

Whether a setting is **just another pack** (containing races, classes, kits and override records)
or a **first-class concept** in the engine. Put to Wagner and deferred along with settings
themselves. Ticket 10 must leave both readings reachable.
