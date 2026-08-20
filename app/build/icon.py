#!/usr/bin/env python3
"""Draw the application icon.

The icon is committed as a PNG, and so is this, because an asset nobody can regenerate is a
mystery binary in a repository that otherwise explains itself. Run it and you get the same file:

    python3 build/icon.py

**A twenty-sided die, seen face on, and it is real geometry rather than a drawing of one.** The
vertices are the icosahedron's own — cyclic permutations of (0, ±1, ±φ) — rotated so that one
face points at the viewer and projected flat. The facets you see are the faces that survive
back-face culling, so the silhouette is a true hexagon and the interior lines are where the
solid's edges actually fall.

Why a d20 at all: it is the one shape that says *tabletop role-playing* without saying anyone's
name. corerules ships no licensed content and the icon must not either — no wordmark, no
trademark, no borrowed art. A Platonic solid described by Plato is safe ground.

The palette is the application's own, from `renderer.css`: ink on a dark ground, so the icon
looks like the window it opens.

Pillow is used to rasterise and is NOT a dependency of anything that ships — it is a tool run by
hand, once, the way a designer's editor would be.
"""
import itertools
import math
import pathlib

from PIL import Image, ImageDraw

SIZE = 1024
SUPER = 4  # supersample, then downscale: cheaper than antialiasing each polygon
PHI = (1 + 5 ** 0.5) / 2

PAPER = (18, 16, 14)      # --paper
INK = (232, 227, 218)     # --ink
RULE = (42, 38, 34)       # --rule


def icosahedron() -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int]]]:
    """The twelve vertices and twenty faces, from the definition rather than from a table."""
    vertices: list[tuple[float, float, float]] = []
    for a, b in itertools.product((-1, 1), (-PHI, PHI)):
        vertices += [(0.0, a, b), (a, b, 0.0), (b, 0.0, a)]

    # Two vertices share an edge when they are the minimum distance apart, and three mutually
    # adjacent vertices are a face. Derived, so a typo in a face table cannot happen.
    edge = min(dist(p, q) for p, q in itertools.combinations(vertices, 2))
    faces = [
        (i, j, k)
        for i, j, k in itertools.combinations(range(len(vertices)), 3)
        if dist(vertices[i], vertices[j]) < edge * 1.01
        and dist(vertices[j], vertices[k]) < edge * 1.01
        and dist(vertices[i], vertices[k]) < edge * 1.01
    ]
    return vertices, faces


def dist(p: tuple[float, ...], q: tuple[float, ...]) -> float:
    return math.dist(p, q)


def basis(normal: tuple[float, float, float]) -> tuple[tuple[float, float, float], ...]:
    """An orthonormal frame with `normal` as the viewing axis.

    Built rather than composed from Euler angles: two rotations in the wrong order is the
    classic way to get a die that is *nearly* face-on, and nearly is exactly what looks wrong
    without anyone being able to say why.
    """
    n = unit(normal)
    # Any vector not parallel to n will do for the first cross product.
    seed = (0.0, 0.0, 1.0) if abs(n[2]) < 0.9 else (1.0, 0.0, 0.0)
    u = unit(cross(seed, n))
    return (u, cross(n, u), n)


def unit(v: tuple[float, float, float]) -> tuple[float, float, float]:
    length = math.hypot(*v) or 1.0
    return (v[0] / length, v[1] / length, v[2] / length)


def cross(a: tuple[float, float, float], b: tuple[float, float, float]) -> tuple[float, float, float]:
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def main() -> None:
    vertices, faces = icosahedron()

    # Turn the solid so one face is square to the viewer. A d20 photographed on a table is
    # always resting on a face, and an icon that is not looks like a mistake nobody can name.
    top = max(faces, key=lambda f: sum(vertices[i][2] for i in f))
    normal = tuple(sum(vertices[i][k] for i in top) / 3 for k in range(3))
    frame = basis(normal)
    turned = [tuple(sum(v[k] * axis[k] for k in range(3)) for axis in frame) for v in vertices]

    # Centred on the shape's own extent rather than on the origin. They coincide for a solid
    # this symmetric, and relying on that would break the moment the angle changed.
    xs = [p[0] for p in turned]
    ys = [p[1] for p in turned]
    mid = ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)
    reach = max(math.hypot(x - mid[0], y - mid[1]) for x, y, _ in turned)
    scale = SIZE * SUPER * 0.40 / reach
    centre = SIZE * SUPER / 2
    flat = [(centre + (x - mid[0]) * scale, centre - (y - mid[1]) * scale) for x, y, _ in turned]

    image = Image.new("RGB", (SIZE * SUPER, SIZE * SUPER), PAPER)
    draw = ImageDraw.Draw(image)

    visible = [f for f in faces if facing(turned, f) > 0]
    # Nearest last, so the painter's algorithm needs no depth buffer.
    visible.sort(key=lambda f: sum(turned[i][2] for i in f))
    for face in visible:
        # Shade by how square the facet is to the viewer. Exactly three values appear at this
        # angle — 1.0 for the face you are looking at, 0.745 for the three around it, 0.333 for
        # the six on the horizon — and that is what gives a d20 its look.
        #
        # Spread hard rather than shaded realistically. A physically plausible falloff puts the
        # near face and its neighbours within a few percent of each other, and at 32 pixels the
        # die turns into a grey blob. An icon is read at a glance or not at all.
        lit = facing(turned, face)
        tone = {1.0: 1.0, 0.745: 0.52, 0.333: 0.18}.get(round(lit, 3), lit)
        shade = tuple(int(RULE[k] + (INK[k] - RULE[k]) * tone) for k in range(3))
        draw.polygon([flat[i] for i in face], fill=shade, outline=PAPER,
                     width=max(1, SIZE * SUPER // 300))

    out = pathlib.Path(__file__).with_name("icon.png")
    image.resize((SIZE, SIZE), Image.LANCZOS).save(out)
    print(f"{out}  {SIZE}x{SIZE}  {len(visible)} of {len(faces)} faces visible")


def facing(vertices: list[tuple[float, float, float]], face: tuple[int, int, int]) -> float:
    """How square this face is to the viewer: 1 head-on, 0 edge-on, negative facing away.

    The winding of the face tuples is whatever `combinations` happened to produce, so the cross
    product points inward as often as out. It is oriented against the CENTROID instead — the
    solid is centred on the origin, so the outward normal is the one pointing away from it.
    Guessing the winding from the face's own depth, which was the first attempt here, culled the
    head-on face and left the die looking hollow.
    """
    a, b, c = (vertices[i] for i in face)
    u = tuple(b[k] - a[k] for k in range(3))
    v = tuple(c[k] - a[k] for k in range(3))
    n = cross(u, v)
    length = math.hypot(*n)
    if length == 0:
        return 0.0
    centroid = tuple((a[k] + b[k] + c[k]) / 3 for k in range(3))
    outward = 1 if sum(n[k] * centroid[k] for k in range(3)) > 0 else -1
    return outward * n[2] / length


if __name__ == "__main__":
    main()
