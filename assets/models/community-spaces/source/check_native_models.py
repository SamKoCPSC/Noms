"""Check packed native masters and keep assembly metadata synchronized with GLBs."""
import bpy,json,struct,math
from pathlib import Path
from mathutils import Vector
R=Path(__file__).resolve().parent.parent
report=[]
for path in sorted((R/'models').glob('*.blend')):
 bpy.ops.wm.open_mainfile(filepath=str(path));s=bpy.context.scene;bpy.context.view_layer.update();groups=[]
 assert s.camera and s.camera.data.type=='ORTHO'
 assert s.render.resolution_x==1600 and s.render.resolution_y==1400 and s.render.resolution_percentage==100
 assert s.cycles.samples==64
 s.render.filepath=f'//../previews/{path.stem}.png'
 if 'README' in bpy.data.texts:
  notes=bpy.data.texts['README'];body=notes.as_string().replace('No baked occlusion shadows.','No baked placement shadows on the walls or floors.');notes.clear();notes.write(body)
 assert not [im.name for im in bpy.data.images if im.source=='FILE' and im.users and not im.packed_file], 'Unpacked texture'
 for root in bpy.data.objects:
  if root.type!='EMPTY' or 'assembly' not in root:continue
  obs=[o for o in root.children_recursive if o.type in ['MESH','CURVE','FONT']];pts=[o.matrix_world@Vector(v)for o in obs for v in o.bound_box]
  assert pts and all(math.isfinite(v)for p in pts for v in p)
  lo=[min(p[i]for p in pts)for i in range(3)];hi=[max(p[i]for p in pts)for i in range(3)];root['pivot']=[(a+b)/2 for a,b in zip(lo,hi)]
  groups.append(dict(name=root.name,assembly=root['assembly'],members=len(obs),bounds=[lo,hi],pivot=list(root['pivot']),incoming_offset=list(root['incoming_offset'])))
 assert len(groups)>=8
 m={'model':s['model_name'],'units':'meters','animation':'metadata only','assemblies':groups}
 path.with_name(path.stem+'-assemblies.json').write_text(json.dumps(m,indent=2))
 bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(path),compress=True)
 glb=path.with_suffix('.glb');data=glb.read_bytes();jlen=struct.unpack_from('<I',data,12)[0];doc=json.loads(data[20:20+jlen]);tail=data[20+jlen:];mapping={g['assembly']:g for g in groups}
 for node in doc['nodes']:
  e=node.get('extras',{});a=e.get('assembly')
  if a in mapping:e['pivot']=mapping[a]['pivot']
 j=json.dumps(doc,separators=(',',':'),ensure_ascii=False).encode();j+=b' '*((-len(j))%4)
 glb.write_bytes(struct.pack('<4sII',b'glTF',2,20+len(j)+len(tail))+struct.pack('<I4s',len(j),b'JSON')+j+tail)
 report.append({'file':path.name,'objects':len(bpy.data.objects),'assembly_groups':len(groups),'packed_texture_images':sum(bool(im.packed_file) for im in bpy.data.images),'camera':'orthographic','render_resolution':[1600,1400],'samples':64,'validation':'PASS'})
 print('NATIVE_CHECK_PASS',path.name,flush=True)
(R/'native-validation.json').write_text(json.dumps(report,indent=2))
