"""Build four walk-up cottages in the same native scene as the tour stops."""

import math

import bpy

from wayside_layout import COTTAGES, on_plot

CREAM, WOOD, STONE = "eadbb9", "655041", "998c77"
DARK, MORTAR, BRICK = "514a43", "b9ae94", "a77360"
GLASS = "glass"


def prepare_joinery_lighting(collection):
    """Bake each corner a short distance inside its face, clear of intersecting trim."""
    for obj in list(collection.objects):
        if not obj.name.startswith("Wayside joinery"):
            continue
        original = obj.data
        # Leave the complete roof and trim in the light transport scene. The
        # inset faces only gather light; the authored mesh still casts shadows.
        occluder = bpy.data.objects.new("Wayside bake occluder", original)
        bpy.context.scene.collection.objects.link(occluder)
        points, faces, offsets = [], [], []
        for face in original.polygons:
            corners = [original.vertices[i].co for i in face.vertices]
            first = len(points)
            for i, corner in enumerate(corners):
                offset = face.normal * 0.003
                # Move independently along both incident edges. Long roof
                # courses and narrow window frames need different margins.
                for neighbor in (corners[i - 1], corners[(i + 1) % len(corners)]):
                    edge = neighbor - corner
                    if edge.length:
                        offset += edge * min(0.15, 0.085 / edge.length)
                points.append(corner + offset)
                offsets.append(-offset)
            faces.append(range(first, len(points)))
        mesh = bpy.data.meshes.new(original.name + " lighting samples")
        mesh.from_pydata(points, [], faces)
        mesh.update()
        for material in original.materials:
            mesh.materials.append(material)
        mesh.polygons.foreach_set("material_index", [p.material_index for p in original.polygons])
        mesh.normals_split_custom_set([p.normal for p in original.polygons for _ in p.loop_indices])
        restore = mesh.attributes.new("WaysideBakeOffset", "FLOAT_VECTOR", "POINT")
        restore.data.foreach_set("vector", [c for offset in offsets for c in offset])
        obj.data = mesh


def restore_joinery_geometry(mesh):
    """Put the vertex colors on the original, closed geometry for export."""
    import numpy as np

    restore = mesh.attributes.get("WaysideBakeOffset")
    if restore is None:
        return
    offsets = np.empty(len(mesh.vertices) * 3, dtype=np.float32)
    positions = np.empty_like(offsets)
    restore.data.foreach_get("vector", offsets)
    mesh.vertices.foreach_get("co", positions)
    mesh.vertices.foreach_set("co", positions + offsets)
    mesh.attributes.remove(restore)
    mesh.update()


class Surface:
    def __init__(self):
        self.vertices, self.faces, self.colors = [], [], []
        self.indices = {}

    def face(self, points, color):
        indices = []
        for p in points:
            key = tuple(round(v, 6) for v in p)
            if key not in self.indices:
                self.indices[key] = len(self.vertices)
                self.vertices.append(key)
            indices.append(self.indices[key])
        self.faces.append(indices)
        self.colors.append(color)


class Frame:
    """Local X across the elevation, Y up, positive Z towards the street."""

    def __init__(self, surface, x=0, z=0, yaw=0):
        self.surface, self.x, self.z = surface, x, z
        self.c, self.s = math.cos(yaw), math.sin(yaw)

    def face(self, points, color):
        self.surface.face(
            [
                (self.x + self.c * x + self.s * z, y, self.z - self.s * x + self.c * z)
                for x, y, z in points
            ],
            color,
        )

    def prism(self, profile, back, front, color):
        self.face([(x, y, front) for x, y in profile], color)
        self.face([(x, y, back) for x, y in reversed(profile)], color)
        for a, b in zip(profile, profile[1:] + profile[:1]):
            self.face([(*a, back), (*b, back), (*b, front), (*a, front)], color)

    def box(self, x, y, z, w, h, d, color):
        self.prism(
            [
                (x - w / 2, y - h / 2),
                (x + w / 2, y - h / 2),
                (x + w / 2, y + h / 2),
                (x - w / 2, y + h / 2),
            ],
            z - d / 2,
            z + d / 2,
            color,
        )

    def beam(self, a, b, z, width, depth, color):
        dx, dy = b[0] - a[0], b[1] - a[1]
        k = width / (2 * math.hypot(dx, dy))
        u, v = -dy * k, dx * k
        self.prism(
            [(a[0] - u, a[1] - v), (b[0] - u, b[1] - v), (b[0] + u, b[1] + v), (a[0] + u, a[1] + v)],
            z - depth / 2,
            z + depth / 2,
            color,
        )


