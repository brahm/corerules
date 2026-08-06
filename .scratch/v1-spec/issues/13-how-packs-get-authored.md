# How Content Packs get authored

Type: grilling
Status: resolved
Blocked by: —

## Question

Graduated from the map's fog by [ticket 04](./04-validate-or-record.md), which turned this from
"somebody types the books in" into something with a shape: **Wagner already holds the entire AD&D
2e corpus as RTF and PDF**, and will author the packs from those. Pack authoring is therefore
*extraction*, not transcription — which makes taking the corpus in bulk plausible rather than a
years-long slog, and changes what the tool should offer.

The size of the job is set by two earlier decisions. Ticket 03 put all ~24 PHBR volumes plus the
core books in v1's engine scope, and ticket 04 made the packs carry not just a catalogue but
**prerequisites, restrictions and slot budgets** — a rules language. So this is not "import a list
of spells"; parts of it are inherently a judgement call no extractor makes for you.

Questions to settle:

- **Is authoring inside the tool at all?** A pack editor is a second product surface with its own
  UI, validation and undo. The alternative is that packs are authored externally — text files in
  an editor, or a conversion script — and corerules only ever *loads* them. Ticket 01's data point:
  PCGen's answer is text files plus an autoformatter plus a converter plus editor syntax
  definitions, and its own docs concede that list editing takes "a text-editing program and
  patience".
- **Where the extraction/judgement line falls.** Spell text, equipment lists and proficiency
  tables are bulk-extractable. Table 6, kit prerequisites and slot budgets have to be *modelled*,
  and the book states them in prose. Which half is the tool's problem?
- **Is any of this v1?** The destination is creating and persisting characters. A tool that cannot
  load a pack is useless, but a tool that cannot *author* one may still be a complete v1 if
  authoring is external. This is the scoping call.
- **Validation of the pack itself.** Ticket 04's A3 means a pack declares which rule-sets it
  provides. Something has to check that a pack's declaration matches its contents, or A3's honesty
  guarantee is hollow. Is that a load-time check, an authoring-time check, or both?
- **The half-authored state is normal.** Ticket 01 recommended typed default values for exactly
  this reason. What does a partially authored pack do — load with gaps, or refuse?

**Unblocked — [ticket 06](./06-content-pack-format.md) gave the thing being authored its shape:** a
**directory** of **JSON** files whose contents are declared by a **manifest**, with book and page
citation required on every record and expressions carried as strings.

That sharpens this ticket's central question rather than answering it. 06 recorded that JSON is
tedious to edit by hand, and said explicitly that how much this matters **depends on what is
decided here** — heavily if authoring is external in a text editor, barely if it goes through the
tool. So the question is no longer abstract:

- A manifest-declared directory means an authoring path must maintain **two things in step** — the
  files and the manifest. Hand-editing can desynchronise them; 06 made that detectable (an orphan
  file is reported), not impossible.
- Ticket 06 also settled that **the same proficiency defined by two books is two objects**, so
  extraction must not deduplicate by name. The natural instinct when parsing 24 books is exactly
  the wrong one.

## Answer

The map's last chance to grow v1, and it did not take it.

### Decision — corerules does not author packs

Packs are authored **outside** the tool: an extraction script, a text editor. corerules loads and
validates, and its whole contribution to authoring is **a validator that names the file, the record
and the field**. Rejected: an in-tool pack editor, and the apparently contained middle of
"edit existing records only" — which is not contained at all, since under ticket 11's closed kinds
it still means a form for each of twenty-odd kinds.

The framing that decided it is that **"authoring" is two activities with different owners**. Bulk
extraction is a one-off over 24 books from RTF and PDF — script work, never a GUI activity. Spot
correction is forever, discovered while *using* the tool. Only the second argues for an editor, and
three settled decisions make it cheap to answer without one:

- **The validator already exists.** A pack must be validated at load anyway, for A3. Making it
  report a location rather than "invalid pack" is small work on something already being built.
- **A pack is a directory of JSON** (ticket 06), which puts the entire transcription under git for
  free — history, diff, revert, branch. An embedded editor would fight that; external editing gets
  version control without a line of code.
