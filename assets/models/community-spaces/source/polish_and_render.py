"""Final review adjustments. Open an existing scene with blender -b scene.blend -P this.py."""
import bpy,sys,math,json
from mathutils import Vector
from pathlib import Path
ROOT_DIR=Path(__file__).resolve().parent.parent
scene=bpy.context.scene
key=Path(bpy.data.filepath).stem
# Correct contrast on light-colored sign faces.
for o in bpy.data.objects:
 if o.type=='FONT' and o.name.endswith('lettering'):
  face=bpy.data.objects.get(o.name.removesuffix('lettering')+'painted face')
  if face and face.data.materials and face.data.materials[0].name in ['ivory','paper','white','linen']:
   o.data.materials.clear();o.data.materials.append(bpy.data.materials.get('ink') or bpy.data.materials['sageDark'])
if key=='farmers-market' and scene.get('revision',1)==1 and not scene.get('egg_stand_added'):
 # Farm eggs are displayed on a small raised stand, never directly on the paving.
 for o in bpy.data.objects:
  if o.name.startswith(('Open egg carton','Free range egg')):o.location.z+=.58
 root=bpy.data.objects['07 Handcart baskets and eggs ROOT'];col=root.users_collection[0]
 def cube(n,loc,dims,ma):
  bpy.ops.mesh.primitive_cube_add(size=1,location=loc);o=bpy.context.object;o.name=n;o.dimensions=dims;bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
  for c in list(o.users_collection):c.objects.unlink(o)
  col.objects.link(o);o.parent=root;o.data.materials.append(bpy.data.materials[ma]);m=o.modifiers.new('Rounded timber','BEVEL');m.width=.012;m.segments=3;o.modifiers.new('Weighted normals','WEIGHTED_NORMAL')
 cube('Farm egg display stand top',(1.98,-.45,.78),(.91,.65,.055),'oak')
 for x in [1.64,2.32]:
  for y in [-.68,-.22]:cube('Farm egg display stand leg',(x,y,.43),(.065,.065,.70),'walnut2')
 scene['egg_stand_added']=True
if key=='night-market':
 bpy.data.objects['Night market welcome lettering'].data.size=.10
 for light in [o for o in bpy.data.objects if o.type=='LIGHT']:
  for coll in light.users_collection:
   roots=[o for o in coll.objects if o.type=='EMPTY' and 'assembly' in o]
   if roots:light.parent=roots[0]
 for prefix in ['Bun kitchen','Noodle kitchen','Sweet kitchen']:
  o=bpy.data.objects[prefix+' menu header'];o.location.y=1.625;o.location.z=2.75;o.dimensions.z=.42
  o=bpy.data.objects[prefix+' sign'];o.location.y=1.556;o.location.z=2.80;o.data.size=.155
  o=bpy.data.objects[prefix+' caption'];o.location.y=1.555;o.location.z=2.635;o.data.size=.058
 # Paper glows gently, retaining the matte shade rather than appearing metallic.
 for base in ['clay','honey']:
  m=bpy.data.materials[base].copy();m.name='Illuminated '+base+' lantern paper';p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Emission Color'].default_value=p.inputs['Base Color'].default_value;p.inputs['Emission Strength'].default_value=.35
  for o in bpy.data.objects:
   if o.name.startswith('Rounded ribbed paper lantern') and o.data.materials[0].name==base:o.data.materials[0]=m
