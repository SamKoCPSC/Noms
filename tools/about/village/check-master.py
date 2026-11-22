"""Verify a saved world can reopen with all native sources and portable images.
blender -b --disable-autoexec --python-exit-code 1 --python tools/about/village/check-master.py -- WORLD.blend REPORT.json
"""
import hashlib
import json
import sys
from pathlib import Path

import bpy

root = Path(__file__).resolve().parents[3]
world, report_path = map(Path, sys.argv[sys.argv.index('--') + 1:])
bpy.ops.wm.open_mainfile(filepath=str(world), load_ui=False, use_scripts=False)
scene = bpy.context.scene
instances = [o for o in scene.objects if o.instance_type == 'COLLECTION' and 'stop' in o]
assert sorted(o['stop'] for o in instances) == list(range(1, 13))
assert len(scene.timeline_markers) == 12
assert scene.camera.name == 'Village walking camera'
poses = json.loads((root / 'assets/models/terraced-village/source/camera-poses.json').read_text())
eye = poses[0]['eye']
assert sum((scene.camera.location[i] - [eye[0], -eye[2], eye[1]][i]) ** 2 for i in range(3)) < 1e-8
assert scene.camera.animation_data and scene.camera.animation_data.action
layout=json.loads((root / 'assets/models/terraced-village/layout.json').read_text())
assert abs(scene['walk_eye_height'] - layout['walk']['eye_height']) < 1e-6
environment = json.loads((root / 'assets/models/terraced-village/source/world-environment.json').read_text())
terrain = bpy.data.collections['World terrain — blockout']
assert len(terrain.objects) == len(environment)
for obj in terrain.objects:
    assert obj.data.has_custom_normals, obj.name
    shader = obj.active_material.node_tree.nodes.get('Principled BSDF')
    assert shader.inputs['Roughness'].default_value == 1, obj.name
    assert shader.inputs['Metallic'].default_value == 0, obj.name
    assert shader.inputs['Coat Weight'].default_value == 0, obj.name
    assert abs(shader.inputs['Specular IOR Level'].default_value - .04) < 1e-6, obj.name
assert sum(o.name.startswith('Path-to-entrance connection') for o in terrain.objects) == 12
assert sum(o.name.startswith('Inset entrance forecourt') for o in terrain.objects) == 12
assert sum(o.name.startswith('Colored cottage walls') for o in terrain.objects) == len(layout['neighborhood']['houses'])
for street in layout['neighborhood']['streets']:
    assert any(o.name == 'Lettering: ' + street for o in terrain.objects),street
assert not bpy.data.libraries, 'World still depends on external Blender libraries'
unpacked = [i.name for i in bpy.data.images if i.source == 'FILE' and i.filepath and not i.packed_file]
assert not unpacked, f'Unpacked image dependencies: {unpacked}'
receipts = json.loads(world.with_suffix('.sources.json').read_text())
assert len(receipts) == 12
for source in receipts:
    assert hashlib.sha256((root / source['source']).read_bytes()).hexdigest() == source['sha256'], source['id']
    collection = bpy.data.collections.get(source['collection'])
    assert collection is not None and len(collection.all_objects) == source['objects'], source['id']
result = {
    'native_world_reopened': True,
    'model_instances': len(instances),
    'native_objects': sum(len(o.instance_collection.all_objects) for o in instances),
    'camera_stops': len(scene.timeline_markers),
    'entrance_connections': 12,
    'village_cottages': len(layout['neighborhood']['houses']),
    'named_streets': len(layout['neighborhood']['streets']),
    'street_lettering_preserved': True,
    'shared_environment_meshes': len(terrain.objects),
    'matte_world_materials_verified': True,
    'custom_edge_normals_preserved': True,
    'active_camera': scene.camera.name,
    'walking_eye_height': scene['walk_eye_height'],
    'external_libraries': len(bpy.data.libraries),
    'unpacked_file_images': unpacked,
    'original_master_hashes_verified': len(receipts),
    'world_sha256': hashlib.sha256(world.read_bytes()).hexdigest(),
    'world_bytes': world.stat().st_size,
    'lighting_rebaked': False,
}
report_path.parent.mkdir(parents=True, exist_ok=True)
report_path.write_text(json.dumps(result, indent=2) + '\n')
print('WORLD_CHECK_PASS', json.dumps(result), flush=True)
