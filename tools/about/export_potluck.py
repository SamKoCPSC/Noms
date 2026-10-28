"""Export the approved potluck master into the About tour's small GLB contract.

blender -b MASTER --threads 5 --python tools/about/export_potluck.py -- OUTPUT_DIR
The editable master is never saved or modified on disk.
"""
import bpy
import hashlib
import json
import sys
import struct
from pathlib import Path

source = Path(bpy.data.filepath)
out = Path(sys.argv[sys.argv.index('--') + 1])
out.mkdir(parents=True, exist_ok=True)
scene = bpy.context.scene
scene.frame_set(160)
groups = json.loads((source.parent / 'assembly-manifest.json').read_text())['assembly_groups']
membership = {name: i for i, group in enumerate(groups) for name in group['members']}
prints = {
    'Original distant window view': 1,
    'Original mountain lake television artwork': 2,
    'Croissant v9 · baked outer dough': 3,
    'Sea salt chips printed film': 4,
    'Smoky BBQ chips printed film': 5,
}
# Preserve round serving ware, the rolled pastry silhouette, and printed surfaces.
def protected(name):
    return any(word in name.lower() for word in (
        'plate', 'platter', 'bowl', 'glass', 'pitcher', 'printed front',
        'landscape screen', 'distant window', 'carved horse', 'upright ear',
        'slotted mitre', 'king cross', 'board file', 'board rank', 'inscription',
        'tabletop', 'table top', 'rounded pillar', 'rounded arm', 'seat cushion',
        'back cushion', 'upholstered back', 'cake exposed layer', 'ganache top',
    ))
# Retain the actual packed approved prints; image preparation creates web derivatives.
for material_name, index in prints.items():
    material = bpy.data.materials[material_name]
    shader = material.node_tree.nodes.get('Principled BSDF')
    node = next(link.from_node for link in shader.inputs['Base Color'].links
                if link.from_node.type == 'TEX_IMAGE')
    image = node.image
    if not image.packed_file:
        raise RuntimeError('Expected the approved print to be packed: ' + material_name)
    (out / f'potluck-texture-{index}.png').write_bytes(image.packed_file.data)
bpy.ops.object.select_all(action='DESELECT')
for name in membership:
    obj = bpy.data.objects[name]
    obj.select_set(True)
    if not protected(name):
        if obj.type in ('CURVE', 'FONT'):
            obj.data.resolution_u = 3
            if obj.type == 'CURVE':
                obj.data.bevel_resolution = 1
                obj.data.render_resolution_u = 3
        for modifier in obj.modifiers:
            if modifier.type == 'BEVEL':
                modifier.segments = min(2, modifier.segments)
# Evaluate the native scene once. Simplify one temporary object at a time in a
# separate scene, avoiding thousands of whole-room dependency-graph rebuilds.
bpy.context.view_layer.update()
source_graph = bpy.context.evaluated_depsgraph_get()
native_objects = [(name, bpy.data.objects[name]) for name in membership]
scene = bpy.data.scenes.new('Potluck web export')
bpy.context.window.scene = scene
objects = []
print('POTLUCK_EXPORT_EVALUATING', len(native_objects), flush=True)


def material_info(material):
    shader = material.node_tree.nodes.get('Principled BSDF') if material.use_nodes else None
    rgb = list(shader.inputs['Base Color'].default_value[:3] if shader else material.diffuse_color[:3])
    opacity = .14 if shader and shader.inputs['Transmission Weight'].default_value > .5 else 1.
    return rgb, opacity


audit = []
opaque, transparent = [], []
for index, (source_name, native) in enumerate(native_objects):
    data = bpy.data.meshes.new_from_object(native.evaluated_get(source_graph),
                                         preserve_all_data_layers=True, depsgraph=source_graph)
    obj = bpy.data.objects.new('Potluck export part', data)
    scene.collection.objects.link(obj)
    obj.matrix_world = native.matrix_world.copy()
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    obj.data.calc_loop_triangles()
    count = len(obj.data.loop_triangles)
    preserve = protected(source_name)
    if preserve:
        ratio = 1.
    elif source_name.startswith('Croissant '):
        ratio = min(1., 3600 / max(1, count))
    elif source_name.startswith('Chess '):
        ratio = .35 if count > 80 else 1.
    elif source_name.startswith(('Oak plank ', 'Rounded oak display', 'Continuous oak floor')):
        ratio = .18 if count > 80 else 1.
    else:
        ratio = .30 if count > 80 else 1.
    if ratio < 1.:
        bpy.context.view_layer.objects.active = obj
        modifier = obj.modifiers.new('Potluck web silhouette budget', 'DECIMATE')
        modifier.ratio = ratio
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    # Keep each approved print's UV mapping in the joined mesh's common UV0 layer.
    uv = obj.data.uv_layers.get('Print UV') or obj.data.uv_layers.active
    if uv is None:
        uv = obj.data.uv_layers.new(name='PotluckUV')
    for other in list(obj.data.uv_layers):
        if other != uv:
            obj.data.uv_layers.remove(other)
    uv.name = 'PotluckUV'
    uv.active_render = True
    obj.data.calc_loop_triangles()
    audit.append(dict(name=source_name, source_triangles=count,
                      web_triangles=len(obj.data.loop_triangles), preserved=preserve))
    for name in ['Assembly', 'PotluckPrint', 'PotluckOpacity']:
        obj.data.attributes.new(name=name, type='INT', domain='FACE')
    all_glass = True
    for face in obj.data.polygons:
        material = obj.data.materials[face.material_index]
        _, opacity = material_info(material)
        obj.data.attributes['Assembly'].data[face.index].value = membership[source_name]
        obj.data.attributes['PotluckPrint'].data[face.index].value = prints.get(material.name, 0)
        obj.data.attributes['PotluckOpacity'].data[face.index].value = round(opacity * 1000)
        all_glass &= opacity < 1.
    (transparent if all_glass else opaque).append(obj)
    objects.append(obj)
    scene.collection.objects.unlink(obj)
    if index % 200 == 0:
        print('POTLUCK_EXPORT_PART', index, source_name, flush=True)
