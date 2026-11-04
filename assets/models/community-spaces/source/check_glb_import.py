"""Independently reimport the portable exports with Blender's glTF importer."""
import bpy,json,math
from pathlib import Path
from mathutils import Vector
R=Path(__file__).resolve().parent.parent
results=[]
for p in sorted((R/'models').glob('*.glb')):
 bpy.ops.wm.read_factory_settings(use_empty=True)
 bpy.ops.import_scene.gltf(filepath=str(p))
 bpy.context.view_layer.update()
 meshes=[o for o in bpy.context.scene.objects if o.type=='MESH'];assert len(meshes)>100
 points=[o.matrix_world@Vector(c)for o in meshes for c in o.bound_box];assert all(math.isfinite(x)for v in points for x in v)
 lo=[min(v[i]for v in points)for i in range(3)];hi=[max(v[i]for v in points)for i in range(3)]
 assert 6<hi[0]-lo[0]<12 and 5<hi[1]-lo[1]<10 and 2.5<hi[2]-lo[2]<5,(p.name,lo,hi)
 assert not [im.name for im in bpy.data.images if im.source=='FILE' and im.users and not im.packed_file]
 results.append(dict(file=p.name,mesh_objects=len(meshes),bounds_blender_z_up=[lo,hi],import_status='PASS'))
 print('GLB_IMPORT_PASS',p.name,flush=True)
(R/'import-validation.json').write_text(json.dumps(results,indent=2))
