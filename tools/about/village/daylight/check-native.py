"""Reopen the complete shared-daylight world and verify source preservation."""
import bpy, hashlib, json, sys
from pathlib import Path
from mathutils import Matrix
sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import *

path = DAY/'models/terraced-village-daylight.blend'
bpy.ops.wm.open_mainfile(filepath=str(path), load_ui=False, use_scripts=False)
scene = bpy.context.scene
assert json.loads(scene['shared_daylight']) == CONFIG
lights = [o for o in scene.objects if o.type == 'LIGHT']
assert len(lights) == 1 and lights[0].data.type == 'SUN'
assert abs(lights[0].data.energy - CONFIG['sun_energy']) < 1e-6
background = scene.world.node_tree.nodes.get('Background')
assert abs(background.inputs['Strength'].default_value - CONFIG['sky_strength']) < 1e-6
assert abs(scene.view_settings.exposure - CONFIG['exposure']) < 1e-6
instances = [o for o in scene.objects if o.instance_type == 'COLLECTION' and 'stop' in o]
assert sorted(o['stop'] for o in instances) == list(range(1, 13))
receipts = json.loads((ASSET/'models/terraced-village-blockout.sources.json').read_text())
for source in receipts:
    assert hashlib.sha256((REPO/source['source']).read_bytes()).hexdigest() == source['sha256']
    assert len(bpy.data.collections[source['collection']].all_objects) == source['objects']
for obj in instances:
    key = LAYOUT['stops'][obj['stop']-1]['id']
    expected = anchor(key) @ Matrix.Translation(obj.instance_collection.instance_offset)
    assert max(abs(obj.matrix_world[i][j]-expected[i][j]) for i in range(4) for j in range(4)) < 1e-5
assert len(scene.timeline_markers) == 12
assert scene.camera.name == 'Village walking camera' and scene.camera.animation_data
assert not bpy.data.libraries
unpacked = [i.name for i in bpy.data.images if i.source == 'FILE' and i.filepath and not i.packed_file]
assert not unpacked, unpacked
terrain = bpy.data.collections['World terrain — blockout']
if LAYOUT.get('stone_paths'):
    paths=json.loads(gzip.decompress((DAY/'source/stone-paths.json.gz').read_bytes()))
    assert not any(o.name in paths['remove_names'] for o in terrain.objects)
    assert all(any(o.name==m['name'] for o in terrain.objects) for m in paths['meshes'])
    for item in paths['meshes']:
        obj=terrain.objects[item['name']]
        assert len(obj.data.vertices)==len(item['vertices'])
        assert len(obj.data.polygons)*3==len(item['indices'])
        assert all(max(abs(a-b) for a,b in zip(vertex.co,xyz(point)))<2e-5
                   for vertex,point in zip(obj.data.vertices,item['vertices']))
if LAYOUT.get('vegetation'):
    plants=json.loads(gzip.decompress((DAY/'source/vegetation.json.gz').read_bytes()))
    for item in plants['meshes']:
        obj=terrain.objects[item['name']]
        assert len(obj.data.vertices)==len(item['vertices'])
        assert len(obj.data.polygons)*3==len(item['indices'])
        assert 'VegetationTint' in obj.data.color_attributes
        assert all(max(abs(a-b) for a,b in zip(vertex.co,xyz(point)))<2e-5 for vertex,point in zip(obj.data.vertices,item['vertices']))
    assert not any(o.name.startswith(('Rounded leafy canopy','Leafy low shrub','Small meadow flower')) for o in terrain.objects)
if LAYOUT.get('signage'):
    signs=json.loads(gzip.decompress((DAY/'source/signage.json.gz').read_bytes()))
    for item in signs['meshes']:
        matches=[o for o in terrain.objects if (o.name==item['name'] or o.name.startswith(item['name']+'.'))
                 and len(o.data.vertices)==len(item['vertices'])]
        assert any(all(max(abs(a-b) for a,b in zip(vertex.co,xyz(point)))<2e-5
                       for vertex,point in zip(obj.data.vertices,item['vertices'])) for obj in matches),item['name']
for obj in terrain.objects:
    assert obj.data.has_custom_normals
    shader = obj.active_material.node_tree.nodes.get('Principled BSDF')
    assert shader.inputs['Roughness'].default_value == 1
report = dict(native_world_reopened=True, model_instances=len(instances),
    native_objects=sum(len(o.instance_collection.all_objects) for o in instances),
    shared_suns=1, lighting=CONFIG, original_master_hashes_verified=len(receipts),
    world_meshes=len(terrain.objects), matte_world_materials=True, preserved_edge_normals=True,
    portable_images=True, external_libraries=0, walking_camera_preserved=True,
    sha256=hashlib.sha256(path.read_bytes()).hexdigest(), bytes=path.stat().st_size)
report['stone_path_geometry_sha256']=path_revision()
report['vegetation_geometry_sha256']=vegetation_revision()
report['signage_geometry_sha256']=signage_revision()
(DAY/'source/native-verification.json').write_text(json.dumps(report, indent=2)+'\n')
print('DAYLIGHT_NATIVE_CHECK_PASS', json.dumps(report), flush=True)
