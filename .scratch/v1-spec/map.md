# Map: corerules v1 spec

## Destination

A written v1 specification for corerules — product surface *and* technical shape — concrete
enough to hand to an implementation session that starts building without further design
questions. v1's scope is **creating and persisting AD&D 2nd Edition characters**.

## Notes

**Domain.** AD&D 2nd Edition: the core books (PHB, DMG) plus the PHBR "Complete Book of…"
series. Old-school RPG character management. Wagner owns 100% of the published AD&D 1e and 2e
material and his table exercises all of it, so the book list is never a scoping filter — scope by
**mechanical shape** instead.

**Release roadmap** (settled in [ticket 03](issues/03-which-complete-handbooks.md)):
**v1** core + PHBR kits, class and racial · **v2** campaign settings, psionics, further books TBD
· **v3** Player's Option.

**Skills every session should consult.** `/grilling` and `/domain-modeling`. Keep the glossary
in `CONTEXT.md` current as terms resolve — do not batch glossary updates.

**Settled during charting — do not re-litigate:**

- **FOSS engine, content supplied by the user.** corerules never bundles licensed 2e content.
  AD&D 2e has no SRD or open licence; the tables, kit descriptions and spell text are Wizards
  of the Coast IP. The engine computes the rules; the user brings the data, having bought the
  books. **Sharpened by [ticket 04](issues/04-validate-or-record.md):** the posture never depended
  on how a pack was produced — Wagner authors his from his own RTF and PDF copies — it depends on
  **packs not circulating**. A pack is derived WotC content; a character is the user's own work.
  The spec must state that boundary rather than imply it.
- **Single-user.** No accounts, no authentication, no permissions.
- **Desktop application.** Electron, TypeScript + React, cross-platform (Linux/Windows/macOS).
  Electron over Tauri because Tauri's system webviews (WebKitGTK / WebView2 / WKWebView) mean
  three rendering engines, which is precisely the cross-platform variance this project wants
  gone; Electron bundles Chromium and keeps the codebase in one language.
- **Unsigned builds.** Not a commercial product. **Amended by
  [ticket 02](issues/02-electron-packaging-and-release.md):** the charting assumption that the
  README can document a way past the warning on each OS is only true on Linux and mostly true on
  Windows. It is false for Windows Smart App Control, which has no per-app override, and unknown
  for macOS, where a genuinely unsigned arm64 build does not execute at all. "Unsigned" is a
  per-OS posture, not one decision.
- **Release via GitHub Actions.** Wagner has never used Actions. Any ticket touching CI must
  produce a literal step-by-step checklist of the manual steps, not a pointer to documentation.
- **The engine is native to AD&D 2e, not a generic RPG engine.** Serving AD&D was always the whole
  intent. Closed set of object kinds, packs supply instances — but **closed kinds, open
  enumerations**: the engine owns the shape, the pack owns the contents.

**Permanent constraints.** Two, both of the same shape — excluded from v1, but no v1 decision may
foreclose them:

- **Sync between clients**, LAN or internet, is a certain future direction. In practice this means
  stable global identifiers rather than local autoincrements, and a persistence format that can be
  diffed and reconciled.
- **v2 and v3** — campaign settings, psionics, Player's Option. In practice this means enumerations
  stay open, the patch mechanism takes scope as a parameter, and the character records which packs
  it was built against.

**Language.** Artifacts in English. Conversation with Wagner in Portuguese.

## Decisions so far

<!-- one line per closed ticket: enough to judge relevance, then follow the link for detail -->

- [Prior art: how existing tools model AD&D 2e content](issues/01-prior-art-2e-content-modelling.md)
  — the engine-without-content architecture is proven three times over, but **nobody anywhere
  models 2e's rules as loadable data**: PCGen has no 2e dataset and never did, the live Foundry
  OSR systems compute nothing, and the Roll20 2e sheet hardcodes everything and has no kits. Borrow
  Foundry's versioned manifest, typed lookup tables, class *group* as an entity, declarative
  prerequisites and choices-not-derived-values character files; avoid name-as-identity, `.MOD`-style
  patching, a game-global XP table, an undeclared type namespace, and above all any second
  expression evaluator. "Engine computes, user supplies the tables" has no shipping precedent.
- [Electron packaging and cross-platform release](issues/02-electron-packaging-and-release.md) —
  **electron-builder** (pinned to 26.x; Forge has no AppImage or NSIS maker and no CI guide at
  all), but the build tool was the easy question. The real finding is that **"unsigned" is not one
  posture**: a config line on Linux, a five-click detour on Windows, and *the app does not start*
  on macOS, where Apple Silicon refuses unsigned arm64 code and the resulting dialog has no Open
  Anyway path. Ad-hoc signing is the mitigation, and whether it survives leaving the build machine
  is unverified — ticket 12. Two findings move other tickets: **Fedora 45 makes `rpm` refuse
  unsigned packages this autumn** (so AppImage is the primary Linux artifact), and **`node:sqlite`
  is built into Electron**, which deletes ticket 08's packaging-cost axis entirely.
