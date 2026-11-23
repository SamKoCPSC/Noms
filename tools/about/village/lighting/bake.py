"""Bake three display-lighting atlases for the kitchen district trial.
Run prepare.py first. Blender: --python bake.py -- OUTPUT [SAMPLES]
The archived sources and the full village are never written.
"""
import bpy, bmesh, json, math, hashlib, sys
from pathlib import Path
from mathutils import Vector
REPO=Path(__file__).resolve().parents[4]
sys.path.insert(0,str(REPO/'assets/models/terracotta-recipe-kitchen/source'))
from finalize_baked_glb import finalize
args=sys.argv[sys.argv.index('--')+1:];OUT=Path(args[0])
bpy.ops.wm.open_mainfile(filepath=str(OUT/'kitchen-district.blend'),load_ui=False,use_scripts=False)
scene=bpy.context.scene;scene.cycles.samples=int(args[1]) if len(args)>1 else 64;scene.cycles.use_denoising=False
meta=json.loads((OUT/'study.json').read_text())
reuse='--reuse-textures' in args
if reuse:
 cached=json.loads((OUT/'bake.json').read_text())
 assert all(cached.get(key)==value for key,value in meta.items()), 'Study changed; bake new textures'
 assert cached['bake_samples']==scene.cycles.samples and cached['atlas_size']==2048
 for label in ('architecture','details','ground'):assert (OUT/(label+'-baked.png')).is_file()

objects=[o for o in scene.objects if o.type=='MESH']
# Save the unchanged local landscape for the before/after browser comparison.
bpy.ops.object.select_all(action='DESELECT')
for o in objects:
 if o['bake_group'] in ('landscape','ground'):o.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(OUT/'district-before.glb'),export_format='GLB',use_selection=True,export_animations=False,export_cameras=False,export_lights=False,export_yup=True)
# Evaluate modifiers and retain procedural coordinate systems before merging.
bpy.ops.object.select_all(action='DESELECT')
for o in objects:o.select_set(True)
bpy.context.view_layer.objects.active=objects[0];bpy.ops.object.convert(target='MESH')
objects=list(bpy.context.selected_objects)
for o in objects:
 mx=o.matrix_world.copy();o.parent=None;o.matrix_world=mx;o.animation_data_clear()
 mesh=o.data
 # These two hand-built open surfaces have inward winding. Cycles renders both
 # sides, but light-map rays otherwise start inside the loaf/lid and bake black.
 if any(name in o.name for name in ('Sourdough · scored baked boule','Pot · domed lid')):
  bm=bmesh.new();bm.from_mesh(mesh)
  assert bm.calc_volume(signed=True)<0
  bmesh.ops.reverse_faces(bm,faces=list(bm.faces));bm.to_mesh(mesh);bm.free()
 for name,values in [('MasterGenerated',[tuple((v.co[i]-mesh.texspace_location[i])/(2*max(mesh.texspace_size[i],1e-8))+.5 for i in range(3))for v in mesh.vertices]),('MasterObject',[tuple(v.co)for v in mesh.vertices])]:
  attr=mesh.attributes.new(name,'FLOAT_VECTOR','POINT')
  for item,value in zip(attr.data,values):item.vector=value
 uv=mesh.uv_layers.active.name if mesh.uv_layers.active else None
 for mat in mesh.materials:
  if not mat or not mat.use_nodes:continue
  tree=mat.node_tree
  for node in list(tree.nodes):
   if node.type=='TEX_COORD':
    for socket,key in [('Generated','MasterGenerated'),('Object','MasterObject')]:
     for link in list(node.outputs[socket].links):
      attr=tree.nodes.new('ShaderNodeAttribute');attr.attribute_name=key;tree.links.new(attr.outputs['Vector'],link.to_socket)
   if node.type.startswith('TEX_') and node.type not in ('TEX_COORD','TEX_IMAGE') and node.inputs.get('Vector') and not node.inputs['Vector'].is_linked:
    attr=tree.nodes.new('ShaderNodeAttribute');attr.attribute_name='MasterGenerated';tree.links.new(attr.outputs['Vector'],node.inputs['Vector'])
   if node.type=='TEX_IMAGE' and uv and not node.inputs['Vector'].is_linked:
    u=tree.nodes.new('ShaderNodeUVMap');u.uv_map=uv;tree.links.new(u.outputs['UV'],node.inputs['Vector'])
   if node.type=='NORMAL_MAP' and uv and not node.uv_map:node.uv_map=uv
# Split reflective/transmitting polygons out; all original geometry is retained.
opaque=[];dynamic=[]
for o in objects:
 kinds=[]
 for mat in o.data.materials:
  p=next((n for n in mat.node_tree.nodes if n.type=='BSDF_PRINCIPLED'),None)if mat and mat.use_nodes else None
  kinds.append(bool(p and (p.inputs['Metallic'].default_value>.3 or p.inputs['Transmission Weight'].default_value>.1)))
 flags={kinds[p.material_index] for p in o.data.polygons}
 if len(flags)==1:(dynamic if next(iter(flags)) else opaque).append(o);continue
 for flag in (False,True):
  copy=o.copy();copy.data=o.data.copy();scene.collection.objects.link(copy)
  bm=bmesh.new();bm.from_mesh(copy.data);bmesh.ops.delete(bm,geom=[f for f in bm.faces if kinds[f.material_index]!=flag],context='FACES');bm.to_mesh(copy.data);bm.free()
  (dynamic if flag else opaque).append(copy)
 bpy.data.objects.remove(o,do_unlink=True)
# Bake the kitchen and its surroundings together, retaining all original
# shaders as bounce-light and shadow contributors until every atlas is done.
groups={key:[] for key in ('architecture','details','ground')}
for o in opaque:
 if o['bake_group'] in groups:groups[o['bake_group']].append(o)