- **Corrections propagate** (ticket 07). The loop closes: notice the error in the tool, fix it in an
  editor, reopen the character, the tool reports what moved.

Also weighed, and it has changed since this ticket was fog: **LLM-assisted extraction is the natural
2026 route** from RTF to structured JSON, which makes the bulk far less manual than "somebody types
the books in" implied, and reduces a graphical editor's value proportionally.

**Accepted cost, and ticket 06 named it in advance:** 06 recorded that JSON is tedious to hand-edit
and said explicitly that the weight of that depends on this ticket. Under this decision the cost
lands in full — every correction goes through a text editor, braces and commas included.

### Decision — rule-set declarations are scoped to what the pack provides

Checking what a validator would actually check surfaced a hole in A3 that no earlier ticket caught.

A3 has a pack declare which rule-sets it provides. But **authority over what?** Concretely: the PHB
declares race/class restrictions and states them as a permit-list. The Complete Barbarian's Handbook
then adds the Barbarian, which the PHB's table predates and does not mention. Under permit-list
semantics **absence is prohibition**, so Barbarian would be unplayable by any race — a fatal result
that appears at the second book transcribed.

So each pack declares restrictions for **the subjects it introduces**, and the engine unions.
Rejected: a global claim of authority per rule-set.

It fits two standing decisions without new machinery. **Union commutes**, so ticket 10's
order-independence holds. And **rule extension works for free**: if the Complete Book of Elves says
elves may be paladins, contradicting PHB Table 6, union simply adds to the permit-list and elf
paladins become legal — which is exactly what the book is saying. Under a global claim, two packs
would be fighting over one table.

Two riders, recorded rather than asked:

- **A declared but empty rule-set is reported as suspicious.** Under strict A3, declaring
  race/class restrictions with an empty table means "nothing is permitted" and no character can be
  built. That is almost always half-finished extraction rather than intent, so the engine warns
  instead of silently obeying.
- **The declaration cannot be derived from the contents**, which would be the obvious way to prevent
  desynchronisation. If the presence of a table *were* the declaration, the distinction between
  "does not restrict" and "not transcribed yet" would be lost — and that distinction is why A3
  exists.

### Decision — a malformed pack does not load at all

Wagner's call, against a recommendation of partial loading with the failures reported.

**The precedent invoked for partial loading does not transfer, and that was an error in the
recommendation rather than in the decision.** Ticket 07 tolerated pack *drift* on availability
grounds — but there the pack is **valid** and has changed. Here the pack is **invalid**. Tolerating
change is not an argument for tolerating corruption.

All-or-nothing also removes the failure mode the recommendation had itself recorded as partial
loading's cost: that warnings ignored for months would leave characters built against a corpus full
of holes. Under this decision **corpus integrity is binary** — "the PHB loads" means the PHB is
entirely there. That is the same posture that rejected YAML, that requires conflicts to be
reported, and that produced A3: never be quietly incomplete.

Two consequences:

- **The known-gap problem disappears.** There is no longer any need to distinguish "this record
  failed to parse" from "this record does not exist", because the engine never runs against a
  half-loaded pack.
- **The precise validator stops being a convenience and becomes load-bearing.** It is the only route
  to the bad record: without file, record and field, the user is left with "the PHB does not load"
  and hundreds of files to search.

### The distinction this ticket's own question was blurring

**Incomplete is not invalid.** A pack that declares fewer rule-sets loads normally — that is A3
working as designed. A pack that is malformed does not load. Missing transcription and extraction
rubbish are different things, and only the second blocks.

### Dissolved

**Where the extraction/judgement line falls** stopped being a spec question once authoring moved
outside the tool. corerules loads and validates; where the parser ends and Wagner's reading of the
book's prose begins is his pipeline's business. What the spec had to guarantee — that the prose of
a rule be *expressible* — is already guaranteed by ticket 06's rules language.

**Do not restate the licence position here** — it is settled and recorded on
[ticket 07](./07-character-file-format-and-identity.md): packs are derived WotC content and do not
circulate, characters are the user's own and may.
