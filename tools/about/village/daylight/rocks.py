"""Sculpt the tour's outdoor stone at the boulder sites in the native landscape."""

import hashlib
import math
import random
import struct

import bpy

# Linear diffuse colors for pale, slightly blue-gray limestone. Recesses expose
# a cooler seam, and fresh chipped edges are a little lighter than the outer rind.
TONES = (
    (0.39, 0.445, 0.46),
    (0.43, 0.475, 0.48),
    (0.345, 0.398, 0.42),
    (0.315, 0.368, 0.392),
    (0.48, 0.525, 0.53),
)


def stone(verts, faces, tones, rng, center, half_x, half_y, height, yaw, form):
    """Sweep an asymmetric, clipped footprint through interrupted bedding planes."""
    # Long, nearly planar faces between the broken corners of the stone.
    outline = [
        (-0.69, -0.91),
        (0.13, -0.99),
        (0.77, -0.91),
        (1, -0.56),
        (0.94, 0.29),
        (0.73, 0.84),
        (0.11, 0.96),
        (-0.64, 0.89),
        (-1, 0.51),
        (-0.94, -0.37),
    ]
    outline = [(x + rng.uniform(-0.055, 0.055), y + rng.uniform(-0.065, 0.065)) for x, y in outline]
    count = len(outline)
    cos, sin = math.cos(yaw), math.sin(yaw)
    lean_x, lean_y = rng.uniform(-0.19, 0.19), rng.uniform(-0.12, 0.12)
    pitch_x, pitch_y = rng.uniform(-0.10, 0.10), rng.uniform(-0.15, 0.15)
    crown_radius = {"spire": 0.43, "block": 0.81, "slab": 0.89, "chip": 0.71}[form]
    fault = rng.uniform(0, math.tau)
    wear = [rng.uniform(0.66, 1.18) for _ in outline]
    previous = []

    def join_ring(elevations, radii, tint):
        nonlocal previous
        ring = []
        for i, (x, y) in enumerate(outline):
            z = elevations[i]
            px = (x * radii[i] + lean_x * z) * half_x
            py = (y * radii[i] + lean_y * z) * half_y
            ring.append(len(verts))
            verts.append(
                (center[0] + px * cos - py * sin, center[1] + px * sin + py * cos, center[2] + height * z)
            )
        if previous:
            for i in range(count):
                j = (i + 1) % count
                # The diagonal shares the stone's broad planes, with an occasional split face.
                faces.extend([(previous[i], previous[j], ring[j]), (previous[i], ring[j], ring[i])])
                tones.extend([tint[i], tint[i]])
        else:
            faces.append(tuple(reversed(ring)))
            tones.append(2)
        previous = ring

    def ring(z, radius, tint):
        join_ring([z] * count, [radius] * count, [tint] * count)

    body = [rng.choice((0, 0, 0, 1, 2)) for _ in outline]
    ring(-0.12, 0.77, 2)
    ring(0.09, 1, 0)
    # A broad, pitched top with a clipped rim and an off-center ridge.
    ridge = rng.uniform(-0.30, 0.30)

    def top(x, y):
        return 1 + pitch_x * x + pitch_y * y - 0.17 * abs(x - ridge)

    rim = [top(x, y) for x, y in outline]
    shoulder = [z - 0.15 * w for z, w in zip(rim, wear)]

    def bed(fractions, radii, tints):
        # Keep every ledge below the chipped rim even on the low side of a sloping cap.
        join_ring([0.09 + t * (edge - 0.09) for t, edge in zip(fractions, shoulder)], radii, tints)

    levels = {"spire": (0.32, 0.69), "block": (0.33, 0.65), "slab": (0.24, 0.47), "chip": ()}[form]
    for index, level in enumerate(levels):
        # A fracture opens on one side and pinches out on the opposite wall.
        az = fault + 1.78 * index
        gaps = [
            (0.026 + 0.155 * max(0, x * math.cos(az) + y * math.sin(az))) * w
            for (x, y), w in zip(outline, wear)
        ]
        line = [
            level + 0.075 * (x * math.sin(az) - y * math.cos(az)) + rng.uniform(-0.018, 0.018)
            for x, y in outline
        ]
        radius = 1 - (1 - crown_radius) * level
        bed(line, [radius + g * 0.32 for g in gaps], body)
        bed([v + 0.028 for v in line], [radius - g for g in gaps], [4 if g > 0.082 else 0 for g in gaps])
        bed(
            [v + 0.054 for v in line],
            [radius - g * 0.83 for g in gaps],
            [3 if g > 0.082 else 0 for g in gaps],
        )

    join_ring(shoulder, [crown_radius + 0.085] * count, body)
    join_ring(rim, [crown_radius - 0.065 * w for w in wear], [rng.choice((0, 1, 4)) for _ in outline])
    # Split the top polygon along the ridge while sharing vertices at the cut.
    xs = [xy[0] for xy in outline]
    halves = [[], []]
    bevel_start = len(faces) - 2 * count
    for i in range(count):
        j = (i + 1) % count
        side = int(xs[i] > ridge)
        halves[side].append(previous[i])
        if side != int(xs[j] > ridge):
            t = (ridge - xs[i]) / (xs[j] - xs[i])
            a, b = verts[previous[i]], verts[previous[j]]
            vertex = [a[k] + (b[k] - a[k]) * t for k in range(3)]
            vertex[2] += height * 0.17 * ((1 - t) * abs(xs[i] - ridge) + t * abs(xs[j] - ridge))
            for half in halves:
                half.append(len(verts))
            verts.append(tuple(vertex))
            # Split the adjoining bevel triangle too, keeping the cap watertight.
            bevel = bevel_start + i * 2 + 1
            low, high_j, high_i = faces[bevel]
            faces[bevel] = (low, high_j, len(verts) - 1)
            faces.append((low, len(verts) - 1, high_i))
            tones.append(tones[bevel])
    faces.extend(map(tuple, halves))
    tones.extend((1, 1))


