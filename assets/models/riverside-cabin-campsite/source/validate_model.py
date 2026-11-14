"""Check native intent, GLB structure, vertex colors and a fresh Blender import."""
import bpy,sys,json,struct,math,hashlib
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parents[1];name=root.name
native=root/'models'/f'{name}.blend';glb=root/'models'/f'{name}.glb'
bpy.ops.wm.open_mainfile(filepath=str(native))
assert bpy.context.scene['reconstructed'] and bpy.context.scene['revision']==4
obs=list(bpy.context.scene.objects)
count=lambda s:sum(s in o.name for o in obs)
report={'revision':4,'reconstructed':True,'native_meshes':sum(o.type=='MESH'for o in obs)}
if 'kitchen' in name:
 assert count('dark stoneware plate')==4
 assert count('rosy beef grain')==18
 assert count('crisp potato chunk')==36
 assert count('green pea')==172
 assert count('peppercorn cream sauce')==4
 assert count('Butter dish')==0
 assert count('Dinner red wine bowl')==4
 bread=[o for o in obs if 'angled slice' in o.name]
 assert len(bread)==4 and all(abs(o.rotation_euler.y+1.4)<.001 for o in bread)
 bpy.context.view_layer.update()
 def bounds(o):
  pts=[o.matrix_world@Vector(v)for v in o.bound_box]
  return [min(v[k]for v in pts)for k in range(3)],[max(v[k]for v in pts)for k in range(3)]
 bl,bh=bounds(bpy.data.objects['Shared bread board']);lo,hi=bounds(bpy.data.objects['Butter block on bread board'])
 assert all(bl[k]<lo[k]<hi[k]<bh[k]for k in [0,1]) and abs(lo[2]-bh[2])<.005
 report.update(plated_dinners=4,beef_slices=18,potato_chunks=36,fanned_bread_slices=4,butter_on_shared_board=True)
else:
 assert bpy.context.scene['storeys']==2 and not bpy.context.scene['has_tent'] and not bpy.context.scene['has_fence']
 assert count('folded tier face')==72
 assert count('Lounge wooden back slat')==5 and count('Lounge wooden seat slat')==8
 assert count('Continuous blue river')==1 and count('Grilled sausage')==3
 report.update(cabin_storeys=2,river=True,trees=5,folded_tree_faces=72,wooden_porch_lounge=True,no_tent_or_fence=True)
for o in obs:assert all(math.isfinite(v)for row in o.matrix_world for v in row)
b=glb.read_bytes();magic,version,total=struct.unpack_from('<4sII',b);assert magic==b'glTF'and version==2 and total==len(b)
n,tag=struct.unpack_from('<I4s',b,12);assert tag==b'JSON';d=json.loads(b[20:20+n])
for node in d['nodes']:
 for c in node.get('children',[]):assert 0<=c<len(d['nodes'])
for view in d['bufferViews']:assert view.get('byteOffset',0)+view['byteLength']<=d['buffers'][view.get('buffer',0)]['byteLength']
if 'kitchen' in name:
 food=[m for m in d['meshes']if any(t in m.get('name','')for t in ['pink carved face','rosy beef grain','crisp potato chunk','uneven pepper crust'])]
 assert len(food)==56 and all('COLOR_0'in p['attributes']for m in food for p in m['primitives'])
 report['food_meshes_with_vertex_colors']=len(food)
report.update(glb_bytes=len(b),glb_meshes=len(d['meshes']),triangles=sum(d['accessors'][p['indices']]['count']//3 for m in d['meshes']for p in m['primitives']),sha256=hashlib.sha256(b).hexdigest(),structure_valid=True)
bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(glb))
meshes=[o for o in bpy.context.scene.objects if o.type=='MESH'];assert len(meshes)==len(d['meshes'])
for o in meshes:assert all(math.isfinite(v)for v in o.dimensions) and max(o.dimensions)>0
report['fresh_blender_glb_import']='passed'
(root/'models/validation.json').write_text(json.dumps(report,indent=2)+'\n');print('VALIDATED',report)
