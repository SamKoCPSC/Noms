"""Bake one full-village room under the shared bright daylight settings.
Blender --python bake-room.py -- MODEL DIRECTORY
"""
import bpy, bmesh, json, math, hashlib, sys
from pathlib import Path
from mathutils import Vector, Matrix
sys.path.insert(0,str(Path(__file__).resolve().parent))
from common import *
sys.path.insert(0,str(REPO/'assets/models/terracotta-recipe-kitchen/source'))
from finalize_baked_glb import finalize
MODEL,directory=sys.argv[sys.argv.index('--')+1:sys.argv.index('--')+3];OUT=Path(directory)
bpy.ops.wm.open_mainfile(filepath=str(OUT/'scene.blend'),load_ui=False,use_scripts=False)
scene=bpy.context.scene;camera=scene.camera;scene.cycles.samples=CONFIG['room_samples'];scene.cycles.use_denoising=False
meta=json.loads((OUT/'metadata.json').read_text());config={'master':True};reuse_lighting=False
objects=[o for o in scene.objects if o.type=='MESH' and 'source_object' in o]
assert objects
# Evaluate modifiers and retain procedural coordinate systems before merging.
bpy.ops.object.select_all(action='DESELECT')
for o in objects:o.select_set(True)
bpy.context.view_layer.objects.active=objects[0]
if any(o.type!='MESH' or o.modifiers for o in objects):bpy.ops.object.convert(target='MESH')
objects=list(bpy.context.selected_objects)
for o in objects:
 mx=o.matrix_world.copy();o.parent=None;o.matrix_world=mx;o.animation_data_clear()
 o.data=o.data.copy();mesh=o.data
 if not mesh.polygons:continue
 # Some authored parametric spheres/lathes have inward winding. Cycles
 # renders both sides, but bake rays otherwise start inside their dark interior.
 # Repair only the derivative; retain source geometry and the archived master.
 if 'master' in config:
  bm=bmesh.new();bm.from_mesh(mesh);bm.normal_update()
  lo=Vector(tuple(min(v.co[i] for v in bm.verts) for i in range(3)))
  hi=Vector(tuple(max(v.co[i] for v in bm.verts) for i in range(3)))
  spans=hi-lo;planar=min(spans)<max(spans)*1e-6
  if planar:
   # Shallow painted relief is two-sided in Cycles' camera render. Bake its
   # visible face, rather than sending lighting rays into the backing canvas.
   normal=sum((f.normal*f.calc_area() for f in bm.faces),Vector())
   normal=o.matrix_world.to_3x3().inverted().transposed()@normal
   center=o.matrix_world@((lo+hi)/2)
   reverse=normal.dot(camera.location-center)<0
   repair='front_facing_relief_repairs'
  else:
   # Signed volume only identifies inward CLOSED shells. Weld coincident
   # UV seams in a temporary probe, leaving the export mesh untouched.
   # Open folded foliage and uncapped arches have no meaningful volume.
   probe=bm.copy()
   bmesh.ops.remove_doubles(probe,verts=list(probe.verts),dist=1e-6)
   bmesh.ops.dissolve_degenerate(probe,edges=list(probe.edges),dist=1e-8)
   probe.normal_update()
   closed=not any(e.is_boundary for e in probe.edges)
   center=sum((v.co for v in probe.verts),Vector())/len(probe.verts)
   bmesh.ops.translate(probe,vec=-center,verts=list(probe.verts))
   reverse=closed and probe.calc_volume(signed=True)<-1e-8;probe.free()
   repair='outward_winding_repairs'
  if reverse:
   bmesh.ops.reverse_faces(bm,faces=list(bm.faces));bm.to_mesh(mesh)
   if mesh.has_custom_normals:mesh.normals_split_custom_set([(0.,0.,0.)]*len(mesh.loops))
   meta.setdefault(repair,[]).append(o.name)
  bm.free()
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
 if not o.data.polygons:continue
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
large=[o for o in opaque if o['bake_group']=='architecture']
small=[o for o in opaque if o['bake_group']=='details']
baked=[]
for label,members in [('architecture',large),('details',small)]:
 bpy.ops.object.select_all(action='DESELECT')
 for o in members:o.select_set(True)
 bpy.context.view_layer.objects.active=members[0];bpy.ops.object.join();o=bpy.context.object;o.name='Baked '+MODEL+' '+label
 bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
 # Keep source UV layers intact; use a new UV layer only for the lighting bake.
 uv=o.data.uv_layers.new(name='BakedLighting');o.data.uv_layers.active=uv
 bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
 bpy.ops.uv.smart_project(angle_limit=math.radians(66),island_margin=.001,margin_method='FRACTION');bpy.ops.object.mode_set(mode='OBJECT')
 # Dedicated copies prevent another atlas from changing these bake targets.
 for i,m in enumerate(list(o.data.materials)):o.data.materials[i]=m.copy()
 image=bpy.data.images.new(MODEL+' '+label+' lighting',width=2048,height=2048,alpha=False,float_buffer=True);image.colorspace_settings.name='Linear Rec.709'
 for m in o.data.materials:
  node=m.node_tree.nodes.new('ShaderNodeTexImage');node.image=image;m.node_tree.nodes.active=node
 scene.render.bake.target='IMAGE_TEXTURES';scene.render.bake.margin=8;scene.render.bake.use_clear=True
 scene.render.bake.use_pass_direct=True;scene.render.bake.use_pass_indirect=True;scene.render.bake.use_pass_color=True
 # Diffuse + emission keeps the original detail and stationary direct/indirect
 # shadows; view-dependent reflective surfaces are exported separately below.
 scene.render.bake.use_pass_diffuse=True;scene.render.bake.use_pass_glossy=False;scene.render.bake.use_pass_transmission=False;scene.render.bake.use_pass_emit=True
 print('REUSE_LIGHTING' if reuse_lighting else 'BAKE_START',label,flush=True)
 if not reuse_lighting:bpy.ops.object.bake(type='COMBINED',uv_layer='BakedLighting')
 # Preserve linear HDR pixels and apply the master's AgX look only to the PNG.
 scene.render.image_settings.file_format='OPEN_EXR'
 if not reuse_lighting:image.save_render(str(OUT/(label+'-linear.exr')),scene=scene)
 scene.render.image_settings.file_format='PNG'
 if not reuse_lighting:image.save_render(str(OUT/(label+'-baked.png')),scene=scene)
 baked.append((o,label))
