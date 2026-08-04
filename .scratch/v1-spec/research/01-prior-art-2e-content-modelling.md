# Prior art: how existing tools model AD&D 2e-era rules content

Research ticket: `.scratch/v1-spec/issues/01-prior-art-2e-content-modelling.md`
Researched 2026-08-04 against live repositories, not from memory.

Primary sources used: the PCGen source tree (cloned at commit `ef03350b`, 2026-07-27, version
`6.09.08.RC1`), its shipped `.lst`/`.pcc` data and its shipped format documentation; the Roll20
`ADnD_2E_Revised` community sheet source; the Foundry VTT OSE, Basic Fantasy and related system
repositories; Foundry's official manifest documentation; and the PCGen mailing-list archive.
Where a claim rests on a secondary source or on inference, it says so.

**Headline:** the architecture corerules has committed to is proven — PCGen, OSE and Basic Fantasy
all ship an engine with zero game content. But **no project anywhere models AD&D 2e's rules as
loadable data**. PCGen's data model cannot express per-class XP tables or race×class level limits
and has no 2e dataset (and its maintainers say it never did). The Foundry OSR systems deliberately
compute nothing. The only substantial open 2e implementation, the Roll20 sheet, hardcodes
everything in JavaScript and has no kits at all. corerules is designing into open ground; the
value of this research is mostly in the failure modes, which are extremely well documented.

---

## 1. Concrete recommendations for corerules

### 1.1 Steal these

