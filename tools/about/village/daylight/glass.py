"""Clear glass derivatives. Archived room masters and blockout stay immutable.

Only explicitly identified window substitutes and authored transmitting glass are
changed; screens, glazed food, liquids and stained-glass colors keep their roles.
"""
import hashlib
from pathlib import Path


def glass_revision():
    return hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


WINDOW_MATERIALS = {
    'kitchen': {'Window · misted daylight'},
    'library': {'Matte pale blue simulated window', 'Matte blue simulated window'},
    'corporate-office': {'sky'},
    'brick-kitchen': {'window'},
    'social-kitchen': {'windowBlue'},
    'riverside-campsite': {'windowBlue'},
    'garden': {'glassSky'},
}
# Cut openings only in identified architectural backing, never window trim.
BACKING_PREFIXES = {
    'kitchen': ('Walls · continuous closed L shel',),
    'riverside-campsite': ('Cabin enclosed body', 'Front cedar clapboard',
                           'Right cedar clapboard', 'Walnut cabin door'),
    'garden': ('House solid facade', 'Horizontal clapboard lower lip',
               'Back door walnut leaf', 'Window shadow recess'),
}
BACKDROPS = ('Window exterior pale sky', 'Window distant mountain', 'Window nearer hill')


def principled(mat):
    return next((n for n in mat.node_tree.nodes if n.type == 'BSDF_PRINCIPLED'), None) if mat and mat.use_nodes else None


def label(name):
    return name.split('::')[1] if '::' in name else name


def clear_material(mat, tinted=False):
    p = principled(mat)
    color = tuple(p.inputs['Base Color'].default_value) if tinted else (1, 1, 1, 1)
    # Fresh nodes remove mist/bump/roughness maps and any artificial emission.
    mat.node_tree.nodes.clear()
    p = mat.node_tree.nodes.new('ShaderNodeBsdfPrincipled')
    output = mat.node_tree.nodes.new('ShaderNodeOutputMaterial')
    mat.node_tree.links.new(p.outputs['BSDF'], output.inputs['Surface'])
    for key, value in {'Base Color': color, 'Transmission Weight': 1.,
                       'Roughness': 0., 'Metallic': 0., 'IOR': 1.45,
                       'Alpha': 1., 'Coat Weight': 0.}.items():
        p.inputs[key].default_value = value
    mat.diffuse_color = color
    mat['village_clear_glass'] = True
    mat['intentional_stained_glass'] = tinted


def bounds(obj):
    from mathutils import Vector
    points = [obj.matrix_world @ Vector(v) for v in obj.bound_box]
    return [Vector([f(p[i] for p in points) for i in range(3)]) for f in (min, max)]