if key=='artisan-bakery' and not scene.get('bread_spacing_refined'):
 for o in bpy.data.objects:
  if o.type=='EMPTY' and o.name.startswith(('Shelf sourdough','Display baguette')):
   i=round((o.location.x-.01)/.73);o.location.x=[.12,.90,1.87,2.44,3.01][i]
  if o.type=='CURVE' and 'opened score' in o.name:
   length=.48 if o.name.startswith('Bread baking') else .52
   for sp in o.data.splines:
    for pt in sp.bezier_points:
     x,y=pt.co.x,pt.co.y;pt.co.z=.12+.14*math.sqrt(max(0,1-(x/(length/2))**2-(y/.16)**2))+.001
   o.data.bevel_depth=.009
 for o in list(bpy.data.objects):
  if o.name.startswith('Bakery wall menu'):bpy.data.objects.remove(o,do_unlink=True)
 root=bpy.data.objects['05 Bread shelves and cooling rack ROOT'];col=root.users_collection[0]
 bpy.ops.mesh.primitive_cube_add(size=1,location=(1.52,2.59,3.035));o=bpy.context.object;o.name='Bread shelf front fascia';o.dimensions=(3.77,.05,.18);bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
 for c in list(o.users_collection):c.objects.unlink(o)
 col.objects.link(o);o.parent=root;o.data.materials.append(bpy.data.materials['walnut2']);m=o.modifiers.new('Rounded fascia','BEVEL');m.width=.012;m.segments=3;o.modifiers.new('Weighted normals','WEIGHTED_NORMAL')
 o=bpy.data.objects['Back shelf bakery name'];o.location=(1.52,2.56,3.02);o.data.size=.095
 scene['bread_spacing_refined']=True
scene.render.resolution_percentage=100;scene.cycles.samples=64;scene.render.threads_mode='FIXED';scene.render.threads=int(sys.argv[sys.argv.index('--render-threads')+1]) if '--render-threads' in sys.argv else 2
scene.render.filepath=f'//../previews/{key}.png'
bpy.context.preferences.filepaths.save_version=0
# Recompute bounds after the review adjustments.
bpy.context.view_layer.update();manifest=[]
for root in list(bpy.data.objects):
 if root.type!='EMPTY' or 'assembly' not in root:continue
 obs=[o for o in root.children_recursive if o.type in ['MESH','CURVE','FONT']];pts=[o.matrix_world@Vector(v)for o in obs for v in o.bound_box]
 if not pts:continue
 lo=[min(p[i]for p in pts)for i in range(3)];hi=[max(p[i]for p in pts)for i in range(3)];root['pivot']=[(a+b)/2 for a,b in zip(lo,hi)]
 manifest.append(dict(name=root.name,members=len(obs),bounds=[lo,hi],pivot=list(root['pivot']),incoming_offset=list(root['incoming_offset'])))
(ROOT_DIR/'models'/f'{key}-assemblies.json').write_text(json.dumps({'model':scene.get('model_name',key),'units':'meters','animation':'metadata only','assemblies':manifest},indent=2))
bpy.ops.object.select_all(action='DESELECT');bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(ROOT_DIR/'models'/f'{key}.blend'),compress=True)
# Export evaluated PBR geometry, with named assembly roots and no studio objects.
bpy.context.view_layer.update();dg=bpy.context.evaluated_depsgraph_get();copies=[];roots=[]
for root in list(bpy.data.objects):
 if root.type!='EMPTY' or 'assembly' not in root:continue
 e=bpy.data.objects.new(root.name+' export',None);scene.collection.objects.link(e)
 for k,v in root.items():e[k]=v
 roots.append(e)
 for src in root.children_recursive:
  if src.type not in ['MESH','CURVE','FONT']:continue
  d=bpy.data.meshes.new_from_object(src.evaluated_get(dg),preserve_all_data_layers=True,depsgraph=dg)
  if not len(d.vertices):continue
  o=bpy.data.objects.new(src.name+' export',d);scene.collection.objects.link(o);o.parent=e;o.matrix_world=src.matrix_world.copy();copies.append(o)
bpy.ops.object.select_all(action='DESELECT')
for o in copies+roots:o.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(ROOT_DIR/'models'/f'{key}.glb'),export_format='GLB',use_selection=True,export_apply=False,export_cameras=False,export_lights=False,export_animations=False,export_extras=True,export_yup=True)
for o in copies+roots:bpy.data.objects.remove(o,do_unlink=True)
print('FINAL_EXPORT_READY',key,flush=True)
bpy.ops.render.render(write_still=True);print('FINAL_RENDER_DONE',key,flush=True)