- [Which Complete handbooks are in v1](issues/03-which-complete-handbooks.md) — **all of them, and
  the book list was the wrong axis**: Wagner owns everything and his table plays everything, so
  scope by mechanical shape. The series reduces to three shapes — kit-on-class, kit-on-race,
  parallel subsystem — and v1 takes the first two. The engine is **native to 2e** (closed kinds,
  open enumerations), which is affordable precisely because the edition is dead and its concept set
  closed by history. Psionics and campaign settings go to v2 despite five settings being in active
  play, because a setting is not content but a set of overrides to the central model; Player's
  Option to v3. Both get the map's non-foreclosure treatment.
- [Does the tool validate the rules, or merely record them?](issues/04-validate-or-record.md) —
  **it validates, hard.** Illegal states are unrepresentable at the point of choice; the table
  plays by the book and "every combination" always meant every *legal* combination. Since corerules
  ships no content, the rules being enforced are Wagner's own transcription, which makes **the pack
  format a small rules language — a constraint on tickets 06 and 11, not an option**. Packs
  *declare* which rule-sets they provide (A3), so the engine can tell "no restriction" from "not
  transcribed yet" and say which; that declaration also subsumes the house-rule escape hatch.
  Loading a character never fails — an invalid one is **quarantined**: readable and printable, but
  locked against anything that extends it. And the books are already digital: Wagner authors packs
  from his own RTF and PDF copies, so authoring is extraction, not typing.
- [Depth of the character generation pipeline](issues/05-generation-pipeline-depth.md) — **v1 owns
  all of it.** Most was already forced by kits (proficiencies, equipment, money, spells are
  preconditions of having kits at all); what this ticket decided is that **advancement is in**
  — which turns a character from a snapshot into a sequence of level events — that **multi-class
  and dual-class are both in**, and that **the tool rolls dice**, which obliges the pack's
  expression language to carry dice semantics and not just arithmetic. Creation and levelling run
  through a guided wizard, correction through direct sheet editing, with identical validation on
  both paths. And automatic character construction is ruled out **permanently**, not deferred.
- [How multi-class and dual-class are modelled](issues/14-multi-class-and-dual-class-model.md) —
  the class arrangement is a **sum type** (`Single` | `Multi` | `Dual`), on the principle this
  ticket named and which now governs the map: **structural for model incoherence, pack-declared for
  game rules**. The engine owns the combination arithmetic, because **A3 governs validation, not
  computation** — a missing validation rule is an unenforced restriction, but a missing computation
  rule is a wrong number. Every advance is recorded event by event, since **hit points are recorded
  randomness** — neither choice nor derivation — and multi-class totals cannot be rebuilt from
  levels alone. One kit, bound at creation to a named target and never rebound; it stays with the
  original class through dual-classing. Abandoning it strips bonuses and penalties but leaves the
  granted proficiencies as a **nominal debt** against future slots — a third kind of effect neither
  this ticket nor 10 had anticipated. A character carrying debt stays valid, because quarantine
  would deadlock: the debt is only payable by levelling, which quarantine locks.
