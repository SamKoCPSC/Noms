"""Reopen the saved lighting trial and verify its scope and source preservation.
Blender --python check-native.py -- STUDY_DIRECTORY
"""
import bpy, hashlib, json, sys
from pathlib import Path

REPO=Path(__file__).resolve().parents[4]
out=Path(sys.argv[sys.argv.index('--')+1]);meta=json.loads((out/'study.json').read_text())
path=out/'kitchen-district.blend'
bpy.ops.wm.open_mainfile(filepath=str(path),load_ui=False,use_scripts=False)
scene=bpy.context.scene
lights=[o for o in scene.objects if o.type=='LIGHT']
assert len(lights)==1 and lights[0].data.type=='SUN'
assert abs(lights[0].data.energy-meta['lighting']['sun_energy'])<1e-6
assert not bpy.data.libraries
unpacked=[i.name for i in bpy.data.images if i.source=='FILE' and i.filepath and not i.packed_file]
assert not unpacked,unpacked
assert sum('source_object' in o for o in scene.objects)==meta['kitchen_objects']
assert sum(o.get('bake_group') in ('landscape','ground') for o in scene.objects)==meta['landscape_objects']
eye=meta['camera']['eye'];assert sum((scene.camera.location[i]-[eye[0],-eye[2],eye[1]][i])**2 for i in range(3))<1e-8
assert not any(o.animation_data for o in scene.objects)
points=[o.matrix_world@v.co for o in scene.objects if 'source_object' in o for v in o.data.vertices]
bounds={'min':[min(p.x for p in points),min(p.z for p in points),-max(p.y for p in points)],
        'max':[max(p.x for p in points),max(p.z for p in points),-min(p.y for p in points)]}
for source,sha in meta['source_hashes'].items():assert hashlib.sha256((REPO/source).read_bytes()).hexdigest()==sha,source
report=dict(native_study_reopened=True,shared_suns=len(lights),original_source_hashes_verified=len(meta['source_hashes']),
    kitchen_objects=meta['kitchen_objects'],landscape_objects=meta['landscape_objects'],animations=False,external_libraries=0,
    unpacked_images=unpacked,original_walking_camera=True,kitchen_vertex_bounds=bounds,
    sha256=hashlib.sha256(path.read_bytes()).hexdigest(),bytes=path.stat().st_size)
(out/'native-verification.json').write_text(json.dumps(report,indent=2)+'\n');print('NATIVE_STUDY_CHECK_PASS',json.dumps(report),flush=True)