def window(frame, pane, x, y, w, h, paint, shutters):
    frame.box(x, y + h / 2 + 0.055, 0.025, w + 0.25, 0.13, 0.18, CREAM)
    frame.box(x, y - h / 2 - 0.06, 0.085, w + 0.36, 0.15, 0.32, MORTAR)
    for sign in (-1, 1):
        frame.box(x + sign * (w / 2 + 0.045), y, 0.025, 0.12, h + 0.08, 0.17, CREAM)
        if shutters:
            sx = x + sign * (w / 2 + 0.32)
            frame.box(sx, y, 0.055, 0.37, h + 0.04, 0.11, paint)
            for j in range(5):
                frame.box(sx, y - h * 0.33 + j * h * 0.165, 0.12, 0.28, 0.038, 0.038, WOOD)
    pane.box(x, y, -0.115, w, h, 0.028, GLASS)
    frame.box(x, y, -0.052, 0.052, h, 0.052, CREAM)
    frame.box(x, y + h * 0.10, -0.052, w, 0.052, 0.052, CREAM)
    # Drapes sit inside the window openings and give clear glass a readable interior.
    for sign in (-1, 1):
        cx = x + sign * w * 0.37
        frame.prism(
            [
                (cx - 0.08, y - h * 0.44),
                (cx + 0.08, y - h * 0.44),
                (cx + 0.14, y + h * 0.44),
                (cx - 0.14, y + h * 0.44),
            ],
            -0.265,
            -0.25,
            "d9c8a8",
        )


def elevation(walls, detail, panes, width, height, windows, color, trim, shutters):
    # Leave an actual hole in both faces of the plaster at every window.
    cuts = [(x - w / 2, x + w / 2, y - h / 2, y + h / 2) for x, y, w, h in windows]
    xs = sorted({-width / 2, width / 2, *[xx for a, b, _, _ in cuts for xx in (a, b)]})
    ys = sorted({0.18, height, *[yy for _, _, c, d in cuts for yy in (c, d)]})
    for a, b in zip(xs, xs[1:]):
        for c, d in zip(ys, ys[1:]):
            if any(
                left < (a + b) / 2 < right and low < (c + d) / 2 < hi for left, right, low, hi in cuts
            ):
                continue
            rect = [(a, c), (b, c), (b, d), (a, d)]
            walls.face([(*p, 0) for p in rect], color)
            walls.face([(*p, -0.14) for p in reversed(rect)], color)
    for (x, y, w, h), (left, right, low, hi) in zip(windows, cuts):
        rect = [(left, low), (right, low), (right, hi), (left, hi)]
        for a, b in zip(rect, rect[1:] + rect[:1]):
            walls.face([(*a, 0), (*b, 0), (*b, -0.14), (*a, -0.14)], color)
        window(detail, panes, x, y, w, h, trim, shutters)


def door(frame, paint, arched):
    radius = 0.56

    def silhouette(r):
        top = 1.91
        crown = [
            (r * math.cos(i * math.pi / 14), top + r * math.sin(i * math.pi / 14)) for i in range(15)
        ]
        return [(-r, 0.22), (r, 0.22), *(crown if arched else [(r, 2.42), (-r, 2.42)])]

    frame.prism(silhouette(radius + 0.13), 0.005, 0.12, CREAM)
    frame.prism(silhouette(radius), 0.125, 0.175, paint)
    for j in range(1, 6):
        x = -radius + 2 * radius * j / 6
        top = 1.90 + math.sqrt(radius**2 - x**2) if arched else 2.41
        frame.box(x, (0.23 + top) / 2, 0.177, 0.014, top - 0.23, 0.012, WOOD)
    frame.box(0.37, 1.28, 0.203, 0.09, 0.16, 0.055, "b89957")
    frame.box(0, 0.175, 0.23, 1.43, 0.17, 0.72, MORTAR)