def subtract_box(obj, lo, hi, collection, rotation=None):
    """Apply an exact closed cutter in source coordinates without operator context."""
    import bpy
    from mathutils import Matrix
    mesh = bpy.data.meshes.new('Temporary window opening')
    verts = [(x, y, z) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
    faces = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1),
             (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    mesh.from_pydata(verts, [], faces); mesh.update()
    cutter = bpy.data.objects.new('Temporary window opening', mesh)
    collection.objects.link(cutter)
    cutter.matrix_world = Matrix.Translation((lo + hi) / 2) @ (rotation or Matrix.Identity(4)) @ Matrix.Diagonal((*((hi-lo)/2), 1))
    # Evaluate cutters in a tiny scene. Rebuilding the dependency graph of the
    # entire 24,000-object world for every window is needlessly expensive.
    previous_scene = bpy.context.window.scene
    operation_scene = bpy.data.scenes.new('Temporary window boolean scene')
    operation_scene.collection.objects.link(obj)
    operation_scene.collection.objects.link(cutter)
    bpy.context.window.scene = operation_scene
    modifier = obj.modifiers.new('Real window opening', 'BOOLEAN')
    modifier.operation = 'DIFFERENCE'; modifier.solver = 'EXACT'; modifier.object = cutter
    bpy.context.view_layer.update()
    graph = bpy.context.evaluated_depsgraph_get()
    evaluated = obj.evaluated_get(graph)
    result = bpy.data.meshes.new_from_object(evaluated, preserve_all_data_layers=True, depsgraph=graph)
    obj.modifiers.clear(); obj.data = result
    bpy.context.window.scene = previous_scene
    bpy.data.scenes.remove(operation_scene)
    bpy.data.objects.remove(cutter, do_unlink=True); bpy.data.meshes.remove(mesh)


def apply_room_glass():
    import bpy
    report = {'revision': glass_revision(), 'rooms': {}}
    for col in list(bpy.data.collections):
        if not col.name.startswith('WORLD_SOURCE_'): continue
        key = col.name.removeprefix('WORLD_SOURCE_')
        objects = list(col.all_objects)
        changed, panes, hidden, cuts = {}, [], [], []
        for obj in objects:
            if obj.type != 'MESH': continue
            for mat in obj.data.materials:
                p = principled(mat)
                if not p: continue
                name = label(mat.name)
                fake = name in WINDOW_MATERIALS.get(key, set())
                glass = bool(mat.get('village_clear_glass')) or (p.inputs['Transmission Weight'].default_value > .1 and any(w in name.lower() for w in ('glass', 'glaz')))
                if not (fake or glass): continue
                if mat.name not in changed:
                    clear_material(mat, tinted=key == 'library' and name in {'glassBlue', 'glassTeal', 'glassPurple', 'glassLime'})
                    changed[mat.name] = dict(transmission=1, roughness=0, ior=1.45, tinted=bool(mat['intentional_stained_glass']))
                if fake: panes.append(obj)
            # Thin clear glass should admit daylight in the diffuse bake without
            # requiring expensive refractive caustics. Camera/reflection rays see
            # the actual Principled glass; only shadow rays pass through.
            if obj.data.materials and all(m and m.get('village_clear_glass') for m in obj.data.materials):
                obj.visible_shadow = False
            if key == 'bedroom' and label(obj.name) in BACKDROPS:
                obj.hide_render = True; obj.hide_viewport = True; hidden.append(obj.name)
        if key == 'riverside-campsite':
            body = next(o for o in objects if label(o.name).startswith('Cabin enclosed body'))
            lo, hi = bounds(body)
            # Hollow the formerly solid cabin, retaining walls, floor and ceiling.
            from mathutils import Vector
            inset = Vector((.13, .13, .13))
            subtract_box(body, lo+inset, hi-inset, col)
            cuts.append(dict(object=body.name, operation='hollow cabin interior'))
        candidates = [o for o in objects if o.type == 'MESH' and label(o.name).startswith(BACKING_PREFIXES.get(key, ('\0',)))]
        for pane in sorted(set(panes),key=lambda o:o.name):
            lo, hi = bounds(pane); axis = min(range(3), key=lambda i: hi[i]-lo[i])
            lo[axis] -= .8; hi[axis] += .8
            for obj in candidates:
                if not obj.data.vertices: continue
                a, b = bounds(obj)
                if not all(min(hi[i], b[i])-max(lo[i], a[i]) > .002 for i in range(3)): continue
                subtract_box(obj, lo, hi, col)
                cuts.append(dict(object=obj.name, pane=pane.name))
        print('CLEAR_GLASS_ROOM',key,len(changed),len(cuts),flush=True)
        report['rooms'][key] = dict(materials=changed, converted_window_objects=sorted(o.name for o in set(panes)), hidden_fake_scenery=hidden, openings=cuts)
    return report


def apply_environment_glass(collection):
    """Give the village cottages actual window openings and hollow interiors."""
    import bpy
    import numpy as np
    from mathutils import Vector, Matrix
    objects = list(collection.objects)
    panes = [o for o in objects if o.name.startswith('Blue window pane')]
    if not panes: return
    casing = [o for o in objects if o.name.startswith('Cream window casing')]
    walls = [o for o in objects if o.name.startswith('Colored cottage walls')]
    def box_frame(points):
        # Horizontal principal axes recover the authored cottage rotation.
        points = np.array(points); center = points.mean(axis=0)
        _, axes = np.linalg.eigh(np.cov((points-center)[:, :2].T))
        x = Vector((*axes[:, 1], 0)); y = Vector((-x.y, x.x, 0))
        rotation = Matrix((x, y, Vector((0, 0, 1)))).transposed().to_4x4()
        local = np.array([tuple(rotation.to_3x3().transposed() @ Vector(p)) for p in points])
        low, high = local.min(axis=0), local.max(axis=0)
        center = rotation.to_3x3() @ Vector((low+high)/2)
        return center, Vector(high-low), rotation
    for obj in walls:
        center, span, rotation = box_frame([obj.matrix_world @ v.co for v in obj.data.vertices])
        half = (span-Vector((.28, .28, .28)))/2
        subtract_box(obj, center-half, center+half, collection, rotation)
    openings = 0
    for obj in panes:
        # Source export batches all windows by color. Weld only in the component
        # lookup, preserving the original rounded glass geometry and normals.
        verts = [tuple(obj.matrix_world @ v.co) for v in obj.data.vertices]
        parent = list(range(len(verts)))
        def find(i):
            while parent[i] != i: parent[i] = parent[parent[i]]; i = parent[i]
            return i
        def union(a, b): parent[find(a)] = find(b)
        same = {}
        for i, p in enumerate(verts):
            key = tuple(round(v, 5) for v in p)
            if key in same: union(i, same[key])
            else: same[key] = i
        for poly in obj.data.polygons:
            for i in poly.vertices[1:]: union(poly.vertices[0], i)
        components = {}
        for i in range(len(verts)): components.setdefault(find(i), []).append(verts[i])
        for points in components.values():
            center, span, rotation = box_frame(points)
            # In this frame X is the wide horizontal axis, Y is pane thickness.
            half = Vector((span.x/2, .7, span.z/2))
            # Limit expensive boolean operations to the nearby house/casing.
            corners = [center + rotation.to_3x3() @ Vector((x*half.x, y*half.y, z*half.z)) for x in (-1,1) for y in (-1,1) for z in (-1,1)]
            lo = Vector([min(v[i] for v in corners) for i in range(3)])
            hi = Vector([max(v[i] for v in corners) for i in range(3)])
            for target in walls+casing:
                a,b = bounds(target)
                if all(min(hi[i],b[i])-max(lo[i],a[i]) > .001 for i in range(3)):
                    subtract_box(target, center-half, center+half, collection, rotation)
            openings += 1
        mat = bpy.data.materials.new('Village clear cottage glazing'); mat.use_nodes = True
        clear_material(mat); obj.data.materials.clear(); obj.data.materials.append(mat)
        for poly in obj.data.polygons: poly.material_index = 0
        obj.visible_shadow = False
    collection['clear_window_openings'] = openings
    for obj in walls+casing:
        # Recompute smooth normals after cutting, retaining rounded edges.
        obj.data.normals_split_custom_set_from_vertices([v.normal for v in obj.data.vertices])
