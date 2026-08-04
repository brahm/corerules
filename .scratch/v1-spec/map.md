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
  books.
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

## Not yet specified

- **How book content actually gets into the tool.** The packs don't ship, so somebody types
  them. Authoring? Import? A pack editor? Shape depends entirely on the pack format. Data point
  from ticket 01: PCGen's answer is text files plus an autoformatter plus a converter plus
  editor syntax definitions, and its own docs concede list editing takes "a text-editing program
  and patience". With Wagner owning 100% of the published material, the volume here is large.
- **The derived-statistic computation model.** THAC0, saving throws, armour class,
  encumbrance — computed on read or stored on the character? Depends on whether the tool
  validates or merely records.
- **The shape of the creation UI.** Guided wizard versus sheet-first editing. Depends on how
  much of the generation pipeline v1 covers.
- **Whether v1 includes advancement at all**, or stops at 1st-level creation. Adjacent to the
  generation-depth ticket but not the same question.
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
- **Multi-user / player logins** — accounts, permissions, GM-hidden data, live sync.
- **Self-hosted or browser delivery** — a different product with a different persistence and
  security model. Returns, if ever, alongside multi-user.