def roof_section(frame, profile, half_length, color, fascia):
    # Broad roof planes are built in courses so their baked shadows stay local.
    for (x1, y1), (x2, y2) in zip(profile, profile[1:]):
        n = max(1, round(math.hypot(x2 - x1, y2 - y1) / 0.38))
        m = max(1, math.ceil(half_length * 2 / 0.8))
        for i in range(n):
            a = (x1 + (x2 - x1) * i / n, y1 + (y2 - y1) * i / n)
            b = (x1 + (x2 - x1) * (i + 1) / n, y1 + (y2 - y1) * (i + 1) / n)
            for j in range(m):
                back = -half_length + 2 * half_length * j / m
                front = -half_length + 2 * half_length * (j + 1) / m
                frame.face([(*a, front), (*b, front), (*b, back), (*a, back)], color)
            if i and fascia:
                frame.box(a[0], a[1] + 0.007, 0, 0.036, 0.026, 2 * half_length, fascia)
        for z in (-half_length, half_length):
            frame.beam((x1, y1 - 0.055), (x2, y2 - 0.055), z, 0.17, 0.14, fascia or WOOD)
    for x, y in (profile[0], profile[-1]):
        frame.box(x, y - 0.065, 0, 0.13, 0.16, half_length * 2 + 0.12, fascia or WOOD)


def make_roof(plot, wall, details):
    kind, w, d, h, r = (plot[k] for k in ("kind", "width", "depth", "eave", "rise"))
    roof, color = plot["roof"], plot["plaster"]
    detail = Frame(details)
    across = kind in ("thatched", "hipped")
    rw, length = (d / 2 + 0.42, w / 2 + 0.42) if across else (w / 2 + 0.38, d / 2 + 0.38)
    roof_frame = Frame(details, yaw=math.pi / 2 if across else 0)
    if kind == "hipped":
        ridge = w * 0.29
        a, b = (-w / 2 - 0.42, h, -rw), (w / 2 + 0.42, h, -rw)
        c, e = (w / 2 + 0.42, h, rw), (-w / 2 - 0.42, h, rw)
        ridge_left, ridge_right = (-ridge, h + r, 0), (ridge, h + r, 0)
        for face in [
            [a, ridge_left, ridge_right, b],
            [b, ridge_right, c],
            [c, ridge_right, ridge_left, e],
            [e, ridge_left, a],
        ]:
            detail.face(face, roof)
        detail.box(0, h - 0.03, 0, w + 0.80, 0.14, d + 0.80, WOOD)
        detail.box(0, h + r, 0, ridge * 2 + 0.1, 0.12, 0.18, roof)
        return
    if kind == "gambrel":
        profile = [(-rw, h), (-0.69 * rw, h + 0.68 * r), (0, h + r), (0.69 * rw, h + 0.68 * r), (rw, h)]
    elif kind == "thatched":
        profile = [
            (-rw, h - 0.06),
            (-rw * 0.80, h + 0.14 * r),
            (-rw * 0.30, h + 0.76 * r),
            (0, h + r),
            (rw * 0.30, h + 0.76 * r),
            (rw * 0.80, h + 0.14 * r),
            (rw, h - 0.06),
        ]
    else:
        profile = [(-rw, h), (0, h + r), (rw, h)]
    wall_frame = Frame(wall, yaw=math.pi / 2 if across else 0)
    half_body = w / 2 if across else d / 2
    # A closed gable beneath the projecting verges.
    end_profile = [(x * (rw - 0.38) / rw, y) for x, y in reversed(profile)]
    wall_frame.prism(end_profile, -half_body, half_body, color)
    roof_section(
        roof_frame,
        profile,
        length,
        roof,
        {"timbered": "8c5445", "gambrel": "686c7b", "thatched": "9f794c"}[kind],
    )
    roof_frame.box(0, h + r + 0.025, 0, 0.17, 0.12, 2 * length + 0.15, roof)
    if kind == "timbered":
        front = Frame(details, z=d / 2)
        front.box(0, h, 0.08, w, 0.18, 0.16, WOOD)
        front.box(0, h + r * 0.45, 0.08, 0.15, r * 0.9, 0.16, WOOD)
        for sign in (-1, 1):
            front.beam(
                (sign * w * 0.42, h + 0.08), (sign * 0.12, h + r * 0.76), 0.072, 0.13, 0.15, WOOD
            )
            front.beam((sign * w * 0.38, 0.40), (sign * w * 0.26, 1.04), 0.065, 0.13, 0.13, WOOD)
    if kind == "gambrel":
        # A projecting loft casement breaks up the tall street-facing gable.
        z = d / 2
        detail.box(0, h + 0.78, z + 0.09, 1.10, 1.04, 0.21, CREAM)
        detail.box(0, h + 0.78, z + 0.20, 0.88, 0.82, 0.06, "50676a")
        detail.box(0, h + 0.78, z + 0.24, 0.066, 0.82, 0.05, CREAM)
        detail.box(0, h + 0.78, z + 0.24, 0.88, 0.055, 0.05, CREAM)
        detail.box(0, h + 0.23, z + 0.16, 1.29, 0.14, 0.42, MORTAR)