- [The engine's closed set of object kinds](issues/11-engine-object-kinds.md) — **a kind is
  anything referenced by stable identity across a pack boundary**, and that criterion splits the
  inventory three ways where the ticket expected one: pack kinds, character structures, and value
  types that carry no identity at all (which corrects ticket 04's loose claim that predicates and
  budgets are kinds). Every enumeration stays open except **rule-set names**, which must be the
  engine's, since A3 cannot act on a declaration it does not understand. Weapon and non-weapon
  proficiency are two kinds whose **slots are convertible currencies** — which makes slot type a
  kind, and lets Player's Option add character points in v3 without breaking anything. Weapon,
  armour and gear are three kinds; **magic items are in v1** as a property of them, and are the one
  place corerules records rather than validates, since acquisition is DM fiat. Deity enters thin, on
  this ticket's own test that a kind added in v2 is a v1 mistake. And **Kit, Deity and Subrace turn
  out to be one closed shape used three times** — Wagner's proposal — which shrinks ticket 10, the
  map's heaviest invention, from designing a kit mechanism to designing one applicable-modifier
  mechanism.
- [How a kit modifies its parent class](issues/10-kit-modifies-parent-class.md) — the mechanism with
  no prior art turned out to need **no new machinery**: three earlier tickets had already forced it.
  An Attachable never touches the shared record, and it does not overwrite at all — the character's
  view is computed as a **stack of layers**, because abandonment must drop a layer, magic items are
  already a stack, and ticket 04's refusals must name their cause, which overwriting erases. The
  operations are **order-independent by design**, which is the direct remedy for the load-order
  dependence ticket 01 found in PCGen's `.MOD`; two `set`s on one field become a **reported
  conflict** rather than last-loaded-wins. The vocabulary **closes at six** — `adjust`, `grant`,
  `forbid`, `except`, `require`, `set` — with anything outside it carried as text and not computed,
  the same line ticket 11 drew for artifacts and charges. And **the character's own choices are the
  topmost layer**, so the sheet is one uniform resolution in which every value can say where it
  came from.
- [Content pack format](issues/06-content-pack-format.md) — billed as the load-bearing decision of
  the project, and it **arrived disarmed**: five resolved tickets had already fixed the modelling,
  leaving only the format. A pack is a **directory** of **JSON**, declared by a **manifest** rather
  than discovered by scanning — which is A3 at the file level, and makes an orphan file detectable
  by construction. JSON beat YAML on one argument: **YAML is silently wrong by specification**
  (`NO` → false, `1.20` → float), and never being silently wrong is this project's posture. Its lack
  of comments was already answered, since ticket 01 requires book and page on every record.
  Directory beat single-file partly because ticket 04 had removed the reason to want one — **packs
  do not circulate**. And the same-name-in-two-books problem dissolved: under pack-scoped IDs
  `phb:set-snares` and `cbarb:set-snares` are simply two proficiencies, disambiguated by the
  character's active pack set, which ticket 03 had already put on the character.

## Not yet specified

- **The derived-statistic computation model.** THAC0, saving throws, armour class,
  encumbrance — computed on read or stored on the character? Ticket 04 settled that the engine
  computes and validates, but not *when* — on read, or snapshotted onto the character.
- **How the level-event history is shaped.** Ticket 05 made a character a sequence of choices per
  level rather than a snapshot, and ticket 14 will add dual-class suppression state on top. How
  much of that sequence the file keeps, and what is stored versus re-derived, is ticket 07's — but
  the shape of the thing itself is not yet sharp.
- **Whether macOS is a v1 platform at all.** Ticket 02 turned "cross-platform" from a settled
  premise into an open one: unsigned macOS may have no viable path, and Wagner develops on Fedora
  with no known Mac access. Ticket 12 produces the fact; what the spec does if the answer is bad —
  ZIP-only, untested-and-documented, or dropped — is not yet sharp enough to ticket.

## Out of scope

- **Campaign settings** — Dark Sun, Dragonlance, Forgotten Realms, Ravenloft, Planescape, and the
  rest. **v2**, ruled out by [ticket 03](issues/03-which-complete-handbooks.md) despite all five
  being in active play at Wagner's table, because a setting is not more content: it is a set of
  overrides to the central model. Dark Sun is the proof — everything it adds is data under the
  2e-native decision *except* ability scores ranging 5–24 and a wild psionic talent on every
  character. Whether a setting is just another pack or a first-class engine concept was put to
  Wagner and deferred with it.
- **Psionics** — the Complete Psionics Handbook's power scores, PSPs, sciences and devotions.
  **v2**, ruled out by ticket 03. It is the one PHBR volume that is not additive: a second
  character subsystem rather than kits layered on the core model. Noted risk, raised and
  overruled: psionics is the format's hardest load test, and deferring it entirely — rather than
  carrying a worked example in the v1 spec as proof of load — risks discovering in v2 that the
  format cannot express it, which is exactly how PCGen failed.
- **Player's Option** (Combat & Tactics, Skills & Powers, Spells & Magic) — these *replace*
  the core character model rather than extend it: Strength becomes Stamina + Muscle, character
  generation becomes point-buy, combat resolution is rewritten. That is a variant-configuration
  problem. **v3**, per ticket 03's roadmap.
- **Unearthed Arcana** — 1st Edition; explicitly dropped during charting.
- **Campaigns** — a genuinely second aggregate: which rule options are switched on, which packs
  are loaded, party membership, session state. Post-v1.
- **Monster stat blocks** — HD, no class, no proficiencies. Not Characters at all.
- **Automatic character construction** — a button that produces a finished NPC. Ruled out
  **permanently** by [ticket 05](issues/05-generation-pipeline-depth.md), not deferred: corerules
  builds step by step, on the model of TSR's AD&D Core Rules 2.0 and D&D Beyond. Rolling dice is a
  rule and comes from the pack; choosing a kit and proficiencies for the user is *taste*, and no
  source book contains it. Under ticket 04's hard validation it would also need a constraint solver
  rather than a sampler, since naive sampling dead-ends.
- **Multi-user / player logins** — accounts, permissions, GM-hidden data, live sync.
- **Self-hosted or browser delivery** — a different product with a different persistence and
  security model. Returns, if ever, alongside multi-user.
