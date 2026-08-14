# Which spec does the Engine implement?

Type: grilling
Status: open

## Question

`spec.md` is the specification. It is also **wrong in 42 places, and we know which**.

The [corpus map](../../corpus-v1/map.md#corrections-owed-to-the-v1-spec) collected every conclusion
that did not survive contact with the books. Nine were fixed in the schema as they were found; **33
are owed to `spec.md` and unwritten there**. So the document an implementation session reads still
says the superseded thing, and the corrected thing lives in a different repository's map, in a list
ordered by discovery rather than by section.

This ticket decides **what an implementation session is supposed to read**, and it has to be decided
first, because every other ticket in this map inherits the answer.

## Why this is a decision and not a chore

The obvious move — *"update `spec.md` from the list, then start"* — is one option, not the answer.
Three things argue against taking it reflexively.

**The corrections are not all the same kind of thing.** Reading the 42: some are outright reversals
(§4.3 needs a clamp between `adjust` and `set`), some are widenings the schema already carries
(`scalar` gained a third arm), some are *measurements* rather than corrections at all (the Engine's
field vocabulary is far larger than the schema's, and nothing checks it). Folding a measurement into
a specification as though it were a rule change would make the spec worse.

**A spec is written to be implementable, and the list is written to be complete.** They are ordered
by different things. Correction 16 became 16b through 16e because one item turned out to be four —
useful history in a map, noise in a specification.

**The spec may not be the artifact that should absorb them.** Nine corrections were absorbed by
`pack-0.1.schema.json` instead, and that turned out well: a schema change is testable the moment it is
made, and `validate.py` proves the corpus still conforms. A correction landed in prose is checked by
nobody. **The question is not only *when* to update the spec but *which corrections belong in a
document at all*.**

## What would settle it

- **A pass over all 42, classifying each**: reversal, widening already carried, measurement, or
  product decision. The classification is the work; the update policy falls out of it.
- **A judgement on §13's known unknowns.** The spec lists nine; the corpus map answered #2 and #4 and
  left #1 open with far more evidence than it had. Whatever happens to the corrections list, §13 is
  stale in a way that matters, because it is the section a reader consults to find out what is still
  risky.
- **A statement of what an implementation session reads on day one.** One document, or a document plus
  a list, or a rewritten spec. Whichever it is, it should be nameable in a sentence.

## What it must not do

**Do not turn this map into a documentation effort.** The destination is an Engine that runs. If the
answer is *"rewrite the spec first"*, that is a decision with a cost, and the cost should be stated
before it is paid rather than discovered halfway through.
