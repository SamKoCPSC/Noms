"""Round-trip the review GLB in Blender and check its assembly contract."""
import bpy,json,struct,re
from pathlib import Path
root=Path(__file__).resolve().parent.parent
path=root/'models/university-library.glb'
raw=path.read_bytes();magic,version,length=struct.unpack_from('<4sII',raw)
assert magic==b'glTF' and version==2 and length==len(raw)
size,kind=struct.unpack_from('<I4s',raw,12);assert kind==b'JSON'
gltf=json.loads(raw[20:20+size])
assert len(gltf['scenes'][gltf.get('scene',0)]['nodes'])==13
assert not gltf.get('animations')
features={}
for label,prefix,expected in [('wooden_chairs','Wooden library chair sculpted wood seat',12),('window_chairs','Window facing lounge chair seat',4),('portraits','Portrait painted canvas',11),('art_glass_ribbons','Hanging glass ribbon',11),('stair_treads','Curved stair tread',24),('blue_window_panels','Blue simulated window panel',18),('study_laptops','Laptop beveled base',4)]:
 count=sum(bool(re.fullmatch(re.escape(prefix)+r'(?:\.\d+)? export',n.get('name',''))) and 'mesh' in n for n in gltf['nodes'])
 assert count==expected,(label,count,expected)
 features[label]=count
assert not any(any(term in n.get('name','').lower() for term in ['campus tree','campus garden','foliage','doorway','exterior garden']) for n in gltf['nodes'])
assert sum('center binding' in n.get('name','') and n.get('name','').startswith('Open study textbook') for n in gltf['nodes'])==5
features['open_study_books']=5
assert all('uri' not in b for b in gltf['buffers'])
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(path))
objects=[o for o in bpy.context.scene.objects if o.type=='MESH']
roots=[o for o in bpy.context.scene.objects if 'assembly' in o]
assert len(roots)==13
assert all(len(o.data.vertices)>0 for o in objects)
report={'glb_bytes':len(raw),'round_trip_import':'passed','mesh_objects':len(objects),'assembly_groups':len(roots),'self_contained':True,'features':features,'animation':'static, assembly metadata only'}
(root/'import-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('EXPORT_VALIDATED',json.dumps(report))