def build_house(plot):
    wall, details, glass = Surface(), Surface(), Surface()
    w, d, h = plot["width"], plot["depth"], plot["eave"]
    kind, paint = plot["kind"], plot["paint"]
    root = Frame(details)
    root.box(0, -0.14, 0, w + 0.18, 0.67, d + 0.18, STONE)
    root.box(0, 0.18, 0, w + 0.27, 0.12, d + 0.27, MORTAR)
    front = Frame(details, z=d / 2)
    window_count = 0
    for x, z, yaw, width, windows in [
        (0, d / 2, 0, w, [(s * w * 0.30, 1.82, 1.03, 1.26) for s in (-1, 1)]),
        (w / 2, 0, math.pi / 2, d, [(0, 1.76, 1.14, 1.24)]),
        (0, -d / 2, math.pi, w, [(s * w * 0.23, 1.73, 0.92, 1.12) for s in (-1, 1)]),
        (-w / 2, 0, -math.pi / 2, d, [(0, 1.76, 1.14, 1.24)]),
    ]:
        window_count += len(windows)
        elevation(
            Frame(wall, x, z, yaw),
            Frame(details, x, z, yaw),
            Frame(glass, x, z, yaw),
            width,
            h,
            windows,
            plot["plaster"],
            paint,
            kind in ("hipped", "gambrel"),
        )
    door(front, paint, kind in ("hipped", "thatched"))
    make_roof(plot, wall, details)
    # Masonry chimney with offset bed joints and two terracotta pots.
    cx, cz = (w * 0.31, -d * 0.22)
    top = h + plot["rise"] + 0.56
    root.box(cx, (top + h - 0.35) / 2, cz, 0.69, top - h + 0.35, 0.66, BRICK)
    for i in range(6):
        y = top - 0.13 - i * 0.23
        root.box(cx, y, cz, 0.706, 0.029, 0.675, "c29c81")
        root.box(cx + (-0.11 if i % 2 else 0.14), y + 0.12, cz + 0.332, 0.027, 0.18, 0.02, "c29c81")
    root.box(cx, top, cz, 0.86, 0.16, 0.82, MORTAR)
    for s in (-1, 1):
        root.box(cx + s * 0.184, top + 0.19, cz, 0.24, 0.31, 0.30, "9e654d")
        root.box(cx + s * 0.184, top + 0.35, cz, 0.30, 0.10, 0.35, "a77455")
    if kind == "hipped":
        # A shallow porch supported by two square posts.
        for x in (-0.91, 0.91):
            front.box(x, 1.32, 1.03, 0.13, 2.40, 0.13, paint)
        front.prism([(-1.13, 2.62), (1.13, 2.62), (0, 3.08)], -0.04, 1.25, plot["roof"])
        front.box(0, 0.17, 0.61, 2.15, 0.16, 1.35, MORTAR)
    if kind == "timbered":
        for x in (-w / 2 + 0.09, -w * 0.16, w * 0.16, w / 2 - 0.09):
            front.box(x, (h + 0.21) / 2, 0.058, 0.15, h - 0.2, 0.13, WOOD)
    if kind == "thatched":
        for x in (-w * 0.30, w * 0.30):
            front.box(x, 1.09, 0.25, 1.35, 0.24, 0.36, "9a6a4f")
            for i in range(7):
                px = x + (i - 3) * 0.155
                front.box(px, 1.25 + 0.025 * (i % 3), 0.25, 0.16, 0.14, 0.14, "6b814c")
                front.box(
                    px + 0.018,
                    1.34 + 0.025 * (i % 3),
                    0.25,
                    0.085,
                    0.075,
                    0.095,
                    ["d9b252", "d98c79", "d1cc9b"][i % 3],
                )
    return (wall, details, glass), window_count