**A pack manifest modelled on Foundry's, not on PCGen's.** Foundry requires `id`, `title`,
`description`, `version`, `compatibility`, `authors`, and recommends `url`, `bugs`, `changelog`,
`readme`, `license`, `manifest`, `download`. `compatibility` is a three-way object —
`minimum` / `verified` / `maximum` — separating "lowest engine version supported", "highest
version actually tested", and an optional hard ceiling, with documented advice to omit `maximum`
so packs don't lock themselves out needlessly. Dependencies live in a `relationships` object whose
entries carry `id`, `type`, an optional `manifest` URL, **and their own nested `compatibility`
object**, so a pack can require "pack X, ≥ a, verified at b"
(https://foundryvtt.com/article/module-development/,
https://foundryvtt.com/article/manifest-migration-guide/,
https://foundryvtt.com/api/v11/interfaces/foundry.packages.PackageManifestData.html).
PCGen by contrast has **no version field on a pack at all** (§2.6) — do not copy that.

**Licence and provenance metadata inside the pack, surfaced at load.** PCGen's `.pcc` carries
`ISOGL:`, `ISLICENSED:` + `LICENSE:<text|FILE=x.html>` (which pops a dialog at load time),
`ISMATURE:`, repeated `COPYRIGHT:` lines, and mandatory `SOURCELONG` / `SOURCESHORT` / `SOURCEWEB`
/ `SOURCEDATE`
(`pcgen/docs/listfilepages/datafilestagpages/datafilespcc.html`; live example
`pcgen/data/35e/wizards_of_the_coast/rsrd/_revised_system_reference_document.pcc`). Since a
corerules pack is by definition a transcription of a book the user owns, its manifest should be
required to name that book.

**Book and page citation on every record.** PCGen mandates `SOURCEPAGE:` on records
(throughout `rsrd_classes.lst`); the Roll20 2e sheet carries `book` and `reference` on every
spell and `book` on every non-weapon proficiency. PCGen's own docs give the rationale: "you can
load sources based on material you don't own… but you don't quite get enough information to use it
accurately or completely without actually buying the source material… it's important that it be
easy to look up for those who do own the books"
(`pcgen/docs/listfilepages/listfileimportanttoknow.html`). For corerules this is stronger still:
the citation is how a user checks their own transcription.

**Named, typed lookup tables as a first-class data construct.** Modern PCGen data declares tables
inline with a header row *and a column-type row*:

```
STARTTABLE:Encumbrance,
Str,Unencumbered,Encumbered,Overloaded
NUMBER,NUMBER,NUMBER,NUMBER
01,10,11,12
…
ENDTABLE:Encumbrance,
```
(`pcgen/data/pathfinder_2e/core_rulebook/c__datatables.lst:3-15`), read from a formula as
`lookup("Item Level",ItemLevel,"Price")`
(`pcgen/data/starfinder/paizo/core/scr_equipmods.lst`). This is exactly the shape for 2e's
saving-throw matrices, per-class XP tables and race×class level-limit grid. The declared column
types let a loader validate a pack before the user hits an error mid-character.

**Support *both* table form and formula form for level progressions — 2e needs both.** The Roll20
sheet demonstrates the split precisely. THAC0 is regular enough to be a closed-form function of
level, one per class group:
```js
const THAC0_FORMULAS = {
    'warrior':    l => 21-l,
    'wizard':     l => 21-Math.ceil(l/3),
    'priest':     l => 22-(Math.ceil(l/3)*2),
    'rogue':      l => 21-Math.ceil(l/2),
    'psionicist': l => 21-Math.ceil(l/2),
}
```
Saving throws are **not** regular and are stored as literal arrays indexed by level
(https://raw.githubusercontent.com/Roll20/roll20-character-sheets/master/ADnD_2E_Revised/javascript/savingThrows.js,
and `.../javascript/sheetWorkers.js`). PCGen makes the same split for d20 — `CAST:`/`KNOWN:`
literal rows per class level, `BONUS:SAVE|…|classlevel(...)/2+2` formulas — so the dual form is
proven in both prior arts.

**Pad table index 0.** The Roll20 saving-throw file's first line: "Index / level 0 uses the value
for a level 0 warrior for all classes to avoid edge cases." A one-cell cost to make `table[level]`
always valid. Also note it clamps: `Math.min(classProperties.level, 21)` before indexing
(`sheetWorkers.js`).

**Class *group* as a real modelling entity.** 2e's saving throws are keyed on the group
(warrior / wizard / priest / rogue / psionicist), not the class — both the Roll20 save tables and
its THAC0 formulas are keyed that way. Proficiency slot budgets work the same. Make group a
first-class object that a class points at, not a UI grouping.

**Data-defined typed fields rather than a closed schema.** PCGen lets a dataset declare its own
fields on engine object types: `FACTDEF:CLASS|SpellType DATAFORMAT:String SELECTABLE:YES
VISIBLE:YES`, `FACTDEF:RACE|BaseSize DATAFORMAT:SIZEADJUSTMENT`, plus multi-valued `FACTSETDEF:`
and `EXPLANATION:`/`DISPLAYNAME:` for the UI
(`pcgen/data/35e/wizards_of_the_coast/rsrd/basics/rsrd__datacontrols.lst:15-33`;
`pcgen/data/pathfinder_2e/core_rulebook/c__datacontrols.lst:43-58`). The file's own header says it
"represents a conversion mechanism for items that were previously stand-alone tokens, but are now
handled internally as FACT or FACTSET" — PCGen *retrofitted* this because a closed token set did
not survive contact with new books. The PHBR series will apply the same pressure to corerules.

**Typed default values.** `DEFAULTVARIABLEVALUE:NUMBER|0`, `…|BOOLEAN|False`, `…|STRING|`,
`…|DICE|0`, `…|ORDEREDPAIR|0,0` (same datacontrols files). Half-authored packs are corerules'
normal state, because a human is typing the books in by hand.

**Declarative predicates for prerequisites.** PCGen's `PRExxx:` family — `PRERACE`, `PRECLASS`,
`PRESTAT`, `PREALIGN`, `PREVARGTEQ`, `PREABILITY`, `PRETYPE`, negation with a leading `!`, and
`PREMULT:n,[…],[…]` for n-of-m
(`pcgen/docs/listfilepages/globalfilestagpages/globalfilesprexxx.html`). They are data, not code,
they compose, and they are attachable inline to almost any tag. 2e kits are essentially a
prerequisite predicate plus a bundle of effects; this is the right shape for the predicate half.

**A per-character set of active books.** The Roll20 2e sheet has a fixed `BOOK_FIELDS` array of 21
toggles — `book-phb`, `book-tcfhb`, `book-tcthb`, `book-tcprhb`, `book-tcwhb`, `book-psionics`,
`book-tom`, `book-aaeg`, `book-dwarves`, `book-bards`, `book-elves`, `book-humanoids`,
`book-rangers`, `book-paladins`, `book-druids`, `book-barbarians`, `book-necromancers`,
`book-ninjas`, plus three Player's Option books (`sheetWorkers.js`). **In practice, which books
are in play is a per-character setting in this genre, not a global one.** The map defers
"Campaigns" (which packs are loaded) to post-v1; this is evidence that at minimum the *character*
needs to record which packs it was built against. That also makes the character file
self-describing for the sync-later constraint.

**Character files record choices, not derived values.** A PCGen `.pcg` stores `STAT:STR|SCORE:12`,
`RACE:Dwarf`, `CLASS:Sorcerer|LEVEL:3|…`, per-level `CLASSABILITIESLEVEL:Sorcerer=1|HITPOINTS:4|…`,
`SKILL:Spellcraft|…|CLASSBOUGHT:[CLASS:Sorcerer|RANKS:6.0|COST:1|CLASSSKILL:Y]`, `EXPERIENCE:3000`
— and **no AC, no saves, no attack values** (`pcgen/characters/Sorcerer.pcg`). Everything derived
is recomputed from the loaded datasets. It also records the app `VERSION:6.03.02`, the `GAMEMODE:`
and the loaded `CAMPAIGN:` names, which is what makes version-scoped migration possible at all.

**CI that loads every pack and rebuilds golden characters.** PCGen's build has `./gradlew datatest`
("Data loading/validation tests") plus per-game-mode character integration tests
(`rsrdinttest`, `srdinttest`, `msrdinttest`, `pfinttest`, `sfinttest`) — `pcgen/AGENTS.md`.
corerules ships no 2e data but can ship a synthetic pack that exercises every awkward shape
(irregular THAC0 steps, a kit, a dual-class, a sphere list) and test the engine against it.

### 1.2 Avoid these

**Name-as-identity.** PCGen identifies objects by display name (optionally `KEY:`) in a single
global namespace, and resolves cross-pack collisions with a load-priority integer: `RANK:x`
"Sets the priority in loading .pcc files. 1 is the highest priority and 9 is the lowest… When
PCGen encounters certain objects with duplicate names from different sources (such as Classes) the
object in the source with the highest priority rank will be loaded while the duplicate will
generate an error message (and will not be loaded)" (PCC tag docs). Two consequences, both
verified: saved characters break whenever a data author renames anything (hence `migration.lst`,
§2.6), and legitimately different things sharing a name silently fight. The map's "stable global
identifiers" constraint already forbids this — make IDs pack-scoped, opaque and never reused, and
treat display names as pure presentation.

**This is not hypothetical in corerules' corpus.** The Roll20 sheet's non-weapon proficiency table
has 217 distinct names but 224 entries, because the same proficiency is defined differently by
different books:
```js
NONWEAPON_PROFICIENCIES_TABLE['Set Snares'].push({slots:1, abilityScore:'@{Dexterity}',
    modifier:-1, classes:'Rogue/Warrior', book:['PHB']});
NONWEAPON_PROFICIENCIES_TABLE['Set Snares'].push({slots:1, abilityScore:'@{Intelligence}',
    modifier:-1, classes:'Fighter', book:["The Complete Barbarian's Handbook"]});
```
`Sign Language` differs between *The Complete Book of Dwarves* (Int, +2) and *The Complete
Barbarian's Handbook* (Dex, 0); `Crowd Working` differs between *The Complete Bard's Handbook*
(Cha, 0) and *The Complete Book of Humanoids* (Cha, +2). The pack format must let `phb:set-snares`
and `cbarb:set-snares` coexist, and must resolve which one applies by an explicit, user-visible
rule — not by load order.

**One physical line per record.** PCGen `.lst` files are tab-separated with no nesting. A third
party who reverse-engineered the format documents the result: "The `.LST` file format has no
concept of nested records. One line is one record, and everything to do with that record must be
on that one line", citing a real 1,051-column single record in the Pathfinder *Advanced Races
Guide*, and noting "1,000 character long lines are challenging to deal with in a text editor"
(https://www.penwatch.net/cms/pcgen_lst_1/). It is visible in the shipped data — race lines in
`pcgen/data/35e/wizards_of_the_coast/rsrd/basics/rsrd_races.lst` are mostly runs of empty tabs
used as column alignment. Any nested serialisation removes this entire class of problem.

**`.MOD` as the mechanism for "extend another pack's object".** PCGen's escape hatch appends (or
overwrites) tags onto an already-loaded record — `CLASS:Fighter.MOD  CSKILL:…` — with siblings
`.COPY=` (derive) and `.FORGET` (delete). It is pervasive: 1,485 files under `pcgen/data/` contain
a `.MOD`. Three verified problems:
1. **Load-order dependence**, papered over by `RANK:` and by `FORWARDREF:<TYPE>|<name>` which lets
   a pack pre-declare that a reference to an as-yet-unloaded object is legal (PCC tag docs).
2. **Confusing syntax** — `.MOD` lines begin with something that looks like a tag; the penwatch
   author calls `CATEGORY:InternalRecord|MyAbility.MOD` "extremely surprising at first".
3. **Abused as a line-splitting device** — declare an empty stub, then emit N `.MOD` lines
   attaching one tag each; "very verbose, but at least the tags are now broken out onto separate
   lines" (penwatch, showing `Race Traits ~ Dwarf` built from ten `.MOD` lines).

The *capability* is exactly what kits need. The *encoding* is what to avoid: make "patch an object
owned by another pack" an explicit, ordered, declarative operation on a structured document.

**A game-global XP table.** PCGen puts XP thresholds in `system/gameModes/<Mode>/level.lst`, as a
formula (`LEVEL:LEVEL MINXP:(LEVEL*LEVEL-LEVEL)*500`, `pcgen/system/gameModes/35e/level.lst`) or
as named alternatives selected per character (`XPTABLE:Medium|Slow|Fast|PFS Rules`,
`pcgen/system/gameModes/Pathfinder/level.lst`; the character file stores
`EXPERIENCETABLE:Default`, `pcgen/characters/Sorcerer.pcg`). `XPTABLE` is read only by
`pcgen/code/src/java/pcgen/persistence/lst/LevelLoader.java:62`. **There is no per-class XP table
anywhere in PCGen**, because no d20 game needs one. AD&D 2e needs one per class, and dual-classing
needs two live simultaneously. Attach the XP progression to the class.

**A class-global level cap where you need a race×class matrix.** PCGen's `MAXLEVEL:x|NOLIMIT` is
per class, overridable with `CLASS:Barbarian.MOD MAXLEVEL:100`, with a global preference to ignore
it (class tag docs; `CLASS:Barbarian … MAXLEVEL:20`, `rsrd_classes.lst:6`). 2e's limits are a
matrix over (race, class) with ability-score exceptions. Races in PCGen carry no such field
(`rsrd_races.lst`); class-restriction-by-race would be a `PRERACE:` prerequisite on the class.
Model the limit as a table owned by the race (or by the pack that publishes both), not as a scalar
on the class.

**Two words to keep out of the glossary collision zone: "kit" and "template".** PCGen's `KIT` /
`STARTPACK` object is a bundle of choices applied once at creation — race, alignment, class level,
stats, skill ranks, an ability, gear
(`pcgen/data/35e/wizards_of_the_coast/rsrd/basics/rsrd_kits.lst:3-19`). It is not what AD&D 2e
calls a kit. `TEMPLATE` is a creature overlay. Both names are taken by other meanings in the most
obvious prior art; `CONTEXT.md` should say so explicitly.

**Any second, fallback expression evaluator. Ever.** See §5.1 — this is PCGen's worst and
best-documented failure.

**Hardcoding the tables in the application.** Every FOSS AD&D 2e tool found does this (§4). The
prior art offers no counterexample; that is precisely the gap corerules fills.

### 1.3 Open design questions this research does not settle

- **How a kit patches its parent class — no prior art exists.** PCGen's four partial shapes
  (`SUBCLASS`, `SUBSTITUTIONCLASS`, `TEMPLATE`, `.MOD`) each fail for a different reason (§2.4);
  the Roll20 sheet, the most complete open 2e implementation in existence, does not model kits at
  all (§4.1). corerules must invent this. My analysis, offered as a starting point and **not**
  sourced from prior art: a kit is a first-class object that (a) names the class IDs it may attach
  to, (b) carries a prerequisite predicate over race/ability scores/alignment, and (c) carries an
  ordered list of declarative effects applied to *the character's computed view of its class*,
  never to the shared class record — because `.MOD`-style mutation of the class definition would
  apply the kit to every character.
- **Compute or record.** The prior art genuinely splits. PCGen computes aggressively and stores
  only choices. The two live Foundry OSR systems compute essentially nothing: OSE stores
  `saves.{breath,death,paralysis,spell,wand}` as five plain integers and `thac0` as an untyped
  object (§3.1), and Basic Fantasy stores `attackBonus.value` and `saves.*.value` as plain numbers
  with defaults (§3.2). corerules wants the middle path — the user supplies the tables so the
  engine *can* compute — which no shipping project in this space has attempted. Ticket 04 should
  know it is choosing an unproven design, not picking between two proven ones.
- **Whether packs may define new *kinds* of object** (psionics as a parallel subsystem) or only new
  instances of engine-known kinds. PCGen's answer is "some of both" via `FACTDEF` and
  `DYNAMICSCOPE:` (`c__datacontrols.lst:101-102`), but I did not establish how far that stretches —
  psionics in PCGen is a d20 subsystem built from ordinary classes and abilities, not a genuinely
  new object kind.
- **Multi-class XP splitting and dual-class suppression: no prior art found.** PCGen's
  multiclassing is d20's (levels in several classes, `XPPENALTY:YES` on a `CLASSTYPE` in
  `miscinfo.lst`) — not 2e's even XP division. Its `EXCHANGELEVEL:w|x|y|z` (donate levels between
  classes, with donor minimum, maximum donated, and donor floor) is mechanically adjacent to
  dual-classing but was built for monster classes, and I found **no** dataset using it that way.
  Its `EXCLASS:x` ("the class to which all current class levels are converted if the class
  prerequisites are no longer met", with shipped hidden `CLASS:Ex Paladin` / `Ex Druid` /
  `Ex Barbarian` records at `rsrd_classes.lst:26,159,268`) is the closest analogue to 2e's "you
  lose your class" states. The Roll20 sheet handles multi-class only for saves, by taking the best
  value across the character's class groups (§4.1) — nothing about XP.
