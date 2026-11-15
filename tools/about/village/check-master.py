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
assert scene.camera.name == 'Village overview — static'
assert scene.camera.location.length > 100
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
