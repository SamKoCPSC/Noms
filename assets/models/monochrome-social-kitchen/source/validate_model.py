"""Check native intent, GLB structure, vertex colors and a fresh Blender import."""
import bpy,sys,json,struct,math,hashlib
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parents[1];name=root.name
native=root/'models'/f'{name}.blend';glb=root/'models'/f'{name}.glb'
bpy.ops.wm.open_mainfile(filepath=str(native))
assert bpy.context.scene['reconstructed'] and bpy.context.scene['revision']==7
obs=list(bpy.context.scene.objects)
count=lambda s:sum(s in o.name for o in obs)
report={'revision':7,'reconstructed':True,'native_meshes':sum(o.type=='MESH'for o in obs)}
if 'kitchen' in name:
 assert count('dark stoneware plate')==4
 assert count('rosy beef grain')==5
 assert count('Carved roast first slice rosy beef grain')==1 and count('Carved roast second slice')==0
 for plate in [o for o in obs if 'dark stoneware plate' in o.name]:
  center=sum((plate.matrix_world@Vector(v) for v in plate.bound_box),Vector())/8
  slices=[o for o in obs if 'carved beef rosy beef grain' in o.name and (o.matrix_world.translation-center).length<.25]
  assert len(slices)==1
 assert count('crisp potato chunk')==54
 assert count('green pea')==172
 assert count('peppercorn cream sauce')==4
 assert count('Butter dish')==0
 assert count('Dinner red wine bowl')==4
 assert count('Dining soup ivory bowl')==4 and count('Dining soup golden soup')==4
 assert count('Island tray crisp potato chunk')==30
 bread=[o for o in obs if 'angled slice' in o.name]
 assert len(bread)==4 and all(abs(o.rotation_euler.y+1.4)<.001 for o in bread)
 bpy.context.view_layer.update()
 def bounds(o):
  pts=[o.matrix_world@v.co for v in o.data.vertices]
  return [min(v[k]for v in pts)for k in range(3)],[max(v[k]for v in pts)for k in range(3)]
 bl,bh=bounds(bpy.data.objects['Shared bread board']);lo,hi=bounds(bpy.data.objects['Butter block on bread board'])
 assert all(bl[k]<lo[k]<hi[k]<bh[k]for k in [0,1]) and abs(lo[2]-bh[2])<.005

 roast=bpy.data.objects['Carved whole roast'];assert abs(roast.rotation_euler.z-math.pi/2)<.001 and all(abs(v-.42)<.001 for v in roast.scale)
 face=bpy.data.objects['Whole prime rib roast pink carved face']
 normal=(face.matrix_world.to_3x3().inverted().transposed()@face.data.polygons[0].normal).normalized()
 assert normal.x>.999 and abs(normal.y)<.001
 for v in list(face.data.vertices)[-96:]:
  assert v.co.z>=.045-1e-6
  assert abs(v.co.z-.045)<1e-6 or abs((v.co.x/.35)**2+((v.co.z-.247)/.23)**2-1)<.0001
 crust=bpy.data.objects['Whole prime rib roast uneven pepper crust']
 base=[v for v in crust.data.vertices if abs(v.co.z-.045)<1e-6]
 assert len(base)>150 and max(v.co.x for v in base)-min(v.co.x for v in base)>.25
 roast_pts=[o.matrix_world@v.co for o in roast.children_recursive if o.type=='MESH'for v in o.data.vertices]
 assert max(v.z for v in roast_pts)-min(v.z for v in roast_pts)<.20
 assert abs(min(v.z for v in roast_pts)-1.194)<.001
 board_low,board_high=bounds(bpy.data.objects['Roast carving board'])
 assert abs(board_high[0]-board_low[0]-1.0)<.001 and abs(board_high[1]-board_low[1]-.65)<.001
 assert all(board_low[k]<min(v[k]for v in roast_pts)<max(v[k]for v in roast_pts)<board_high[k]for k in [0,1])
 board_slice=bpy.data.objects['Carved roast first slice rosy beef grain']
 slice_faces=[o for o in obs if 'rosy beef grain' in o.name]
 for cut in slice_faces:
  assert all(abs(a-b)<1e-6 for a,b in zip(cut.parent.scale,(1.10,1.0,.85)))
  assert len(cut.data.vertices)==len(board_slice.data.vertices)
  assert all((a.co-b.co).length<1e-6 for a,b in zip(cut.data.vertices,board_slice.data.vertices))
  edge=list(cut.data.vertices)[-96:]
  assert sum(abs(v.co.y+.07272)<1e-6 for v in edge)>=13
  for v in edge:
   xx=v.co.x/.29;zz=v.co.y/.36+.247
   assert abs(zz-.045)<1e-6 or abs((xx/.35)**2+((zz-.247)/.23)**2-1)<.0001
 for part in board_slice.parent.children:
  low,high=bounds(part)
  assert all(board_low[k]<low[k]<high[k]<board_high[k]for k in [0,1])
 for o in [o for o in obs if 'Island tray crisp potato chunk'in o.name]:
  low,high=bounds(o);assert .335<low[0]<high[0]<.985 and -.366<low[1]<high[1]<.126
  assert high[2]-low[2]<.10
 tl,th=bounds(bpy.data.objects['Dining table rounded top'])
 for o in [o for o in obs if 'Dining soup ivory bowl'in o.name]:
  low,high=bounds(o)
  assert all(tl[k]<low[k]<high[k]<th[k]for k in [0,1]) and abs(low[2]-th[2])<.005
  assert high[0]<bl[0] or low[0]>bh[0]
 report.update(plated_dinners=4,beef_slices=5,board_slices=1,slices_per_plate=1,flat_roast_base=True,matching_oval_slices=True,carving_board_size=[1.0,.65],potato_chunks=54,tray_potatoes=30,soup_bowls=4,oval_roast=True,roast_cut_face_direction='+X',roast_scale=.42,fanned_bread_slices=4,butter_on_shared_board=True)

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
 assert len(food)==61 and all('COLOR_0'in p['attributes']for m in food for p in m['primitives'])
 report['food_meshes_with_vertex_colors']=len(food)
report.update(glb_bytes=len(b),glb_meshes=len(d['meshes']),triangles=sum(d['accessors'][p['indices']]['count']//3 for m in d['meshes']for p in m['primitives']),sha256=hashlib.sha256(b).hexdigest(),structure_valid=True)
bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(glb))
meshes=[o for o in bpy.context.scene.objects if o.type=='MESH'];assert len(meshes)==len(d['meshes'])
for o in meshes:assert all(math.isfinite(v)for v in o.dimensions) and max(o.dimensions)>0
report['fresh_blender_glb_import']='passed'
(root/'models/validation.json').write_text(json.dumps(report,indent=2)+'\n');print('VALIDATED',report)