- **Whether an authoring UI or a hand-editable text format should be primary.** Not investigated.
  Relevant data point: PCGen's docs say list editing needs "a text-editing program and patience"
  and that "Editing or creating a list file can be a hardy experience"
  (`pcgen/docs/listfilepages/listfileimportanttoknow.html`), and PCGen's own LST editor was
  retired — release notes advise users of "the old LST Editor" to read upgrade guides. I did not
  verify the retirement in the source tree.

---

## 2. PCGen — the primary case study

https://github.com/PCGen/pcgen. Cloned at `ef03350b` (2026-07-27). `gradle.properties` gives
version `6.09.08.RC1`, Java toolchain 25 — actively maintained. The README advertises D&D 3.5/4/5,
Pathfinder 1e and Starfinder; **no AD&D edition** (`pcgen/README.md`).

### 2.1 There is no AD&D 2e dataset, and there never was one

On the PCGen mailing list (30 January 2017), replying to a user asking whether a 2nd Edition
source still existed, Andrew Wilson wrote that PCGen was built to automate 3rd edition character
generation and that **"There has never been an official second edition data set"**, that given the
licensing difficulty with non-OGL 3rd-edition books they are "unlikely to ever get permission from
Wizards of the Coast", and that once a new edition ships publishers "go out of their way to make
using previous editions difficult"
(http://bugs-news.blogspot.com/2017/01/re-pcgen-looking-for-2nd-edition_34.html).

**Verified in the tree:** `system/gameModes/` contains an `OSRIC` mode, but `data/` has no OSRIC
directory and `find . -iname "*osric*"` matches only the game-mode folder. The engine ships the
mode; nobody ever shipped the data.

**And the OSRIC mode does not model OSRIC.** It is a near-copy of the 3.5e mode:
`miscinfo.lst` declares `BABMAXATT:4` / `BABATTCYC:5` (iterative attacks off a base attack bonus),
`ABILITYCATEGORY:FEAT`, `SKILLMULTIPLIER:4`, `BONUSFEATLEVELSTARTINTERVAL:3|3`,
`SPELLBASEDC:10+SPELLLEVEL+BASESPELLSTAT`, and points its info sheets at
`preview/summary/35e_info.html.ftl` (lines 115-117, 252, 85-86, 94, 259-260); `statsandchecks.lst`
carries commented-out Fortitude/Reflex/Will definitions; `level.lst` is the generic
`MINXP:(LEVEL*LEVEL-LEVEL)*500`. `ALLOWEDMODES:OSRIC|1e` references a `1e` mode that does not
exist. **PCGen therefore has no working prior art for THAC0, 2e save matrices, race level limits,
kits, proficiency slots or spheres.**

### 2.2 Licensing posture

PCGen's data-development docs describe a formal gate: a source needs "a review… for OGL compliance
and Publisher permission… handled on an internal issue by the Publisher Liaison, Data License and
Content Silverbacks" before entering the repository
(`pcgen/docs/listfilepages/listfiledatadevelopment.html`).

Historical context, **secondary sources, not verified against a primary PCGen statement**: in
February 2003 PCGen announced it would charge for some data files after talks with Wizards of the
Coast, the engine remaining LGPL
(https://games.slashdot.org/story/03/02/23/146253/pcgen-to-charge-for-data-files); commercial WotC
datasets were later produced under licence by Code Monkey Publishing until WotC declined renewal
(https://www.enworld.org/threads/clearing-the-air-about-pcgen-data-files.66810/page-4).

**Read for corerules:** an actively maintained, twenty-plus-year-old, explicitly data-driven
engine with a volunteer data team and a formal publisher-liaison process still has no AD&D 2e
content, because it cannot get permission. Shipping no content is not merely defensible for
corerules; it is the only configuration under which public 2e support exists at all. Worth stating
plainly in the README.

### 2.3 The format, concretely

**Pack manifest (`.pcc`).** `CAMPAIGN:` and `GAMEMODE:` must be the first two lines. Also:
`KEY:`, `GENRE:`, `BOOKTYPE:`, `SETTING:`, `TYPE:` (dot-delimited), `PUBNAMELONG/SHORT/WEB`,
`SOURCELONG/SHORT/WEB/DATE`, `RANK:`, `ISOGL:`, repeated `COPYRIGHT:`, `INFOTEXT:`, `DESC:`,
`COVER:`, `LOGO:`, `URL:<site>|<link>|<desc>` (e-commerce/affiliate links), `HELP:`,
`SHOWINMENU:`, `ISLICENSED:`/`LICENSE:`, `ISMATURE:`, `FORWARDREF:`, `HIDETYPE:<OBJECT>|<types>`,
`PCC:<path>` to include sub-manifests, one tag per data-file kind (`CLASS:`, `RACE:`, `SPELL:`,
`DEITY:`, `DOMAIN:`, `SKILL:`, `TEMPLATE:`, `DATACONTROL:`, `VARIABLE:` …), and global `BONUS:`
lines. Path sigils: `@` = shipped `data/`, `&` = `vendordata/`, `$` = `homebrewdata/`
(`pcgen/docs/listfilepages/datafilestagpages/datafilespcc.html`). Those two extra roots exist in
the tree (`pcgen/vendordata/readme.md`, `pcgen/homebrewdata/readme.md`, both containing only
"default install"), are configured at
`pcgen/code/src/java/pcgen/system/PCGenSettings.java:73-95`, and are shipped by
`pcgen/code/gradle/distribution.gradle:31-32`.

**Data files (`.lst`).** Tab-separated, one record per line, `#` comments, `TAG:value` fields,
`|` intra-tag separator, `.` list separator inside `TYPE:`. Hard rule: "There can _NEVER_ be a
space between fields. Always use a TAB" (`listfileimportanttoknow.html`).

**Classes** are the one genuinely multi-line record type
(`pcgen/docs/listfilepages/datafilestagpages/datafilesclasses.html`; data at
`pcgen/data/35e/wizards_of_the_coast/rsrd/basics/rsrd_classes.lst`):
- one or more `CLASS:<name>` lines with class-wide tags;
- `SUBCLASS:<name>` + `SUBCLASSLEVEL:<n>` lines — designed "to implement the wizard subclasses…
  take the subclass and gain 'spell' related advantages and disadvantages in comparison to the base
  class";
- `SUBSTITUTIONCLASS:` + `SUBSTITUTIONLEVEL:<n>` — "replace a standard level, removing and
  replacing abilities with a different set without effecting any previous or subsequent levels",
  player-elected at that level;
- bare numeric lines `1`…`20` carrying per-level tags.

Progressions appear in both literal and formula form:
`16  CAST:6,6,6,6,6,6,6,5,3  KNOWN:9,5,5,4,4,4,3,2,1` (line 355), the Wizard's whole table at
lines 392ff (`1 CAST:3,1`, `2 CAST:4,2`, `3 CAST:4,2,1`, …); versus
`BONUS:COMBAT|BASEAB|classlevel("APPLIEDAS=NONEPIC")*3/4|TYPE=Base.REPLACE|PREVAREQ:UseFractionalBAB,0`
and `BONUS:SAVE|BASE.Fortitude|classlevel("APPLIEDAS=NONEPIC")/2+2` (lines 38, 73).
`CAST:`/`KNOWN:` accept a formula too, with a documented gotcha: "The formula used cannot have
commas (,) embedded in it. If a formula requires a comma use a DEFINE to set a variable" — i.e.
the comma is the row separator and there is no escaping.

Class tags of direct 2e relevance: `HD:`, `MAXLEVEL:`, `EXCLASS:`, `EXCHANGELEVEL:w|x|y|z`,
`STARTSKILLPTS:`, `CSKILL:`, `MEMORIZE:YES|NO` ("Determines if the class is required to memorize
spells as a wizard does" — vancian preparation is already first-class),
`PROHIBITED:<school>,<school>`, `PROHIBITSPELL:SCHOOL.<x>|PREVARGTEQ:Prohibit<x>,1` and
`PROHIBITSPELL:ALIGNMENT.<x>|…` (the RSRD Wizard line carries one per school, line 362),
`SPELLSTAT:`, `KNOWNSPELLS:LEVEL=n|…`, `SPELLBOOK:YES`, `BONUS:CASTERLEVEL|<class>|CL`,
`KNOWNSPELLSFROMSPECIALTY:`, `ADDDOMAINS:x|x` with `BONUS:DOMAIN|NUMBER|x`, `ALLOWBASECLASS:NO`,
`FACT:Abb|`, `FACT:SpellType|`, `ROLE:`.

`STARTSKILLPTS:` is the nearest thing to a proficiency-slot budget, but it is one number per level
— not 2e's "N weapon slots at start, +1 every M levels, M varying by class group".

**Specialist wizards** are the one 2e-shaped thing PCGen models well:
```
SUBCLASS:Abjurer   COST:2  PROHIBITCOST:1  CHOICE:SCHOOL|Abjuration  KNOWNSPELLSFROMSPECIALTY:1
SUBCLASSLEVEL:1    ABILITY:Special Ability|AUTOMATIC|Abjurer Learning Bonus
```
(lines 367-368). Note it lives **inside the parent class's own file** — a variant authored by
whoever authored the class, not a patch from another pack. That distinction is the crux of §2.4.
The docs also record a known defect: "`SPELLLIST` does not work on Class Line if a subclass or
substitution class is present… Another noted behavior - mainly code bug: Such classes will NOT
display their Known Spells OR cast information on the spells tab. This behavior is expected to be
fixed once CDOM is operational."

**Races** carry `SIZE:`, `MOVE:`, `LANGBONUS:`, `AUTO:LANG|`, `ABILITY:Internal|AUTOMATIC|Race
Traits ~ <race>`, `RACETYPE:`, `RACESUBTYPE:`, `TYPE:`, `STARTFEATS:`, `XTRASKILLPTSPERLVL:`,
`LEGS:`, `HANDS:`, `DEFINESTAT:MINVALUE|INT|3`, `UNENCUMBEREDMOVE:`, `FACE:`, `REACH:`, `CR:`,
`SOURCEPAGE:` (`rsrd_races.lst`). **No level-limit-by-class field exists.**

**Cross-pack operations:** `.MOD`, `.COPY=` (`Thieves' tools.COPY=Thieves' tools (masterwork)`),
`.FORGET` (`CATEGORY=FEAT|Burst Fire.FORGET`).

### 2.4 The kit problem, stated precisely

A 2e kit is authored in book B and modifies a class authored in book A: it narrows allowed races
and ability minimums, grants and forbids proficiencies, changes starting equipment and money, adds
benefits and hindrances, sometimes alters advancement. In corerules this is cross-*pack*.

| PCGen shape | What it does | Why it does not fit |
|---|---|---|
| `SUBCLASS`/`SUBCLASSLEVEL` | class variant, incl. per-level overrides | must live in the parent class's own file; not a cross-pack patch |
| `SUBSTITUTIONCLASS`/`SUBSTITUTIONLEVEL` | replace specific levels' abilities, player-elected | same file-locality problem; replaces levels rather than layering restrictions |
| `TEMPLATE` | creature overlay (`FAVOREDCLASS:`, `SUBRACE:`, language grants — `rsrd_templates.lst`) | overlays a creature; no notion of "only legal on class X" |
| `KIT`/`STARTPACK` | one-shot bundle of creation choices + gear (`rsrd_kits.lst`) | not a persistent modifier; wrong meaning entirely |
| `.MOD` | genuine cross-pack patch | load-order dependent, unstructured, and mutates the **shared** class record — the kit would apply to every character |

The last row is the blocker. **PCGen has no mechanism for what a 2e kit is.** See §1.3 for the
shape I would propose, flagged as unsourced.

### 2.5 Declarative data, or a language? Both — and the boundary keeps moving

- The **data** is declarative tag/value.
- **Formulas** appear inside tag values wherever a number is expected: `BONUS:…`, `CAST:`,
  `KNOWN:`, `MINXP:`, `CSKILLMAX:`, `SPELLRANGE:CLOSE|floor(CASTERLEVEL/2)*5+25`,
  `WEAPONREACH:(RACEREACH+(max(0,REACH-5)))*REACHMULT`
  (`pcgen/system/gameModes/OSRIC/miscinfo.lst:79,246`).
- The **current** evaluator is a JavaCC/JJTree grammar,
  `pcgen/PCGen-Formula/code/src/jjtree/pcgen/base/formula/parse/formula.jjt`, with productions for
  logical, equality, relational, arithmetic, geometric, unary, exponent, paren, function and
  quoted-string expressions — **no assignment, no loops, no I/O**. The built-in function set is
  closed and small: `abs, ceil, floor, if, isempty, ispresent, length, max, min, round, slice,
  value, getOptional, arg`, plus `DefinedFunction`/`GenericFunction`
  (`pcgen/PCGen-Formula/code/src/java/pcgen/base/formula/function/`).
- Data can define its own named functions declaratively:
  `FUNCTION:d20Mod  VALUE:floor((arg(0)-10)/2)`
  (`pcgen/data/pathfinder_2e/core_rulebook/c__datacontrols.lst:98`), and its own variable scopes:
  `DYNAMICSCOPE:MOVEMENT`, `DYNAMICSCOPE:VISION` (same file, lines 101-102).
- Variables: older `DEFINE:<var>|<formula>` + `BONUS:VAR|<var>|<formula>`; newer
  `MODIFY:<var>|SET|ADD|…|<formula>` against variables declared in a `VARIABLE:` file listed by the
  `.pcc`. `MODIFY:` appears in only 34 files under `data/` — the migration is real but far from
  complete.

**Safety verdict: there is no sandbox and none is needed.** An expression grammar cannot express
side effects. corerules should hold exactly this line and never let a content pack ship
JavaScript. The cost is that anything the expression language cannot say must become an engine
feature or a new declarative construct — which is precisely the pressure that produced `FACTDEF`,
`DYNAMICSCOPE` and `STARTTABLE`. Budget for that pressure rather than reaching for a scripting
language when it arrives.

### 2.6 Pack identity, versioning, dependency

- **Identity:** `CAMPAIGN:<long name>` plus optional `KEY:`; objects inside are identified by name
  (+ optional `KEY:`) in one global namespace across all loaded packs.
- **Versioning:** **none.** Searching the rendered PCC tag reference for any `VERSION` tag returns
  nothing.
- **Dependency:** `PRECAMPAIGN:<n>,<campaign name>` or `BOOKTYPE=` / `INCLUDES=` /
  `INCLUDESBOOKTYPE=` (`globalfilesprexxx.html`) — a name/type match with **no version
  constraint**. Composition via `PCC:<path>` includes. Ordering via `RANK:` (1 highest, default 9;
  negatives load before positives). `FORWARDREF:<TYPE>|<name>,<name>` declares legal forward
  references per object type, for ~17 object types.
- **Character compatibility:** `system/gameModes/*/migration.lst` rewrites old object keys to new
  ones, scoped by `MINVER`/`MAXVER`/`MINDEVVER`/`MAXDEVVER` against the PCGen version the
  character was last saved under. Example from the docs:
  `ABILITY:Special Ability|Animal Fury  NEWKEY:Animal Fury ~ Rage Power  MAXVER:6.00.00
  MAXDEVVER:6.01.01`
  (`pcgen/docs/listfilepages/systemfilestagpages/gamemodemigrationlist.html`; live files
  `pcgen/system/gameModes/35e/migration.lst`, `.../5e/migration.lst`, `.../3e/migration.lst`).
  **Only `SOURCE`, `ABILITY`, `EQUIPMENT` and `RACE` are supported** — not classes, spells, skills
  or templates. The whole mechanism exists only because identity is a mutable display name.

---

## 3. Adjacent FOSS tools that separate engine from data

### 3.1 Old-School Essentials for Foundry VTT (`vttred/ose`) — the closest live analogue

https://github.com/vttred/ose — GPL-3.0, 106 stars, last push 2026-01-22 (GitHub API, 2026-08-04).
The healthiest engine-without-content project in this space.

Its README states the system "Requires ***Old-School Essentials*** Classic Fantasy or Advanced
Fantasy" from Necrotic Gnome; content ships as **separate Foundry modules** — a free
"Community-Made Classic Fantasy Compendium" (module id `classicfantasy`, "100% Open Game Content,
and free for all") and paid Necrotic Gnome premium modules on DriveThruRPG. Licensing is split
across `LICENSE.GPL` (code), `LICENSE.OGL` (open game content),
`LICENSE.OTHER.OSE_THIRD_PARTY_V1_5`, and a required trademark notice
(https://raw.githubusercontent.com/vttred/ose/main/README.md). Its manifest confirms it:
`"packs": []` (https://raw.githubusercontent.com/vttred/ose/main/system.json). There is a separate
`vttred/ose-content` repository ("Old School Essentials OGL Compendium module") — **archived**,
last push 2023-01-20 (GitHub API).

**But the engine computes almost nothing.** `src/module/actor/data-model-character.js` declares
`saves` as five nested integer fields (`breath`, `death`, `paralysis`, `spell`, `wand`) and
`thac0` as an untyped `ObjectField` read as `this.thac0.bba` plus `this.thac0.mod.melee` /
`.missile`. There are **no class progression tables, no XP thresholds, no class/level lookup**;
saves and THAC0 are numbers the user types, with small arithmetic modifiers layered on.

### 3.2 Basic Fantasy RPG for Foundry (`orffen/basicfantasyrpg`) — same conclusion

https://github.com/orffen/basicfantasyrpg — 14 stars, last push 2025-09-18. Its `system.json` has
`"id": "basicfantasyrpg"`, `"version": "r15"`, `"compatibility": {"minimum": "11", "verified":
"13"}`, and again **`"packs": []`** — content is a separate compendium module. Notable because
Basic Fantasy's rules text is openly licensed, so the split here is an *architectural* choice, not
a legal necessity.

Its `template.json` stores `armorClass.value`, `attackBonus.value`, and
`saves.{death,wands,paralysis,breath,…}.value` as plain numbers with defaults. The only hardcoded
table in `module/documents/actor.mjs` is the **monster** XP-award lookup
(`let xpLookup = [10, 25, 75, 145, 240, …]`, line 155) used by `_calculateMonsterXPValue()`.
Player class progressions are not modelled.

### 3.3 The rest of the OSR Foundry field

Checked via the GitHub API on 2026-08-04: `thevinter/osric` ("Advanced Dungeons & Dragons 1E –
Foundry VTT Edition", GPL-3.0, 1 star, last push 2022-09-18) and `vonkow/swords-wizardry`
(11 stars, last push 2026-04-10) exist; `Sisyphus192/swords-n-wizardry` and
`hadesrofl/swords-and-wizardry-foundry` appear to be OSE forks. **I did not read these
codebases**; given §3.1 and §3.2 I expect the same recorder architecture, but that is an
inference, not a verified finding.

**The pattern across §3 is consistent and worth internalising:** every successful
engine-without-content project in this genre chose to be a *recorder*. corerules wants the user to
supply the tables so the engine can *compute* — a genuinely more ambitious design with no shipping
precedent in this space.

---

## 4. Dedicated AD&D 2e tools — all hardcode the content

### 4.1 Roll20 `ADnD_2E_Revised` — the largest open 2e implementation in existence

https://github.com/Roll20/roll20-character-sheets/tree/master/ADnD_2E_Revised. Content lives in
plain JavaScript under `javascript/`: `savingThrows.js`, `nonweaponProficiencies.js` (57 KB),
`weaponProficiencies.js` (20 KB), `weapons.js` (192 KB), `priestSpells.js` (857 KB),
`wizardSpells.js` (1.01 MB), `psionicPowers.js` (286 KB), `sheetWorkers.js` (173 KB), plus
Player's Option files. The built artefact `2ESheet.html` is 5.65 MB.

**Saving throws** — flat arrays indexed by level, keyed by class group, five groups
(priest / rogue / warrior / wizard / psionicist), eight categories each
(`paralyzePoisonDeath`, `rodStaffWand`, `petrificationPolymorph`, `breath`, `spell`, plus
Ravenloft's `fear`, `horror`, `madness`):
```js
const SAVING_THROWS = {
    priest: {
        paralyzePoisonDeath: [16,10,10,10,9,9,9,7,7,7,6,6,6,5,5,5,4,4,4,2,2,2],
        rodStaffWand:        [18,14,14,14,13,13,13,11,11,11,10,10,10,9,9,9,8,8,8,6,6,6],
        ...
```
(https://raw.githubusercontent.com/Roll20/roll20-character-sheets/master/ADnD_2E_Revised/javascript/savingThrows.js).
The presence of `fear`/`horror`/`madness` bolted onto the same five-group structure is evidence
that the **set of save categories must itself be data**, not a hardcoded enum.

**THAC0** — closed-form per class group, `THAC0_FORMULAS` in `sheetWorkers.js` (quoted in §1.1).
So in 2e THAC0 is regular and saves are not; a pack format must support both encodings.

**Multi-class** — five fixed class slots where the *slot index hardcodes the group*: slot 1 =
warrior, 2 = wizard, 3 = priest, 4 = rogue, 5 = psionicist (`sheetWorkers.js`, the
`switch (levelField.slice(-1))` block). Saves are then resolved with
`Math.min(...Object.values(classesWithLevels).map(cp => SAVING_THROWS[cp.classGroup][category][cp.level]))`
— best (lowest) value across the character's classes. Crude but correct for 2e, and it makes an
illegal state (two classes from the same group) unrepresentable. **Nothing handles XP splitting.**

**Non-weapon proficiencies** — `{slots, abilityScore, modifier, classes, book}` per entry, 217
distinct names / 224 entries because of cross-book redefinition (quoted in §1.2). `classes` is a
free-text group restriction (`'Rogue/Warrior'`, `'Fighter'`, `''`). Books referenced: PHB;
Player's Option: Spells & Magic; The Complete Barbarian's / Bard's / Ninja's / Paladin's /
Ranger's / Thief's Handbook; The Complete Book of Dwarves / Humanoids / Necromancers; The Complete
Psionics Handbook. **Free-text strings, no IDs, no versions.**

**Active-book toggles** — the fixed `BOOK_FIELDS` array of 21 slugs, listed in §1.1. Directly
relevant to corerules ticket 03 (which Complete Handbooks) as a real-world prioritisation signal:
the PHBR series plus Tome of Magic and the Arms & Equipment Guide.

**Spells** — objects keyed by spell level (`pri1`, `pri2`, …), each entry
`{level, school, sphere, range, duration, aoe, components, cast-time, saving-throw, materials,
reference, book, damage, damage-type, healing, effect}`. `school` and `sphere` are **free-text and
multi-valued by string concatenation**: `'school': 'Enchantment/Charm'`; spheres such as
`'Animal, Guardian, Summoning'`, `'Astral, Divination, Time'`, `'Chaos, Law, Wards'`. A spell
belongs to many spheres; a class or kit grants sphere access at major/minor grade. corerules
should model that as a real many-to-many over IDs. Access thresholds are a separate table,
`SPELL_LEVEL_REQUIREMENT` in `sheetWorkers.js`: Wizard `{1:1, 2:3, 3:5, 4:7, 5:9, 6:12, 7:14,
8:16, 9:18}`, Priest `{1:1, 2:3, 3:5, 4:7, 5:9, 6:11, 7:14, q:10}`.

**Kits are not modelled.** The component list under `pug/` is `nonweaponProficienciesDatalist`,
`nonweaponProficienciesOverview`, `psionicCorePowerDatalists`, `schoolsOverview`,
`simpleDatalists`, `spellScrollsDatalists`, `spellTemplate`, `spellsDatalists`, `spheresOverview`,
`thievingSkillsDexterityAdjustments`, `weaponsDatalists`, `weaponsOverview` — schools and spheres
yes, kits no. `futureIdeas.txt` mentions kits nowhere; its only PHBR-derived entry is a Complete
Druid's Handbook herbalism rule. **The most complete open 2e implementation in existence has
schools, spheres, proficiencies, psionics and Player's Option crit tables — and no kits.**

### 4.2 Standalone 2e generators — all small, all hardcoded

Recorded so the negative result is on the record. Characterised from READMEs, file listings and
GitHub API metadata; **I did not deep-read these repositories.**
- `4161726f6e/DD-2E-Character-Generator` — Python; externalises name lists and
  `priest_spells.csv` / `wizard_spells.csv`, but rules live in `priest.py`, `rogue.py`,
  `warrior.py`, `wizard.py`, `nonWeaponProfs.py`.
- `bigbadmad/CharacterGenerator` — TypeScript; tables isolated in a `data.ts` module (a code
  module, not a loadable file).
- `lockout87/characterGenerator` — Python; tables in `tables.py`.
- `zimmermannliam/guygen2e` — self-described unfinished.
- `dcandido/ADnD-Character-Creator` — VB.NET.
- Foundry attempts: `nickkeane/adnd2e` (MIT, last push 2023-06-13, 0 stars),
  `thelensrpg/dnd2e-foundry` (Unlicense, last push 2024-12-29, 1 star) — both dormant;
  `keithhannen/FVTT_ADnD2E` appears to be unmodified system boilerplate.

---

## 5. What the maintainers themselves say went wrong

Almost entirely PCGen, because it is the only project here old enough — and candid enough — to
have documented its own mistakes inside shipping artefacts.

### 5.1 Three formula parsers, all still live, with different semantics

PCGen's own formula documentation
(`pcgen/docs/listfilepages/globalfilestagpages/globalfilesformulas.html`) opens:

> "Starting with version 5.7.1 PCGen has incorporated the JEP (Java Mathematical Expression
> Parser) library. **This was done because the original code which has evolved over time is
> problematic due to its complexity and lack of documentation.** The JEP library has a clearly
> defined grammar which is available on the web site… **As a fall back, if the JEP parser fails to
> parse the function then the old code is called. At some point in the future the old code support
> will be dropped and all formulas must be in JEP syntax.**"

"At some point in the future" was version 5.7.1. **In the 6.09.08 tree cloned for this research it
still has not happened.** `pcgen/build.gradle:241` declares `implementation 'org.scijava:jep:2.4.2'`;
19 JEP command plugins live in `pcgen/code/src/java/plugin/jepcommands/`; and
`pcgen/code/src/java/pcgen/core/VariableProcessor.java:125-139` reads:

```java
public Float getVariableValue(CharacterSpell aSpell, String varString, String src, int spellLevelTemp) {
    Float result = getJepOnlyVariableValue(aSpell, varString, src, spellLevelTemp);
    if (null == result) {
        result = processBrokenParser(aSpell, varString, src, spellLevelTemp);
        ...
```

The fallback method is named **`processBrokenParser`**. Its javadoc: "Evaluate the variable using
the old non-JEP variable parser. Use of this parser is being phased out." (lines 205-215). Its
first statement is `aString = aString.toUpperCase()` (line 218). Meanwhile a *third* engine — the
JavaCC grammar in `PCGen-Formula/` — backs the newer `MODIFY:`/`VARIABLE:`/`FUNCTION:` tags, used
by only 34 data files.

The semantics differ, and the docs say so with a worked example: for `((CL+1)+(3*TL)/2)+4` with
`CL=TL=4`, "JEP Expression … returns 15" and "Non-JEP Expression … returns 12", because JEP uses
standard mathematical precedence while the old parser resolves parentheses and then processes
strictly left-to-right (same page). **The same characters in the same data file mean two different
numbers depending on which parser happened to succeed.**

Also documented there: "PCGen truncates (or rounds down to the nearest integer) the results of each
formula" and "PCGen *does* round each tag down rather than add the tags together and then round
down." Rounding is per-tag, not global — a rule no reimplementer would guess, and 2e is full of
`/2`.

**Recommendation:** one evaluator, specified before any content is authored, versioned explicitly,
with no fallback path ever. If the language must change, ship a converter and bump the pack format
version. Fix rounding semantics in writing, with worked examples.

### 5.2 Identifiers and expression syntax share one namespace

The official variables documentation
(`pcgen/docs/listfilepages/globalfilestagpages/globalfilesvariables.html`, lines 36-53 of the
rendered text):

> "A problem with one of the parser. When it looks at some of the variables it parses them
> strangely. It is best to avoid the use of MIN and MAX in your variables till the parser is
> replaced in the area of 6.0+
> The variable CHAMIN3 is coming out as min(CHA,3).
> The variable MindBladeECL is coming out as ()MIN(dBladeECL).
> The TYPE Illumination is apparently coming out as (Illu)MIN(ation)."

A type named `Illumination` breaks because it contains the substring `MIN` — the direct consequence
of upper-casing the formula and doing substring substitution over an untyped identifier namespace.

The leak runs the other way too: identifiers the grammar cannot tokenise need an escape function —
"the variable `CL` can be used in a formula without a problem but the variable `CL=Fighter` cannot
be used because of the `=` symbol. In these cases you must use `var("CL=Fighter")`", with further
examples `var("COUNT[FEATS]")`, `var("COUNT[FEATTYPE=type]")` (formulas doc page). The formal JEP
identifier rule is quoted there as "a letter followed by one or more letters and digits" where a
letter is `$`, `_`, `a-z`, `A-Z`.

**Recommendation:** identifiers referenced from expressions must be lexically distinguishable from
operators and function names — a sigil, a mandatory `var("…")`-style reference, or a scoped path
syntax. Never substitute text into a formula string. Never case-fold.

### 5.3 The primary taxonomy is an undeclared free-text namespace

`TYPE:` is PCGen's main grouping / filtering / prerequisite key (`TYPE:Base.PC`,
`TYPE:Humanoid.PC.Base`, `TYPE:Class.Hidden.RadiantEnlightenment`), dot-delimited, declared
nowhere. The docs' own glossary opens: "This is a list in progress. Just because the `TYPE:`
variable that you want to use is not listed does not mean that you cannot use it, just that it has
not been listed yet" (variables page, lines 5-7), then hand-lists types per dataset, file by file.
The only rules are conventions: "The preferred version of multiple word TYPE: variables is
OneTwoThree… The use of `Heavy GyroJet Launcher` will cause indexing problems and is not
permitted" (lines 13-19).

**Recommendation:** packs will invent categories (the PHBR series does it constantly). Require
them to be *declared* in the manifest, the way PCGen's own later `FACTDEF` / `DYNAMICSCOPE`
mechanism requires. An undeclared string namespace cannot be validated, migrated, or listed in a
UI.

### 5.4 Documentation drift is a first-order failure mode

PCGen's class-file documentation still says a known spell-list bug "is expected to be fixed once
CDOM is operational" (`datafilesclasses.html`) — CDOM being an internal rewrite begun in the 5.x
era. The beginner course carries a caveat that its lessons were written many years ago and don't
always show the modern way of doing things (reported in the PCGen list-file help group;
**I did not verify that specific sentence in the repository**, though the repo's `lstfileclass/`
lessons are visibly of that vintage, referencing Yahoo Groups). For corerules the format reference
is a product surface, not an appendix: the people writing packs are not programmers.

### 5.5 Data authoring is a bottleneck, and PCGen knows it

`pcgen/docs/listfilepages/listfileimportanttoknow.html`: "You need only 2 things to get going, a
text-editing program and patience… Editing or creating a list file can be a hardy experience and
not one to take on lightly." PCGen mitigates with an autoformatter, `prettylst.pl` / `PCGen
TidyLst`, whose banner comment heads 2,434 of the 6,311 `.lst` files under `pcgen/data/`
(e.g. "reformatted by PCGen TidyLst v1.04.00", `c__datatables.lst:1`); with a Data Set Converter
shipped in the application; and with a separate `PCGen/pcgen-utilities` repository of text-editor
syntax definitions (https://github.com/PCGen/pcgen-utilities). corerules
has the same problem in a harder form — the map already lists "How book content actually gets into
the tool" as unspecified — and PCGen's answer (text files plus a formatter plus a converter plus
editor syntax highlighting) is a real, if unambitious, data point.

---

## 6. Things I could not establish

- **Any tool, anywhere, that models AD&D 2e kits as loadable data.** Searched PCGen (no 2e data at
  all), the Roll20 2e sheet (no kit support), the Foundry 2e attempts (dormant), and the
  standalone generators (hardcoded). Absence of evidence here is fairly strong given how much of
  the space was covered, but it is not proof.
- **Whether PCGen's `EXCHANGELEVEL` has ever been used for a dual-class-like mechanic.** The tag
  exists and is documented; I found no dataset using it that way.
- **PCGen developers' own design retrospectives in prose.** The GitHub issue tracker has almost
  nothing on the data format (a search for `lst format`, `data format`, `json`, `yaml` in issue
  titles on `PCGen/pcgen` returns 2 results total, both about array data formats:
  https://github.com/PCGen/pcgen/pull/1070, https://github.com/PCGen/pcgen/pull/1068). The
  historical design discussion lives on `pcgendevelopers@pcgen.groups.io` and in a retired JIRA;
  I could not retrieve a substantive retrospective thread. **The best retrospectives PCGen has are
  the ones embedded in its own shipping code and documentation, quoted in §5** — a method worth
  remembering: read the source's apologies, not its blog.
- **Whether PCGen's LST Editor was formally retired**, and what replaced it. Release notes
  reference "the old LST Editor" and upgrade guides; I did not confirm in the tree.
- **Hero Lab, Fantasy Grounds and the TSR "Core Rules 2.0" CD-ROM** as prior art. All closed
  source; not investigated. Hero Lab in particular is reputed to have a rich archetype/adjustment
  model that would be the nearest commercial analogue to 2e kits — **unverified, and I would not
  build on that reputation without seeing the format.**
- **Whether the OSR Foundry systems in §3.3 confirm the recorder pattern.** Inferred from §3.1 and
  §3.2, not read.
