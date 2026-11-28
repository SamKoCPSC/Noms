"""Prepare the cut cottage shells for the outdoor vertex-lighting bake."""
import math
from itertools import count

import bmesh
import numpy as np
from mathutils import Vector, kdtree


def prepare_cottage_walls(collection):
    walls = [obj for obj in collection.objects if obj.name.startswith('Colored cottage walls')]
    panel_numbers = count(1)
    centers = {}
    for shell, obj in enumerate(walls, 1):
        mesh = obj.data
        center = obj.matrix_world @ (sum((Vector(p) for p in obj.bound_box), Vector()) / 8)
        centers[shell] = center.xy
        # The houses are rotated in the village. Cut along each house's own
        # axes so the bake gets samples across the facade and close to its trim.
        up = Vector((0, 0, 1))
        side = max((p for p in mesh.polygons if abs(p.normal.z) < 1e-5),
                   key=lambda p: p.area).normal.copy()
        axes = (side, up.cross(side).normalized(), up)
        bm = bmesh.new()
        bm.from_mesh(mesh)
        # Tessellate concave window cutouts before slicing them.
        bmesh.ops.triangulate(bm, faces=list(bm.faces), ngon_method='BEAUTY')
        for axis in axes:
            projections = [axis.dot(v.co) for v in bm.verts]
            low, high = min(projections), max(projections)
            # Half-metre spacing keeps shadows local to the roof, sill or porch.
            cuts = math.ceil((high - low) / .5)
            for i in range(1, cuts):
                offset = low + (high - low) * i / cuts
                bmesh.ops.bisect_plane(bm, geom=[*bm.verts, *bm.edges, *bm.faces],
                                       plane_co=axis * offset, plane_no=axis, dist=1e-5)
        bm.to_mesh(mesh)
        bm.free()
        mesh.update()
        shell_ids = mesh.attributes.new('CottageShell', 'INT', 'FACE')
        shell_ids.data.foreach_set('value', np.full(len(mesh.polygons), shell, dtype=np.int32))
        # Window reveals and inside faces meet the facade at right angles.
        # They need independent corner normals. Keep interpolation on the bevels.
        normals = [None] * len(mesh.loops)
        panels = {}
        tags = mesh.attributes.new('CottagePanel', 'INT', 'FACE')
        for p in mesh.polygons:
            axis = max(axes, key=lambda v: abs(p.normal.dot(v)))
            alignment = p.normal.dot(axis)
            planar = abs(alignment) > .99999
            if planar:
                # Wall faces, the inner lining and the returns around a window
                # each have their own plane. Never filter through a wall or corner.
                key = (axes.index(axis), alignment > 0, round(axis.dot(p.center), 2))
                if key not in panels:
                    panels[key] = next(panel_numbers)
                tags.data[p.index].value = panels[key]
            for i in p.loop_indices:
                normals[i] = p.normal if planar else mesh.vertices[mesh.loops[i].vertex_index].normal
        mesh.normals_split_custom_set(normals)
    return centers


def smooth_panel_lighting(mesh, colors):
    """Denoise corner samples on each planar surface before exporting the bake."""
    tags = mesh.attributes['CottagePanel']
    ids = np.empty(len(mesh.polygons), dtype=np.int32)
    tags.data.foreach_get('value', ids)
    for panel in np.unique(ids[ids > 0]):
        faces = [mesh.polygons[int(i)] for i in np.flatnonzero(ids == panel)]
        corners = np.array([i for face in faces for i in face.loop_indices])
        # Large faces represent more surface than the slivers around the cutouts.
        weights = np.repeat([face.area / face.loop_total for face in faces],
                            [face.loop_total for face in faces])
        vertex_ids, first, inverse = np.unique([mesh.loops[int(i)].vertex_index for i in corners],
                                               return_index=True, return_inverse=True)
        mass = np.bincount(inverse, weights)
        light = np.column_stack([np.bincount(inverse, weights * colors[corners, c]) for c in range(3)])
        points = [mesh.vertices[int(i)].co.copy() for i in vertex_ids]
        tree = kdtree.KDTree(len(points))
        for i, point in enumerate(points):
            tree.insert(point, i)
        tree.balance()
        filtered = colors[corners[first], :3].copy()
        for i, point in enumerate(points):
            # Average roughly one grid interval in each direction; taper the
            # influence with distance so real contact shadows still have shape.
            nearby = tree.find_range(point, 1.5)
            indices = [j for _, j, _ in nearby]
            kernel = np.exp(-.5 * (np.array([dist for _, _, dist in nearby]) / .6) ** 2)
            total = kernel @ mass[indices]
            if total > 0:
                filtered[i] = (kernel @ light[indices]) / total
        colors[corners, :3] = filtered[inverse]
    mesh.attributes.remove(tags)
