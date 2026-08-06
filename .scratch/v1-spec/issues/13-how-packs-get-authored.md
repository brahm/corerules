# How Content Packs get authored

Type: grilling
Status: open
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

**Do not restate the licence position here** — it is settled and recorded on
[ticket 07](./07-character-file-format-and-identity.md): packs are derived WotC content and do not
circulate, characters are the user's own and may.
