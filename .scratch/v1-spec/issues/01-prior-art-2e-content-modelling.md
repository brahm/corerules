# Prior art: how existing tools model AD&D 2e content

Type: research
Status: claimed
Blocked by: —
Findings: ../research/01-prior-art-2e-content-modelling.md

## Question

How do existing character tools model AD&D 2e-era rules content, and what should corerules
borrow or deliberately avoid?

PCGen is the closest known prior art: it ships a free engine and distributes licensed data
separately in its own LST format — the exact architecture corerules has committed to. Find out
how it, and any other FOSS tools targeting 2e or its retroclones, express the awkward shapes:

- per-class experience tables and level progressions
- THAC0 and saving-throw matrices
- race-based level limits and class restrictions
- kits that *modify* their parent class rather than standing alone
- weapon and non-weapon proficiency groups
- spell lists split by school (wizard) and sphere (priest)

Where do those formats break down, and what do their maintainers regret?
