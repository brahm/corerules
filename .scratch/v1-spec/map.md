# Map: corerules v1 spec

## Destination

A written v1 specification for corerules — product surface *and* technical shape — concrete
enough to hand to an implementation session that starts building without further design
questions. v1's scope is **creating and persisting AD&D 2nd Edition characters**.

## Notes

**Domain.** AD&D 2nd Edition: the core books (PHB, DMG) plus the PHBR "Complete Book of…"
series. Old-school RPG character management.

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
- **Unsigned builds.** Not a commercial product. The README must document the Gatekeeper and
  SmartScreen warnings and how to get past them on each OS.
- **Release via GitHub Actions.** Wagner has never used Actions. Any ticket touching CI must
  produce a literal step-by-step checklist of the manual steps, not a pointer to documentation.

**Permanent constraint.** Sync between clients — LAN or internet — is a certain future
direction, deliberately excluded from v1. No v1 decision may foreclose it. In practice this
means stable global identifiers rather than local autoincrements, and a persistence format
that can be diffed and reconciled.

**Language.** Artifacts in English. Conversation with Wagner in Portuguese.

## Decisions so far

<!-- one line per closed ticket: enough to judge relevance, then follow the link for detail -->

_Nothing resolved yet — the map was charted in one session and resolves no tickets._

## Not yet specified

- **Psionics as a parallel subsystem.** The Complete Psionics Handbook brings power scores,
  PSPs and disciplines — not kits layered on the core model, but a second character subsystem.
  Cannot be phrased sharply until the handbook scope and the content pack format settle.
- **How book content actually gets into the tool.** The packs don't ship, so somebody types
  them. Authoring? Import? A pack editor? Shape depends entirely on the pack format.
- **The derived-statistic computation model.** THAC0, saving throws, armour class,
  encumbrance — computed on read or stored on the character? Depends on whether the tool
  validates or merely records.
- **The shape of the creation UI.** Guided wizard versus sheet-first editing. Depends on how
  much of the generation pipeline v1 covers.
- **Whether v1 includes advancement at all**, or stops at 1st-level creation. Adjacent to the
  generation-depth ticket but not the same question.

## Out of scope

- **Player's Option** (Combat & Tactics, Skills & Powers, Spells & Magic) — these *replace*
  the core character model rather than extend it: Strength becomes Stamina + Muscle, character
  generation becomes point-buy, combat resolution is rewritten. That is a variant-configuration
  problem, and a separate effort.
- **Unearthed Arcana** — 1st Edition; explicitly dropped during charting.
- **Campaigns** — a genuinely second aggregate: which rule options are switched on, which packs
  are loaded, party membership, session state. Post-v1.
- **Monster stat blocks** — HD, no class, no proficiencies. Not Characters at all.
- **Multi-user / player logins** — accounts, permissions, GM-hidden data, live sync.
- **Self-hosted or browser delivery** — a different product with a different persistence and
  security model. Returns, if ever, alongside multi-user.