for obj in objects:
    scene.collection.objects.link(obj)
print('POTLUCK_EXPORT_SIMPLIFIED', sum(a['web_triangles'] for a in audit), flush=True)


def join(items):
    bpy.ops.object.select_all(action='DESELECT')
    for obj in items:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = items[0]
    bpy.ops.object.join()
    obj = bpy.context.object
    world = obj.matrix_world.copy()
    obj.parent = None
    obj.matrix_world = world
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    return obj


# Glazing must not become an opaque occluder while baking the drink interiors.
for obj in transparent:
    obj.hide_render = True
obj = join(opaque)
palette = [material_info(m)[0] for m in obj.data.materials]
colors = obj.data.color_attributes.new(name='PotluckColor', type='FLOAT_COLOR', domain='CORNER')
obj.data.color_attributes.active_color = colors
scene.render.engine = 'CYCLES'
scene.cycles.samples = 12
scene.cycles.use_denoising = False
scene.render.bake.target = 'VERTEX_COLORS'
bpy.ops.object.bake(type='AO', use_clear=True)
for face in obj.data.polygons:
    rgb = [1., 1., 1.] if obj.data.attributes['PotluckPrint'].data[face.index].value else palette[face.material_index]
    for loop in face.loop_indices:
        ao = max(.72, min(1., colors.data[loop].color[0]))
        colors.data[loop].color = (*[c * (.45 + .55 * ao) for c in rgb], 1.)
for glass in transparent:
    glass.hide_render = False
    color = glass.data.color_attributes.new(name='PotluckColor', type='FLOAT_COLOR', domain='CORNER')
    for face in glass.data.polygons:
        rgb, _ = material_info(glass.data.materials[face.material_index])
        for loop in face.loop_indices:
            color.data[loop].color = (*rgb, 1.)
obj = join([obj] + transparent)
obj.name = 'Noms shared potluck room'
obj.data.materials.clear()
batches = {}
for face in obj.data.polygons:
    gi = obj.data.attributes['Assembly'].data[face.index].value
    ti = obj.data.attributes['PotluckPrint'].data[face.index].value
    alpha = obj.data.attributes['PotluckOpacity'].data[face.index].value / 1000
    key = (gi, ti, alpha)
    if key not in batches:
        group = groups[gi]
        material = bpy.data.materials.new(f"{group['name']} print {ti} opacity {alpha}")
        material.use_nodes = True
        material['assembly'] = dict(
            name=group['name'], pivot=[group['pivot'][0], group['pivot'][2], -group['pivot'][1]],
            offset=[group['incoming_offset'][0], group['incoming_offset'][2], -group['incoming_offset'][1]],
            start=group['start_frame'], end=group['end_frame'], texture=ti, opacity=alpha)
        shader = material.node_tree.nodes.get('Principled BSDF')
        vertex = material.node_tree.nodes.new('ShaderNodeVertexColor')
        vertex.layer_name = 'PotluckColor'
        material.node_tree.links.new(vertex.outputs['Color'], shader.inputs['Base Color'])
        batches[key] = len(obj.data.materials)
        obj.data.materials.append(material)
    face.material_index = batches[key]
bpy.ops.export_scene.gltf(filepath=str(out / 'sams-potluck.glb'), export_format='GLB',
                          use_selection=True, use_active_scene=True, export_apply=True, export_cameras=False,
                          export_lights=False, export_yup=True, export_extras=True)
blob = (out / 'sams-potluck.glb').read_bytes()
json_length = struct.unpack_from('<I', blob, 12)[0]
document = json.loads(blob[20:20 + json_length])
assert len(document['meshes']) == 1 and len(document['scenes']) == 1
assert not document.get('images'), 'Print images are supplied separately by the app'
obj.data.calc_loop_triangles()
assert len(obj.data.loop_triangles) <= 260000, 'Potluck exceeds the shared loader geometry budget'
report = dict(source_revision=6, source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
              triangles=len(obj.data.loop_triangles), draw_calls=len(batches), textures=5,
              assembly_groups=len(groups), contact_shading='vertex ambient occlusion, glazing excluded',
              glb_bytes=(out / 'sams-potluck.glb').stat().st_size, native_master_unchanged=True)
(out / 'potluck-budget.json').write_text(json.dumps(report, indent=2))
(out / 'potluck-geometry-audit.json').write_text(json.dumps(audit, indent=2))
print('POTLUCK_WEB_ASSET', json.dumps(report), flush=True)