def install_rocks():
    terrain = bpy.data.collections["World terrain — blockout"]
    original = sorted(
        (o for o in terrain.objects if o.name.startswith("Weathered landscape rock")), key=lambda o: o.name
    )
    companions = [o for o in terrain.objects if o.name.startswith("Small companion stone")]
    verts, faces, tones = [], [], []
    formations = 0
    for batch in original:
        points = [batch.matrix_world @ v.co for v in batch.data.vertices]
        assert len(points) % 240 == 0
        for start in range(0, len(points), 240):
            # Every original detail-one icosahedron has 80 independently indexed triangles.
            group = points[start : start + 240]
            lo = [min(p[k] for p in group) for k in range(3)]
            hi = [max(p[k] for p in group) for k in range(3)]
            x, y, z = [(a + b) / 2 for a, b in zip(lo, hi)]
            width = max(hi[0] - lo[0], hi[1] - lo[1])
            floor = z - (hi[2] - lo[2]) * 0.24
            rng = random.Random(4319 + formations * 971)
            yaw = rng.uniform(0, math.tau)
            # Rotate the styles through the village, including occasional paired pinnacles.
            form = ("block", "slab", "block", "spire", "slab", "block", "spire")[formations % 7]
            height = width * {"block": 0.73, "slab": 0.35, "spire": 1.24}[form] * rng.uniform(0.90, 1.08)

            def piece(dx, dy, rx, ry, h, angle, shape):
                cx = x + width * (dx * math.cos(yaw) - dy * math.sin(yaw))
                cy = y + width * (dx * math.sin(yaw) + dy * math.cos(yaw))
                stone(
                    verts, faces, tones, rng, (cx, cy, floor), width * rx, width * ry, h, yaw + angle, shape
                )

            if form == "spire":
                piece(-0.10, 0, 0.32, 0.28, height, 0, "spire")
                piece(0.31, 0.05, 0.15, 0.15, height * 0.68, 0.38, "spire")
                piece(-0.30, -0.19, 0.20, 0.21, height * 0.36, -0.21, "block")
            elif form == "block":
                piece(-0.09, 0.02, 0.46, 0.39, height, 0, "block")
                piece(0.31, -0.11, 0.25, 0.24, height * 0.52, 0.25, "slab")
            else:
                piece(-0.03, 0, 0.52, 0.37, height, 0, "slab")
                piece(-0.30, 0.20, 0.26, 0.21, height * 0.81, -0.28, "block")
            for i in range(rng.randint(2, 4)):
                a = yaw + i * 2.399
                size = rng.uniform(0.075, 0.14)
                piece(
                    math.cos(a) * 0.53,
                    math.sin(a) * 0.42,
                    size,
                    size * 0.76,
                    width * size * rng.uniform(0.85, 1.35),
                    a,
                    "chip",
                )
            formations += 1
    assert formations == 81 and len(companions) == 1, "Unexpected village rock placements"
    for obj in original + companions:
        bpy.data.objects.remove(obj, do_unlink=True)

    mesh = bpy.data.meshes.new("Sculpted landscape stone")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    for rgb in TONES:
        mat = bpy.data.materials.new("Cool limestone")
        mat.use_nodes = True
        shader = mat.node_tree.nodes.get("Principled BSDF")
        shader.inputs["Base Color"].default_value = (*rgb, 1)
        shader.inputs["Roughness"].default_value = 0.91
        mesh.materials.append(mat)
    mesh.polygons.foreach_set("material_index", tones)
    obj = bpy.data.objects.new(mesh.name, mesh)
    terrain.objects.link(obj)
    # Both daylight passes must use identical shapes, placements and surface tones.
    digest = hashlib.sha256()
    for point in (*TONES, *verts):
        digest.update(struct.pack("<3f", *point))
    for face, tint in zip(faces, tones):
        digest.update(struct.pack("<BB", len(face), tint))
        digest.update(struct.pack(f"<{len(face)}I", *face))
    print("LANDSCAPE_ROCKS_READY", formations, len(faces), digest.hexdigest(), flush=True)
    return digest.hexdigest()
