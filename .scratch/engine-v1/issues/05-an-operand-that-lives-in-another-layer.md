# An operand that lives in another layer

Type: grilling
Status: open

## Question

**§4.3's effects name fields and refs. They cannot name another layer's contents, and 62 effects need
to.**

[Ticket 03](./03-precedence-when-two-attachables-contradict.md) went looking for precedence between
Attachables and found none — one arm out of five ever restricts anything, and 14,910 sheets produced
zero contested values. What it found instead, underneath all three of the cases it was opened for, is
a different shape:

| | |
|---|---|
| **Temple Guard** (CBD) | `require weaponProficiency count 1` with **no `from`**. The list is *the character's deity's*. |
| **Pariah** (CBD) | `forbid weapon`, subject written inline as *"weapons forbidden by the character's religion"* — the **complement** of another layer's list. |
| **60 priesthoods** (CPRH) | *"Weapons Permitted: …"* means **only** those. Transcribed as marked grants, because no operation says "only". |

None of these is a contradiction. Each is an effect whose **operand is another record's effect set**,
and the layer model has no expression for that. An effect can say *grant `phb:sickle`*; it cannot say
*grant the intersection of what my Deity permits with what my Kit lists*, nor *forbid everything my
Deity does not permit*.

## Why it is the same problem three times

The permit-list is not a separate issue from the two kits. A permit-list **is** a set-valued fact
about one layer, and the two kits are effects that want to **read** it. Give the priesthoods a
`limitation` record naming their permitted set — [correction 46](../corpus-v1/map.md#corrections-owed-to-the-v1-spec)
— and the Pariah's forbid has a subject and the Temple Guard's require has a list. **Sixty-two effects
collapse to one missing capability**, which is a good sign that the capability is real rather than a
pile of special cases.

## Why it cannot be deferred to the evaluator

Because the character sheet is wrong in a direction that matters. Today a priest of a permit-listed
faith is granted **one representative weapon** and prohibited from nothing, so the Engine will let a
player take a weapon the book plainly forbids **and will not know it is doing so.** That is the exact
failure §5.2 calls survivable — an unenforced restriction, with the user told — except that nothing
tells the user, because the prohibition was never written.

## The options

| | |
|---|---|
| **A seventh operation** | `permit`, `restrictTo`, whatever it is called: the complement of `grant`. §4.3's six operations have survived 1,910 effects and correction 13 measured that they suffice; adding one is a real cost and this is the first thing that has ever plainly wanted it. |
| **A set-valued `limitation` record, read by `ref`** | The permitted list becomes a record; `forbid`/`require` point at it. **No new operation** and it reuses the machinery `except` already uses. The cost is that "the set of things I may use" becomes a first-class thing the Engine must combine across layers. |
| **Resolve at transcription** | Expand the permit-list into explicit `forbid`s at pack-build time. Honest arithmetic, no schema change — and it writes out every weapon in the game per priesthood, and the two kits still cannot name the deity's list. |
| **Leave it marked** | Ticket 02's rule already makes it *visible*: applied, with the rider printed. It stays wrong and says so. Defensible for v1 and it forfeits the promise for 60 of the 59 priesthoods plus every CBD priest kit. |

## What would settle it

- **How many permit-lists there are outside the CPRH.** 60 in one book is a strong case; if the CFH,
  CRH and CTH weapon lists are the same shape the number is much larger and option 3 dies.
- **Whether "the set of things I may use" is genuinely one concept** across weapons, armour, spheres
  and schools, or four that happen to rhyme. The pack already forbids by `spellSchool`, `armor`,
  `sphere` and `weapon`, which suggests one.
- **What it costs the evaluator.** A layer that publishes a *set* other layers read is a different
  computation from a layer that writes a field, and §4.3's commutation guarantee has to be re-checked
  against it. That is the question, and it is not obviously answerable without trying it.

## Note on scope

This is the gap the corpus map's known unknown #1 was pointing at without naming — ticket 03 answered
the question as asked (there is no precedence to find) and this is what was underneath. It is
**62 effects wide against that ticket's one**, and it is the largest remaining hole in §4.3.