# Replace only export materials after BOTH bakes; original shaders light each other.
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
 for attr in list(o.data.attributes):
  if attr.name in ('MasterGenerated','MasterObject') or attr.data_type in ('FLOAT_COLOR','BYTE_COLOR'):o.data.attributes.remove(attr)
# Batch dynamic surfaces by authored material while preserving transforms.
bpy.ops.object.select_all(action='DESELECT')
for o in dynamic:o.select_set(True)
assert dynamic, 'Expected glass or metal in source'
bpy.context.view_layer.objects.active=dynamic[0];bpy.ops.object.join();live=bpy.context.object;live.name='Live glass and reflective metal'
bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
# Remove procedural links unsupported by glTF only from live material copies.
for i,m in enumerate(list(live.data.materials)):
 copy=m.copy();live.data.materials[i]=copy
 for n in copy.node_tree.nodes:
  if n.type=='BSDF_PRINCIPLED':
   for key in ('Base Color','Roughness','Normal'):
    for link in list(n.inputs[key].links):
     # glTF supports image textures and vertex colors. Recovered glass/metal
     # rely on vertex tint, so retain those links instead of exporting gray.
     if key=='Base Color' and link.from_node.type in ('VERTEX_COLOR','TEX_IMAGE'):continue
     copy.node_tree.links.remove(link)
bpy.ops.object.select_all(action='DESELECT')
for o,_ in baked:o.select_set(True)
live.select_set(True)
triangles=0
for o in bpy.context.selected_objects:o.data.calc_loop_triangles();triangles+=len(o.data.loop_triangles)
meta.update(triangles=triangles,atlases=2,atlas_size=2048,bake_samples=scene.cycles.samples,animations=False,penguini=True,world_coordinates=True)
bpy.ops.export_scene.gltf(filepath=str(OUT/(MODEL+'-baked.glb')),export_format='GLB',use_selection=True,export_apply=True,export_animations=False,export_cameras=False,export_lights=False,export_yup=True,export_extras=True)
finalize(OUT/(MODEL+'-baked.glb'))
meta['glb_bytes']=(OUT/(MODEL+'-baked.glb')).stat().st_size
(OUT/'bake.json').write_text(json.dumps(meta,indent=2)+'\n')
for path,sha in meta['source_hashes'].items():assert hashlib.sha256((REPO/path).read_bytes()).hexdigest()==sha
print('BAKE_COMPLETE',json.dumps(meta),flush=True)