def install(collection, layout, bounds=None):
    from common import xyz

    materials = {}

    def material(color):
        if color == GLASS and color not in materials:
            from glass import clear_material

            mat = next(
                (m for m in bpy.data.materials if m.name.startswith("Village clear cottage glazing")),
                None,
            )
            if mat is None:
                mat = bpy.data.materials.new("Village clear cottage glazing")
                mat.use_nodes = True
                clear_material(mat)
            materials[color] = mat
        if color not in materials:
            mat = bpy.data.materials.new("Wayside " + color)
            rgb = [int(color[i : i + 2], 16) / 255 for i in (0, 2, 4)]
            rgb = [v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4 for v in rgb]
            mat.diffuse_color = (*rgb, 1)
            mat.use_nodes = True
            shader = mat.node_tree.nodes.get("Principled BSDF")
            shader.inputs["Base Color"].default_value = (*rgb, 1)
            shader.inputs["Roughness"].default_value = 1
            shader.inputs["Specular IOR Level"].default_value = 0.04
            materials[color] = mat
        return materials[color]

    for plot in COTTAGES:
        cx, cz = plot["center"]
        if bounds and not (bounds[0] - 7 < cx < bounds[1] + 7 and bounds[2] - 7 < cz < bounds[3] + 7):
            continue
        base = 0.18 - cz * 0.014 + math.sin(cx * 0.045) * math.cos(cz * 0.035) * 0.3
        for stop in layout["stops"]:
            distance = math.hypot(cx - stop["position"][0], cz - stop["position"][2])
            t = min(1, max(0, (distance - stop["width"] * 0.76) / 7))
            base += (stop["position"][1] - 0.15 - base) * (1 - t * t * (3 - 2 * t))
        surfaces, window_count = build_house(plot)
        collection["clear_window_openings"] = collection.get("clear_window_openings", 0) + window_count
        for label, surface in zip(
            ("Colored cottage walls", "Wayside joinery", "Wayside glazing"), surfaces
        ):
            points = []
            for x, y, z in surface.vertices:
                px, pz = on_plot(plot, x, z)
                points.append(xyz((px, base + y, pz)))
            name = label + " – " + plot["id"]
            mesh = bpy.data.meshes.new(name)
            mesh.from_pydata(points, [], surface.faces)
            mesh.update()
            mesh.normals_split_custom_set([p.normal for p in mesh.polygons for _ in p.loop_indices])
            palette = list(dict.fromkeys(surface.colors))
            for color in palette:
                mesh.materials.append(material(color))
            lookup = {color: i for i, color in enumerate(palette)}
            mesh.polygons.foreach_set("material_index", [lookup[c] for c in surface.colors])
            obj = bpy.data.objects.new(name, mesh)
            collection.objects.link(obj)
            if surface.colors[0] == GLASS:
                obj.visible_shadow = False
