"""Prepare a non-destructive kitchen district for a shared outdoor bake.
blender -b -t 2 --python-exit-code 1 --python tools/about/village/lighting/prepare.py -- OUTPUT
"""
import bpy, hashlib, json, math, sys
from pathlib import Path
from mathutils import Matrix, Vector

REPO = Path(__file__).resolve().parents[4]
OUT = Path(sys.argv[sys.argv.index('--') + 1]); OUT.mkdir(parents=True, exist_ok=True)
ASSET = REPO / 'assets/models/terraced-village'
world_file = ASSET / 'models/terraced-village-blockout.blend'
layout = json.loads((ASSET / 'layout.json').read_text())
entry = layout['stops'][0]
hash_file = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
receipts = {str(p.relative_to(REPO)): hash_file(p) for p in [world_file, REPO / entry['master'], REPO / entry['model']]}
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
with bpy.data.libraries.load(str(world_file), link=False) as (src, dst):
    dst.collections = ['WORLD_SOURCE_kitchen']
source = dst.collections[0]
scene.collection.children.link(source)
objects = [o for o in source.all_objects if o.type in {'MESH', 'CURVE', 'FONT'} and not o.hide_render]
bpy.ops.object.select_all(action='DESELECT')
for obj in objects: obj.select_set(True)
bpy.context.view_layer.objects.active = objects[0]
bpy.ops.object.convert(target='MESH')
objects = list(bpy.context.selected_objects)
# Match the existing browser GLB's anchor exactly, rather than recomputing bounds
# from rotated bounding-box corners of individual native objects.
source_lo = [-3.75, -.29005609265999466, -3.049955795844976]
source_hi = [3.75, 4.200055910269782, 3.1499557004775443]
scale = entry['width'] / (source_hi[0] - source_lo[0])
center = [(source_lo[i] + source_hi[i]) / 2 for i in range(3)]
xyz = lambda p: (p[0], -p[2], p[1])
anchor = Matrix.Translation(xyz(entry['position'])) @ Matrix.Rotation(entry['yaw'], 4, 'Z') @ Matrix.Scale(scale, 4) @ Matrix.Translation((-center[0], center[2], -source_lo[1]))
for obj in objects:
    obj['bake_group'] = 'architecture' if max(obj.dimensions) > .85 else 'details'
    obj['source_object'] = obj.name
    obj.matrix_world = anchor @ obj.matrix_world
    obj.animation_data_clear()
for obj in list(source.objects):
    if obj not in objects: bpy.data.objects.remove(obj, do_unlink=True)

environment = json.loads((ASSET / 'source/world-environment.json').read_text())
terrain = bpy.data.collections.new('Kitchen district landscape'); scene.collection.children.link(terrain)
materials = {}
# Deliberately bounded lighting trial, away from the next room. The ground grid
# lands on these bounds; no original room or full-world asset is overwritten.
bounds = [-80, -8, 10, 68]
for index, item in enumerate(environment):
    verts = item['vertices']; normals = item['normals']; indices = item['indices']
    inside = [bounds[0] <= v[0] <= bounds[1] and bounds[2] <= v[2] <= bounds[3] for v in verts]
    faces = [indices[i:i+3] for i in range(0, len(indices), 3) if all(inside[j] for j in indices[i:i+3])]
    if not faces: continue
    used = sorted({j for f in faces for j in f}); remap = {old:new for new,old in enumerate(used)}
    mesh = bpy.data.meshes.new(item['name']); mesh.from_pydata([xyz(verts[j]) for j in used], [], [[remap[j] for j in f] for f in faces]); mesh.update()
    mesh.polygons.foreach_set('use_smooth', [True] * len(mesh.polygons))
    mesh.normals_split_custom_set_from_vertices([xyz(normals[j]) for j in used])
    key = tuple(item['color'])
    if key not in materials:
        mat = bpy.data.materials.new('Matte district ' + str(len(materials))); mat.use_nodes = True
        p = mat.node_tree.nodes.get('Principled BSDF'); p.inputs['Base Color'].default_value = (*key, 1)
        mat.diffuse_color = (*key,1); p.inputs['Roughness'].default_value = 1
        p.inputs['Specular IOR Level'].default_value = .04; materials[key] = mat
    mesh.materials.append(materials[key])
    obj = bpy.data.objects.new(item['name'], mesh); terrain.objects.link(obj)
    obj['bake_group'] = 'ground' if index == 0 else 'landscape'

scene.world = bpy.data.worlds.new('Shared cool daylight'); scene.world.use_nodes = True
bg = scene.world.node_tree.nodes.get('Background')
bg.inputs['Color'].default_value = (.65,.78,1,1); bg.inputs['Strength'].default_value = .5
sun = bpy.data.lights.new('Shared warm sun', 'SUN'); sun.energy = 2; sun.angle = math.radians(9); sun.color = (1,.93,.83)
light = bpy.data.objects.new(sun.name, sun); scene.collection.objects.link(light)
direction = Vector((-60,-50,100)).normalized(); light.rotation_euler = (-direction).to_track_quat('-Z','Y').to_euler()
poses = json.loads((ASSET / 'source/camera-poses.json').read_text())
camera_data = bpy.data.cameras.new('Kitchen walking camera'); camera = bpy.data.objects.new(camera_data.name, camera_data); scene.collection.objects.link(camera)
camera.location = xyz(poses[0]['eye']); camera.rotation_euler = (Vector(xyz(poses[0]['target'])) - camera.location).to_track_quat('-Z','Y').to_euler()
camera_data.sensor_fit = 'VERTICAL'; camera_data.sensor_height = 24; camera_data.lens = 24/(2*math.tan(math.radians(60)/2)); camera_data.clip_end = 650
scene.camera = camera; scene.render.engine = 'CYCLES'; scene.cycles.samples = 64; scene.cycles.use_denoising = False
scene.render.resolution_x = 1200; scene.render.resolution_y = 900; scene.render.resolution_percentage = 100
scene.view_settings.view_transform = 'AgX'; scene.view_settings.look = 'AgX - Medium High Contrast'; scene.view_settings.exposure = -.18
scene.render.image_settings.file_format = 'PNG'; scene.render.image_settings.color_mode = 'RGB'
scene['scope'] = 'Kitchen district lighting trial only; original world and room masters unchanged'
notice = bpy.data.texts.new('Droid Sans font notice')
notice.write((REPO / 'tools/about/village/DROID-FONT-NOTICE.txt').read_text())
bpy.context.preferences.filepaths.save_version = 0
bpy.ops.file.pack_all(); bpy.ops.wm.save_as_mainfile(filepath=str(OUT / 'kitchen-district.blend'), compress=True)
meta = dict(source_hashes=receipts, kitchen_objects=len(objects), landscape_objects=len(terrain.objects), crop_xz=bounds,
    source_bounds=dict(min=source_lo,max=source_hi), kitchen_transform=dict(position=entry['position'],yaw=entry['yaw'],scale=scale),
    camera=poses[0], lighting=dict(sun_direction_y_up=[-60,100,50],sun_energy=2,sun_color=list(sun.color),sun_angle_degrees=9,
        sky_color=[.65,.78,1],sky_strength=.5,view_transform='AgX',look=scene.view_settings.look,exposure=-.18))
(OUT / 'study.json').write_text(json.dumps(meta,indent=2)+'\n')
scene.render.filepath = str(OUT / 'shared-lighting-reference.png'); bpy.ops.render.render(write_still=True)
for name,sha in receipts.items(): assert hash_file(REPO / name) == sha
print('DISTRICT_PREPARED', json.dumps(meta), flush=True)