baked=[]
for label,members in groups.items():
 bpy.ops.object.select_all(action='DESELECT')
 for o in members:o.select_set(True)
 bpy.context.view_layer.objects.active=members[0];bpy.ops.object.join();o=bpy.context.object;o.name='Baked district '+label
 bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
 # Keep source UV layers intact; use a new UV layer only for the lighting bake.
 uv=o.data.uv_layers.new(name='BakedLighting');o.data.uv_layers.active=uv
 if label=='ground':
  x0,x1,z0,z1=meta['crop_xz']
  for polygon in o.data.polygons:
   for loop in polygon.loop_indices:
    v=o.data.vertices[o.data.loops[loop].vertex_index].co
    uv.data[loop].uv=((v.x-x0)/(x1-x0),(-v.y-z0)/(z1-z0))
 else:
  bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
  bpy.ops.uv.smart_project(angle_limit=math.radians(66),island_margin=.002,margin_method='FRACTION');bpy.ops.object.mode_set(mode='OBJECT')
 # Dedicated copies prevent another atlas from changing these bake targets.
 for i,m in enumerate(list(o.data.materials)):o.data.materials[i]=m.copy()
 image=bpy.data.images.new('Kitchen '+label+' lighting',width=2048,height=2048,alpha=False,float_buffer=True);image.colorspace_settings.name='Linear Rec.709'
 for m in o.data.materials:
  node=m.node_tree.nodes.new('ShaderNodeTexImage');node.image=image;m.node_tree.nodes.active=node
 scene.render.bake.target='IMAGE_TEXTURES';scene.render.bake.margin=8;scene.render.bake.use_clear=True
 scene.render.bake.use_pass_direct=True;scene.render.bake.use_pass_indirect=True;scene.render.bake.use_pass_color=True
 # Diffuse + emission keeps the original detail and stationary direct/indirect
 # shadows; view-dependent reflective surfaces are exported separately below.
 scene.render.bake.use_pass_diffuse=True;scene.render.bake.use_pass_glossy=False;scene.render.bake.use_pass_transmission=False;scene.render.bake.use_pass_emit=True
 if not reuse:
  print('BAKE_START',label,flush=True)
  bpy.ops.object.bake(type='COMBINED',uv_layer='BakedLighting')
  # Preserve linear HDR pixels and apply the master's AgX look only to the PNG.
  scene.render.image_settings.file_format='OPEN_EXR'
  image.save_render(str(OUT/(label+'-linear.exr')),scene=scene)
  scene.render.image_settings.file_format='PNG'
  image.save_render(str(OUT/(label+'-baked.png')),scene=scene)
 baked.append((o,label))
# Replace only export materials after ALL bakes; original shaders light each other.
for o,label in baked:
 tex=bpy.data.images.load(str(OUT/(label+'-baked.png')));tex.colorspace_settings.name='sRGB';tex.pack()
 mat=bpy.data.materials.new('Baked master lighting '+label);mat.use_nodes=True;mat.node_tree.nodes.clear()
 image=mat.node_tree.nodes.new('ShaderNodeTexImage');image.image=tex
 uv=mat.node_tree.nodes.new('ShaderNodeUVMap');uv.uv_map='BakedLighting';mat.node_tree.links.new(uv.outputs[0],image.inputs['Vector'])
 emission=mat.node_tree.nodes.new('ShaderNodeEmission');mat.node_tree.links.new(image.outputs['Color'],emission.inputs['Color'])
 output=mat.node_tree.nodes.new('ShaderNodeOutputMaterial');mat.node_tree.links.new(emission.outputs[0],output.inputs['Surface'])
 o.data.materials.clear();o.data.materials.append(mat)
 for p in o.data.polygons:p.material_index=0
 # Source UV layers are no longer needed for display-baked opaque geometry.
 for layer in list(o.data.uv_layers):
  if layer.name!='BakedLighting':o.data.uv_layers.remove(layer)
# Batch dynamic surfaces by authored material while preserving transforms.
bpy.ops.object.select_all(action='DESELECT')
for o in dynamic:o.select_set(True)
bpy.context.view_layer.objects.active=dynamic[0];bpy.ops.object.join();live=bpy.context.object;live.name='Live glass and reflective metal'
bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
# Remove procedural links unsupported by glTF only from live material copies.
for i,m in enumerate(list(live.data.materials)):
 copy=m.copy();live.data.materials[i]=copy
 for n in copy.node_tree.nodes:
  if n.type=='BSDF_PRINCIPLED':
   for key in ('Base Color','Roughness','Normal'):
    for link in list(n.inputs[key].links):copy.node_tree.links.remove(link)
bpy.ops.object.select_all(action='DESELECT')
for o,_ in baked:o.select_set(True)
live.select_set(True)
triangles=0
for o in bpy.context.selected_objects:o.data.calc_loop_triangles();triangles+=len(o.data.loop_triangles)
meta.update(triangles=triangles,atlases=3,atlas_size=2048,bake_samples=scene.cycles.samples,animations=False,penguini=True)
bpy.ops.export_scene.gltf(filepath=str(OUT/'district-baked.glb'),export_format='GLB',use_selection=True,export_apply=True,export_animations=False,export_cameras=False,export_lights=False,export_yup=True,export_extras=True)
finalize(OUT/'district-baked.glb', expected_count=3)
meta['glb_bytes']=(OUT/'district-baked.glb').stat().st_size
(OUT/'bake.json').write_text(json.dumps(meta,indent=2)+'\n')
for path,sha in meta['source_hashes'].items():assert hashlib.sha256((REPO/path).read_bytes()).hexdigest()==sha
print('BAKE_COMPLETE',json.dumps(meta),flush=True)
